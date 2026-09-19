//! One ACP session is one `ante serve --stdio` child, driven through the SDK.

use std::path::PathBuf;
use std::time::Duration;

use agent_client_protocol::schema::v1::{SessionMode, SessionModeState};
use ante_sdk::protocol::{Evt, Op, PermissionMode, SessionRequest};
use ante_sdk::{Client, EventReceiver, OpSender};
use anyhow::{Context, Result, bail};
use tracing::debug;

/// How long a fresh host may take to announce its session.
const START_TIMEOUT: Duration = Duration::from_secs(30);

/// A session the host has announced.
pub struct Started {
    pub id: String,
    pub mode: PermissionMode,
    pub ops: OpSender,
    /// The host's remaining events, for the caller to pump to the client.
    pub events: EventReceiver,
}

/// Start a session in `cwd` on a freshly connected host and wait for its
/// announcement.
pub async fn start(client: Client, cwd: PathBuf) -> Result<Started> {
    let request = SessionRequest {
        cwd: Some(cwd),
        // Nothing can answer a question yet, so the tool stays out.
        exclude_tools: Some(vec!["AskUser".to_string()]),
        // Nothing can answer an approval yet either: a call that would pause
        // for one is denied instead, until permission requests land.
        unattended: Some(true),
        ..Default::default()
    };
    client.send(Op::StartSession(request)).await.context("host connection closed")?;
    let (ops, mut events) = client.into_parts();
    let info = tokio::time::timeout(START_TIMEOUT, async {
        while let Some(msg) = events.recv().await {
            match msg.event {
                Evt::SessionStart(info) => return Ok(info),
                Evt::Error(message) => bail!("{message}"),
                other => debug!(?other, "event before the session started"),
            }
        }
        bail!("host closed before announcing the session")
    })
    .await
    .context("host did not announce the session in time")??;
    Ok(Started { id: info.session_id.to_string(), mode: info.permission_mode, ops, events })
}

/// Ante's permission modes, offered to the client as session modes.
pub fn mode_state(current: PermissionMode) -> SessionModeState {
    let modes = [PermissionMode::Strict, PermissionMode::Auto, PermissionMode::Yolo]
        .into_iter()
        .map(|mode| {
            let (name, description) = match mode {
                PermissionMode::Strict => {
                    ("Strict", "Ask before a tool call unless it is provably safe")
                }
                PermissionMode::Auto => ("Auto", "Run a tool call unless it is provably dangerous"),
                PermissionMode::Yolo => {
                    ("Yolo", "Bypass every permission check, including deny rules")
                }
            };
            SessionMode::new(mode_id(mode), name).description(description)
        })
        .collect();
    SessionModeState::new(mode_id(current), modes)
}

pub fn mode_id(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Strict => "strict",
        PermissionMode::Auto => "auto",
        PermissionMode::Yolo => "yolo",
    }
}

pub fn parse_mode(id: &str) -> Option<PermissionMode> {
    match id {
        "strict" => Some(PermissionMode::Strict),
        "auto" => Some(PermissionMode::Auto),
        "yolo" => Some(PermissionMode::Yolo),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante_sdk::protocol::{
        EventMsg, Id, ModelSpec, OpMsg, ProviderSpec, SessionInfo, event_msg,
    };
    use tokio::sync::mpsc::{Receiver, UnboundedSender};

    /// A client whose host end the test holds.
    fn fake_host() -> (Client, Receiver<OpMsg>, UnboundedSender<EventMsg>) {
        let (op_tx, op_rx) = tokio::sync::mpsc::channel(8);
        let (evt_tx, evt_rx) = tokio::sync::mpsc::unbounded_channel();
        (Client::from_parts(OpSender::new(op_tx), evt_rx), op_rx, evt_tx)
    }

    fn session_info(permission_mode: PermissionMode) -> SessionInfo {
        SessionInfo {
            model: ModelSpec::default(),
            provider: ProviderSpec {
                id: "test".into(),
                display_name: "Test".into(),
                base_url: String::new(),
            },
            session_id: Id::ses(),
            cwd: PathBuf::from("/work"),
            permission_mode,
            skills: Vec::new(),
            subagents: Vec::new(),
            title: None,
        }
    }

    #[tokio::test]
    async fn start_requests_the_session_and_returns_its_announcement() {
        let (client, mut ops, events) = fake_host();
        let started = tokio::spawn(start(client, PathBuf::from("/work")));

        let Op::StartSession(request) = ops.recv().await.expect("an op").op else {
            panic!("expected StartSession");
        };
        assert_eq!(request.cwd, Some(PathBuf::from("/work")));
        assert_eq!(request.exclude_tools, Some(vec!["AskUser".to_string()]));

        events.send(event_msg(Evt::Info("settings notice".into()), None)).unwrap();
        let info = session_info(PermissionMode::Auto);
        let id = info.session_id.to_string();
        events.send(event_msg(Evt::SessionStart(Box::new(info)), None)).unwrap();

        let started = started.await.unwrap().expect("session starts");
        assert_eq!(started.id, id);
        assert_eq!(started.mode, PermissionMode::Auto);
    }

    #[tokio::test]
    async fn start_reports_the_hosts_error() {
        let (client, _ops, events) = fake_host();
        events.send(event_msg(Evt::Error("no provider configured".into()), None)).unwrap();

        let error = start(client, PathBuf::from("/work")).await.err().expect("start fails");
        assert!(error.to_string().contains("no provider configured"), "{error:#}");
    }

    #[tokio::test]
    async fn start_fails_when_the_host_goes_away() {
        let (client, _ops, events) = fake_host();
        drop(events);

        let error = start(client, PathBuf::from("/work")).await.err().expect("start fails");
        assert!(error.to_string().contains("host closed"), "{error:#}");
    }

    #[test]
    fn modes_round_trip_and_reject_unknown_ids() {
        for mode in [PermissionMode::Strict, PermissionMode::Auto, PermissionMode::Yolo] {
            assert_eq!(parse_mode(mode_id(mode)), Some(mode));
        }
        assert_eq!(parse_mode("architect"), None);

        let state = mode_state(PermissionMode::Yolo);
        assert_eq!(&*state.current_mode_id.0, "yolo");
        assert_eq!(state.available_modes.len(), 3);
    }
}
