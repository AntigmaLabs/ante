use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::AcServerError;
use crate::state::AcState;
use crate::types::*;

/// GET /ping
pub async fn ping_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

/// GET /agents
pub async fn agents_list_handler(State(state): State<AcState>) -> Json<AgentsListResponse> {
    let manifest = state.manifest();
    Json(AgentsListResponse {
        agents: vec![manifest],
    })
}

/// GET /agents/:name
pub async fn agents_get_handler(
    State(state): State<AcState>,
    Path(name): Path<String>,
) -> Result<Json<AgentManifest>, AcServerError> {
    if name != "ante" {
        return Err(AcServerError::NotFound(format!(
            "agent '{name}' not found"
        )));
    }
    Ok(Json(state.manifest()))
}

/// POST /runs — Create and start a new run
pub async fn run_create_handler(
    State(state): State<AcState>,
    Json(req): Json<RunCreateRequest>,
) -> Result<Response, AcServerError> {
    // Validate agent name
    if req.agent_name != "ante" {
        return Err(AcServerError::NotFound(format!(
            "agent '{}' not found",
            req.agent_name
        )));
    }

    // Validate input
    if req.input.is_empty() {
        return Err(AcServerError::InvalidInput(
            "input must contain at least one message".into(),
        ));
    }

    // Validate messages have content
    for msg in &req.input {
        if msg.parts.is_empty() {
            return Err(AcServerError::InvalidInput(
                "each message must have at least one part".into(),
            ));
        }
    }

    let mode = req.mode.clone().unwrap_or(RunMode::Sync);

    match mode {
        RunMode::Sync => {
            // Create run, execute synchronously, return completed run
            let run = state.create_run(req).await?;
            let completed_run = state.execute_run_sync(&run.run_id).await?;
            Ok((StatusCode::OK, Json(completed_run)).into_response())
        }
        RunMode::Async => {
            // Create run, spawn execution in background, return immediately
            let run = state.create_run(req).await?;
            state.spawn_run_execution(&run.run_id);
            Ok((StatusCode::ACCEPTED, Json(run)).into_response())
        }
        RunMode::Stream => {
            // Create run, return SSE stream of events
            let input_text = crate::state::extract_text_from_messages(&req.input);
            let run = state.create_run(req).await?;
            let sse_stream = crate::sse::create_run_sse_stream(run, input_text);
            Ok(sse_stream.into_response())
        }
    }
}

/// GET /runs/:run_id — Get run status
pub async fn run_get_handler(
    State(state): State<AcState>,
    Path(run_id): Path<String>,
) -> Result<Json<Run>, AcServerError> {
    Ok(Json(state.get_run(&run_id).await?))
}

/// POST /runs/:run_id — Resume an awaiting run
pub async fn run_resume_handler(
    State(state): State<AcState>,
    Path(run_id): Path<String>,
    Json(req): Json<RunResumeRequest>,
) -> Result<Json<Run>, AcServerError> {
    Ok(Json(state.resume_run(&run_id, req).await?))
}

/// POST /runs/:run_id/cancel — Cancel a run
pub async fn run_cancel_handler(
    State(state): State<AcState>,
    Path(run_id): Path<String>,
) -> Result<(StatusCode, Json<Run>), AcServerError> {
    let (status, run) = state.cancel_run(&run_id).await?;
    Ok((status, Json(run)))
}
