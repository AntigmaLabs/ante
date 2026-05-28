use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMsg {
    pub timestamp: DateTime<Utc>,
    pub id: Id,
    pub event: Evt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpMsg {
    pub op: Op,
    pub id: Id,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Op {
    StartSession(SessionConfig),
    UpdateSession(SessionUpdate),
    Interrupt,
    UserInput(String),
    Steer(String),
    ApprovalResponse {
        turn_id: Id,
        responses: Vec<(String, ReviewDecision)>,
    },
    SlashCommand {
        name: String,
        args: String,
    },
    ResumeSession {
        session_id: Id,
    },
    RegisterLocalProvider {
        port: u16,
        model: Option<ModelSpec>,
    },
    RestoreLocalProvider,
    /// Manually trigger conversation compaction on the active session.
    Compact,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evt {
    SessionStart(Box<SessionInitialized>),
    SessionUpdated(Box<SessionInitialized>),
    ExtensionRefreshed(Box<ExtensionRefreshed>),
    SessionEnd,
    UserInput(String),
    AgentMessage(String),
    Thinking(String),
    MessageDelta(String),
    ThinkingDelta(String),
    Info(String),
    /// Open a grouped Info entry with `header`. Subsequent
    /// `InfoBlockAppend` events with the same `id` are rendered as
    /// tree-indented child lines under it. Use for multi-step background
    /// notifications (e.g. MCP warm-up) that should visually cluster.
    ///
    /// When `loading` is true the renderer appends an animated `.`/`../...`
    /// suffix to the header until the first `InfoBlockAppend` arrives,
    /// signalling that background work is still in flight.
    InfoBlockStart {
        id: String,
        header: String,
        #[serde(default)]
        loading: bool,
    },
    /// Append a child detail line to the `InfoBlockStart` with the same `id`.
    /// Drops silently if the matching block isn't present.
    InfoBlockAppend {
        id: String,
        detail: String,
    },
    Error(String),
    ToolStart(ToolUse),
    ToolUpdate(ToolUpdate),
    ToolEnd(ToolEnd),
    CompactStart,
    CompactEnd,
    TurnStart {
        turn_id: Id,
    },
    TurnPause {
        turn_id: Id,
        reason: TurnPauseReason,
    },
    TurnEnd {
        turn_id: Id,
        status: TurnEndStatus,
    },
    UsageUpdate {
        usage: Usage,
    },
    Goodbye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnPauseReason {
    Approval { tools: Vec<ToolUse>, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnEndStatus {
    Completed,
    Interrupted {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUpdate {
    pub tool_use_id: String,
    pub seq: u64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolEndStatus {
    Completed,
    Cancelled,
    Denied,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnd {
    pub tool_use_id: String,
    pub status: ToolEndStatus,
    pub result_json: serde_json::Value,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewDecision {
    Accept,
    Skip,
    AcceptForSession,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: Option<String>,
    pub scope: Scope,
    pub argument_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentMetadata {
    pub name: String,
    pub description: String,
    pub scope: Scope,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderSpec {
    #[serde(alias = "name")]
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_models: Vec<ModelSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInitialized {
    pub model: ModelSpec,
    pub provider: ProviderSpec,
    pub session_id: Id,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub model: ModelSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRefreshed {
    pub session_id: Id,
    pub skills: Vec<SkillMetadata>,
    pub subagents: Vec<SubagentMetadata>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub tools: Vec<McpToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub qualified_name: String,
    pub description: String,
    pub parameters: Vec<McpToolParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionConfig {
    pub model: String,
    pub provider: String,
    pub policy: Option<PermissionMode>,
    pub streaming: bool,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub cwd: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl fmt::Display for ToolUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        if let serde_json::Value::Object(obj) = &self.args {
            let mut first = true;
            for (k, v) in obj {
                if v.is_null() {
                    continue;
                }
                f.write_str(if first { "(" } else { ", " })?;
                first = false;
                let rendered = v.to_string();
                write!(f, "{k}={}", elide(&rendered, 16))?;
            }
            if !first {
                f.write_str(")")?;
            }
        }
        Ok(())
    }
}

fn elide(s: &str, max: usize) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= max {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    std::borrow::Cow::Owned(out)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSpec {
    #[serde(alias = "name")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Hash)]
pub enum Thinking {
    Disabled,
    Enabled,
    Deep,
    Max,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, Copy)]
#[serde(default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u32>,
}

impl Usage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self { input_tokens, output_tokens, cache_read_tokens: None, cache_creation_tokens: None }
    }

    pub fn total(&self) -> u32 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}

impl std::ops::Add<Usage> for Usage {
    type Output = Usage;

    fn add(self, other: Usage) -> Usage {
        Usage {
            input_tokens: self.input_tokens.saturating_add(other.input_tokens),
            output_tokens: self.output_tokens.saturating_add(other.output_tokens),
            cache_read_tokens: add_optional_u32(self.cache_read_tokens, other.cache_read_tokens),
            cache_creation_tokens: add_optional_u32(
                self.cache_creation_tokens,
                other.cache_creation_tokens,
            ),
        }
    }
}

fn add_optional_u32(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (None, None) => None,
        _ => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
    }
}

impl std::ops::AddAssign<Usage> for Usage {
    fn add_assign(&mut self, other: Usage) {
        *self = *self + other;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    #[default]
    Default,
    Yolo,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    Project,
    User,
    System,
}

#[cfg(test)]
mod tests {
    use super::{
        Evt, ExtensionRefreshed, Id, ModelSpec, Op, ProviderSpec, SessionInitialized,
        SessionUpdate, ToolUse, Usage,
    };
    use std::path::PathBuf;

    fn model_spec(name: &str) -> ModelSpec {
        ModelSpec {
            id: name.to_string(),
            display_name: None,
            description: None,
            temperature: None,
            top_p: None,
            top_k: None,
            max_tokens: None,
            stop_sequences: None,
            context_limit: None,
            thinking: None,
        }
    }

    fn provider_spec(name: &str) -> ProviderSpec {
        ProviderSpec {
            id: name.to_string(),
            display_name: name.to_string(),
            base_url: format!("https://api.{name}.test/v1"),
            preferred_models: vec![model_spec("preferred-model")],
        }
    }

    #[test]
    fn compact_events_serde_roundtrip() {
        let compact_start =
            serde_json::to_string(&Evt::CompactStart).expect("serialize CompactStart");
        let compact_end = serde_json::to_string(&Evt::CompactEnd).expect("serialize CompactEnd");

        assert_eq!(compact_start, "\"CompactStart\"");
        assert_eq!(compact_end, "\"CompactEnd\"");

        assert!(matches!(
            serde_json::from_str::<Evt>(&compact_start).expect("deserialize CompactStart"),
            Evt::CompactStart
        ));
        assert!(matches!(
            serde_json::from_str::<Evt>(&compact_end).expect("deserialize CompactEnd"),
            Evt::CompactEnd
        ));
    }

    #[test]
    fn extension_refreshed_serde_roundtrip() {
        let event = Evt::ExtensionRefreshed(Box::new(ExtensionRefreshed {
            session_id: Id::new("ses"),
            skills: Vec::new(),
            subagents: Vec::new(),
            mcp_servers: Vec::new(),
        }));

        let json = serde_json::to_string(&event).expect("serialize ExtensionRefreshed");
        let decoded = serde_json::from_str::<Evt>(&json).expect("deserialize ExtensionRefreshed");

        assert!(matches!(
            decoded,
            Evt::ExtensionRefreshed(payload)
                if payload.skills.is_empty() && payload.subagents.is_empty()
        ));
    }

    #[test]
    fn session_update_op_serde_roundtrip() {
        let op = Op::UpdateSession(SessionUpdate {
            model: ModelSpec { temperature: Some(0.2), ..model_spec("gpt-5.4") },
        });

        let json = serde_json::to_string(&op).expect("serialize UpdateSession");
        let decoded = serde_json::from_str::<Op>(&json).expect("deserialize UpdateSession");

        assert!(matches!(
            decoded,
            Op::UpdateSession(SessionUpdate { model })
                if model.id == "gpt-5.4" && model.temperature == Some(0.2)
        ));
    }

    #[test]
    fn session_updated_event_serde_roundtrip() {
        let session_id = Id::new("ses");
        let event = Evt::SessionUpdated(Box::new(SessionInitialized {
            model: model_spec("claude-sonnet-4-6"),
            provider: provider_spec("anthropic"),
            session_id,
            cwd: PathBuf::from("/tmp/session-updated"),
        }));

        let json = serde_json::to_string(&event).expect("serialize SessionUpdated");
        let decoded = serde_json::from_str::<Evt>(&json).expect("deserialize SessionUpdated");

        assert!(matches!(
            decoded,
            Evt::SessionUpdated(payload)
                if payload.model.id == "claude-sonnet-4-6"
                    && payload.provider.id == "anthropic"
                    && payload.provider.base_url == "https://api.anthropic.test/v1"
                    && payload.provider.preferred_models.len() == 1
                    && payload.session_id == session_id
                    && payload.cwd == std::path::Path::new("/tmp/session-updated")
        ));
    }

    #[test]
    fn tool_use_display_examples() {
        let tu = |name: &str, args: serde_json::Value| ToolUse {
            id: "1".into(),
            name: name.into(),
            args,
            signature: None,
        };

        // Typical: null fields are skipped.
        assert_eq!(
            tu(
                "Bash",
                serde_json::json!({
                    "command": "ls -la",
                    "timeout": null,
                    "run_in_background": false,
                }),
            )
            .to_string(),
            r#"Bash(command="ls -la", run_in_background=false)"#,
        );

        // All-null and empty objects render as the bare name.
        assert_eq!(tu("Noop", serde_json::json!({ "x": null })).to_string(), "Noop");
        assert_eq!(tu("Noop", serde_json::json!({})).to_string(), "Noop");

        // Long values get elided.
        let long = tu("Run", serde_json::json!({ "command": "x".repeat(200) })).to_string();
        assert!(long.ends_with("…)"), "expected elision, got {long:?}");
    }

    #[test]
    fn usage_adds_cache_fields_without_overflowing() {
        let mut usage = Usage {
            input_tokens: 10,
            output_tokens: 20,
            cache_read_tokens: Some(3),
            cache_creation_tokens: None,
        };
        usage += Usage {
            input_tokens: 5,
            output_tokens: 6,
            cache_read_tokens: Some(4),
            cache_creation_tokens: Some(8),
        };

        assert_eq!(usage.input_tokens, 15);
        assert_eq!(usage.output_tokens, 26);
        assert_eq!(usage.total(), 41);
        assert_eq!(usage.cache_read_tokens, Some(7));
        assert_eq!(usage.cache_creation_tokens, Some(8));
    }
}
