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

    /// Path to the forecasts JSON file (from forecaster crate).
    #[arg(long, default_value = "data/2026/men/forecasts.json")]
    forecasts_file: PathBuf,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let state = AppState::new(cli.forecasts_file.clone(), Duration::from_secs(5)).await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/entries", get(routes::get_entries))
        .route("/entries/{address}", get(routes::get_entry))
        .route("/stats", get(routes::get_stats))
        .route("/tournament-status", get(routes::get_tournament_status))
        .route("/forecasts", get(routes::get_forecasts))
        // Group routes
        .route("/groups", get(routes::get_groups))
        .route("/groups/{slug}", get(routes::get_group))
        .route("/groups/{slug}/members", get(routes::get_group_members))
        // Address routes
        .route("/address/{address}/groups", get(routes::get_address_groups))
        // Mirror routes
        .route("/mirrors", get(routes::get_mirrors))
        .route("/mirrors/{slug}", get(routes::get_mirror))
        .route("/mirrors/{slug}/entries", get(routes::get_mirror_entries))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = TcpListener::bind(&addr).await?;
    info!(port = cli.port, "server listening");

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
