//! Typed messages from Claude Code's streaming protocol.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeMessage {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Result(ResultMessage),
    StreamEvent(StreamEventMessage),
    ControlRequest(ControlRequestMessage),
    ControlResponse(ControlResponseMessage),
    Other(Value),
}

impl ClaudeMessage {
    pub fn raw(&self) -> &Value {
        match self {
            Self::User(message) => &message.raw,
            Self::Assistant(message) => &message.raw,
            Self::System(message) => &message.raw,
            Self::Result(message) => &message.raw,
            Self::StreamEvent(message) => &message.raw,
            Self::ControlRequest(message) => &message.raw,
            Self::ControlResponse(message) => &message.raw,
            Self::Other(raw) => raw,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurnResponse {
    pub messages: Vec<ClaudeMessage>,
}

impl TurnResponse {
    pub fn result(&self) -> Option<&ResultMessage> {
        self.messages.iter().rev().find_map(|message| match message {
            ClaudeMessage::Result(result) => Some(result),
            _ => None,
        })
    }

    pub fn assistant_text(&self) -> Option<String> {
        let texts: Vec<String> = self
            .messages
            .iter()
            .filter_map(|message| match message {
                ClaudeMessage::Assistant(message) => message.text(),
                _ => None,
            })
            .collect();

        if texts.is_empty() { None } else { Some(texts.join("\n\n")) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserMessage {
    pub content: MessageContent,
    pub uuid: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub tool_use_result: Option<Value>,
    pub raw: Value,
}

impl UserMessage {
    pub fn text(&self) -> Option<String> {
        self.content.text()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub model: Option<String>,
    pub parent_tool_use_id: Option<String>,
    pub error: Option<String>,
    pub raw: Value,
}

impl AssistantMessage {
    pub fn text(&self) -> Option<String> {
        let texts: Vec<&str> = self.content.iter().filter_map(ContentBlock::text).collect();

        if texts.is_empty() { None } else { Some(texts.join("\n")) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemMessage {
    pub subtype: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultMessage {
    pub subtype: Option<String>,
    pub duration_ms: Option<f64>,
    pub duration_api_ms: Option<f64>,
    pub is_error: Option<bool>,
    pub num_turns: Option<u64>,
    pub session_id: Option<String>,
    pub total_cost_usd: Option<f64>,
    pub usage: Option<Value>,
    pub result: Option<Value>,
    pub structured_output: Option<Value>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamEventMessage {
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    pub event: Option<Value>,
    pub parent_tool_use_id: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlRequestMessage {
    pub request_id: Option<String>,
    pub subtype: Option<String>,
    pub request: Option<Value>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlResponseMessage {
    pub request_id: Option<String>,
    pub subtype: Option<String>,
    pub response: Option<Value>,
    pub error: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
    Other(Value),
}

impl MessageContent {
    pub fn text(&self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text.clone()),
            Self::Blocks(blocks) => {
                let texts: Vec<&str> = blocks.iter().filter_map(ContentBlock::text).collect();
                if texts.is_empty() { None } else { Some(texts.join("\n")) }
            }
            Self::Other(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
    Other(Value),
}

impl ContentBlock {
    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(block) => Some(&block.text),
            Self::Thinking(_) | Self::ToolUse(_) | Self::ToolResult(_) | Self::Other(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    pub text: String,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: String,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    pub content: Option<Value>,
    pub is_error: Option<bool>,
    pub raw: Value,
}

pub fn parse_message(raw: Value) -> ClaudeMessage {
    match raw.get("type").and_then(Value::as_str) {
        Some("user") => ClaudeMessage::User(UserMessage {
            content: raw
                .get("message")
                .and_then(|message| message.get("content"))
                .cloned()
                .map(parse_message_content)
                .unwrap_or(MessageContent::Other(Value::Null)),
            uuid: raw.get("uuid").and_then(Value::as_str).map(ToOwned::to_owned),
            parent_tool_use_id: raw
                .get("parent_tool_use_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            tool_use_result: raw.get("tool_use_result").cloned(),
            raw,
        }),
        Some("assistant") => ClaudeMessage::Assistant(AssistantMessage {
            content: raw
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(Value::as_array)
                .map(|blocks| blocks.iter().cloned().map(parse_content_block).collect())
                .unwrap_or_default(),
            model: raw
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            parent_tool_use_id: raw
                .get("parent_tool_use_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            error: raw.get("error").and_then(Value::as_str).map(ToOwned::to_owned),
            raw,
        }),
        Some("system") => ClaudeMessage::System(SystemMessage {
            subtype: raw.get("subtype").and_then(Value::as_str).map(ToOwned::to_owned),
            raw,
        }),
        Some("result") => ClaudeMessage::Result(ResultMessage {
            subtype: raw.get("subtype").and_then(Value::as_str).map(ToOwned::to_owned),
            duration_ms: raw.get("duration_ms").and_then(Value::as_f64),
            duration_api_ms: raw.get("duration_api_ms").and_then(Value::as_f64),
            is_error: raw.get("is_error").and_then(Value::as_bool),
            num_turns: raw.get("num_turns").and_then(Value::as_u64),
            session_id: raw.get("session_id").and_then(Value::as_str).map(ToOwned::to_owned),
            total_cost_usd: raw.get("total_cost_usd").and_then(Value::as_f64),
            usage: raw.get("usage").cloned(),
            result: raw.get("result").cloned(),
            structured_output: raw.get("structured_output").cloned(),
            raw,
        }),
        Some("stream_event") => ClaudeMessage::StreamEvent(StreamEventMessage {
            uuid: raw.get("uuid").and_then(Value::as_str).map(ToOwned::to_owned),
            session_id: raw.get("session_id").and_then(Value::as_str).map(ToOwned::to_owned),
            event: raw.get("event").cloned(),
            parent_tool_use_id: raw
                .get("parent_tool_use_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            raw,
        }),
        Some("control_request") => ClaudeMessage::ControlRequest(ControlRequestMessage {
            request_id: raw.get("request_id").and_then(Value::as_str).map(ToOwned::to_owned),
            subtype: raw
                .get("request")
                .and_then(|request| request.get("subtype"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            request: raw.get("request").cloned(),
            raw,
        }),
        Some("control_response") => {
            let response = raw.get("response");
            ClaudeMessage::ControlResponse(ControlResponseMessage {
                request_id: response
                    .and_then(|value| value.get("request_id"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                subtype: response
                    .and_then(|value| value.get("subtype"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                response: response.and_then(|value| value.get("response")).cloned(),
                error: response
                    .and_then(|value| value.get("error"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                raw,
            })
        }
        _ => ClaudeMessage::Other(raw),
    }
}

fn parse_message_content(raw: Value) -> MessageContent {
    match raw {
        Value::String(text) => MessageContent::Text(text),
        Value::Array(blocks) => {
            MessageContent::Blocks(blocks.into_iter().map(parse_content_block).collect())
        }
        other => MessageContent::Other(other),
    }
}

fn parse_content_block(raw: Value) -> ContentBlock {
    match raw.get("type").and_then(Value::as_str) {
        Some("text") => {
            let Some(text) = raw.get("text").and_then(Value::as_str) else {
                return ContentBlock::Other(raw);
            };

            ContentBlock::Text(TextBlock { text: text.to_string(), raw })
        }
        Some("thinking") => match (
            raw.get("thinking").and_then(Value::as_str),
            raw.get("signature").and_then(Value::as_str),
        ) {
            (Some(thinking), Some(signature)) => ContentBlock::Thinking(ThinkingBlock {
                thinking: thinking.to_string(),
                signature: signature.to_string(),
                raw,
            }),
            _ => ContentBlock::Other(raw),
        },
        Some("tool_use") => match (
            raw.get("id").and_then(Value::as_str),
            raw.get("name").and_then(Value::as_str),
            raw.get("input").cloned(),
        ) {
            (Some(id), Some(name), Some(input)) => ContentBlock::ToolUse(ToolUseBlock {
                id: id.to_string(),
                name: name.to_string(),
                input,
                raw,
            }),
            _ => ContentBlock::Other(raw),
        },
        Some("tool_result") => {
            let Some(tool_use_id) = raw.get("tool_use_id").and_then(Value::as_str) else {
                return ContentBlock::Other(raw);
            };

            ContentBlock::ToolResult(ToolResultBlock {
                tool_use_id: tool_use_id.to_string(),
                content: raw.get("content").cloned(),
                is_error: raw.get("is_error").and_then(Value::as_bool),
                raw,
            })
        }
        _ => ContentBlock::Other(raw),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_assistant_message_extracts_typed_blocks() {
        let message = parse_message(json!({
            "type": "assistant",
            "message": {
                "model": "claude-sonnet-4-5",
                "content": [
                    { "type": "text", "text": "hello" },
                    { "type": "tool_use", "id": "toolu_1", "name": "Read", "input": { "file": "README.md" } },
                    { "type": "text", "text": "world" }
                ]
            }
        }));

        match message {
            ClaudeMessage::Assistant(message) => {
                assert_eq!(message.model.as_deref(), Some("claude-sonnet-4-5"));
                assert_eq!(message.text(), Some("hello\nworld".to_string()));
                assert_eq!(message.content.len(), 3);
            }
            other => panic!("expected assistant message, got {other:?}"),
        }
    }

    #[test]
    fn parse_result_message_extracts_usage_fields() {
        let message = parse_message(json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1234.0,
            "duration_api_ms": 900.0,
            "is_error": false,
            "num_turns": 2,
            "session_id": "session-123",
            "total_cost_usd": 0.42,
            "result": "done"
        }));

        match message {
            ClaudeMessage::Result(result) => {
                assert_eq!(
                    result,
                    ResultMessage {
                        subtype: Some("success".to_string()),
                        duration_ms: Some(1234.0),
                        duration_api_ms: Some(900.0),
                        is_error: Some(false),
                        num_turns: Some(2),
                        session_id: Some("session-123".to_string()),
                        total_cost_usd: Some(0.42),
                        usage: None,
                        result: Some(json!("done")),
                        structured_output: None,
                        raw: json!({
                            "type": "result",
                            "subtype": "success",
                            "duration_ms": 1234.0,
                            "duration_api_ms": 900.0,
                            "is_error": false,
                            "num_turns": 2,
                            "session_id": "session-123",
                            "total_cost_usd": 0.42,
                            "result": "done"
                        }),
                    }
                );
            }
            other => panic!("expected result message, got {other:?}"),
        }
    }
}
