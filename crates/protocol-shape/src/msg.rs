use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

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
    StartSession(SessionOverrides),
    UpdateSession(SessionUpdate),
    Interrupt,
    UserInput(String),
    ShellInput(String),
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
    /// Request a per-category breakdown of the active session's context-window
    /// occupancy. Answered with [`Evt::ContextReport`].
    ContextReport,
    /// Set, clear, or report a goal-driven execution loop on the active
    /// session. A set goal keeps the session working — re-running turns and
    /// judging the condition after each one — until it is met, judged
    /// unreachable, or cleared.
    Goal(GoalCommand),
    /// Request an ad-hoc "thinking phrase" prediction for the in-progress
    /// draft. Runs off the conversation critical path on a cheap model and
    /// answers with `Evt::Ambient`. `req_id` lets the client discard stale
    /// results when a newer request supersedes this one.
    AmbientPhrase {
        draft: String,
        req_id: u64,
    },
    /// Request an ad-hoc next-prompt suggestion from the last exchange, shown as
    /// input ghost text. Like [`Op::AmbientPhrase`] it runs off the critical
    /// path on a cheap model and answers with `Evt::Ambient`. The client carries
    /// the context (so this stays a client-only feature — headless never fires
    /// it) and a monotonic `req_id` to drop stale results.
    AmbientSuggestion {
        recent_user: String,
        recent_agent: String,
        req_id: u64,
    },
    Shutdown,
}

/// A `/goal` sub-command carried by [`Op::Goal`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalCommand {
    /// Set (or replace) the active goal condition.
    Set(String),
    /// Clear the active goal, stopping the loop.
    Clear,
    /// Report the current goal status.
    Status,
}

/// Which ambient feature produced an [`Evt::Ambient`]. Both run off the main
/// conversation on a cheap model; they differ in trigger, prompt, and sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AmbientKind {
    /// Predicted phrase for the in-progress draft, shown in the status spinner.
    ThinkingPhrase,
    /// Suggested next prompt from the conversation so far, shown as input ghost text.
    PromptSuggestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evt {
    SessionStart(Box<SessionInitialized>),
    SessionUpdated(Box<SessionInitialized>),
    ExtensionRefreshed(Box<ExtensionRefreshed>),
    /// The session span closed. Mirrors `TurnEnd`: carries the span's
    /// identity, why it ended, and its final usage accounting.
    SessionEnd {
        session_id: Id,
        reason: SessionEndReason,
        usage: Usage,
    },
    UserInput(String),
    ShellOutput {
        command: String,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
    },
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
    /// Compaction finished. `summary` is the text that replaced the
    /// compacted history and carries forward as the session's context;
    /// `None` when compaction failed (history unchanged) or produced no
    /// displayable text.
    CompactEnd {
        #[serde(default)]
        summary: Option<String>,
    },
    TurnStart {
        turn_id: Id,
    },
    TurnPause {
        turn_id: Id,
        reason: TurnPauseReason,
    },
    /// The turn resumed after a `TurnPause` (e.g. the approval was answered
    /// or a steer arrived). Closes the pause bracket so clients never have to
    /// infer resumption from the next tool event.
    TurnResume {
        turn_id: Id,
    },
    TurnEnd {
        turn_id: Id,
        status: TurnEndStatus,
        /// Number of turn-loop steps attempted before the turn ended.
        #[serde(default)]
        steps: usize,
    },
    UsageUpdate {
        usage: Usage,
        /// Context-window occupancy for the root session, pre-calculated in core.
        /// `None` before the first response or when the model's context limit is
        /// unverified (so clients never render a confidently-wrong percentage).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<ContextWindow>,
    },
    /// Answer to [`Op::ContextReport`]: a per-category breakdown of the
    /// session's context-window occupancy at the time of the request.
    ContextReport(ContextBreakdown),
    /// An ephemeral ambient hint produced off the main conversation (a predicted
    /// "thinking phrase" for the draft, or a suggested next prompt — see
    /// [`AmbientKind`]). Never persisted to the event log. `req_id` lets clients
    /// drop superseded results.
    Ambient {
        kind: AmbientKind,
        req_id: u64,
        text: String,
    },
    Goodbye,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnPauseReason {
    Approval { tools: Vec<ToolUse>, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEndReason {
    /// The session was replaced by a new or resumed session.
    Replaced,
    /// The daemon is shutting down.
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnEndStatus {
    Completed,
    Interrupted {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    Error {
        /// Stable machine-readable LLM error kind. Absent for non-LLM errors
        /// and events produced by older daemons. Notably `"oauth"` means the
        /// provider's sign-in is missing, expired, or revoked, and only
        /// re-authenticating can recover; clients may offer their sign-in
        /// flow for the session's provider.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        /// One-line summary. For a classified LLM failure this is the semantic
        /// error kind, e.g. "rate limited"; otherwise the top of the error chain.
        headline: String,
        /// Expanded cause shown as indented child rows beneath the headline,
        /// e.g. ["HTTP 400 Bad Request", "<server-provided message>"]. May be
        /// empty when there is nothing useful to add.
        details: Vec<String>,
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

impl ToolEndStatus {
    /// Whether this terminal status represents an error result. Mirrors the
    /// LLM-facing `ToolResult::is_error`: anything but a clean completion
    /// (failure, denial, cancellation) is an error.
    pub fn is_error(self) -> bool {
        !matches!(self, ToolEndStatus::Completed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnd {
    pub tool_use_id: String,
    pub status: ToolEndStatus,
    pub result_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewDecision {
    Accept,
    Skip,
    AcceptForSession,
    /// Approve and persist an allow rule to settings.json so the same call is
    /// auto-approved across future sessions ("always allow").
    AcceptAlways,
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

/// The provider a session resolved to.
///
/// Carries only what a client cannot look up for itself: the id to key state
/// on, a name to show a user, and the endpoint actually in use — which env
/// overrides can move away from the published default, so it is a property of
/// this session rather than of the provider. The provider's model list is not
/// here; it is the same for every session and is published by `ante catalog`.
/// Unknown fields are ignored, so payloads still carrying it decode fine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderSpec {
    #[serde(alias = "name")]
    pub id: String,
    pub display_name: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInitialized {
    pub model: ModelSpec,
    pub provider: ProviderSpec,
    pub session_id: Id,
    pub cwd: PathBuf,
    pub permission_mode: PermissionMode,
}

/// Partial update to a live session's mutable state. Each field is optional so
/// a caller patches only what changed; absent fields are left untouched.
/// Catalog-dependent fields are resolved before the update takes effect.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUpdate {
    /// Model change, taking effect on the next turn. The spec carries the
    /// whole request, `effort` included: a set `effort` overrides the
    /// catalog default; unset fields resolve from the catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelSpec>,
    /// Permission mode change, taking effect on the next turn without
    /// aborting an in-flight one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
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

/// A patch of session overrides produced by every caller (CLI, TUI, gateway,
/// external `serve` clients). `None` means "leave unchanged" for every field —
/// there is exactly one meaning, regardless of who built the value.
///
/// This is the wire payload of [`Op::StartSession`]: the requested session
/// configuration. Unset fields fall back to the daemon's defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_system_prompt: Option<String>,
    /// Exactly these tools, replacing the default tool set as the base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Tools added on top of the base set (`tools`, or the default set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_tools: Option<Vec<String>>,
    /// Tools removed from the session; wins over `tools` and `include_tools`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<Effort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_auto_memory: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_prompt: Option<bool>,
    /// When true, the session loads no skills: none are discovered,
    /// advertised, or invocable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_skills: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MalformedToolArgs {
    /// Exact argument text emitted by the model before JSON decoding failed.
    pub raw: String,
    /// JSON decoder diagnostic. This must not contain the raw argument text.
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    /// Present when `args` could not be decoded from the model's raw JSON.
    /// Such a call is non-executable and must be returned as an error result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub malformed_args: Option<MalformedToolArgs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl ToolUse {
    /// A well-formed call: decoded `args`, no malformed metadata, no signature.
    pub fn new(id: impl Into<String>, name: impl Into<String>, args: serde_json::Value) -> Self {
        Self { id: id.into(), name: name.into(), args, malformed_args: None, signature: None }
    }
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
    pub effort: Option<Effort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_vision: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_class: Option<WeightClass>,
}

/// Requested output/reasoning effort for model turns, on an ordinal scale.
///
/// `min` is the lowest effort the model supports — thinking is disabled where
/// the model allows that; models with always-on reasoning clamp to their
/// lowest level. Providers that expose fewer levels round a requested effort
/// down to the nearest supported one. Variants are declared in ascending
/// order so the derived `Ord` sorts `Min < Low < ... < Max`.
#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Min,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Effort {
    /// All levels in ascending order.
    pub const ALL: [Effort; 6] =
        [Effort::Min, Effort::Low, Effort::Medium, Effort::High, Effort::XHigh, Effort::Max];

    /// The wire token for this level (`"min"`, `"low"`, ..., `"max"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Effort::Min => "min",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
            Effort::Max => "max",
        }
    }

    /// Round this requested effort down to the nearest level the provider
    /// exposes, implementing the contract documented on the enum: providers
    /// that expose fewer levels round a requested effort down to the nearest
    /// supported one.
    ///
    /// Returns the largest supported level at or below this one, or `None`
    /// when the requested level sits below every supported level — the caller
    /// then omits the effort knob entirely (the same treatment a model with
    /// no reasoning knob gets).
    pub fn round_down_to(self, provider_levels: &[Effort]) -> Option<Effort> {
        Effort::ALL
            .iter()
            .rev()
            .find(|level| **level <= self && provider_levels.contains(level))
            .copied()
    }

    /// The OpenAI-compatible `reasoning_effort` wire token for this level,
    /// per the three-level `low | medium | high` contract (issue #155).
    ///
    /// Levels above `high` round down to `high`; [`Effort::Min`] maps to
    /// `None` — thinking-off has no token on that contract, so the field is
    /// omitted rather than sent a value the server rejects. This is the
    /// concrete instance of the enum's documented rounding contract.
    pub fn openai_reasoning_effort(self) -> Option<&'static str> {
        match self.round_down_to(&[Effort::Low, Effort::Medium, Effort::High]) {
            Some(Effort::Low) => Some("low"),
            Some(Effort::Medium) => Some("medium"),
            Some(Effort::High) => Some("high"),
            _ => None,
        }
    }
}

impl std::fmt::Display for Effort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Effort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Effort::ALL.into_iter().find(|e| e.as_str() == s).ok_or_else(|| {
            format!("unknown effort `{s}` (expected min, low, medium, high, xhigh, max)")
        })
    }
}

/// Innate size/cost class of a model, set once per model in the catalog.
///
/// Orthogonal to the per-request [`Effort`] and to `context_limit`:
/// a model's weight class reflects roughly how large and costly it is to run,
/// not how hard it is asked to think on a given turn. Variants are declared in
/// ascending order so the derived `Ord` sorts `Feather < Middle < Heavy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WeightClass {
    /// Small, fast, cheap (Haiku- / GPT-nano-class).
    Feather,
    /// Mid workhorse (Sonnet- / GPT-mini- / Gemini-Flash-class).
    Middle,
    /// Largest, most capable, costliest (Opus- / GPT-5.x- / Gemini-Pro-class).
    Heavy,
}

impl<'de> Deserialize<'de> for WeightClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.to_ascii_lowercase().as_str() {
            "feather" => Ok(Self::Feather),
            "middle" => Ok(Self::Middle),
            "heavy" => Ok(Self::Heavy),
            _ => Err(serde::de::Error::unknown_variant(&value, &["feather", "middle", "heavy"])),
        }
    }
}

/// Token usage for one model response.
///
/// Convention, uniform across every provider mapping: `input_tokens` is the
/// **full, cache-inclusive prompt size**. It always contains `cache_read_tokens`
/// as a subset (verified live: OpenAI/OpenRouter/DeepSeek report it inside
/// `prompt_tokens`; the Anthropic mapping adds it back since that API reports
/// input net of cache). `cache_creation_tokens` is likewise inside `input_tokens`
/// for providers that report cache writes (Anthropic); the OpenAI-style
/// providers we use don't report writes at all. So [`Usage::total`]
/// (`input + output`) is the context-window occupancy.
///
/// For **cost**, the cache buckets bill at different rates, so subtract them
/// from the input rate instead of charging the full rate twice:
/// `cost = (input - cache_read - cache_creation)·p_in
///        + cache_read·p_cache_read + cache_creation·p_cache_write
///        + output·p_out`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, Copy)]
#[serde(default)]
pub struct Usage {
    /// Full prompt tokens, cache-inclusive (a superset of the two cache fields).
    pub input_tokens: u32,
    /// Generated output (completion) tokens.
    pub output_tokens: u32,
    /// Subset of `input_tokens` served from the prompt cache (cheaper rate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    /// Subset of `input_tokens` written into the prompt cache (surcharge rate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u32>,
}

/// Context-window occupancy snapshot for the current (root) session, surfaced in
/// the statusline. Raw measurements only — any percentage is a presentation
/// concern derived at the edges, with no policy (e.g. auto-compaction) baked in.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextWindow {
    /// Tokens currently occupying the window (cache-inclusive input + output of
    /// the most recent response).
    pub used_tokens: u32,
    /// Raw model context limit (e.g. 200_000).
    pub limit_tokens: u32,
}

/// Per-category breakdown of context-window occupancy.
///
/// `used_tokens` is anchored on the provider-reported occupancy once the
/// session has seen a model response; before that it is estimated. The
/// per-category fields are estimates that normally sum to `used_tokens`, but
/// estimation error can make them disagree slightly — clients should clamp
/// rather than assume an exact identity.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContextBreakdown {
    /// System prompt, excluding the skills and memory sections counted below.
    pub system_prompt_tokens: u32,
    /// Built-in tool schemas.
    pub system_tools_tokens: u32,
    /// MCP tool schemas.
    pub mcp_tools_tokens: u32,
    /// Memory content: project/user instruction files and the auto-memory prompt.
    pub memory_tokens: u32,
    /// The available-skills listing.
    pub skills_tokens: u32,
    /// Conversation messages: everything not attributed to a category above.
    pub messages_tokens: u32,
    /// Total context-window occupancy.
    pub used_tokens: u32,
    /// Model context limit. `None` when unverified, so clients never render a
    /// confidently-wrong percentage.
    pub limit_tokens: Option<u32>,
    /// Tokens reserved at the top of the window; auto-compaction triggers once
    /// occupancy grows into this reserve.
    pub compact_buffer_tokens: u32,
}

impl Usage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self { input_tokens, output_tokens, cache_read_tokens: None, cache_creation_tokens: None }
    }

    /// Context-window occupancy: the full (cache-inclusive) prompt plus output.
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
    /// Honor user rules; an unmatched call asks unless it is provably safe.
    #[default]
    Strict,
    /// Honor user rules; an unmatched call runs unless it is provably
    /// dangerous (a deliberately narrow classification).
    Auto,
    /// Bypass all permission checks, including user deny rules.
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
        Evt, ExtensionRefreshed, Id, ModelSpec, Op, PermissionMode, ProviderSpec,
        SessionInitialized, SessionUpdate, ToolUse, Usage, WeightClass,
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
            effort: None,
            support_vision: None,
            weight_class: None,
        }
    }

    #[test]
    fn tool_use_without_malformed_args_remains_backward_compatible() {
        let tool_use: ToolUse = serde_json::from_value(serde_json::json!({
            "id": "call-1",
            "name": "Read",
            "args": { "file_path": "README.md" }
        }))
        .unwrap();

        assert!(tool_use.malformed_args.is_none());
        let encoded = serde_json::to_value(tool_use).unwrap();
        assert!(encoded.get("malformed_args").is_none());
    }

    #[test]
    fn turn_end_error_kind_is_optional_on_the_wire() {
        // Payloads from daemons predating the field still deserialize.
        let old: super::TurnEndStatus = serde_json::from_value(serde_json::json!({
            "Error": { "headline": "authentication error", "details": [] }
        }))
        .unwrap();
        let super::TurnEndStatus::Error { kind, .. } = &old else {
            panic!("expected Error variant");
        };
        assert!(kind.is_none());

        // None is skipped, not emitted as null.
        let json = serde_json::to_value(&old).unwrap();
        assert!(json["Error"].get("kind").is_none());

        // Some round-trips.
        let with = super::TurnEndStatus::Error {
            kind: Some("oauth".to_string()),
            headline: "OAuth sign-in required".to_string(),
            details: vec![],
        };
        let json = serde_json::to_value(&with).unwrap();
        assert_eq!(json["Error"]["kind"], "oauth");
    }

    #[test]
    fn turn_end_steps_default_for_older_events() {
        let event = super::Evt::TurnEnd {
            turn_id: super::Id::new("turn"),
            status: super::TurnEndStatus::Completed,
            steps: 3,
        };
        let mut json = serde_json::to_value(&event).unwrap();
        json["TurnEnd"].as_object_mut().unwrap().remove("steps");
        let old: super::Evt = serde_json::from_value(json).unwrap();

        assert!(matches!(old, super::Evt::TurnEnd { steps: 0, .. }));

        let json = serde_json::to_value(event).unwrap();
        assert_eq!(json["TurnEnd"]["steps"], 3);
    }

    #[test]
    fn effort_serializes_as_lowercase_tokens() {
        let mut spec = model_spec("m");
        spec.effort = Some(super::Effort::XHigh);
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["effort"], "xhigh");

        // None is skipped, not emitted as null.
        let json_none = serde_json::to_value(model_spec("m")).unwrap();
        assert!(json_none.get("effort").is_none());

        // Every level round-trips through its wire token.
        for level in super::Effort::ALL {
            let parsed: ModelSpec =
                serde_json::from_value(serde_json::json!({"id": "m", "effort": level.as_str()}))
                    .unwrap();
            assert_eq!(parsed.effort, Some(level), "round-trip {level}");
        }
    }

    #[test]
    fn effort_orders_min_lowest_to_max_highest() {
        let mut sorted = super::Effort::ALL;
        sorted.sort();
        assert_eq!(sorted, super::Effort::ALL);
        assert!(super::Effort::Min < super::Effort::Low);
        assert!(super::Effort::XHigh < super::Effort::Max);
    }

    #[test]
    fn effort_rounds_down_to_nearest_supported_level() {
        // Full ladder: every level lands on itself.
        for level in super::Effort::ALL {
            assert_eq!(level.round_down_to(&super::Effort::ALL), Some(level), "full ladder {level}");
        }

        // A three-level provider ladder: levels above the top clamp to the
        // top, `min` (thinking-off) falls below the ladder and is omitted.
        let three = [super::Effort::Low, super::Effort::Medium, super::Effort::High];
        assert_eq!(super::Effort::Min.round_down_to(&three), None);
        assert_eq!(super::Effort::Low.round_down_to(&three), Some(super::Effort::Low));
        assert_eq!(super::Effort::Medium.round_down_to(&three), Some(super::Effort::Medium));
        assert_eq!(super::Effort::High.round_down_to(&three), Some(super::Effort::High));
        assert_eq!(super::Effort::XHigh.round_down_to(&three), Some(super::Effort::High));
        assert_eq!(super::Effort::Max.round_down_to(&three), Some(super::Effort::High));

        // Sparse ladder, unordered on purpose: containment, not position, wins.
        let sparse = [super::Effort::Max, super::Effort::Medium];
        assert_eq!(super::Effort::Low.round_down_to(&sparse), None);
        assert_eq!(super::Effort::Medium.round_down_to(&sparse), Some(super::Effort::Medium));
        assert_eq!(super::Effort::XHigh.round_down_to(&sparse), Some(super::Effort::Medium));
        assert_eq!(super::Effort::Max.round_down_to(&sparse), Some(super::Effort::Max));

        // Empty ladder: nothing to round down to.
        assert_eq!(super::Effort::Max.round_down_to(&[]), None);
    }

    #[test]
    fn effort_openai_reasoning_effort_maps_to_three_level_contract() {
        // OpenAI-compatible providers (catalog `OpenAiCompatible`) take
        // `reasoning_effort: low | medium | high`. Ante's six levels collapse
        // by rounding down; `min` (thinking-off) has no token and is omitted.
        let cases = [
            (super::Effort::Min, None),
            (super::Effort::Low, Some("low")),
            (super::Effort::Medium, Some("medium")),
            (super::Effort::High, Some("high")),
            (super::Effort::XHigh, Some("high")),
            (super::Effort::Max, Some("high")),
        ];
        for (level, expected) in cases {
            assert_eq!(level.openai_reasoning_effort(), expected, "openai token {level}");
        }
    }

    #[test]
    fn session_overrides_effort_round_trips() {
        let parsed: super::SessionOverrides =
            serde_json::from_value(serde_json::json!({"effort": "max"})).unwrap();
        assert_eq!(parsed.effort, Some(super::Effort::Max));

        let parsed: super::SessionOverrides =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(parsed.effort, None);
    }

    #[test]
    fn short_prompt_round_trips_and_defaults_to_unset() {
        let parsed: super::SessionOverrides =
            serde_json::from_value(serde_json::json!({"short_prompt": true})).unwrap();
        assert_eq!(parsed.short_prompt, Some(true));

        let parsed: super::SessionOverrides =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(parsed.short_prompt, None);
    }

    #[test]
    fn weight_class_serializes_lowercase_and_is_omitted_when_none() {
        let mut spec = model_spec("m");
        spec.weight_class = Some(WeightClass::Heavy);
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["weight_class"], "heavy");

        // None is skipped, not emitted as null.
        let json_none = serde_json::to_value(model_spec("m")).unwrap();
        assert!(json_none.get("weight_class").is_none());

        // Round-trips from the lowercase wire form.
        let parsed: ModelSpec =
            serde_json::from_value(serde_json::json!({"id": "m", "weight_class": "feather"}))
                .unwrap();
        assert_eq!(parsed.weight_class, Some(WeightClass::Feather));
    }

    #[test]
    fn weight_class_deserializes_case_insensitively() {
        for (value, expected) in [
            ("Feather", WeightClass::Feather),
            ("MIDDLE", WeightClass::Middle),
            ("hEaVy", WeightClass::Heavy),
        ] {
            let parsed: ModelSpec =
                serde_json::from_value(serde_json::json!({"id": "m", "weight_class": value}))
                    .unwrap();
            assert_eq!(parsed.weight_class, Some(expected));
        }
    }

    #[test]
    fn weight_class_orders_feather_lightest_to_heavy_heaviest() {
        assert!(WeightClass::Feather < WeightClass::Middle);
        assert!(WeightClass::Middle < WeightClass::Heavy);
    }

    fn provider_spec(name: &str) -> ProviderSpec {
        ProviderSpec {
            id: name.to_string(),
            display_name: name.to_string(),
            base_url: format!("https://api.{name}.test/v1"),
        }
    }

    #[test]
    fn compact_events_serde_roundtrip() {
        let compact_start =
            serde_json::to_string(&Evt::CompactStart).expect("serialize CompactStart");
        let compact_end =
            serde_json::to_string(&Evt::CompactEnd { summary: Some("the summary".to_string()) })
                .expect("serialize CompactEnd");

        assert_eq!(compact_start, "\"CompactStart\"");
        assert_eq!(compact_end, r#"{"CompactEnd":{"summary":"the summary"}}"#);

        assert!(matches!(
            serde_json::from_str::<Evt>(&compact_start).expect("deserialize CompactStart"),
            Evt::CompactStart
        ));
        assert!(matches!(
            serde_json::from_str::<Evt>(&compact_end).expect("deserialize CompactEnd"),
            Evt::CompactEnd { summary: Some(s) } if s == "the summary"
        ));
        assert!(matches!(
            serde_json::from_str::<Evt>(r#"{"CompactEnd":{}}"#)
                .expect("deserialize CompactEnd without summary"),
            Evt::CompactEnd { summary: None }
        ));
    }

    #[test]
    fn session_end_and_turn_resume_serde_roundtrip() {
        let session_id = Id::new("ses");
        let end = Evt::SessionEnd {
            session_id,
            reason: super::SessionEndReason::Shutdown,
            usage: Usage::new(10, 5),
        };
        let json = serde_json::to_string(&end).expect("serialize SessionEnd");
        let decoded = serde_json::from_str::<Evt>(&json).expect("deserialize SessionEnd");
        assert!(matches!(
            decoded,
            Evt::SessionEnd { session_id: id, reason: super::SessionEndReason::Shutdown, usage }
                if id == session_id && usage.total() == 15
        ));

        let turn_id = Id::new("op");
        let resume = Evt::TurnResume { turn_id };
        let json = serde_json::to_string(&resume).expect("serialize TurnResume");
        let decoded = serde_json::from_str::<Evt>(&json).expect("deserialize TurnResume");
        assert!(matches!(decoded, Evt::TurnResume { turn_id: id } if id == turn_id));
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
            model: Some(ModelSpec {
                temperature: Some(0.2),
                effort: Some(super::Effort::High),
                ..model_spec("gpt-5.4")
            }),
            permission_mode: Some(PermissionMode::Yolo),
        });

        let json = serde_json::to_string(&op).expect("serialize UpdateSession");
        let decoded = serde_json::from_str::<Op>(&json).expect("deserialize UpdateSession");

        assert!(matches!(
            decoded,
            Op::UpdateSession(SessionUpdate {
                model: Some(model),
                permission_mode: Some(PermissionMode::Yolo),
            })
                if model.id == "gpt-5.4"
                    && model.temperature == Some(0.2)
                    && model.effort == Some(super::Effort::High)
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
            permission_mode: PermissionMode::default(),
        }));

        let json = serde_json::to_string(&event).expect("serialize SessionUpdated");
        let decoded = serde_json::from_str::<Evt>(&json).expect("deserialize SessionUpdated");

        assert!(matches!(
            decoded,
            Evt::SessionUpdated(payload)
                if payload.model.id == "claude-sonnet-4-6"
                    && payload.provider.id == "anthropic"
                    && payload.provider.base_url == "https://api.anthropic.test/v1"
                    && payload.session_id == session_id
                    && payload.cwd == std::path::Path::new("/tmp/session-updated")
        ));
    }

    #[test]
    fn provider_spec_ignores_the_dropped_model_list() {
        // Payloads from daemons that still send the provider's model list
        // decode against the narrowed shape.
        let spec: ProviderSpec = serde_json::from_value(serde_json::json!({
            "id": "anthropic",
            "display_name": "Anthropic",
            "base_url": "https://api.anthropic.test/v1",
            "preferred_models": [{ "id": "claude-sonnet-4-6" }],
        }))
        .unwrap();

        assert_eq!(spec.id, "anthropic");
        assert_eq!(spec.display_name, "Anthropic");
        assert_eq!(spec.base_url, "https://api.anthropic.test/v1");

        // And the catalog data does not go back out.
        let encoded = serde_json::to_value(&spec).unwrap();
        assert!(encoded.get("preferred_models").is_none());
    }

    #[test]
    fn context_report_serde_roundtrip() {
        let breakdown = super::ContextBreakdown {
            system_prompt_tokens: 1200,
            system_tools_tokens: 3400,
            mcp_tools_tokens: 0,
            memory_tokens: 800,
            skills_tokens: 150,
            messages_tokens: 42_000,
            used_tokens: 47_550,
            limit_tokens: Some(200_000),
            compact_buffer_tokens: 20_000,
        };
        let json = serde_json::to_value(Evt::ContextReport(breakdown)).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "ContextReport": {
                    "system_prompt_tokens": 1200,
                    "system_tools_tokens": 3400,
                    "mcp_tools_tokens": 0,
                    "memory_tokens": 800,
                    "skills_tokens": 150,
                    "messages_tokens": 42000,
                    "used_tokens": 47550,
                    "limit_tokens": 200000,
                    "compact_buffer_tokens": 20000
                }
            })
        );
        let decoded = serde_json::from_value::<Evt>(json).expect("deserialize");
        assert!(matches!(decoded, Evt::ContextReport(b) if b == breakdown));

        // Fields absent on the wire (older daemons) fall back to defaults.
        let sparse: super::ContextBreakdown =
            serde_json::from_value(serde_json::json!({"used_tokens": 10})).unwrap();
        assert_eq!(sparse.used_tokens, 10);
        assert_eq!(sparse.limit_tokens, None);

        let op = serde_json::to_value(Op::ContextReport).expect("serialize op");
        assert_eq!(op, serde_json::json!("ContextReport"));
        assert!(matches!(serde_json::from_value::<Op>(op).unwrap(), Op::ContextReport));
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
