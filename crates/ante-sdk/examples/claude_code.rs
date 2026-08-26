//! Claude Code example: one-shot query or interactive REPL.
//!
//! Run with:
//!   cargo run --example claude_code -- "Say hello from Rust"
//!   cargo run --example claude_code -- --model claude-sonnet-4-5 "Summarize this repo"
//!   cargo run --example claude_code -- --cli-path /path/to/claude "Hi"
//!   cargo run --example claude_code --                                  # REPL (default)
//!   cargo run --example claude_code -- --model claude-sonnet-4-5        # REPL with model

use std::io::{self, Write};
use std::path::PathBuf;

use ante_sdk::claude::{
    AssistantMessage, Claude, ClaudeMessage, ClaudeOptions, ContentBlock, ControlRequestMessage,
    ControlResponseMessage, PermissionMode, ResultMessage, StreamEventMessage, SystemMessage,
    UserMessage,
};
use clap::Parser;
use serde_json::Value;

/// Claude Code example: one-shot query or interactive REPL.
#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    /// Path to the Claude CLI binary (defaults to `which claude`).
    #[arg(long)]
    cli_path: Option<PathBuf>,

    /// Model name to use.
    #[arg(long)]
    model: Option<String>,

    /// Prompt text. If omitted, starts an interactive REPL.
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let options =
        ClaudeOptions { cli_path: cli.cli_path, model: cli.model, ..ClaudeOptions::default() };

    if cli.prompt.is_empty() {
        run_repl(options).await
    } else {
        run_one_shot(cli.prompt.join(" "), options).await
    }
}

async fn run_one_shot(
    prompt: String,
    options: ClaudeOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Claude::connect(options).await?;
    let response = client.query(prompt).await?;
    client.shutdown().await?;

    for message in response.messages {
        render(&message);
    }

    Ok(())
}

async fn run_repl(options: ClaudeOptions) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Connecting to Claude CLI...");
    let mut client = Claude::connect(options).await?;
    eprintln!("Connected. Type /help for commands.\n");

    let mut line = String::new();
    loop {
        print!("you> ");
        io::stdout().flush()?;

        line.clear();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            let should_quit = handle_command(input, &mut client).await?;
            if should_quit {
                break;
            }
            continue;
        }

        client.send_user_text(input).await?;
        println!();
        stream_response(&mut client).await?;
        println!();
    }

    client.shutdown().await?;
    eprintln!("Session ended.");
    Ok(())
}

async fn handle_command(
    input: &str,
    client: &mut Claude,
) -> Result<bool, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|value| value.trim()).unwrap_or("");

    match cmd {
        "/exit" | "/quit" => return Ok(true),
        "/help" => {
            eprintln!("Commands:");
            eprintln!("  /exit, /quit        End session");
            eprintln!("  /model <name>       Switch model");
            eprintln!("  /permission <mode>  Set permission mode");
            eprintln!("  /interrupt          Interrupt current generation");
            eprintln!("  /info               Show initialize response");
            eprintln!("  /help               Show this help");
        }
        "/model" => {
            if arg.is_empty() {
                eprintln!("Usage: /model <model-name>");
            } else {
                let _ = client.set_model(arg).await?;
                eprintln!("Model switched to: {arg}");
            }
        }
        "/permission" => {
            if arg.is_empty() {
                eprintln!("Usage: /permission <default|accept-edits|plan|bypass>");
            } else {
                let mode = match arg {
                    "default" => PermissionMode::Default,
                    "accept-edits" => PermissionMode::AcceptEdits,
                    "plan" => PermissionMode::Plan,
                    "bypass" => PermissionMode::BypassPermissions,
                    _ => {
                        eprintln!("Unknown permission mode: {arg}");
                        return Ok(false);
                    }
                };
                let _ = client.set_permission_mode(mode).await?;
                eprintln!("Permission mode updated.");
            }
        }
        "/interrupt" => {
            let response = client.interrupt().await?;
            eprintln!("Interrupted: {response}");
        }
        "/info" => {
            if let Some(server_info) = client.server_info() {
                eprintln!("Server info: {server_info}");
            } else {
                eprintln!("No server info available.");
            }
        }
        _ => {
            eprintln!("Unknown command: {cmd}. Type /help for available commands.");
        }
    }

    Ok(false)
}

async fn stream_response(client: &mut Claude) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let message = client.next_message().await?;

        if let ClaudeMessage::ControlRequest(control_request) = &message
            && let Some(request_id) = control_request.request_id.as_deref()
        {
            client
                .respond_control_request_error(
                    request_id,
                    "unsupported control request in repl example",
                )
                .await?;
        }

        let is_result = matches!(message, ClaudeMessage::Result(_));
        render(&message);
        if is_result {
            return Ok(());
        }
    }
}

fn render(message: &ClaudeMessage) {
    match message {
        ClaudeMessage::Assistant(message) => render_assistant(message),
        ClaudeMessage::User(message) => render_user(message),
        ClaudeMessage::System(message) => render_system(message),
        ClaudeMessage::StreamEvent(message) => render_stream_event(message),
        ClaudeMessage::ControlRequest(message) => render_control_request(message),
        ClaudeMessage::ControlResponse(message) => render_control_response(message),
        ClaudeMessage::Result(message) => render_result(message),
        ClaudeMessage::Other(value) => {
            print_header("other", None);
            print_json(value);
        }
    }
}

fn render_assistant(message: &AssistantMessage) {
    let model = message.model.as_deref();
    print_header("assistant", model);

    for block in &message.content {
        match block {
            ContentBlock::Text(text) => {
                for line in text.text.lines() {
                    println!("  {line}");
                }
            }
            ContentBlock::Thinking(thinking) => {
                println!("  [thinking]");
                for line in thinking.thinking.lines() {
                    println!("  {line}");
                }
            }
            ContentBlock::ToolUse(tool) => {
                println!("  [tool_use] {} (id={})", tool.name, tool.id);
                print_json(&tool.input);
            }
            ContentBlock::ToolResult(result) => {
                let tag = match result.is_error {
                    Some(true) => "tool_result:error",
                    _ => "tool_result",
                };
                println!("  [{tag}] (tool_use_id={})", result.tool_use_id);
                if let Some(content) = &result.content {
                    print_json(content);
                }
            }
            ContentBlock::Other(value) => {
                println!("  [unknown block]");
                print_json(value);
            }
        }
    }

    if let Some(error) = &message.error {
        println!("  error: {error}");
    }
}

fn render_user(message: &UserMessage) {
    print_header("user", None);
    if let Some(text) = message.text() {
        for line in text.lines() {
            println!("  {line}");
        }
    } else {
        print_json(&message.raw);
    }
}

fn render_system(message: &SystemMessage) {
    print_header("system", message.subtype.as_deref());
    print_json(&message.raw);
}

fn render_stream_event(message: &StreamEventMessage) {
    print_header("stream_event", None);
    if let Some(event) = &message.event {
        print_json(event);
    } else {
        print_json(&message.raw);
    }
}

fn render_control_request(message: &ControlRequestMessage) {
    print_header("control_request", message.subtype.as_deref());
    if let Some(request) = &message.request {
        print_json(request);
    } else {
        print_json(&message.raw);
    }
}

fn render_control_response(message: &ControlResponseMessage) {
    print_header("control_response", message.subtype.as_deref());
    if let Some(error) = &message.error {
        println!("  error: {error}");
    }
    if let Some(response) = &message.response {
        print_json(response);
    }
}

fn render_result(message: &ResultMessage) {
    print_header("result", message.subtype.as_deref());
    let cost = message.total_cost_usd.unwrap_or(0.0);
    let duration_s = message.duration_ms.unwrap_or(0.0) / 1000.0;
    let api_s = message.duration_api_ms.unwrap_or(0.0) / 1000.0;
    let turns = message.num_turns.unwrap_or(0);
    println!("  turns: {turns}");
    println!("  cost:  ${cost:.4}");
    println!("  time:  {duration_s:.2}s (api {api_s:.2}s)");
    if let Some(session_id) = &message.session_id {
        println!("  session: {session_id}");
    }
    if let Some(usage) = &message.usage {
        println!("  usage:");
        print_json(usage);
    }
    if let Some(result) = &message.result {
        match result {
            Value::String(text) => {
                println!("  result:");
                for line in text.lines() {
                    println!("  {line}");
                }
            }
            other => {
                println!("  result:");
                print_json(other);
            }
        }
    }
}

fn print_header(kind: &str, tag: Option<&str>) {
    println!();
    match tag {
        Some(tag) => println!("── {kind} ({tag}) ──"),
        None => println!("── {kind} ──"),
    }
}

fn print_json(value: &Value) {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    for line in pretty.lines() {
        println!("  {line}");
    }
}
