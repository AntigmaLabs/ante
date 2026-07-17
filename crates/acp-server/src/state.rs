use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::agent;
use crate::error::AcServerError;
use crate::types::*;

/// Shared application state for the ACP server.
#[derive(Clone)]
pub struct AcState {
    inner: Arc<AcStateInner>,
}

struct AcStateInner {
    runs: RwLock<HashMap<String, Run>>,
    inputs: RwLock<HashMap<String, Vec<Message>>>,
    manifest: AgentManifest,
}

impl AcState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AcStateInner {
                runs: RwLock::new(HashMap::new()),
                inputs: RwLock::new(HashMap::new()),
                manifest: agent::build_manifest(None),
            }),
        }
    }

    pub fn manifest(&self) -> AgentManifest {
        self.inner.manifest.clone()
    }

    pub async fn create_run(&self, req: RunCreateRequest) -> Result<Run, AcServerError> {
        let run_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let run = Run {
            agent_name: req.agent_name,
            session_id: req.session_id,
            run_id: run_id.clone(),
            status: RunStatus::Created,
            await_request: None,
            output: Vec::new(),
            error: None,
            created_at: now,
            finished_at: None,
        };

        self.inner.inputs.write().await.insert(run_id.clone(), req.input);
        self.inner.runs.write().await.insert(run_id, run.clone());
        Ok(run)
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Run, AcServerError> {
        self.inner
            .runs
            .read()
            .await
            .get(run_id)
            .cloned()
            .ok_or_else(|| AcServerError::NotFound(format!("run '{run_id}' not found")))
    }

    pub async fn execute_run_sync(&self, run_id: &str) -> Result<Run, AcServerError> {
        // 1. Extract input text from stored input messages
        let input_text = {
            let inputs = self.inner.inputs.read().await;
            let input_msgs = inputs.get(run_id).ok_or_else(|| {
                AcServerError::NotFound(format!("run '{run_id}' has no stored input"))
            })?;
            extract_text_from_messages(input_msgs)
        };

        // 2. Set status to in-progress
        {
            let mut runs = self.inner.runs.write().await;
            if let Some(run) = runs.get_mut(run_id) {
                run.status = RunStatus::InProgress;
            }
        }

        // 3. Execute Ante subprocess
        let execution = crate::executor::execute_ante(&input_text, None, None).await;

        // 4. Update run with result
        let mut runs = self.inner.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| AcServerError::NotFound(format!("run '{run_id}' not found")))?;

        match execution {
            Ok(exec) => {
                run.output = vec![Message {
                    role: format!("agent/{}", run.agent_name),
                    parts: vec![MessagePart::text(exec.result_text)],
                    created_at: Some(chrono::Utc::now()),
                    completed_at: Some(chrono::Utc::now()),
                }];
                run.status = RunStatus::Completed;
                run.finished_at = Some(chrono::Utc::now());
            }
            Err(err) => {
                run.status = RunStatus::Failed;
                run.error = Some(AcError {
                    code: AcErrorCode::ServerError,
                    message: err.message.clone(),
                    data: Some(serde_json::json!(err.details)),
                });
                run.output = vec![];
                run.finished_at = Some(chrono::Utc::now());
            }
        }

        Ok(run.clone())
    }

    pub fn spawn_run_execution(&self, run_id: &str) {
        let state = self.clone();
        let run_id = run_id.to_string();
        tokio::spawn(async move {
            let _ = state.execute_run_sync(&run_id).await;
        });
    }

    pub async fn resume_run(
        &self,
        run_id: &str,
        _req: RunResumeRequest,
    ) -> Result<Run, AcServerError> {
        let mut runs = self.inner.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| AcServerError::NotFound(format!("run '{run_id}' not found")))?;

        if run.status != RunStatus::Awaiting {
            return Err(AcServerError::InvalidInput(format!(
                "run '{run_id}' is not in awaiting state (current: {:?})",
                run.status
            )));
        }

        // Resume: set back to in-progress, then complete
        run.status = RunStatus::InProgress;
        run.output.push(Message {
            role: "user".into(),
            parts: vec![MessagePart::text("Resumed with additional input")],
            created_at: Some(chrono::Utc::now()),
            completed_at: None,
        });
        run.status = RunStatus::Completed;
        run.finished_at = Some(chrono::Utc::now());

        Ok(run.clone())
    }

    pub async fn cancel_run(&self, run_id: &str) -> Result<(StatusCode, Run), AcServerError> {
        let mut runs = self.inner.runs.write().await;
        let run = runs
            .get_mut(run_id)
            .ok_or_else(|| AcServerError::NotFound(format!("run '{run_id}' not found")))?;

        match run.status {
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled => {
                return Err(AcServerError::InvalidInput(format!(
                    "cannot cancel run in terminal state: {:?}",
                    run.status
                )));
            }
            _ => {}
        }

        run.status = RunStatus::Cancelled;
        run.finished_at = Some(chrono::Utc::now());
        Ok((StatusCode::OK, run.clone()))
    }
}

/// Extract plain text content from all message parts in a list of messages.
pub(crate) fn extract_text_from_messages(messages: &[Message]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        for part in &msg.parts {
            if let Some(content) = &part.content {
                parts.push(content.as_str());
            }
        }
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_single_message_single_part() {
        let messages = vec![Message {
            role: "user".into(),
            parts: vec![MessagePart::text("hello world")],
            created_at: None,
            completed_at: None,
        }];
        assert_eq!(extract_text_from_messages(&messages), "hello world");
    }

    #[test]
    fn extract_text_multiple_parts_joined_by_newline() {
        let messages = vec![Message {
            role: "user".into(),
            parts: vec![MessagePart::text("line one"), MessagePart::text("line two")],
            created_at: None,
            completed_at: None,
        }];
        assert_eq!(extract_text_from_messages(&messages), "line one\nline two");
    }

    #[test]
    fn extract_text_multiple_messages() {
        let messages = vec![
            Message {
                role: "user".into(),
                parts: vec![MessagePart::text("first")],
                created_at: None,
                completed_at: None,
            },
            Message {
                role: "user".into(),
                parts: vec![MessagePart::text("second")],
                created_at: None,
                completed_at: None,
            },
        ];
        assert_eq!(extract_text_from_messages(&messages), "first\nsecond");
    }

    #[test]
    fn extract_text_empty_messages() {
        let messages: Vec<Message> = vec![];
        assert_eq!(extract_text_from_messages(&messages), "");
    }

    #[test]
    fn extract_text_skips_none_content() {
        let messages = vec![Message {
            role: "user".into(),
            parts: vec![MessagePart::url("text/plain", "http://example.com")],
            created_at: None,
            completed_at: None,
        }];
        assert_eq!(extract_text_from_messages(&messages), "");
    }

    #[test]
    fn extract_text_mixed_content_types() {
        let messages = vec![Message {
            role: "user".into(),
            parts: vec![
                MessagePart::text("text content"),
                MessagePart::url("text/plain", "http://example.com"),
                MessagePart::artifact("file.rs", "text/plain", "fn main() {}"),
            ],
            created_at: None,
            completed_at: None,
        }];
        assert_eq!(
            extract_text_from_messages(&messages),
            "text content\nfn main() {}"
        );
    }
}
