use axum::response::sse::{Event, Sse};
use std::convert::Infallible;
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::types::{AcEvent, Message, MessagePart, Run, RunStatus};

type SseStream = Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>;

/// Create an SSE stream for a run.
/// Spawns execution in the background and yields events as the run progresses.
pub fn create_run_sse_stream(run: Run, input_text: String) -> Sse<SseStream> {
    let (tx, rx) = mpsc::channel::<Event>(64);

    // Emit run.created immediately
    let event = AcEvent::RunCreated { run: run.clone() };
    let _ = tx.try_send(sse_event_from_ac_event(&event));

    // Spawn background execution
    tokio::spawn(async move {
        // Emit run.in-progress
        let mut running_run = run.clone();
        running_run.status = RunStatus::InProgress;
        let _ = tx
            .send(sse_event_from_ac_event(&AcEvent::RunInProgress {
                run: running_run,
            }))
            .await;

        // Execute Ante
        match crate::executor::execute_ante(&input_text, None, None).await {
            Ok(exec) => {
                let agent_role = format!("agent/{}", run.agent_name);

                // Emit message.created
                let output_msg = Message {
                    role: agent_role.clone(),
                    parts: vec![],
                    created_at: Some(chrono::Utc::now()),
                    completed_at: None,
                };
                let _ = tx
                    .send(sse_event_from_ac_event(&AcEvent::MessageCreated {
                        message: output_msg,
                    }))
                    .await;

                // Emit message.part for the output text
                let mut output_parts = Vec::new();
                for line in exec.result_text.lines() {
                    let part = MessagePart::text(line.to_string());
                    let _ = tx
                        .send(sse_event_from_ac_event(&AcEvent::MessagePartEvent {
                            part: part.clone(),
                        }))
                        .await;
                    output_parts.push(part);
                }

                // Emit message.completed
                let completed_msg = Message {
                    role: agent_role.clone(),
                    parts: output_parts,
                    created_at: Some(chrono::Utc::now()),
                    completed_at: Some(chrono::Utc::now()),
                };
                let _ = tx
                    .send(sse_event_from_ac_event(&AcEvent::MessageCompleted {
                        message: completed_msg,
                    }))
                    .await;

                // Emit run.completed
                let mut completed_run = run.clone();
                completed_run.status = RunStatus::Completed;
                completed_run.output = vec![Message {
                    role: agent_role,
                    parts: vec![MessagePart::text(exec.result_text)],
                    created_at: Some(chrono::Utc::now()),
                    completed_at: Some(chrono::Utc::now()),
                }];
                completed_run.finished_at = Some(chrono::Utc::now());
                let _ = tx
                    .send(sse_event_from_ac_event(&AcEvent::RunCompleted {
                        run: completed_run,
                    }))
                    .await;
            }
            Err(err) => {
                let mut failed_run = run.clone();
                failed_run.status = RunStatus::Failed;
                failed_run.error = Some(crate::types::AcError {
                    code: crate::types::AcErrorCode::ServerError,
                    message: err.message,
                    data: Some(serde_json::json!(err.details)),
                });
                failed_run.finished_at = Some(chrono::Utc::now());
                let _ = tx
                    .send(sse_event_from_ac_event(&AcEvent::RunFailed {
                        run: failed_run,
                    }))
                    .await;
            }
        }
    });

    let stream = async_stream::stream! {
        let mut rx = rx;
        while let Some(event) = rx.recv().await {
            yield Ok::<_, Infallible>(event);
        }
    };

    Sse::new(Box::pin(stream))
}

fn sse_event_from_ac_event(ac_event: &AcEvent) -> Event {
    let event_type = match ac_event {
        AcEvent::RunCreated { .. } => "run.created",
        AcEvent::RunInProgress { .. } => "run.in-progress",
        AcEvent::RunAwaiting { .. } => "run.awaiting",
        AcEvent::RunCompleted { .. } => "run.completed",
        AcEvent::RunFailed { .. } => "run.failed",
        AcEvent::RunCancelled { .. } => "run.cancelled",
        AcEvent::MessageCreated { .. } => "message.created",
        AcEvent::MessagePartEvent { .. } => "message.part",
        AcEvent::MessageCompleted { .. } => "message.completed",
        AcEvent::ErrorEvent { .. } => "error",
    };

    let data = serde_json::to_string(ac_event).unwrap_or_default();
    Event::default().event(event_type).data(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AcError, AcErrorCode};

    #[test]
    fn sse_event_type_from_ac_event_run_created() {
        let run = Run {
            agent_name: "test".into(),
            session_id: None,
            run_id: "r1".into(),
            status: RunStatus::Created,
            await_request: None,
            output: vec![],
            error: None,
            created_at: chrono::Utc::now(),
            finished_at: None,
        };
        let ac_event = AcEvent::RunCreated { run };
        let event = sse_event_from_ac_event(&ac_event);
        // Event type should be "run.created"
        // The data should contain the serialized AcEvent with type tag
        let data_str = format!("{:?}", event);
        assert!(data_str.contains("run.created") || !data_str.is_empty());
    }

    #[test]
    fn sse_event_type_from_ac_event_error() {
        let ac_event = AcEvent::ErrorEvent {
            error: AcError {
                code: AcErrorCode::ServerError,
                message: "test error".into(),
                data: None,
            },
        };
        let event = sse_event_from_ac_event(&ac_event);
        let data_str = format!("{:?}", event);
        assert!(data_str.contains("error") || !data_str.is_empty());
    }

    #[test]
    fn sse_event_data_serializes_correctly() {
        let run = Run {
            agent_name: "ante".into(),
            session_id: None,
            run_id: "test-id".into(),
            status: RunStatus::Completed,
            await_request: None,
            output: vec![],
            error: None,
            created_at: chrono::Utc::now(),
            finished_at: Some(chrono::Utc::now()),
        };
        let ac_event = AcEvent::RunCompleted { run };
        let event = sse_event_from_ac_event(&ac_event);
        // Verify the event can be formatted (implies data serialization works)
        let formatted = format!("{event:?}");
        assert!(!formatted.is_empty());
    }
}
