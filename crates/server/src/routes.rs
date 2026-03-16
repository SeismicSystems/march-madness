use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
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
    pub scored: usize,
}

/// GET /api/stats — basic stats derived from the index.
pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_index().await {
        Ok(index) => {
            let total_entries = index.len();
            // Currently EntryRecord doesn't have a score field, so scored = 0.
            // This will be updated when the indexer tracks scoring events.
            let scored = 0;
            Json(Stats {
                total_entries,
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

/// GET /api/tournament-status — serve tournament status JSON.
pub async fn get_tournament_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_tournament_status().await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "tournament status not available").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read tournament status: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read tournament status",
            )
                .into_response()
        }
    }
}

/// GET /api/forecasts — serve forecasts JSON (from forecaster crate output).
pub async fn get_forecasts(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_forecasts().await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "forecasts not available").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read forecasts: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read forecasts",
            )
                .into_response()
        }
    }
}

/// GET /api/groups — stub endpoint returning an empty list of public groups.
/// Placeholder for a future registry of public groups.
pub async fn get_groups() -> impl IntoResponse {
    Json(serde_json::json!([])).into_response()
}

/// POST /api/tournament-status — update tournament status JSON (requires API key).
pub async fn post_tournament_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Check API key from Authorization header
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let key = auth.strip_prefix("Bearer ").unwrap_or(auth);
    if !state.check_api_key(key) {
        return (StatusCode::UNAUTHORIZED, "invalid or missing API key").into_response();
    }

    match state.set_tournament_status(body).await {
        Ok(()) => (StatusCode::OK, "updated").into_response(),
        Err(e) => {
            tracing::error!("failed to write tournament status: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to write tournament status",
            )
                .into_response()
        }
    }
}
