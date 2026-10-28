use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ante_sdk::protocol::{
    self, EventMsg, Evt, Id, Op, QuestionAnswer, QuestionReply, QuestionSpec, ReviewDecision,
    SessionRequest, ToolDecision, TurnPauseReason,
};
use ante_sdk::{ConnectOptions, Endpoint, OpSender};
use anyhow::{Context, Result};
use clap::Parser;
use futures_util::future::{join_all, try_join_all};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::channels::config::load_channels_config;
use crate::channels::discord::DiscordChannel;
use crate::channels::slack::SlackChannel;
use crate::channels::{Channel, InboundMessage};
use crate::output::{OutputFormat, print_event};

const SESSION_START_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Parser, Debug)]
#[command(name = "ante-gateway", version, about = "Connect Slack and Discord to an Ante host")]
pub struct GatewayArgs {
    /// Path to the channels configuration file
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Ante host endpoint: unix:<path>, ws://<addr>, wss://<addr>, or stdio.
    /// Defaults to unix:<ANTE_HOME>/run/serve.sock; start it with `ante serve --sock`.
    #[arg(long, value_name = "ENDPOINT")]
    pub connect: Option<Endpoint>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Ante executable for --connect stdio (default: $ANTE, then ante on PATH)"
    )]
    pub executable: Option<PathBuf>,

    /// Model to request (otherwise use the host's settings)
    #[arg(long)]
    pub model: Option<String>,

    /// Provider to request (otherwise resolved by the host)
    #[arg(long)]
    pub provider: Option<String>,

    /// Output protocol events to stdout (same as headless mode)
    #[arg(long, default_value = "minimal")]
    pub output_format: OutputFormat,
}

/// Key that uniquely identifies a conversation — one connection per key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConversationKey {
    platform: &'static str,
    channel_id: String,
    thread_id: Option<String>,
}

impl std::fmt::Display for ConversationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.platform, self.channel_id)?;
        if let Some(t) = &self.thread_id {
            write!(f, "/{t}")?;
        }
        Ok(())
    }
}

impl ConversationKey {
    fn from_msg(msg: &InboundMessage) -> Self {
        Self {
            platform: msg.platform,
            channel_id: msg.channel_id.clone(),
            thread_id: msg.thread_id.clone(),
        }
    }
}

/// Per-conversation state: its own connection and what its turn is waiting
/// on, if anything.
struct ConversationSession {
    op_sender: OpSender,
    events: JoinHandle<()>,
    /// Approval state: (turn_id, tool_use_ids).
    awaiting_approval: Option<(Id, Vec<String>)>,
    /// The pending `AskUser` prompt: (turn_id, tool_use_id, questions).
    awaiting_question: Option<(Id, String, Vec<QuestionSpec>)>,
}

impl ConversationSession {
    async fn close(mut self) -> Result<()> {
        timeout(SESSION_CLOSE_TIMEOUT, async {
            let _ = self.op_sender.send(protocol::op_msg(Op::Shutdown)).await;
            (&mut self.events).await.context("gateway event task failed")
        })
        .await
        .context("timed out closing gateway session")?
    }
}

impl Drop for ConversationSession {
    fn drop(&mut self) {
        self.events.abort();
    }
}

/// Render a pending `AskUser` prompt for a chat channel.
fn question_prompt(questions: &[QuestionSpec]) -> String {
    let mut text = String::from("❓ The agent has a question:");
    for (number, question) in questions.iter().enumerate() {
        text.push_str("\n\n");
        if questions.len() > 1 {
            text.push_str(&format!("{}. ", number + 1));
        }
        text.push_str(&question.question);
        for (index, option) in question.options.iter().enumerate() {
            text.push_str(&format!("\n  {}. {}", index + 1, option.label));
            if !option.description.is_empty() {
                text.push_str(&format!(" — {}", option.description));
            }
        }
    }
    text.push_str(if questions.len() > 1 {
        "\n\nReply with one option number per question (e.g. `1 2`), or in your own words."
    } else {
        "\n\nReply with an option number, or in your own words."
    });
    text
}

/// Map a chat reply onto the pending question: option numbers, one per
/// question, answer it; anything else is the user talking about it and rides
/// back as their words, so the model reads them instead of racing a default.
fn question_reply(reply: &str, questions: &[QuestionSpec]) -> QuestionReply {
    option_picks(reply, questions).map_or_else(
        || QuestionReply::Discuss { message: Some(reply.to_string()) },
        QuestionReply::Answered,
    )
}

/// Option numbers, one per question, or `None` when the reply is not that.
fn option_picks(reply: &str, questions: &[QuestionSpec]) -> Option<Vec<QuestionAnswer>> {
    let picks = reply
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<usize>().ok())
        .collect::<Option<Vec<_>>>()?;
    if picks.len() != questions.len() {
        return None;
    }
    picks
        .iter()
        .zip(questions)
        .map(|(&pick, question)| {
            let option = pick.checked_sub(1).and_then(|index| question.options.get(index))?;
            Some(QuestionAnswer { selected: vec![option.label.clone()], note: None })
        })
        .collect()
}

/// Best-effort outbound text to a conversation's channel. Failures are logged
/// and never break the gateway loop.
async fn send_to_conv(
    channels: &HashMap<&'static str, Arc<dyn Channel>>,
    key: &ConversationKey,
    text: &str,
) {
    let Some(channel) = channels.get(key.platform) else { return };
    if let Err(e) = channel.send_text(&key.channel_id, text, key.thread_id.as_deref()).await {
        warn!("Failed to send message to {key}: {e}");
    }
}

fn resolve_home(home: Option<std::ffi::OsString>, user_home: Option<PathBuf>) -> PathBuf {
    home.map(PathBuf::from)
        .unwrap_or_else(|| user_home.unwrap_or_else(|| PathBuf::from(".")).join(".ante"))
}

fn resolve_executable(env: Option<std::ffi::OsString>, flag: Option<PathBuf>) -> Option<PathBuf> {
    env.filter(|value| !value.is_empty()).map(PathBuf::from).or(flag)
}

pub async fn run(mut args: GatewayArgs) -> Result<()> {
    let home = resolve_home(std::env::var_os("ANTE_HOME"), std::env::home_dir());
    let config_path = args.config.take().unwrap_or_else(|| home.join("channels.json"));
    info!(path = %config_path.display(), "Loading gateway config");
    let channels_config = load_channels_config(&config_path)
        .with_context(|| format!("failed to load {}", config_path.display()))?;

    let endpoint =
        args.connect.clone().unwrap_or_else(|| Endpoint::Unix(home.join("run/serve.sock")));
    let options = ConnectOptions {
        executable: resolve_executable(std::env::var_os("ANTE"), args.executable.clone()),
        token: std::env::var("ANTE_SERVE_TOKEN").ok(),
        ..Default::default()
    };

    // Collect enabled channels.
    let mut channels: HashMap<&'static str, Arc<dyn Channel>> = HashMap::new();

    if let Some(cfg) = channels_config.get("slack")
        && cfg.enabled
    {
        let bot_token = cfg.resolve_secret("bot_token").context("slack: missing bot_token")?;
        let app_token = cfg.resolve_secret("app_token").context("slack: missing app_token")?;
        channels.insert("slack", Arc::new(SlackChannel::new(bot_token, app_token)));
    }
    if let Some(cfg) = channels_config.get("discord")
        && cfg.enabled
    {
        let bot_token = cfg.resolve_secret("bot_token").context("discord: missing bot_token")?;
        channels.insert("discord", Arc::new(DiscordChannel::new(bot_token)));
    }

    if channels.is_empty() {
        anyhow::bail!("no channels enabled in {}", config_path.display());
    }

    // Shared inbound message channel.
    let (msg_tx, mut msg_rx) = mpsc::channel::<InboundMessage>(256);

    // Start all channels concurrently — each start is a full network
    // handshake, so startup pays for the slowest channel, not the sum.
    try_join_all(channels.iter().map(|(name, ch)| {
        let msg_tx = msg_tx.clone();
        async move {
            info!(channel = *name, "Starting gateway channel");
            ch.start(msg_tx).await.with_context(|| format!("failed to start {name}"))?;
            info!(channel = *name, "Gateway channel started");
            anyhow::Ok(())
        }
    }))
    .await?;
    drop(msg_tx);

    // Per-conversation connections. Each conversation gets its own connection + session.
    let mut sessions: HashMap<ConversationKey, ConversationSession> = HashMap::new();

    // Aggregated event receiver: all per-conversation events merge here.
    // None marks a transport disconnect without a Goodbye.
    let (evt_tx, mut evt_rx) = mpsc::channel::<(ConversationKey, Option<EventMsg>)>(4096);

    let access = &channels_config;

    info!(%endpoint, "Gateway ready; waiting for messages");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received SIGINT; shutting down gateway");
                break;
            }
            _ = super::wait_for_sigterm() => {
                info!("Received SIGTERM; shutting down gateway");
                break;
            }
            msg = msg_rx.recv() => {
                let Some(msg) = msg else {
                    info!("All channels disconnected");
                    break;
                };

                // Access check.
                if let Some(cfg) = access.get(msg.platform)
                    && !cfg.allow_from.is_allowed(&msg.sender_id)
                {
                    warn!(
                        platform = msg.platform,
                        sender = msg.sender_id,
                        "Rejected by allow_from policy"
                    );
                    continue;
                }

                let conv_key = ConversationKey::from_msg(&msg);
                let trimmed = msg.text.trim().to_lowercase();

                // Handle interrupt command — equivalent to Ctrl+C in TUI.
                if trimmed == "stop" {
                    if let Some(session) = sessions.get(&conv_key) {
                        if let Err(e) = session.op_sender.send(protocol::op_msg(Op::Interrupt)).await {
                            error!("Failed to send interrupt for {conv_key}: {e}");
                        }
                        send_to_conv(&channels, &conv_key, "⏹ Turn interrupted.").await;
                        info!(conversation = %conv_key, "Interrupted gateway conversation");
                    }
                    continue;
                }

                // Check if this is an approval reply for an existing session.
                if let Some(session) = sessions.get_mut(&conv_key)
                    && let Some((turn_id, ref tool_ids)) = session.awaiting_approval
                {
                    let decision = match trimmed.as_str() {
                        "yes" | "y" => Some(ReviewDecision::Accept),
                        "no" | "n" => Some(ReviewDecision::Deny),
                        "always" => Some(ReviewDecision::AcceptForSession),
                        _ => None,
                    };

                    if let Some(decision) = decision {
                        let responses: Vec<_> = tool_ids
                            .iter()
                            .map(|id| ToolDecision {
                                tool_use_id: id.clone(),
                                decision: decision.clone(),
                                message: None,
                            })
                            .collect();
                        if let Err(e) = session
                            .op_sender
                            .send(protocol::op_msg(Op::ApprovalResponse { turn_id, responses }))
                            .await
                        {
                            error!("Failed to send approval response: {e}");
                        }
                        session.awaiting_approval = None;
                        continue;
                    }
                }

                // A pending question takes the whole reply: option numbers
                // answer it, anything else is the user's words about it.
                if let Some(session) = sessions.get_mut(&conv_key)
                    && let Some((turn_id, tool_use_id, questions)) =
                        session.awaiting_question.take()
                {
                    let reply = question_reply(&trimmed, &questions);
                    let op = Op::QuestionResponse { turn_id, tool_use_id, reply };
                    if let Err(e) = session.op_sender.send(protocol::op_msg(op)).await {
                        error!("Failed to send question response: {e}");
                    }
                    continue;
                }

                info!(conversation = %conv_key, "Received inbound gateway message");

                // Get or create a session for this conversation.
                let session = match sessions.entry(conv_key.clone()) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        info!(conversation = %conv_key, "Creating gateway conversation");
                        match spawn_session(&args, &endpoint, &options, &evt_tx, &conv_key).await {
                            Ok(session) => {
                                send_to_conv(&channels, &conv_key, "Tip: send `stop` to cancel the current task.").await;
                                entry.insert(session)
                            }
                            Err(e) => {
                                error!("Failed to create session for {conv_key}: {e:#}");
                                send_to_conv(&channels, &conv_key, &format!("Failed to start an Ante session: {e:#}")).await;
                                continue;
                            }
                        }
                    }
                };
                if let Err(e) = session.op_sender.send(protocol::op_msg(Op::UserInput(msg.text))).await {
                    error!("Failed to send to the connection for {conv_key}: {e}");
                }
            }
            evt = evt_rx.recv() => {
                let Some((conv_key, evt)) = evt else { break };
                let Some(evt) = evt else {
                    sessions.remove(&conv_key);
                    warn!(conversation = %conv_key, "Ante host disconnected");
                    send_to_conv(&channels, &conv_key, "Connection to the Ante host closed. Send another message to start a new session.").await;
                    continue;
                };

                print_event(&args.output_format, &evt);

                let Some(session) = sessions.get_mut(&conv_key) else { continue };

                match &evt.event {
                    Evt::AgentMessage(text) => {
                        info!(conversation = %conv_key, "Sending gateway reply");
                        send_to_conv(&channels, &conv_key, text).await;
                    }
                    Evt::TurnPause {
                        turn_id,
                        reason: TurnPauseReason::Approval { tools, message },
                    } => {
                        let tool_names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
                        let tool_ids: Vec<_> = tools.iter().map(|t| t.id.clone()).collect();
                        let prompt = format!(
                            "⏸ Approval needed: {}\n{message}\n\nReply `yes` to approve, `always` to approve for this session, or `no` to skip.",
                            tool_names.join(", ")
                        );

                        session.awaiting_approval = Some((*turn_id, tool_ids));

                        send_to_conv(&channels, &conv_key, &prompt).await;
                    }
                    Evt::TurnPause {
                        turn_id,
                        reason: TurnPauseReason::Question { tool_use_id, questions },
                    } => {
                        let prompt = question_prompt(questions);
                        session.awaiting_question =
                            Some((*turn_id, tool_use_id.clone(), questions.clone()));
                        send_to_conv(&channels, &conv_key, &prompt).await;
                    }
                    // Answered or released: either way nothing pends anymore.
                    Evt::TurnResume { .. } => {
                        session.awaiting_question = None;
                    }
                    Evt::TurnEnd { .. } | Evt::SessionEnd { .. } => {
                        session.awaiting_approval = None;
                        session.awaiting_question = None;
                    }
                    Evt::Error(err) => {
                        send_to_conv(&channels, &conv_key, &format!("Error: {err}")).await;
                    }
                    Evt::Goodbye => {
                        info!(conversation = %conv_key, "Gateway session ended");
                        sessions.remove(&conv_key);
                    }
                    _ => {}
                }
            }
        }
    }

    // Stop forwarding into the main loop so a full queue cannot block
    // teardown. Each task keeps draining its host until Goodbye or EOF.
    drop(evt_rx);
    let closed = join_all(sessions.into_iter().map(|(key, session)| async move {
        session.close().await.with_context(|| format!("failed to close {key}"))
    }))
    .await;

    info!("Gateway shut down");
    closed.into_iter().collect()
}

/// Open a connection + start a session for a conversation and wire its
/// events into the shared event channel.
async fn spawn_session(
    args: &GatewayArgs,
    endpoint: &Endpoint,
    options: &ConnectOptions,
    evt_tx: &mpsc::Sender<(ConversationKey, Option<EventMsg>)>,
    conv_key: &ConversationKey,
) -> Result<ConversationSession> {
    let client = timeout(SESSION_START_TIMEOUT, async {
        let mut client = ante_sdk::connect(endpoint.clone(), options.clone())
            .await
            .with_context(|| format!("failed to connect to {endpoint}; start an Ante host with `ante serve --sock` or set --connect"))?;
        client
            .send(Op::StartSession(SessionRequest {
                model: args.model.clone(),
                provider: args.provider.clone(),
                // A programmatic relay, not an interactive session: no memory writes.
                enable_auto_memory: Some(false),
                ..Default::default()
            }))
            .await?;
        while let Some(evt) = client.next_event().await {
            match evt.event {
                Evt::SessionStart(_) => return Ok(client),
                Evt::Error(error) => anyhow::bail!("{error}"),
                Evt::Goodbye => break,
                _ => {}
            }
        }
        anyhow::bail!("connection ended before session started")
    })
    .await
    .context("timed out starting gateway session")??;
    let (op_sender, mut conn_evt_rx) = client.into_parts();

    // Forward this connection's events into the shared channel, tagged with conv_key.
    let key = conv_key.clone();
    let tx = evt_tx.clone();
    let events = tokio::spawn(async move {
        while let Some(evt) = conn_evt_rx.recv().await {
            let goodbye = matches!(evt.event, Evt::Goodbye);
            let _ = tx.send((key.clone(), Some(evt))).await;
            if goodbye {
                return;
            }
        }
        let _ = tx.send((key, None)).await;
    });

    Ok(ConversationSession { op_sender, events, awaiting_approval: None, awaiting_question: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante_sdk::protocol::QuestionOption;

    #[test]
    fn standalone_cli_preserves_gateway_defaults() {
        let args = GatewayArgs::try_parse_from(["ante-gateway"]).unwrap();
        assert!(args.config.is_none());
        assert!(args.connect.is_none());
        assert!(args.executable.is_none());
        assert!(args.model.is_none());
        assert!(args.provider.is_none());
        assert!(matches!(args.output_format, OutputFormat::Minimal));
    }

    #[test]
    fn standalone_cli_accepts_host_options() {
        for endpoint in ["stdio", "unix:/tmp/ante.sock", "ws://127.0.0.1:8080", "wss://host"] {
            let args = GatewayArgs::try_parse_from([
                "ante-gateway",
                "--config",
                "/config/channels.json",
                "--connect",
                endpoint,
                "--executable",
                "/bin/ante",
                "--model",
                "test-model",
                "--provider",
                "test-provider",
                "--output-format",
                "json",
            ])
            .unwrap();
            assert_eq!(args.config, Some(PathBuf::from("/config/channels.json")));
            assert_eq!(args.connect, Some(endpoint.parse().unwrap()));
            assert_eq!(args.executable, Some(PathBuf::from("/bin/ante")));
            assert_eq!(args.model.as_deref(), Some("test-model"));
            assert_eq!(args.provider.as_deref(), Some("test-provider"));
            assert!(matches!(args.output_format, OutputFormat::Json));
        }
        for args in [
            vec!["ante-gateway", "--connect", "invalid"],
            vec!["ante-gateway", "--output-format", "invalid"],
            vec!["ante-gateway", "--config"],
        ] {
            assert!(GatewayArgs::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn home_resolution_matches_ante() {
        let user_home = Some(PathBuf::from("/home/user"));
        assert_eq!(
            resolve_home(Some("/custom/ante".into()), user_home.clone()),
            PathBuf::from("/custom/ante")
        );
        assert_eq!(resolve_home(None, user_home), PathBuf::from("/home/user/.ante"));
        assert_eq!(resolve_home(None, None), PathBuf::from("./.ante"));
    }

    #[test]
    fn stdio_uses_dispatching_ante_then_flag_then_sdk_path_lookup() {
        let flag = Some(PathBuf::from("/flag/ante"));
        assert_eq!(
            resolve_executable(Some("/env/ante".into()), flag.clone()),
            Some(PathBuf::from("/env/ante"))
        );
        assert_eq!(resolve_executable(Some("".into()), flag.clone()), flag);
        assert_eq!(resolve_executable(None, flag.clone()), flag);
        assert_eq!(resolve_executable(None, None), None);
    }

    fn questions(count: usize) -> Vec<QuestionSpec> {
        (0..count)
            .map(|n| QuestionSpec {
                header: format!("Q{n}"),
                question: format!("Question {n}?"),
                multi_select: false,
                options: ["A", "B"]
                    .iter()
                    .map(|label| QuestionOption {
                        label: label.to_string(),
                        description: String::new(),
                        preview: None,
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn option_numbers_answer_one_question_each() {
        let QuestionReply::Answered(answers) = question_reply("2", &questions(1)) else {
            panic!("a valid pick answers");
        };
        assert_eq!(answers[0].selected, vec!["B".to_string()]);
        let QuestionReply::Answered(answers) = question_reply("1, 2", &questions(2)) else {
            panic!("one pick per question answers");
        };
        assert_eq!(answers[0].selected, vec!["A".to_string()]);
        assert_eq!(answers[1].selected, vec!["B".to_string()]);
    }

    #[test]
    fn anything_else_is_the_users_words() {
        let one = questions(1);
        // Out of range, 0-based, too many picks, and prose all ride back as
        // the user's words rather than picking anything.
        for reply in ["3", "0", "1 2", "actually, do neither"] {
            assert_eq!(
                question_reply(reply, &one),
                QuestionReply::Discuss { message: Some(reply.to_string()) },
                "{reply:?}"
            );
        }
    }

    #[test]
    fn prompt_numbers_options_and_invites_words() {
        let text = question_prompt(&questions(1));
        assert!(text.contains("Question 0?"), "{text}");
        assert!(text.contains("\n  1. A"), "{text}");
        assert!(text.contains("in your own words"), "{text}");
    }

    #[cfg(unix)]
    mod sdk {
        use super::*;
        use ante_sdk::protocol::{OpMsg, SessionInfo, event_msg};
        use tempfile::TempDir;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixListener;
        use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

        const STEP: Duration = Duration::from_secs(5);

        struct Peer {
            reader: BufReader<OwnedReadHalf>,
            writer: OwnedWriteHalf,
        }

        impl Peer {
            async fn accept(listener: &UnixListener) -> Self {
                let (stream, _) = timeout(STEP, listener.accept()).await.unwrap().unwrap();
                let (reader, writer) = stream.into_split();
                Self { reader: BufReader::new(reader), writer }
            }

            async fn next_op(&mut self) -> Option<OpMsg> {
                let mut line = String::new();
                let len = timeout(STEP, self.reader.read_line(&mut line)).await.unwrap().unwrap();
                (len != 0).then(|| serde_json::from_str(&line).unwrap())
            }

            async fn emit(&mut self, event: Evt) {
                let mut line = serde_json::to_vec(&event_msg(event, None)).unwrap();
                line.push(b'\n');
                timeout(STEP, self.writer.write_all(&line)).await.unwrap().unwrap();
            }
        }

        fn listener() -> (TempDir, UnixListener, Endpoint) {
            // Keep the socket path under macOS's sockaddr_un limit.
            let dir = tempfile::tempdir_in("/tmp").unwrap();
            let path = dir.path().join("host.sock");
            let listener = UnixListener::bind(&path).unwrap();
            (dir, listener, Endpoint::Unix(path))
        }

        fn args() -> GatewayArgs {
            GatewayArgs {
                config: None,
                connect: None,
                executable: None,
                model: None,
                provider: None,
                output_format: OutputFormat::Minimal,
            }
        }

        fn key(thread: &str) -> ConversationKey {
            ConversationKey {
                platform: "slack",
                channel_id: "channel".into(),
                thread_id: Some(thread.into()),
            }
        }

        async fn start_peer(listener: &UnixListener) -> (Peer, SessionRequest) {
            let mut peer = Peer::accept(listener).await;
            let Op::StartSession(request) = peer.next_op().await.unwrap().op else {
                panic!("expected StartSession");
            };
            peer.emit(Evt::SessionStart(Box::new(SessionInfo {
                session_id: Id::ses(),
                cwd: PathBuf::from("/tmp"),
                ..Default::default()
            })))
            .await;
            (peer, request)
        }

        #[tokio::test]
        #[ignore = "uses a local Unix socket"]
        async fn sdk_sessions_share_a_host_with_independent_connections() {
            let (_dir, listener, endpoint) = listener();
            let options = ConnectOptions::default();
            let (tx, mut rx) = mpsc::channel(16);
            let first_key = key("first");
            let second_key = key("second");
            let mut args = args();
            let (first, (mut first_peer, request)) = tokio::join!(
                spawn_session(&args, &endpoint, &options, &tx, &first_key),
                start_peer(&listener),
            );
            let first = first.unwrap();
            assert!(request.model.is_none());
            assert!(request.provider.is_none());
            assert_eq!(request.enable_auto_memory, Some(false));

            args.model = Some("requested-model".into());
            args.provider = Some("requested-provider".into());
            let (second, (mut second_peer, request)) = tokio::join!(
                spawn_session(&args, &endpoint, &options, &tx, &second_key),
                start_peer(&listener),
            );
            let second = second.unwrap();
            assert_eq!(request.model, args.model);
            assert_eq!(request.provider, args.provider);
            assert_eq!(request.enable_auto_memory, Some(false));

            for (session, peer, key) in
                [(&first, &mut first_peer, &first_key), (&second, &mut second_peer, &second_key)]
            {
                session
                    .op_sender
                    .send(protocol::op_msg(Op::UserInput(key.to_string())))
                    .await
                    .unwrap();
                let Op::UserInput(input) = peer.next_op().await.unwrap().op else {
                    panic!("expected UserInput on this conversation's connection");
                };
                assert_eq!(input, key.to_string());
                peer.emit(Evt::AgentMessage(input.clone())).await;
                let (received_key, event) = timeout(STEP, rx.recv()).await.unwrap().unwrap();
                assert_eq!(&received_key, key);
                assert!(matches!(event.unwrap().event, Evt::AgentMessage(text) if text == input));
            }

            let (closed, ()) = tokio::join!(first.close(), async {
                assert!(matches!(first_peer.next_op().await.unwrap().op, Op::Shutdown));
                first_peer.emit(Evt::Goodbye).await;
            });
            closed.unwrap();
            let (received_key, event) = timeout(STEP, rx.recv()).await.unwrap().unwrap();
            assert_eq!(received_key, first_key);
            assert!(matches!(event.unwrap().event, Evt::Goodbye));

            // Closing one conversation leaves the other connected; a lost
            // transport produces exactly one terminal notification for it.
            second.op_sender.send(protocol::op_msg(Op::Interrupt)).await.unwrap();
            assert!(matches!(second_peer.next_op().await.unwrap().op, Op::Interrupt));
            drop(second_peer);
            let (received_key, event) = timeout(STEP, rx.recv()).await.unwrap().unwrap();
            assert_eq!(received_key, second_key);
            assert!(event.is_none());
            second.close().await.unwrap();
            assert!(rx.try_recv().is_err());
        }

        #[tokio::test]
        #[ignore = "uses a local Unix socket"]
        async fn sdk_start_errors_and_early_disconnects_return_without_hanging() {
            let (_dir, listener, endpoint) = listener();
            let options = ConnectOptions::default();
            let (tx, _rx) = mpsc::channel(1);
            let args = args();
            let key = key("failed");
            for event in [Some(Evt::Error("model unavailable".into())), Some(Evt::Goodbye), None] {
                let (session, peer) = tokio::join!(
                    timeout(STEP, spawn_session(&args, &endpoint, &options, &tx, &key)),
                    async {
                        let mut peer = Peer::accept(&listener).await;
                        assert!(matches!(peer.next_op().await.unwrap().op, Op::StartSession(_)));
                        if let Some(event) = event.clone() {
                            peer.emit(event).await;
                            Some(peer)
                        } else {
                            None
                        }
                    },
                );
                let error = session.unwrap().err().expect("startup must fail");
                let expected = if matches!(event, Some(Evt::Error(_))) {
                    "model unavailable"
                } else {
                    "connection ended before session started"
                };
                assert_eq!(error.to_string(), expected);
                if let Some(mut peer) = peer {
                    assert!(peer.next_op().await.is_none(), "failed startup must disconnect");
                }
            }
        }

        #[tokio::test]
        #[ignore = "uses a local Unix socket"]
        async fn sdk_shutdown_drains_events_after_the_gateway_stops_receiving() {
            let (_dir, listener, endpoint) = listener();
            let options = ConnectOptions::default();
            let (tx, rx) = mpsc::channel(1);
            let args = args();
            let key = key("closing");
            let (session, (mut peer, _)) = tokio::join!(
                spawn_session(&args, &endpoint, &options, &tx, &key),
                start_peer(&listener),
            );
            for _ in 0..4 {
                peer.emit(Evt::AgentMessage("queued".into())).await;
            }
            drop(rx);
            let (closed, ()) = tokio::join!(session.unwrap().close(), async {
                assert!(matches!(peer.next_op().await.unwrap().op, Op::Shutdown));
                peer.emit(Evt::Goodbye).await;
            });
            closed.unwrap();
            assert!(peer.next_op().await.is_none());
        }
    }
}
