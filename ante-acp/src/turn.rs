//! Translating a session's host events into client updates and prompt outcomes.

use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, CurrentModeUpdate, SessionUpdate, StopReason, TextContent,
};
use agent_client_protocol::{Error, ErrorCode};
use ante_sdk::protocol::{Evt, TurnEndStatus};
use serde_json::json;

use crate::session::mode_id;

/// What one host event means to the client.
pub enum Out {
    /// A `session/update` notification to send.
    Update(Box<SessionUpdate>),
    /// A turn began; prompts from now on steer it.
    TurnStarted,
    /// The turn ended: the answer to every prompt request that rode it.
    TurnEnded(Result<StopReason, Error>),
}

/// Per-session translation state. The host streams deltas and then repeats
/// the full text in a final event; a stream that was delivered as deltas
/// has its final event dropped.
#[derive(Default)]
pub struct Translator {
    streamed_message: bool,
    streamed_thinking: bool,
}

impl Translator {
    pub fn handle(&mut self, event: Evt) -> Option<Out> {
        match event {
            Evt::TurnStart { .. } => {
                *self = Self::default();
                Some(Out::TurnStarted)
            }
            Evt::TurnEnd { status, .. } => Some(Out::TurnEnded(outcome(status))),
            Evt::MessageDelta(text) => {
                self.streamed_message |= !text.is_empty();
                message(text)
            }
            Evt::AgentMessage(text) => {
                (!std::mem::take(&mut self.streamed_message)).then(|| message(text)).flatten()
            }
            Evt::ThinkingDelta(text) => {
                self.streamed_thinking |= !text.is_empty();
                thought(text)
            }
            Evt::Thinking(text) => {
                (!std::mem::take(&mut self.streamed_thinking)).then(|| thought(text)).flatten()
            }
            Evt::Info(text)
            | Evt::InfoBlockStart { header: text, .. }
            | Evt::InfoBlockAppend { detail: text, .. } => thought(text),
            Evt::Error(text) => message(format!("Error: {text}")),
            Evt::SessionUpdated(info) => {
                Some(Out::Update(Box::new(SessionUpdate::CurrentModeUpdate(
                    CurrentModeUpdate::new(mode_id(info.permission_mode)),
                ))))
            }
            _ => None,
        }
    }
}

fn message(text: String) -> Option<Out> {
    (!text.is_empty()).then(|| Out::Update(Box::new(SessionUpdate::AgentMessageChunk(chunk(text)))))
}

fn thought(text: String) -> Option<Out> {
    (!text.is_empty()).then(|| Out::Update(Box::new(SessionUpdate::AgentThoughtChunk(chunk(text)))))
}

fn chunk(text: String) -> ContentChunk {
    ContentChunk::new(ContentBlock::Text(TextContent::new(text)))
}

/// A finished turn as the client sees it. ACP has no error stop reason, so
/// a failed turn is a JSON-RPC error; a dead sign-in is `auth_required`.
fn outcome(status: TurnEndStatus) -> Result<StopReason, Error> {
    match status {
        TurnEndStatus::Completed => Ok(StopReason::EndTurn),
        TurnEndStatus::Interrupted { .. } => Ok(StopReason::Cancelled),
        TurnEndStatus::Error { kind, headline, details } => {
            let (code, message) = match kind.as_deref() {
                Some("oauth") => (
                    ErrorCode::AuthRequired,
                    format!("{headline}; run `ante auth login` in a terminal"),
                ),
                _ => (ErrorCode::InternalError, headline),
            };
            Err(Error::new(code.into(), message).data(json!({ "kind": kind, "details": details })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante_sdk::protocol::{Id, ModelSpec, PermissionMode, ProviderSpec, SessionInfo};

    /// The text of a message (`false`) or thought (`true`) chunk.
    fn chunk_text(out: Option<Out>) -> Option<(bool, String)> {
        let Out::Update(update) = out? else { return None };
        let (thought, chunk) = match *update {
            SessionUpdate::AgentMessageChunk(chunk) => (false, chunk),
            SessionUpdate::AgentThoughtChunk(chunk) => (true, chunk),
            _ => return None,
        };
        match chunk.content {
            ContentBlock::Text(text) => Some((thought, text.text)),
            _ => None,
        }
    }

    fn turn_end(status: TurnEndStatus) -> Evt {
        Evt::TurnEnd { turn_id: Id::new("turn"), status, steps: 1 }
    }

    #[test]
    fn streamed_text_is_not_repeated_by_its_final_event() {
        let mut translator = Translator::default();
        assert!(matches!(
            translator.handle(Evt::TurnStart { turn_id: Id::new("turn") }),
            Some(Out::TurnStarted)
        ));

        assert_eq!(
            chunk_text(translator.handle(Evt::MessageDelta("po".into()))),
            Some((false, "po".into()))
        );
        assert_eq!(
            chunk_text(translator.handle(Evt::MessageDelta("ng".into()))),
            Some((false, "ng".into()))
        );
        assert!(translator.handle(Evt::AgentMessage("pong".into())).is_none());

        assert_eq!(
            chunk_text(translator.handle(Evt::ThinkingDelta("hm".into()))),
            Some((true, "hm".into()))
        );
        assert!(translator.handle(Evt::Thinking("hm".into())).is_none());
    }

    #[test]
    fn unstreamed_final_text_is_delivered() {
        let mut translator = Translator::default();
        // An empty delta is not a delivery.
        assert!(translator.handle(Evt::MessageDelta(String::new())).is_none());
        assert_eq!(
            chunk_text(translator.handle(Evt::AgentMessage("pong".into()))),
            Some((false, "pong".into()))
        );
        assert_eq!(
            chunk_text(translator.handle(Evt::Thinking("hm".into()))),
            Some((true, "hm".into()))
        );
        assert!(translator.handle(Evt::AgentMessage(String::new())).is_none());
    }

    #[test]
    fn turn_end_maps_to_stop_reasons_and_errors() {
        let mut translator = Translator::default();

        let Some(Out::TurnEnded(Ok(reason))) =
            translator.handle(turn_end(TurnEndStatus::Completed))
        else {
            panic!("expected a completed turn");
        };
        assert_eq!(reason, StopReason::EndTurn);

        let Some(Out::TurnEnded(Ok(reason))) =
            translator.handle(turn_end(TurnEndStatus::Interrupted { reason: None }))
        else {
            panic!("expected a cancelled turn");
        };
        assert_eq!(reason, StopReason::Cancelled);

        let expired = TurnEndStatus::Error {
            kind: Some("oauth".into()),
            headline: "sign-in expired".into(),
            details: vec!["HTTP 401".into()],
        };
        let Some(Out::TurnEnded(Err(error))) = translator.handle(turn_end(expired)) else {
            panic!("expected a failed turn");
        };
        assert_eq!(error.code, ErrorCode::AuthRequired);
        assert!(
            error.message.contains("sign-in expired") && error.message.contains("ante auth login")
        );
        assert_eq!(error.data.unwrap()["details"][0], "HTTP 401");

        let limited = TurnEndStatus::Error {
            kind: Some("rate_limited".into()),
            headline: "rate limited".into(),
            details: vec![],
        };
        let Some(Out::TurnEnded(Err(error))) = translator.handle(turn_end(limited)) else {
            panic!("expected a failed turn");
        };
        assert_eq!(error.code, ErrorCode::InternalError);
        assert_eq!(error.message, "rate limited");
    }

    #[test]
    fn notices_become_thought_or_error_chunks() {
        let mut translator = Translator::default();
        assert_eq!(
            chunk_text(translator.handle(Evt::Info("settings notice".into()))),
            Some((true, "settings notice".into()))
        );
        assert_eq!(
            chunk_text(translator.handle(Evt::InfoBlockStart {
                id: "mcp".into(),
                header: "Connecting MCP".into(),
                loading: true
            })),
            Some((true, "Connecting MCP".into()))
        );
        assert_eq!(
            chunk_text(translator.handle(Evt::Error("boom".into()))),
            Some((false, "Error: boom".into()))
        );
        assert!(translator.handle(Evt::CompactStart).is_none());
    }

    #[test]
    fn a_mode_change_becomes_a_current_mode_update() {
        let info = SessionInfo {
            model: ModelSpec::default(),
            provider: ProviderSpec {
                id: "test".into(),
                display_name: "Test".into(),
                base_url: String::new(),
            },
            session_id: Id::ses(),
            cwd: "/work".into(),
            permission_mode: PermissionMode::Yolo,
            skills: Vec::new(),
            subagents: Vec::new(),
            title: None,
        };
        let Some(Out::Update(update)) =
            Translator::default().handle(Evt::SessionUpdated(Box::new(info)))
        else {
            panic!("expected an update");
        };
        let SessionUpdate::CurrentModeUpdate(update) = *update else {
            panic!("expected a mode update");
        };
        assert_eq!(&*update.current_mode_id.0, "yolo");
    }
}
