use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use regex::Regex;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use super::{Channel, InboundMessage};

pub struct SlackChannel {
    bot_token: String,
    app_token: String,
}

impl SlackChannel {
    pub fn new(bot_token: String, app_token: String) -> Self {
        Self { bot_token, app_token }
    }

    /// Fetch the bot's own user ID so we can ignore self-messages.
    async fn resolve_bot_id(&self) -> Result<String> {
        let client = crate::http::client();
        let resp: Value = client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&self.bot_token)
            .send()
            .await?
            .json()
            .await?;
        resp["user_id"].as_str().context("auth.test did not return user_id").map(String::from)
    }

    /// Call apps.connections.open to get the Socket Mode WSS URL.
    async fn open_socket_url(&self) -> Result<String> {
        let client = crate::http::client();
        let resp: Value = client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(&self.app_token)
            .send()
            .await?
            .json()
            .await?;
        if resp["ok"].as_bool() != Some(true) {
            bail!("apps.connections.open failed: {}", resp["error"]);
        }
        resp["url"].as_str().map(String::from).context("missing url in response")
    }
}

/// Strip all `<@U...>` mentions from text and normalize whitespace.
fn strip_mentions(text: &str) -> String {
    let re = Regex::new(r"<@[^>]+>").unwrap();
    let stripped = re.replace_all(text, "");
    // Collapse multiple spaces into one and trim.
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check whether the message text contains a mention of the given bot user ID.
fn contains_bot_mention(text: &str, bot_user_id: &str) -> bool {
    let pattern = format!("<@{bot_user_id}>");
    text.contains(&pattern)
}

/// DMs have channel IDs starting with 'D'.
fn is_dm(channel_id: &str) -> bool {
    channel_id.starts_with('D')
}

#[async_trait]
impl Channel for SlackChannel {
    fn platform(&self) -> &'static str {
        "slack"
    }

    async fn start(&self, tx: mpsc::Sender<InboundMessage>) -> Result<()> {
        let bot_id = self.resolve_bot_id().await?;
        info!(%bot_id, "Resolved Slack bot user");

        let wss_url = self.open_socket_url().await?;
        info!("Connecting Slack Socket Mode");

        let (ws, _) = tokio_tungstenite::connect_async(&wss_url)
            .await
            .context("failed to connect Slack Socket Mode")?;
        info!("Slack Socket Mode connected");

        let bot_user_id = bot_id.clone();
        let (mut ws_write, mut ws_read) = ws.split();

        tokio::spawn(async move {
            while let Some(frame) = ws_read.next().await {
                let msg = match frame {
                    Ok(Message::Text(t)) => t,
                    Ok(Message::Ping(p)) => {
                        if let Err(e) = ws_write.send(Message::Pong(p)).await {
                            warn!("Slack pong failed: {e}");
                        }
                        continue;
                    }
                    Ok(Message::Close(_)) => {
                        info!("Slack WebSocket closed");
                        break;
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        error!("Slack WebSocket error: {e}");
                        break;
                    }
                };

                let Ok(envelope) = serde_json::from_str::<Value>(&msg) else {
                    continue;
                };

                // Ack the envelope immediately.
                if let Some(eid) = envelope["envelope_id"].as_str() {
                    let ack = serde_json::json!({"envelope_id": eid});
                    if let Err(e) = ws_write.send(Message::Text(ack.to_string().into())).await {
                        warn!("Slack ack failed: {e}");
                    }
                }

                // Only handle events_api messages.
                if envelope["type"].as_str() != Some("events_api") {
                    continue;
                }

                let event = &envelope["payload"]["event"];
                let event_type = event["type"].as_str().unwrap_or("");

                // Accept "message" and "app_mention" events.
                if event_type != "message" && event_type != "app_mention" {
                    continue;
                }

                // Skip message subtypes (edits, deletes, bot_message, etc).
                if event_type == "message" && event["subtype"].is_string() {
                    continue;
                }

                let Some(user) = event["user"].as_str() else { continue };
                if user == bot_user_id {
                    continue;
                }
                let Some(raw_text) = event["text"].as_str() else { continue };
                let Some(channel_id) = event["channel"].as_str() else { continue };

                // In channels: only respond if the bot is @mentioned.
                // In DMs: always respond.
                if !is_dm(channel_id) && !contains_bot_mention(raw_text, &bot_user_id) {
                    continue;
                }

                // Strip @mentions from the text before sending to the agent.
                let clean_text = strip_mentions(raw_text);
                if clean_text.is_empty() {
                    continue;
                }

                // Use the message ts as thread_id so replies go to a thread.
                // If already in a thread, use the existing thread_ts.
                let thread_id =
                    event["thread_ts"].as_str().or_else(|| event["ts"].as_str()).map(String::from);

                let inbound = InboundMessage {
                    platform: "slack",
                    channel_id: channel_id.to_string(),
                    thread_id,
                    sender_id: user.to_string(),
                    text: clean_text,
                };

                if tx.send(inbound).await.is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    async fn send_text(&self, channel_id: &str, text: &str, thread_id: Option<&str>) -> Result<()> {
        let client = crate::http::client();
        let mut body = serde_json::json!({
            "channel": channel_id,
            "text": text,
        });
        if let Some(ts) = thread_id {
            body["thread_ts"] = Value::String(ts.to_string());
        }
        let resp: Value = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        if resp["ok"].as_bool() != Some(true) {
            bail!("chat.postMessage failed: {}", resp["error"]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_mentions_removes_slack_mentions_and_normalizes_text() {
        for (input, expected) in [
            ("<@U123> hello", "hello"),
            ("<@U123> <@U456> hi there", "hi there"),
            ("<@U123>", ""),
            ("just text", "just text"),
        ] {
            assert_eq!(strip_mentions(input), expected, "input: {input}");
        }
    }

    #[test]
    fn detect_bot_mention() {
        assert!(contains_bot_mention("<@U123> do something", "U123"));
        assert!(!contains_bot_mention("<@U456> do something", "U123"));
        assert!(!contains_bot_mention("no mention here", "U123"));
    }

    #[test]
    fn dm_detection() {
        assert!(is_dm("D01ABC23DEF"));
        assert!(!is_dm("C01ABC23DEF"));
        assert!(!is_dm("G01ABC23DEF"));
    }
}
