use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::state::AppState;

/// GET /health — simple health check.
pub async fn health() -> &'static str {
    "OK"
}

/// GET /api/entries — return the full entry index.
pub async fn get_entries(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_index().await {
        Ok(index) => Json(index).into_response(),
        Err(e) => {
            tracing::error!("failed to read index: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read index").into_response()
        }
    }
}

/// GET /api/entries/:address — return a single entry by address.
pub async fn get_entry(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match state.get_index().await {
        Ok(index) => {
            // Try both the raw address and lowercased version.
            let key = address.to_lowercase();
            if let Some(entry) = index.get(&key).or_else(|| index.get(&address)) {
                Json(entry.clone()).into_response()
            } else {
                (StatusCode::NOT_FOUND, "entry not found").into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read index: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read index").into_response()
        }
    }
}

#[derive(Serialize)]
pub struct Stats {
    pub total_entries: usize,
    pub brackets_revealed: usize,
    pub scored: usize,
}

/// GET /api/stats — basic stats derived from the index.
pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_index().await {
        Ok(index) => {
            let total_entries = index.len();
            let brackets_revealed = index.values().filter(|e| e.bracket.is_some()).count();
            // Currently EntryRecord doesn't have a score field, so scored = 0.
            // This will be updated when the indexer tracks scoring events.
            let scored = 0;
            Json(Stats {
                total_entries,
                brackets_revealed,
                scored,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("failed to read index: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read index").into_response()
        }
    }
}
