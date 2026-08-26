# Claude Code integration

Rust client for [Claude Code](https://code.claude.com/docs/en/cli-reference).

## Long-lived runtime

The integration turns Claude Code into a long-lived agent runtime. Instead of
launching separate `claude -p "…"` invocations and stitching sessions together
with `--resume`, `Claude::connect` spawns one subprocess that remains alive
across turns. Calls to `query` and `send_user_text` reuse its conversation
history and tool state until `shutdown` is called.

## What it provides

- `Claude::connect(options)` with `query` and `send_user_text`
- typed control helpers such as `set_model`, `set_permission_mode`,
  `interrupt`, `rewind_files`, and `get_mcp_status`
- typed parsing for assistant, user, system, result, stream-event, and control
  protocol frames
- access to raw JSON on parsed messages for forward compatibility

## Usage

```no_run
use ante_sdk::claude::{Claude, ClaudeMessage, ClaudeOptions};

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

The `claude_code` example supports one-shot prompts and an interactive REPL:

```bash
cargo run -p ante-sdk --example claude_code -- "What is 2 + 2?"
cargo run -p ante-sdk --example claude_code -- --model claude-sonnet-4-5
```

## Requirements and limitations

- The external `claude` executable must be installed and authenticated.
- The crate does not bundle Claude Code.
- In-process MCP servers and hook callbacks are not implemented.
