use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use regex::Regex;

use super::{Channel, InboundMessage};

/// Discord Gateway intents:
/// GUILDS (1<<0) | GUILD_MESSAGES (1<<9) | DIRECT_MESSAGES (1<<12) | MESSAGE_CONTENT (1<<15)
const INTENTS: u64 = (1 << 0) | (1 << 9) | (1 << 12) | (1 << 15);
const API_BASE: &str = "https://discord.com/api/v10";

/// Strip all `<@123456>` mentions from text and normalize whitespace.
fn strip_mentions(text: &str) -> String {
    let re = Regex::new(r"<@!?\d+>").unwrap();
    let stripped = re.replace_all(text, "");
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check if message text mentions the given bot user ID.
fn contains_bot_mention(text: &str, bot_user_id: &str) -> bool {
    text.contains(&format!("<@{bot_user_id}>")) || text.contains(&format!("<@!{bot_user_id}>"))
}

/// DM channels in Discord have guild_id absent or null.
fn is_dm_event(d: &Value) -> bool {
    d["guild_id"].is_null()
}

fn inbound_message(d: &Value, bot_user_id: Option<&str>) -> Option<InboundMessage> {
    let author = &d["author"];
    let author_id = author["id"].as_str().unwrap_or("");
    if author["bot"].as_bool() == Some(true) || bot_user_id == Some(author_id) {
        return None;
    }

    let content = d["content"].as_str()?;
    let channel_id = d["channel_id"].as_str()?;
    let is_dm = is_dm_event(d);
    let referenced = &d["referenced_message"];
    let reply_to_bot =
        bot_user_id.is_some_and(|id| referenced["author"]["id"].as_str() == Some(id));
    let bot_mentioned = bot_user_id.is_some_and(|id| contains_bot_mention(content, id));
    if !is_dm && !bot_mentioned && !reply_to_bot {
        return None;
    }

    let text = strip_mentions(content);
    if text.is_empty() {
        return None;
    }

    let thread_id = if is_dm {
        None
    } else {
        // Every message we send references the conversation's original message.
        // Follow that reference when a user replies to us, so approval answers
        // and followups reach the same session instead of starting a new one.
        let original_id = if reply_to_bot {
            referenced["message_reference"]["message_id"].as_str()
        } else {
            None
        };
        original_id
            .or_else(|| d["message_reference"]["message_id"].as_str())
            .or_else(|| d["id"].as_str())
            .map(String::from)
    };

    Some(InboundMessage {
        platform: "discord",
        channel_id: channel_id.to_string(),
        thread_id,
        sender_id: author_id.to_string(),
        text,
    })
}

pub struct DiscordChannel {
    bot_token: String,
}

impl DiscordChannel {
    pub fn new(bot_token: String) -> Self {
        Self { bot_token }
    }

    async fn gateway_url(&self) -> Result<String> {
        let client = crate::http::client();
        let resp: Value = client
            .get(format!("{API_BASE}/gateway/bot"))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?
            .json()
            .await?;
        let url = resp["url"].as_str().context("missing gateway url")?;
        Ok(format!("{url}/?v=10&encoding=json"))
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn platform(&self) -> &'static str {
        "discord"
    }

    async fn start(&self, tx: mpsc::Sender<InboundMessage>) -> Result<()> {
        let wss_url = self.gateway_url().await?;
        info!("Discord Gateway connecting...");

        let (ws, _) = tokio_tungstenite::connect_async(&wss_url)
            .await
            .context("failed to connect Discord Gateway")?;
        info!("Discord Gateway connected");

        let token = self.bot_token.clone();
        let bot_id_cell: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));
        let bot_id_cell2 = Arc::clone(&bot_id_cell);
        let last_seq: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let last_seq2 = Arc::clone(&last_seq);

        let (mut ws_write, mut ws_read) = ws.split();

        // Read the Hello message to get heartbeat_interval.
        let hello = loop {
            let Some(Ok(Message::Text(t))) = ws_read.next().await else {
                bail!("Discord Gateway closed before Hello");
            };
            let v: Value = serde_json::from_str(&t)?;
            if v["op"].as_u64() == Some(10) {
                break v;
            }
        };
        let hb_interval =
            hello["d"]["heartbeat_interval"].as_u64().context("missing heartbeat_interval")?;

        // Send Identify.
        let identify = serde_json::json!({
            "op": 2,
            "d": {
                "token": token,
                "intents": INTENTS,
                "properties": {
                    "os": std::env::consts::OS,
                    "browser": "ante",
                    "device": "ante",
                }
            }
        });
        ws_write.send(Message::Text(identify.to_string().into())).await?;

        // Heartbeat task: sends op 1 at the required interval.
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(hb_interval));
            loop {
                interval.tick().await;
                let seq = last_seq2.load(Ordering::Relaxed);
                let payload = if seq == 0 {
                    serde_json::json!({"op": 1, "d": null})
                } else {
                    serde_json::json!({"op": 1, "d": seq})
                };
                if ws_write.send(Message::Text(payload.to_string().into())).await.is_err() {
                    break;
                }
            }
        });

        // Message read task.
        tokio::spawn(async move {
            while let Some(frame) = ws_read.next().await {
                let text = match frame {
                    Ok(Message::Text(t)) => t,
                    Ok(Message::Close(_)) => {
                        info!("Discord Gateway closed");
                        break;
                    }
                    Err(e) => {
                        error!("Discord WebSocket error: {e}");
                        break;
                    }
                    _ => continue,
                };

                let Ok(payload) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };

                let op = payload["op"].as_u64().unwrap_or(0);

                // Track sequence number.
                if let Some(s) = payload["s"].as_u64() {
                    last_seq.store(s, Ordering::Relaxed);
                }

                match op {
                    // Dispatch
                    0 => {
                        let event_type = payload["t"].as_str().unwrap_or("");

                        // Capture bot user id from READY.
                        if event_type == "READY" {
                            if let Some(id) = payload["d"]["user"]["id"].as_str() {
                                *bot_id_cell2.lock().unwrap() = Some(id.to_string());
                                info!("Discord bot user: {id}");
                            }
                            continue;
                        }

                        if event_type != "MESSAGE_CREATE" {
                            continue;
                        }

                        let Some(inbound) =
                            inbound_message(&payload["d"], bot_id_cell2.lock().unwrap().as_deref())
                        else {
                            continue;
                        };

                        if tx.send(inbound).await.is_err() {
                            break;
                        }
                    }
                    // Heartbeat ACK — nothing to do.
                    11 => {}
                    // Reconnect request.
                    7 => {
                        warn!("Discord requested reconnect");
                        break;
                    }
                    // Invalid session.
                    9 => {
                        warn!("Discord invalid session");
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    async fn send_text(&self, channel_id: &str, text: &str, thread_id: Option<&str>) -> Result<()> {
        let client = crate::http::client();
        let mut body = serde_json::json!({"content": text});
        // Reply to the original message so the response is visually linked.
        if let Some(msg_id) = thread_id {
            body["message_reference"] = serde_json::json!({"message_id": msg_id});
        }
        let resp = client
            .post(format!("{API_BASE}/channels/{channel_id}/messages"))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("Discord send failed: {body}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BOT_ID: &str = "999";

    fn message(id: &str, content: &str) -> Value {
        json!({
            "id": id,
            "guild_id": "10",
            "channel_id": "50",
            "author": {"id": "42"},
            "content": content,
        })
    }

    fn reply(content: &str) -> Value {
        let mut reply = message("300", content);
        reply["message_reference"] = json!({"message_id": "200", "channel_id": "50"});
        reply["referenced_message"] = json!({
            "id": "200",
            "channel_id": "50",
            "author": {"id": BOT_ID, "bot": true},
            "message_reference": {"message_id": "100", "channel_id": "50"},
        });
        reply
    }

    #[test]
    fn approval_replies_return_to_original_conversation() {
        let original = inbound_message(&message("100", "<@999> run this"), Some(BOT_ID)).unwrap();
        for (content, expected) in [
            ("<@999> yes", "yes"),
            ("yes", "yes"),
            ("no", "no"),
            ("always", "always"),
            ("<@!999> always", "always"),
        ] {
            let inbound = inbound_message(&reply(content), Some(BOT_ID))
                .expect("a reply to our bot must be delivered without requiring a mention");
            assert_eq!(inbound.channel_id, original.channel_id);
            assert_eq!(inbound.thread_id, original.thread_id, "{content}");
            assert_eq!(inbound.sender_id, "42");
            assert_eq!(inbound.text, expected);
        }
    }

    #[test]
    fn followups_to_different_bot_messages_stay_in_the_same_conversation() {
        for bot_message_id in ["200", "201", "202"] {
            let mut event = reply("continue");
            event["message_reference"]["message_id"] = json!(bot_message_id);
            event["referenced_message"]["id"] = json!(bot_message_id);
            let inbound = inbound_message(&event, Some(BOT_ID)).unwrap();
            assert_eq!(inbound.thread_id.as_deref(), Some("100"));
            assert_eq!(inbound.text, "continue");
        }
    }

    #[test]
    fn replies_to_other_authors_require_a_mention_and_keep_the_direct_reference() {
        for author in [json!({"id": "43"}), json!({"id": "998", "bot": true})] {
            let mut event = reply("yes");
            event["referenced_message"]["author"] = author;
            assert!(inbound_message(&event, Some(BOT_ID)).is_none());
            event["content"] = json!("<@999> yes");
            let inbound = inbound_message(&event, Some(BOT_ID)).unwrap();
            assert_eq!(inbound.thread_id.as_deref(), Some("200"));
        }
    }

    #[test]
    fn unrelated_mentions_start_separate_conversations() {
        for id in ["100", "101"] {
            let inbound = inbound_message(&message(id, "<@999> hello"), Some(BOT_ID)).unwrap();
            assert_eq!(inbound.thread_id.as_deref(), Some(id));
            assert_eq!(inbound.text, "hello");
        }
    }

    #[test]
    fn direct_messages_share_one_conversation() {
        for mut event in [message("100", "hello"), reply("yes")] {
            event.as_object_mut().unwrap().remove("guild_id");
            let inbound = inbound_message(&event, Some(BOT_ID)).unwrap();
            assert_eq!(inbound.channel_id, "50");
            assert_eq!(inbound.thread_id, None);
        }
    }

    #[test]
    fn missing_or_deleted_references_require_a_mention() {
        for missing in [true, false] {
            let mut event = reply("yes");
            if missing {
                event.as_object_mut().unwrap().remove("referenced_message");
            } else {
                event["referenced_message"] = Value::Null;
            }
            assert!(inbound_message(&event, Some(BOT_ID)).is_none());
            event["content"] = json!("<@999> yes");
            let inbound = inbound_message(&event, Some(BOT_ID)).unwrap();
            assert_eq!(inbound.thread_id.as_deref(), Some("200"));
        }
    }

    #[test]
    fn bot_messages_and_empty_replies_are_ignored() {
        let mut event = reply("yes");
        event["author"]["bot"] = json!(true);
        assert!(inbound_message(&event, Some(BOT_ID)).is_none());
        for content in ["", " ", "<@999>"] {
            assert!(inbound_message(&reply(content), Some(BOT_ID)).is_none());
        }
        assert!(inbound_message(&reply("yes"), None).is_none());
    }
}
