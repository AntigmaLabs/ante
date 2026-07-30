# ACP Server Integration Guide

Step-by-step instructions for wiring `ante-acp-server` into the main Ante binary.

## 1. Add the Cargo dependency

In the main binary's `Cargo.toml` (or workspace root if the binary is a workspace member):

```toml
[dependencies]
ante-acp-server = { path = "../acp-server" }
# If the binary is inside crates/ and acp-server is a sibling:
# ante-acp-server = { path = "../acp-server" }
# If both are workspace members, you can also use:
# ante-acp-server.workspace = true
```

Make sure the workspace root `Cargo.toml` already includes `"crates/acp-server"` in `[workspace] members`.

## 2. Add the CLI subcommand

Add an `Acp` variant to your existing CLI enum (clap derive). The exact location depends on how subcommands are structured, but the pattern is:

```rust
use clap::Subcommand;

#[derive(Subcommand)]
enum Commands {
    // ... existing variants ...

    /// Start the ACP (Agent Communication Protocol) server
    Acp {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8420)]
        port: u16,
    },
}
```

## 3. Add the handler function

Somewhere in your command dispatch (typically `main.rs` or a `commands` module):

```rust
use ante_acp_server::server;

async fn handle_acp(host: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    server::start_server(&host, port).await
}
```

## 4. Wire it into the match block

In the main entry point where you dispatch commands:

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "ante", about = "A ghost in your shell")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        // ... existing arms ...

        Commands::Acp { host, port } => {
            handle_acp(host, port).await
        }
    }
}
```

## 5. Full working example

Here is a self-contained `main.rs` that demonstrates the integration:

```rust
use clap::{Parser, Subcommand};
use ante_acp_server::server;

#[derive(Parser)]
#[command(name = "ante", about = "A ghost in your shell")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a prompt headlessly
    Prompt {
        /// The prompt text
        #[arg(short, long)]
        prompt: String,
    },

    /// Start the ACP server
    Acp {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8420)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Prompt { prompt } => {
            println!("Running: {prompt}");
            // ... existing prompt handling ...
            Ok(())
        }
        Commands::Acp { host, port } => {
            server::start_server(&host, port).await
        }
    }
}
```

## Public API reference

The crate re-exports the following items from `ante_acp_server`:

| Item | Type | Purpose |
|------|------|---------|
| `server::start_server(host, port)` | `async fn` | Start the ACP server with graceful shutdown |
| `server::build_router()` | `fn -> Router` | Build the Axum router (useful for embedding in an existing server) |
| `server::init_logging()` | `fn` | Initialize tracing with `ANTE_ACP_LOG` env filter |
| `AcState` | struct | Shared application state (runs, inputs, manifest) |
| `AcServerError` | enum | Error type that maps to HTTP status codes |

### Embedding in an existing Axum server

If Ante already runs an Axum server (e.g., `ante serve --ws`), you can mount the ACP routes instead of starting a standalone server:

```rust
use ante_acp_server::server::build_router;

// Merge ACP routes into an existing router
let acp_routes = build_router();
let app = existing_router.merge(acp_routes);
```

## Configuration

| Environment variable | Default | Description |
|---------------------|---------|-------------|
| `ANTE_ACP_LOG` | `ante_acp_server=info,tower_http=info` | Tracing filter for the ACP server |

## Testing the integration

```bash
# Build
cargo build

# Start the server
cargo run -- acp --port 8420

# Verify it's running
curl http://127.0.0.1:8420/ping

# List agents
curl http://127.0.0.1:8420/agents

# Create a run
curl -X POST http://127.0.0.1:8420/runs \
  -H "Content-Type: application/json" \
  -d '{
    "agent_name": "ante",
    "input": [{"role": "user", "parts": [{"content_type": "text/plain", "content": "hello"}]}],
    "mode": "sync"
  }'
```
