use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

/// Agent Client Protocol agent for Ante: speaks ACP over stdio and drives an
/// installed `ante`. Launch it from an ACP client.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The `ante` executable to drive (default: `$ANTE`, then `ante` on PATH)
    #[arg(long, value_name = "PATH")]
    executable: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // stdout is the wire; logs go to stderr, which ACP clients capture.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let executable =
        ante_acp::ante_bin::resolve(std::env::var_os(ante_acp::ante_bin::ANTE_ENV), cli.executable);
    tracing::info!(executable = %executable.display(), "ante-acp starting");
    ante_acp::agent::run(executable).await?;
    Ok(())
}
