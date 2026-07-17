use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// -- Enums --

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Created,
    #[serde(rename = "in-progress")]
    InProgress,
    Awaiting,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Sync,
    Async,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcErrorCode {
    ServerError,
    InvalidInput,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentEncoding {
    Plain,
    Base64,
}

// -- Core types --

/// Agent name: must match ^[a-z0-9]([-a-z0-9]*[a-z0-9])?$, 1-63 chars
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentName(pub String);

impl AgentName {
    pub fn validate(s: &str) -> Result<(), String> {
        // RFC 1123 DNS label validation
        if s.is_empty() || s.len() > 63 {
            return Err(format!("agent name must be 1-63 chars, got {}", s.len()));
        }
        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("agent name must contain only [a-z0-9-]".into());
        }
        if s.starts_with('-') || s.ends_with('-') {
            return Err("agent name must not start or end with -".into());
        }
        Ok(())
    }
}

impl std::fmt::Display for AgentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for AgentName {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s)?;
        Ok(AgentName(s.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcError {
    pub code: AcErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// -- Messages --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" | "agent" | "agent/{name}"
    pub parts: Vec<MessagePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>, // If set -> Artifact
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default = "default_encoding", skip_serializing_if = "is_plain")]
    pub content_encoding: ContentEncoding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>, // CitationMetadata or TrajectoryMetadata
}

fn default_encoding() -> ContentEncoding {
    ContentEncoding::Plain
}
fn is_plain(e: &ContentEncoding) -> bool {
    *e == ContentEncoding::Plain
}

impl MessagePart {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            name: None,
            content_type: "text/plain".into(),
            content: Some(content.into()),
            content_encoding: ContentEncoding::Plain,
            content_url: None,
            metadata: None,
        }
    }

    pub fn artifact(
        name: impl Into<String>,
        content_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            name: Some(name.into()),
            content_type: content_type.into(),
            content: Some(content.into()),
            content_encoding: ContentEncoding::Plain,
            content_url: None,
            metadata: None,
        }
    }

    pub fn url(content_type: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: None,
            content_type: content_type.into(),
            content: None,
            content_encoding: ContentEncoding::Plain,
            content_url: Some(url.into()),
            metadata: None,
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                write!(f, "\n")?;
            }
            if let Some(content) = &part.content {
                write!(f, "{content}")?;
            }
        }
        Ok(())
    }
}

// -- Run --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub agent_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub run_id: String,
    pub status: RunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_request: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcError>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
}

// -- Agent Manifest --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub description: String,
    pub input_content_types: Vec<String>,
    pub output_content_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AgentMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub programming_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub natural_languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<Capability>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_time_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
}

// -- Request/Response types --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCreateRequest {
    pub agent_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub input: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RunMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResumeRequest {
    pub await_resume: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RunMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsListResponse {
    pub agents: Vec<AgentManifest>,
}

// -- SSE Events --

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcEvent {
    #[serde(rename = "run.created")]
    RunCreated { run: Run },
    #[serde(rename = "run.in-progress")]
    RunInProgress { run: Run },
    #[serde(rename = "run.awaiting")]
    RunAwaiting { run: Run },
    #[serde(rename = "run.completed")]
    RunCompleted { run: Run },
    #[serde(rename = "run.failed")]
    RunFailed { run: Run },
    #[serde(rename = "run.cancelled")]
    RunCancelled { run: Run },
    #[serde(rename = "message.created")]
    MessageCreated { message: Message },
    #[serde(rename = "message.part")]
    MessagePartEvent { part: MessagePart },
    #[serde(rename = "message.completed")]
    MessageCompleted { message: Message },
    #[serde(rename = "error")]
    ErrorEvent { error: AcError },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_part_text_roundtrip() {
        let part = MessagePart::text("hello world");
        let json = serde_json::to_string(&part).unwrap();
        let decoded: MessagePart = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.content_type, "text/plain");
        assert_eq!(decoded.content.as_deref(), Some("hello world"));
        assert!(decoded.name.is_none());
        assert!(decoded.content_url.is_none());
    }

    #[test]
    fn message_display() {
        let msg = Message {
            role: "user".into(),
            parts: vec![MessagePart::text("hello"), MessagePart::text("world")],
            created_at: None,
            completed_at: None,
        };
        assert_eq!(format!("{msg}"), "hello\nworld");
    }

    #[test]
    fn agent_name_validation() {
        assert!(AgentName::validate("ante").is_ok());
        assert!(AgentName::validate("my-agent").is_ok());
        assert!(AgentName::validate("a1").is_ok());
        assert!(AgentName::validate("-bad").is_err());
        assert!(AgentName::validate("bad-").is_err());
        assert!(AgentName::validate("BAD").is_err());
        assert!(AgentName::validate("").is_err());
        assert!(AgentName::validate(&"x".repeat(64)).is_err());
    }

    #[test]
    fn run_status_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RunStatus::InProgress).unwrap(),
            "\"in-progress\""
        );
        assert_eq!(
            serde_json::to_string(&RunStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn run_roundtrip() {
        let run = Run {
            agent_name: "ante".into(),
            session_id: None,
            run_id: uuid::Uuid::new_v4().to_string(),
            status: RunStatus::Created,
            await_request: None,
            output: vec![],
            error: None,
            created_at: Utc::now(),
            finished_at: None,
        };
        let json = serde_json::to_string(&run).unwrap();
        let decoded: Run = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.status, RunStatus::Created);
        assert!(decoded.finished_at.is_none());
    }

    #[test]
    fn ac_event_tagged_serialization() {
        let event = AcEvent::RunCompleted {
            run: Run {
                agent_name: "ante".into(),
                session_id: None,
                run_id: "test".into(),
                status: RunStatus::Completed,
                await_request: None,
                output: vec![],
                error: None,
                created_at: Utc::now(),
                finished_at: Some(Utc::now()),
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "run.completed");
    }
}
