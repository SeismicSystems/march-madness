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

/// GET /entries — return the full entry index from Redis.
pub async fn get_entries(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_entries().await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => {
            tracing::error!("failed to read entries: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read entries").into_response()
        }
    }
}

/// GET /entries/:address — return a single entry by address.
pub async fn get_entry(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match state.get_entry(&address).await {
        Ok(Some(entry)) => Json(entry).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "entry not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read entry: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read entry").into_response()
        }
    }
}

#[derive(Serialize)]
pub struct Stats {
    pub total_entries: usize,
    pub scored: usize,
}

/// GET /stats — basic stats from Redis.
pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_entry_count().await {
        Ok(total_entries) => Json(Stats {
            total_entries,
            scored: 0,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("failed to read stats: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read stats").into_response()
        }
    }
}

/// GET /tournament-status — serve tournament status JSON from Redis.
pub async fn get_tournament_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_tournament_status().await {
        Ok(Some(status)) => Json(status).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "tournament status not available").into_response(),
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

/// GET /forecasts — serve forecasts JSON from Redis.
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

// ── Group routes ─────────────────────────────────────────────────────

/// GET /groups — list all groups from Redis.
pub async fn get_groups(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_groups().await {
        Ok(groups) => Json(groups).into_response(),
        Err(e) => {
            tracing::error!("failed to read groups: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read groups").into_response()
        }
    }
}

/// GET /groups/:slug — get a group by slug.
pub async fn get_group(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_group_by_slug(&slug).await {
        Ok(Some(group)) => Json(group).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "group not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read group: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read group").into_response()
        }
    }
}

/// GET /groups/:slug/members — get members of a group.
pub async fn get_group_members(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_group_members(&slug).await {
        Ok(Some(members)) => Json(members).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "group not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read group members: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read group members",
            )
                .into_response()
        }
    }
}

/// GET /address/:address/groups — get groups an address belongs to.
pub async fn get_address_groups(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match state.get_address_groups(&address).await {
        Ok(groups) => Json(groups).into_response(),
        Err(e) => {
            tracing::error!("failed to read address groups: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read address groups",
            )
                .into_response()
        }
    }
}

// ── Mirror routes ────────────────────────────────────────────────────

/// GET /mirrors — list all mirrors from Redis.
pub async fn get_mirrors(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_mirrors().await {
        Ok(mirrors) => Json(mirrors).into_response(),
        Err(e) => {
            tracing::error!("failed to read mirrors: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read mirrors").into_response()
        }
    }
}

/// GET /mirrors/:slug — get a mirror by slug.
pub async fn get_mirror(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_by_slug(&slug).await {
        Ok(Some(mirror)) => Json(mirror).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "mirror not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read mirror: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read mirror").into_response()
        }
    }
}

// ── Forecast routes ─────────────────────────────────────────────────

/// GET /team-probs — per-team advance probabilities from Redis.
pub async fn get_team_probs(State(state): State<AppState>) -> impl IntoResponse {
    match state.get_team_probs().await {
        Ok(probs) => {
            if probs.is_empty() {
                (StatusCode::NOT_FOUND, "team probs not available").into_response()
            } else {
                Json(probs).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read team probs: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read team probs",
            )
                .into_response()
        }
    }
}

/// GET /forecasts/groups/s/:slug — group forecast by slug.
pub async fn get_group_forecast_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_group_forecast_by_slug(&slug).await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "group forecast not found").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read group forecast: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read group forecast",
            )
                .into_response()
        }
    }
}

/// GET /forecasts/groups/id/:id — group forecast by ID.
pub async fn get_group_forecast_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_group_forecast_by_id(&id).await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "group forecast not found").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read group forecast: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read group forecast",
            )
                .into_response()
        }
    }
}

/// GET /forecasts/mirrors/s/:slug — mirror forecast by slug.
pub async fn get_mirror_forecast_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_forecast_by_slug(&slug).await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "mirror forecast not found").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read mirror forecast: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read mirror forecast",
            )
                .into_response()
        }
    }
}

/// GET /forecasts/mirrors/id/:id — mirror forecast by ID.
pub async fn get_mirror_forecast_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_forecast_by_id(&id).await {
        Ok(data) => {
            if data.is_null() {
                (StatusCode::NOT_FOUND, "mirror forecast not found").into_response()
            } else {
                Json(data).into_response()
            }
        }
        Err(e) => {
            tracing::error!("failed to read mirror forecast: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read mirror forecast",
            )
                .into_response()
        }
    }
}

/// GET /mirrors/id/:id — get a mirror by ID.
pub async fn get_mirror_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_by_id(&id).await {
        Ok(Some(mirror)) => Json(mirror).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "mirror not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read mirror: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to read mirror").into_response()
        }
    }
}

/// GET /mirrors/id/:id/entries — get entries for a mirror by ID.
pub async fn get_mirror_entries_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_entries_by_id(&id).await {
        Ok(Some(entries)) => Json(entries).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "mirror not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read mirror entries: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read mirror entries",
            )
                .into_response()
        }
    }
}

/// GET /mirrors/:slug/entries — get all entries in a mirror.
pub async fn get_mirror_entries(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.get_mirror_entries(&slug).await {
        Ok(Some(entries)) => Json(entries).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "mirror not found").into_response(),
        Err(e) => {
            tracing::error!("failed to read mirror entries: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read mirror entries",
            )
                .into_response()
        }
    }
}
