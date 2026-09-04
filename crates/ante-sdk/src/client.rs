//! The client end of one connection: an op sender and an event receiver.

use ante_protocol_shape::{EventMsg, Evt, Op, OpMsg, op_msg};
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender, error::TrySendError};

/// The connection is gone: the host end no longer receives ops.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("connection closed")]
pub struct Closed;

/// Sends ops to the host. Clone freely; every clone feeds the same connection.
#[derive(Clone)]
pub struct OpSender {
    inner: Sender<OpMsg>,
}

impl OpSender {
    pub fn new(inner: Sender<OpMsg>) -> Self {
        Self { inner }
    }

    /// Non-blocking send for callers that cannot await (UI event handlers).
    /// A full channel is a backpressure bug and is logged at error level; a
    /// closed one is expected during shutdown and logged at debug.
    pub fn try_send(&self, msg: OpMsg) {
        match self.inner.try_send(msg) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => tracing::error!("op channel full, dropping message"),
            Err(TrySendError::Closed(_)) => {
                tracing::debug!("op channel closed, dropping message")
            }
        }
    }

    pub async fn send(&self, msg: OpMsg) -> Result<(), Closed> {
        self.inner.send(msg).await.map_err(|_| Closed)
    }
}

pub type EventReceiver = Receiver<EventMsg>;

/// One connection to an Ante host, seen from the client. Which session it
/// drives is decided by the ops sent over it (`StartSession`,
/// `ResumeSession`), never by how it was opened.
pub struct Client {
    sender: OpSender,
    receiver: EventReceiver,
}

impl Client {
    /// Assemble a client from its two halves — the inverse of
    /// [`Self::into_parts`]. Hosts and connectors build clients this way.
    pub fn from_parts(sender: OpSender, receiver: EventReceiver) -> Self {
        Self { sender, receiver }
    }

    /// Split into the op sender and the event receiver, for callers that
    /// drive the two sides from different places.
    pub fn into_parts(self) -> (OpSender, EventReceiver) {
        (self.sender, self.receiver)
    }

    pub fn sender(&self) -> OpSender {
        self.sender.clone()
    }

    pub async fn send(&self, op: Op) -> Result<(), Closed> {
        self.sender.send(op_msg(op)).await
    }

    /// The next event, or `None` once the connection has ended.
    pub async fn next_event(&mut self) -> Option<EventMsg> {
        self.receiver.recv().await
    }

    /// End this connection's session and wait for the host to confirm.
    /// Sends `Shutdown`, then drains events until `Goodbye` or the end of
    /// the stream; the host emits `Goodbye` only after the session has been
    /// torn down and persisted. A failed send is not an error here: a closed
    /// channel means the connection already ended.
    pub async fn close(mut self) {
        let _ = self.send(Op::Shutdown).await;
        while let Some(msg) = self.receiver.recv().await {
            if matches!(msg.event, Evt::Goodbye) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante_protocol_shape::event_msg;

    #[tokio::test]
    async fn close_stops_at_goodbye_and_tolerates_a_gone_host() {
        let (op_tx, op_rx) = tokio::sync::mpsc::channel(1);
        let (evt_tx, evt_rx) = tokio::sync::mpsc::channel(4);
        // The host end is already gone: sending Shutdown fails, and the
        // pre-queued events end with Goodbye ahead of a trailing event that
        // must not be waited for.
        drop(op_rx);
        for event in [Evt::Info("bye".into()), Evt::Goodbye, Evt::Info("late".into())] {
            evt_tx.try_send(event_msg(event, None)).expect("queue event");
        }

        let client = Client::from_parts(OpSender::new(op_tx), evt_rx);
        tokio::time::timeout(std::time::Duration::from_secs(1), client.close())
            .await
            .expect("close must return once Goodbye is seen");
    }
}
