//! `ncaa-api` — NCAA basketball API client.
//!
//! Provides a rate-limited HTTP client for the NCAA's GraphQL API,
//! focused on basketball (MBB/WBB, Division 1) scoreboard and schedule data.

pub mod client;
pub mod schedule;
pub mod scoreboard;
pub mod types;

pub use client::NcaaClient;
pub use schedule::fetch_schedule;
pub use scoreboard::fetch_scoreboard;
pub use types::{Contest, ContestDate, ContestState, Period, SportCode, Team};

/// Errors from the NCAA API client.
#[derive(Debug, thiserror::Error)]
pub enum NcaaApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("unexpected HTTP status {status} from {url}")]
    HttpStatus {
        status: reqwest::StatusCode,
        url: String,
    },

    #[error("failed to parse API response: {0}")]
    Parse(String),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid configuration: {0}")]
    Config(String),
}
