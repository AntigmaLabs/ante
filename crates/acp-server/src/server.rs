use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::routes::*;
use crate::state::AcState;

/// Build the ACP router with all routes.
pub fn build_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let state = AcState::new();

    Router::new()
        .route("/ping", get(ping_handler))
        .route("/agents", get(agents_list_handler))
        .route("/agents/{name}", get(agents_get_handler))
        .route("/runs", post(run_create_handler))
        .route(
            "/runs/{run_id}",
            get(run_get_handler).post(run_resume_handler),
        )
        .route("/runs/{run_id}/cancel", post(run_cancel_handler))
        .layer(cors)
        .with_state(state)
}

/// Initialize tracing subscriber.
pub fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("ante_acp_server=info,tower_http=info")
            }),
        )
        .init();
}

/// Start the ACP server.
pub async fn start_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let app = build_router();
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Ante ACP server listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutting down ACP server...");
}
