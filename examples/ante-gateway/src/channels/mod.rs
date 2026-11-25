pub mod access;
pub mod config;
pub mod discord;
pub mod slack;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// An inbound message from any platform, normalized to plain text.
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub platform: &'static str,
    pub channel_id: String,
    pub thread_id: Option<String>,
    pub sender_id: String,
    pub text: String,
}

/// Minimal interface for a messaging platform.
///
/// Each implementation spawns its own background task that pushes
/// `InboundMessage`s to the provided sender. The gateway loop reads
/// from the receiver side.
#[async_trait]
pub trait Channel: Send + Sync {
    fn platform(&self) -> &'static str;

    /// Connect to the platform and start forwarding messages to `tx`.
    /// Should spawn background tasks and return immediately.
    async fn start(&self, tx: mpsc::Sender<InboundMessage>) -> Result<()>;

    /// Send a text reply to a specific conversation.
    async fn send_text(&self, channel_id: &str, text: &str, thread_id: Option<&str>) -> Result<()>;
}
