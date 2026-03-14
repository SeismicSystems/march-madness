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
    #[arg(long, default_value = "3001")]
    port: u16,

    /// Path to the JSON index file written by the indexer.
    #[arg(long, default_value = "data/entries.json")]
    index_file: PathBuf,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let state = AppState::new(cli.index_file.clone(), Duration::from_secs(5));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/api/entries", get(routes::get_entries))
        .route("/api/entries/{address}", get(routes::get_entry))
        .route("/api/stats", get(routes::get_stats))
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
