//! The ACP agent: answers the client's requests over stdio.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, Implementation, InitializeRequest, InitializeResponse,
    NewSessionRequest, NewSessionResponse, PromptCapabilities, PromptRequest, PromptResponse,
    SessionNotification, SetSessionModeRequest, SetSessionModeResponse,
};
use agent_client_protocol::{
    Agent, ConnectionTo, Error, ErrorCode, Responder, Stdio, on_receive_notification,
    on_receive_request,
};
use ante_sdk::protocol::{Op, op_msg};
use ante_sdk::{ConnectOptions, Endpoint, EventReceiver, OpSender};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::turn::{Out, Translator};
use crate::{ante_bin, prompt, session};

/// What every handler shares: the `ante` to drive and the live sessions of
/// this connection, keyed by ACP session id.
struct State {
    executable: PathBuf,
    sessions: Mutex<HashMap<String, Session>>,
}

struct Session {
    ops: OpSender,
    /// A turn is running: further prompts steer it instead of starting one.
    turn_live: bool,
    /// Prompt requests answered when the current turn ends.
    pending: Vec<Responder<PromptResponse>>,
}

/// Serve ACP on this process's stdin/stdout until the client closes them.
pub async fn run(executable: PathBuf) -> Result<(), Error> {
    let state = Arc::new(State { executable, sessions: Mutex::new(HashMap::new()) });
    let (init, new, prompt, mode, cancel) =
        (state.clone(), state.clone(), state.clone(), state.clone(), state);
    Agent
        .builder()
        .name("ante-acp")
        .on_receive_request(
            async move |_request: InitializeRequest, responder, _connection| {
                match ante_bin::check_version(&init.executable).await {
                    Ok(()) => responder.respond(
                        InitializeResponse::new(ProtocolVersion::V1)
                            .agent_info(Implementation::new("ante-acp", env!("CARGO_PKG_VERSION")))
                            .agent_capabilities(AgentCapabilities::new().prompt_capabilities(
                                PromptCapabilities::new().image(true).embedded_context(true),
                            )),
                    ),
                    Err(error) => responder.respond_with_error(error_with(
                        ErrorCode::InternalError,
                        format!("{error:#}"),
                    )),
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async move |request: NewSessionRequest, responder, connection| {
                if !request.mcp_servers.is_empty() {
                    info!(count = request.mcp_servers.len(), "ignoring the client's MCP servers");
                }
                if !request.additional_directories.is_empty() {
                    info!(
                        count = request.additional_directories.len(),
                        "ignoring the client's additional directories"
                    );
                }
                // Starting a host takes a while; answer from a task so the
                // dispatch loop keeps serving other sessions meanwhile.
                let state = new.clone();
                let cx = connection.clone();
                connection.spawn(async move {
                    match new_session(&state, request.cwd, cx).await {
                        Ok((id, mode)) => responder
                            .respond(NewSessionResponse::new(id).modes(session::mode_state(mode))),
                        Err(error) => responder.respond_with_error(error_with(
                            ErrorCode::InternalError,
                            format!("{error:#}"),
                        )),
                    }
                })
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, _connection| {
                let text = match prompt::flatten(request.prompt, &prompt::image_cache_dir()) {
                    Ok(text) => text,
                    Err(error) => {
                        return responder.respond_with_error(error_with(
                            ErrorCode::InvalidParams,
                            format!("{error:#}"),
                        ));
                    }
                };
                let mut sessions = prompt.sessions.lock().await;
                let Some(session) = sessions.get_mut(&*request.session_id.0) else {
                    return responder.respond_with_error(unknown_session(&request.session_id.0));
                };
                // A prompt during a live turn steers it; every request that
                // rode the turn is answered when it ends. Known gap: a prompt
                // landing after the host ended the turn but before the pump
                // saw it is still treated as steering that turn.
                let op = if session.turn_live { Op::Steer(text) } else { Op::UserInput(text) };
                // Awaited so a gone host fails the prompt instead of parking
                // it; the channel only fills if the host stops taking ops.
                if session.ops.send(op_msg(op)).await.is_err() {
                    return responder.respond_with_error(error_with(
                        ErrorCode::InternalError,
                        "ante exited".to_string(),
                    ));
                }
                session.turn_live = true;
                session.pending.push(responder);
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionModeRequest, responder, _connection| {
                let Some(permission_mode) = session::parse_mode(&request.mode_id.0) else {
                    return responder.respond_with_error(error_with(
                        ErrorCode::InvalidParams,
                        format!("unknown mode `{}`", request.mode_id.0),
                    ));
                };
                let Some(ops) =
                    mode.sessions.lock().await.get(&*request.session_id.0).map(|s| s.ops.clone())
                else {
                    return responder.respond_with_error(unknown_session(&request.session_id.0));
                };
                let update = ante_sdk::protocol::SessionUpdate {
                    permission_mode: Some(permission_mode),
                    ..Default::default()
                };
                // Non-blocking: a host that stops taking ops must not stall
                // the dispatch loop.
                ops.try_send(op_msg(Op::UpdateSession(update)));
                responder.respond(SetSessionModeResponse::new())
            },
            on_receive_request!(),
        )
        .on_receive_notification(
            async move |notification: CancelNotification, _connection| {
                let id = &*notification.session_id.0;
                match cancel.sessions.lock().await.get(id) {
                    Some(session) => session.ops.try_send(op_msg(Op::Interrupt)),
                    None => warn!(session = id, "cancel for an unknown session"),
                }
                Ok(())
            },
            on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}

/// Spawn a host for `cwd`, start its session, and pump its events to the
/// client until the host goes away.
async fn new_session(
    state: &Arc<State>,
    cwd: PathBuf,
    connection: ConnectionTo<agent_client_protocol::Client>,
) -> anyhow::Result<(String, ante_sdk::protocol::PermissionMode)> {
    let options = ConnectOptions {
        executable: Some(state.executable.clone()),
        cwd: Some(cwd.clone()),
        ..Default::default()
    };
    let client = ante_sdk::connect(Endpoint::Stdio, options).await?;
    let started = session::start(client, cwd.clone()).await?;
    let session = Session { ops: started.ops, turn_live: false, pending: Vec::new() };
    state.sessions.lock().await.insert(started.id.clone(), session);
    info!(session = %started.id, "ante session started");
    tokio::spawn(pump(started.id.clone(), cwd, started.events, connection, state.clone()));
    Ok((started.id, started.mode))
}

async fn pump(
    id: String,
    cwd: PathBuf,
    mut events: EventReceiver,
    connection: ConnectionTo<agent_client_protocol::Client>,
    state: Arc<State>,
) {
    let mut translator = Translator::new(cwd);
    while let Some(msg) = events.recv().await {
        match translator.handle(msg.event) {
            Some(Out::Update(update)) => {
                if let Err(error) =
                    connection.send_notification(SessionNotification::new(id.clone(), *update))
                {
                    warn!(session = %id, %error, "could not notify the client");
                    break;
                }
            }
            Some(Out::TurnStarted) => {
                if let Some(session) = state.sessions.lock().await.get_mut(&id) {
                    session.turn_live = true;
                }
            }
            Some(Out::TurnEnded(outcome)) => {
                let pending = match state.sessions.lock().await.get_mut(&id) {
                    Some(session) => {
                        session.turn_live = false;
                        std::mem::take(&mut session.pending)
                    }
                    None => Vec::new(),
                };
                for responder in pending {
                    let _ = match &outcome {
                        Ok(reason) => responder.respond(PromptResponse::new(*reason)),
                        Err(error) => responder.respond_with_error(error.clone()),
                    };
                }
            }
            None => {}
        }
    }
    // The host is gone: nothing will answer what is still pending.
    let pending = state.sessions.lock().await.remove(&id).map(|s| s.pending).unwrap_or_default();
    for responder in pending {
        let _ = responder.respond_with_error(error_with(
            ErrorCode::InternalError,
            "ante exited before the turn ended".to_string(),
        ));
    }
    info!(session = %id, "ante session ended");
}

fn error_with(code: ErrorCode, message: String) -> Error {
    Error::new(code.into(), message)
}

fn unknown_session(id: &str) -> Error {
    error_with(ErrorCode::InvalidParams, format!("unknown session `{id}`"))
}
