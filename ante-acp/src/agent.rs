//! The ACP agent: answers the client's requests over stdio.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    CancelNotification, Implementation, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, SessionNotification, SetSessionModeRequest, SetSessionModeResponse,
};
use agent_client_protocol::{
    Agent, ConnectionTo, Error, ErrorCode, Stdio, on_receive_notification, on_receive_request,
};
use ante_sdk::protocol::{Op, op_msg};
use ante_sdk::{ConnectOptions, Endpoint, EventReceiver, OpSender};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::{ante_bin, session};

/// What every handler shares: the `ante` to drive and the live sessions of
/// this connection, keyed by ACP session id.
struct State {
    executable: PathBuf,
    sessions: Mutex<HashMap<String, OpSender>>,
}

/// Serve ACP on this process's stdin/stdout until the client closes them.
pub async fn run(executable: PathBuf) -> Result<(), Error> {
    let state = Arc::new(State { executable, sessions: Mutex::new(HashMap::new()) });
    let (init, new, mode, cancel) = (state.clone(), state.clone(), state.clone(), state);
    Agent
        .builder()
        .name("ante-acp")
        .on_receive_request(
            async move |_request: InitializeRequest, responder, _connection| {
                match ante_bin::check_version(&init.executable).await {
                    Ok(()) => responder
                        .respond(InitializeResponse::new(ProtocolVersion::V1).agent_info(
                            Implementation::new("ante-acp", env!("CARGO_PKG_VERSION")),
                        )),
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
            async move |request: SetSessionModeRequest, responder, _connection| {
                let Some(permission_mode) = session::parse_mode(&request.mode_id.0) else {
                    return responder.respond_with_error(error_with(
                        ErrorCode::InvalidParams,
                        format!("unknown mode `{}`", request.mode_id.0),
                    ));
                };
                let Some(ops) = mode.sessions.lock().await.get(&*request.session_id.0).cloned()
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
                    Some(ops) => ops.try_send(op_msg(Op::Interrupt)),
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
    let started = session::start(client, cwd).await?;
    state.sessions.lock().await.insert(started.id.clone(), started.ops);
    info!(session = %started.id, "ante session started");
    tokio::spawn(pump(started.id.clone(), started.events, connection, state.clone()));
    Ok((started.id, started.mode))
}

async fn pump(
    id: String,
    mut events: EventReceiver,
    connection: ConnectionTo<agent_client_protocol::Client>,
    state: Arc<State>,
) {
    while let Some(msg) = events.recv().await {
        if let Some(update) = session::update_for(&msg.event)
            && let Err(error) =
                connection.send_notification(SessionNotification::new(id.clone(), update))
        {
            warn!(session = %id, %error, "could not notify the client");
            break;
        }
    }
    state.sessions.lock().await.remove(&id);
    info!(session = %id, "ante session ended");
}

fn error_with(code: ErrorCode, message: String) -> Error {
    Error::new(code.into(), message)
}

fn unknown_session(id: &str) -> Error {
    error_with(ErrorCode::InvalidParams, format!("unknown session `{id}`"))
}
