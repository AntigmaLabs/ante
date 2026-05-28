# agent-sdk

A Rust SDK and client for agent runtimes. Currently only Claude Code is
supported.

## Claude Code

Rust client for [Claude Code](https://code.claude.com/docs/en/cli-reference).

Other SDKs:
- [Python SDK](https://github.com/anthropics/claude-agent-sdk-python).

### Long-lived agent runtime

The SDK turns Claude Code into a long-lived agent runtime. Instead of
launching separate `claude -p "…"` invocations and stitching sessions back
together with `--resume`, `Claude::connect` spawns a single subprocess that
stays alive across turns. Call `query` or `send_user_text` as many times as
you need — the underlying process, conversation history, and tool state
persist for the lifetime of the connection. This makes it straightforward to
build multi-turn agents, orchestration loops, and interactive applications on
top of Claude Code.

### What it provides

- `Claude::connect(options)` then `query` / `send_user_text` for sessions;
  call `shutdown` when done
- typed control helpers (`set_model`, `set_permission_mode`, `interrupt`,
  `rewind_files`, `get_mcp_status`, …)
- typed `ClaudeMessage` parsing for `assistant`, `user`, `system`, `result`,
  `stream_event`, and control protocol frames
- low-level `Stdio` transport for raw newline-delimited JSON access

### Usage

```rust
use agent_sdk::claude::{Claude, ClaudeMessage, ClaudeOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeOptions {
        model: Some("claude-sonnet-4-5".to_string()),
        ..ClaudeOptions::default()
    };
    let mut client = Claude::connect(options).await?;
    let response = client.query("Summarize this repo").await?;
    client.shutdown().await?;

    for message in response.messages {
        if let ClaudeMessage::Assistant(message) = message
            && let Some(text) = message.text()
        {
            println!("{text}");
        }
    }

    Ok(())
}
```

### Example

`examples/claude_code.rs` covers both one-shot and interactive modes. When no
prompt is given, it starts an interactive REPL:

```bash
cargo run --example claude_code -- "What is 2 + 2?"
cargo run --example claude_code -- --model claude-sonnet-4-5 "Summarize this repo"
cargo run --example claude_code -- --cli-path /path/to/claude "Hello"
cargo run --example claude_code --                              # REPL
cargo run --example claude_code -- --model claude-sonnet-4-5    # REPL with model
```

### Notes

- The SDK shells out to the external `claude` executable; it does not bundle
  Claude Code.
- In-process MCP servers and hook callbacks (as in the Python SDK) are not
  implemented yet.
- Additional agent runtimes may be added in the future behind the same SDK
  surface.
