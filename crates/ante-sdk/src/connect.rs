//! Open a connection to a host at an [`Endpoint`].

use std::{collections::BTreeMap, path::PathBuf, process::Stdio};

use ante_protocol_shape::{EventMsg, OpMsg};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{Client, Endpoint, OpSender};

/// How to reach or start the host. Every field is optional.
#[derive(Debug, Clone, Default)]
pub struct ConnectOptions {
    /// The `ante` executable for [`Endpoint::Stdio`]. Defaults to `ante` on
    /// `PATH`.
    pub executable: Option<PathBuf>,
    /// Extra arguments appended after `serve --stdio` (for example
    /// `--offline-model <path>`).
    pub args: Vec<String>,
    /// Working directory for the spawned host. Defaults to the caller's.
    pub cwd: Option<PathBuf>,
    /// Environment added to the spawned host's; the caller's environment is
    /// inherited otherwise.
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("`{0}` endpoints are not supported by this build")]
    Unsupported(Endpoint),
    #[error("could not find the `ante` executable on PATH: {0}")]
    ExecutableNotFound(#[source] which::Error),
    #[error("failed to spawn `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("spawned host has no stdin/stdout pipe")]
    MissingPipe,
    #[error("failed to dial `{endpoint}`: {source}")]
    Dial {
        endpoint: Endpoint,
        #[source]
        source: std::io::Error,
    },
}

/// Depths of the bridge between the client and the host's byte stream.
const OP_CHANNEL: usize = 256;
const EVENT_CHANNEL: usize = 4096;

/// Connect to the host at `endpoint`. Success is transport-level: the pipe
/// opened or the socket dialed. There is no greeting yet, so the first op's
/// reply is the liveness check.
pub async fn connect(endpoint: Endpoint, options: ConnectOptions) -> Result<Client, ConnectError> {
    match endpoint {
        Endpoint::Stdio => connect_stdio(options),
        #[cfg(unix)]
        Endpoint::Unix(path) => connect_unix(path).await,
        other => Err(ConnectError::Unsupported(other)),
    }
}

/// Spawn `ante serve --stdio` and bridge its pipes onto a client's channel
/// pair. Dropping every `OpSender` closes the child's stdin, which is how
/// the child learns to shut down; its exit ends the event stream, and the
/// task that read it reaps the child.
fn connect_stdio(options: ConnectOptions) -> Result<Client, ConnectError> {
    let executable = match options.executable {
        Some(path) => path,
        None => which::which("ante").map_err(ConnectError::ExecutableNotFound)?,
    };

    let mut command = Command::new(&executable);
    command
        .arg("serve")
        .arg("--stdio")
        .args(&options.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }
    command.envs(&options.env);

    let mut child = command.spawn().map_err(|source| ConnectError::Spawn {
        command: format!("{} serve --stdio", executable.display()),
        source,
    })?;
    let stdin = child.stdin.take().ok_or(ConnectError::MissingPipe)?;
    let stdout = child.stdout.take().ok_or(ConnectError::MissingPipe)?;

    let (op_tx, op_rx) = mpsc::channel(OP_CHANNEL);
    let (evt_tx, evt_rx) = mpsc::channel(EVENT_CHANNEL);
    tokio::spawn(pump_ops(op_rx, stdin));
    tokio::spawn(async move {
        pump_events(stdout, evt_tx).await;
        match child.wait().await {
            Ok(status) => tracing::debug!(%status, "spawned host exited"),
            Err(error) => tracing::warn!("waiting for the spawned host failed: {error}"),
        }
    });

    Ok(Client::from_parts(OpSender::new(op_tx), evt_rx))
}

/// Dial the socket file an `ante serve --sock` host listens on and bridge
/// it onto a client's channel pair. Dropping every `OpSender` shuts down
/// the write side, which is how the host learns the peer is gone; the host
/// closing the socket ends the event stream.
#[cfg(unix)]
async fn connect_unix(path: PathBuf) -> Result<Client, ConnectError> {
    let stream = tokio::net::UnixStream::connect(&path)
        .await
        .map_err(|source| ConnectError::Dial { endpoint: Endpoint::Unix(path), source })?;
    let (reader, writer) = stream.into_split();

    let (op_tx, op_rx) = mpsc::channel(OP_CHANNEL);
    let (evt_tx, evt_rx) = mpsc::channel(EVENT_CHANNEL);
    tokio::spawn(pump_ops(op_rx, writer));
    tokio::spawn(pump_events(reader, evt_tx));

    Ok(Client::from_parts(OpSender::new(op_tx), evt_rx))
}

/// Write each op to the host as one JSON line, until the client drops its
/// senders or the host stops reading. Dropping `writer` at the end is what
/// tells the host the client is gone.
async fn pump_ops<W: AsyncWrite + Unpin>(mut ops: Receiver<OpMsg>, mut writer: W) {
    while let Some(msg) = ops.recv().await {
        let mut line = match serde_json::to_vec(&msg) {
            Ok(line) => line,
            Err(error) => {
                tracing::error!("failed to encode op for the host: {error}");
                continue;
            }
        };
        line.push(b'\n');
        if let Err(error) = writer.write_all(&line).await {
            tracing::debug!("host stopped reading ops: {error}");
            break;
        }
    }
}

/// Parse each line the host writes into an event, until the stream ends.
/// Dropping `events` at the end is what ends the client's event stream.
async fn pump_events<R: AsyncRead + Unpin>(reader: R, events: Sender<EventMsg>) {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let frame = line.trim_ascii();
                if frame.is_empty() {
                    continue;
                }
                match serde_json::from_slice::<EventMsg>(frame) {
                    Ok(msg) => {
                        if events.send(msg).await.is_err() {
                            // The client stopped listening. Keep the stream
                            // drained so a host blocked on a full write
                            // buffer can still notice the peer going away.
                            let _ = tokio::io::copy(&mut reader, &mut tokio::io::sink()).await;
                            break;
                        }
                    }
                    Err(error) => {
                        tracing::warn!("dropping an unparseable event line from the host: {error}")
                    }
                }
            }
            Err(error) => {
                tracing::warn!("reading the host's events failed: {error}");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ws_endpoints_report_unsupported_until_their_connector_lands() {
        let endpoint = Endpoint::Ws("ws://127.0.0.1:1".to_string());
        let error = connect(endpoint.clone(), ConnectOptions::default())
            .await
            .err()
            .expect("ws endpoints are not connectable yet");
        assert!(matches!(error, ConnectError::Unsupported(e) if e == endpoint));
    }

    #[tokio::test]
    async fn a_missing_executable_is_a_spawn_error() {
        let options = ConnectOptions {
            executable: Some(PathBuf::from("/definitely/not/ante")),
            ..Default::default()
        };
        let error = connect(Endpoint::Stdio, options).await.err().expect("spawn must fail");
        assert!(matches!(error, ConnectError::Spawn { .. }), "got {error:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_missing_socket_file_is_a_dial_error() {
        let endpoint = Endpoint::Unix(PathBuf::from("/definitely/not/ante.sock"));
        let error = connect(endpoint.clone(), ConnectOptions::default())
            .await
            .err()
            .expect("dialing a missing socket file must fail");
        assert!(
            matches!(error, ConnectError::Dial { endpoint: ref dialed, .. } if *dialed == endpoint),
            "got {error:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_endpoint_dials_a_socket_file_and_round_trips() {
        use ante_protocol_shape::{Evt, Op, event_msg};

        let path = std::env::temp_dir()
            .join(format!("ante-sdk-unix-roundtrip-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = tokio::net::UnixListener::bind(&path).expect("bind test socket");

        let mut client = connect(Endpoint::Unix(path.clone()), ConnectOptions::default())
            .await
            .expect("dial the test socket");
        let (stream, _) = listener.accept().await.expect("accept the client");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        // Client → host: one op, one line.
        client.send(Op::Interrupt).await.expect("send an op");
        let line = lines.next_line().await.expect("read the op line").expect("an op line");
        let op: OpMsg = serde_json::from_str(&line).expect("the line is an OpMsg");
        assert!(matches!(op.op, Op::Interrupt), "got {op:?}");

        // Host → client: one event line, correlated to the op.
        let mut reply = serde_json::to_vec(&event_msg(Evt::Goodbye, Some(op.id))).expect("encode");
        reply.push(b'\n');
        writer.write_all(&reply).await.expect("write the event line");
        let event = client.next_event().await.expect("the event arrives");
        assert!(
            matches!(event.event, Evt::Goodbye) && event.parent == Some(op.id),
            "got {event:?}"
        );

        // The host closing the socket ends the event stream.
        drop(writer);
        drop(lines);
        assert!(client.next_event().await.is_none(), "stream end must end the events");

        let _ = std::fs::remove_file(&path);
    }
}
