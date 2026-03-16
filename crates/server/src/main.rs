mod routes;
mod state;

use std::path::PathBuf;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::state::AppState;

#[derive(Parser, Debug)]
#[command(name = "march-madness-server", about = "Serve bracket index data")]
struct Cli {
    /// Port to listen on.
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Path to the JSON index file written by the indexer.
    #[arg(long, default_value = "data/entries.json")]
    index_file: PathBuf,

    /// Path to the tournament status JSON file.
    #[arg(long, default_value = "data/2026/men/status.json")]
    tournament_status_file: PathBuf,

    /// Path to the forecasts JSON file (from forecaster crate).
    #[arg(long, default_value = "data/2026/men/forecasts.json")]
    forecasts_file: PathBuf,

    /// API key for POST /api/tournament-status. Read from TOURNAMENT_API_KEY env var if not set.
    #[arg(long, env = "TOURNAMENT_API_KEY")]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let state = AppState::new(
        cli.index_file.clone(),
        Duration::from_secs(5),
        cli.tournament_status_file.clone(),
        cli.forecasts_file.clone(),
        cli.api_key.clone(),
    );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/api/entries", get(routes::get_entries))
        .route("/api/entries/{address}", get(routes::get_entry))
        .route("/api/stats", get(routes::get_stats))
        .route(
            "/api/tournament-status",
            get(routes::get_tournament_status).post(routes::post_tournament_status),
        )
        .route("/api/forecasts", get(routes::get_forecasts))
        .route("/api/groups", get(routes::get_groups))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = TcpListener::bind(&addr).await?;
    info!(
        port = cli.port,
        index_file = %cli.index_file.display(),
        "server listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("server shut down");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
