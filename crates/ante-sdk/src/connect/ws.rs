//! The `ws://` and `wss://` connector: one text frame per op or event.

use std::net::IpAddr;

use ante_protocol_shape::{EventMsg, OpMsg};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::{
    mpsc::{Receiver, UnboundedSender},
    oneshot,
};
use tokio_tungstenite::tungstenite::{ClientRequestBuilder, Error as WsError, Message, http::Uri};

use super::{ConnectError, bridge};
use crate::{Client, Endpoint};

/// Dial the websocket URL an `ante serve --ws` host listens on and bridge
/// its frames onto a client's channel pair. Plain `ws://` is dialed only to
/// a loopback host; the bearer token rides the upgrade request and must not
/// cross a network in clear. `wss://` needs the `wss` feature; without it
/// the endpoint is unsupported. Dropping every `OpSender` sends the close
/// frame, which is how the host learns the peer is gone; the host closing
/// the socket ends the event stream.
pub(super) async fn connect(url: String, token: Option<String>) -> Result<Client, ConnectError> {
    let uri: Uri = match url.parse() {
        Ok(uri) => uri,
        Err(error) => {
            return Err(ConnectError::Handshake {
                endpoint: Endpoint::Ws(url),
                source: Box::new(error),
            });
        }
    };
    let endpoint = Endpoint::Ws(url);
    if uri.scheme_str() == Some("ws") && !is_loopback_host(&uri) {
        return Err(ConnectError::Insecure(endpoint));
    }
    #[cfg(not(feature = "wss"))]
    if uri.scheme_str() == Some("wss") {
        return Err(ConnectError::Unsupported(endpoint));
    }

    let mut request = ClientRequestBuilder::new(uri);
    if let Some(token) = token {
        request = request.with_header("Authorization", format!("Bearer {token}"));
    }
    // Nagle off (the `true`): ops are small frames sent back to back, and
    // each must leave at once rather than wait on the previous one's ack.
    #[cfg(feature = "wss")]
    let dialed = match tls_connector() {
        Ok(connector) => {
            tokio_tungstenite::connect_async_tls_with_config(request, None, true, Some(connector))
                .await
        }
        Err(error) => Err(error),
    };
    #[cfg(not(feature = "wss"))]
    let dialed = tokio_tungstenite::connect_async_with_config(request, None, true).await;
    let (socket, _response) = dialed.map_err(|error| match error {
        WsError::Io(source) => ConnectError::Dial { endpoint, source },
        WsError::Http(response) => {
            ConnectError::Refused { endpoint, status: response.status().as_u16() }
        }
        other => ConnectError::Handshake { endpoint, source: Box::new(other) },
    })?;
    let (writer, reader) = socket.split();

    let (client, op_rx, evt_tx) = bridge();
    let (reader_done, peer_gone) = oneshot::channel();
    tokio::spawn(pump_ops(op_rx, writer, peer_gone));
    tokio::spawn(pump_events(reader, evt_tx, reader_done));
    Ok(client)
}

/// Whether the URL's host is the local machine by address (`127.0.0.0/8`,
/// `::1`) or by the conventional name.
fn is_loopback_host(uri: &Uri) -> bool {
    let Some(host) = uri.host() else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// The TLS client for `wss://`: rustls on aws-lc-rs with the bundled Mozilla
/// roots. The provider is chosen here rather than left to rustls's process
/// default, which panics when a consumer's dependency graph enables a second
/// provider (`ring`) without installing one.
#[cfg(feature = "wss")]
fn tls_connector() -> Result<tokio_tungstenite::Connector, WsError> {
    use std::sync::Arc;

    use rustls::{ClientConfig, RootCertStore, crypto::aws_lc_rs};
    use tokio_tungstenite::{Connector, tungstenite::error::TlsError};

    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|error| TlsError::Rustls(Box::new(error)))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

/// Send each op to the host as one text frame, until the client drops its
/// senders or the host's side ends (`peer_gone` resolves when the reading
/// pump exits). Either way the socket is then closed: that sends the
/// client's close frame, or flushes the reply tungstenite queued for the
/// host's, so the close handshake completes without waiting on the client
/// to drop anything.
async fn pump_ops(
    mut ops: Receiver<OpMsg>,
    mut writer: impl Sink<Message, Error = WsError> + Unpin,
    mut peer_gone: oneshot::Receiver<()>,
) {
    loop {
        let msg = tokio::select! {
            msg = ops.recv() => msg,
            _ = &mut peer_gone => None,
        };
        let Some(msg) = msg else { break };
        let payload = match serde_json::to_string(&msg) {
            Ok(payload) => payload,
            Err(error) => {
                tracing::error!("failed to encode op for the host: {error}");
                continue;
            }
        };
        if let Err(error) = writer.send(Message::Text(payload.into())).await {
            tracing::debug!("host stopped reading ops: {error}");
            return;
        }
    }
    if let Err(error) = writer.close().await {
        tracing::debug!("closing the websocket to the host failed: {error}");
    }
}

/// Parse each text frame the host sends into an event, until the host
/// closes the socket. Dropping `events` at the end is what ends the client's
/// event stream, and dropping `_reader_done` is what tells the writing pump
/// to close the socket.
async fn pump_events(
    mut reader: impl Stream<Item = Result<Message, WsError>> + Unpin,
    events: UnboundedSender<EventMsg>,
    _reader_done: oneshot::Sender<()>,
) {
    while let Some(frame) = reader.next().await {
        match frame {
            Ok(Message::Text(text)) => match serde_json::from_str::<EventMsg>(&text) {
                Ok(msg) => {
                    if events.send(msg).is_err() {
                        // The client stopped listening. Keep the socket
                        // drained so a host blocked on a full write buffer
                        // can still notice the peer going away.
                        while let Some(Ok(_)) = reader.next().await {}
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!("dropping an unparseable event frame from the host: {error}")
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(error) => {
                tracing::warn!("reading the host's events failed: {error}");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ante_protocol_shape::{Evt, Op, OpMsg, event_msg};
    use futures_util::{SinkExt, StreamExt};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use tokio_tungstenite::tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request, Response},
        http::header::AUTHORIZATION,
    };

    use super::is_loopback_host;
    use crate::{ConnectError, ConnectOptions, Endpoint, connect};

    fn options(token: &str) -> ConnectOptions {
        ConnectOptions { token: Some(token.to_string()), ..Default::default() }
    }

    /// A listener on a free loopback port and the `ws://` URL that reaches it.
    async fn listen() -> (TcpListener, String) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind a test listener");
        let url = format!("ws://{}", listener.local_addr().expect("local addr"));
        (listener, url)
    }

    /// Read one HTTP request's head off a raw stream, for a peer that is
    /// not a websocket server.
    async fn read_request_head(stream: &mut tokio::net::TcpStream) {
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).await.expect("read the upgrade request");
            head.push(byte[0]);
        }
    }

    #[tokio::test]
    async fn ws_endpoint_presents_the_bearer_and_round_trips_frames() {
        let (listener, url) = listen().await;
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept the client");
            let mut header = None;
            #[expect(clippy::result_large_err, reason = "signature fixed by the Callback trait")]
            let capture =
                |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
                    header = request.headers().get(AUTHORIZATION).cloned();
                    Ok(response)
                };
            let mut socket =
                tokio_tungstenite::accept_hdr_async(stream, capture).await.expect("upgrade");
            assert_eq!(header.as_ref().and_then(|h| h.to_str().ok()), Some("Bearer s3cret"));

            // Client → host: one op, one text frame.
            let frame = socket.next().await.expect("an op frame").expect("valid frame");
            let Message::Text(text) = frame else { panic!("expected a text frame, got {frame:?}") };
            let op: OpMsg = serde_json::from_str(&text).expect("the frame is an OpMsg");
            assert!(matches!(op.op, Op::Interrupt), "got {op:?}");

            // Host → client: one event frame, correlated to the op, then close.
            let reply =
                serde_json::to_string(&event_msg(Evt::Goodbye, Some(op.id))).expect("encode");
            socket.send(Message::Text(reply.into())).await.expect("send the event frame");
            socket.close(None).await.expect("close");
            // The client completes the close handshake on its own, while it
            // still holds its sender; the host must not be left waiting.
            let answer = tokio::time::timeout(Duration::from_secs(5), socket.next())
                .await
                .expect("the client answers the close frame");
            assert!(matches!(answer, Some(Ok(Message::Close(_)))), "got {answer:?}");
            op.id
        });

        let mut client = connect(Endpoint::Ws(url), options("s3cret")).await.expect("dial");
        client.send(Op::Interrupt).await.expect("send an op");
        let op_id = server.await.expect("server task");
        let event = client.next_event().await.expect("the event arrives");
        assert!(
            matches!(event.event, Evt::Goodbye) && event.parent == Some(op_id),
            "got {event:?}"
        );
        assert!(client.next_event().await.is_none(), "the host's close must end the events");
    }

    #[tokio::test]
    async fn a_refused_upgrade_reports_the_status() {
        let (listener, url) = listen().await;
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept the client");
            read_request_head(&mut stream).await;
            // What `ante serve --ws` answers a peer without the token.
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n")
                .await
                .expect("refuse the upgrade");
        });

        let endpoint = Endpoint::Ws(url);
        let error = connect(endpoint.clone(), ConnectOptions::default())
            .await
            .err()
            .expect("a 401 is not a connection");
        assert!(
            matches!(error, ConnectError::Refused { endpoint: ref e, status: 401 } if *e == endpoint),
            "got {error:?}"
        );
    }

    /// `wss://` needs a TLS stack with a crypto provider. A standalone
    /// consumer of this crate has only what the `wss` feature brings, so the
    /// handshake must fail on the far end, never on the near one. (rustls
    /// reports the far end's garbage as an io error, hence `Dial`.)
    #[cfg(feature = "wss")]
    #[tokio::test]
    async fn wss_reaches_the_tls_handshake() {
        let (listener, url) = listen().await;
        tokio::spawn(async move {
            // A peer that speaks no TLS: it answers the client hello in the clear.
            let (mut stream, _) = listener.accept().await.expect("accept the client");
            let mut hello = [0u8; 1];
            let _ = stream.read(&mut hello).await;
            let _ =
                stream.write_all(b"HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n").await;
        });

        let endpoint = Endpoint::Ws(url.replacen("ws://", "wss://", 1));
        let error =
            connect(endpoint.clone(), options("t")).await.err().expect("no TLS on the far end");
        assert!(
            matches!(error, ConnectError::Dial { endpoint: ref e, .. } if *e == endpoint),
            "got {error:?}"
        );
    }

    #[tokio::test]
    async fn plain_ws_is_dialed_to_loopback_only() {
        for url in ["ws://example.com:1", "ws://10.0.0.1:1", "ws://[2001:db8::1]:1"] {
            let endpoint = Endpoint::Ws(url.to_string());
            let error = connect(endpoint.clone(), options("t")).await.err().expect("refused");
            assert!(
                matches!(error, ConnectError::Insecure(ref e) if *e == endpoint),
                "got {error:?}"
            );
        }
        for url in ["ws://127.0.0.1:1", "ws://127.8.8.8:1", "ws://localhost:1", "ws://[::1]:1"] {
            assert!(
                is_loopback_host(&url.parse().expect("uri")),
                "{url} reaches the local machine"
            );
        }
    }
}
