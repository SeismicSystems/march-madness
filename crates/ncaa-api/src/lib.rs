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
pub use types::{Contest, SportCode, Team};

/// Errors from the NCAA API client.
#[derive(Debug, thiserror::Error)]
pub enum NcaaApiError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("config error: {0}")]
    Config(String),
}
