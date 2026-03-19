use serde::{Deserialize, Serialize};

/// An indexed bracket entry, keyed by address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRecord {
    /// Optional display name (from setTag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// When this entry was last updated on-chain.
    pub updated: UpdateInfo,

    /// Hex-encoded bracket bytes (after reveal / post-deadline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub block: u64,
    pub ts: u64,
}

/// The full index file written by the indexer and served by the server.
pub type EntryIndex = std::collections::BTreeMap<String, EntryRecord>;

// ── Tournament Status ────────────────────────────────────────────────

/// Status of a single game in the bracket (indexed 0-62).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub game_index: u8,
    pub status: GameState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<GameScore>,
    /// true = team1 won (final only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<bool>,
    /// Probability that team1 wins (0-1). For live games — conditional on
    /// current in-game score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team1_win_probability: Option<f64>,
    /// Seconds remaining in the current period (live games only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds_remaining: Option<i32>,
    /// Current period number (1 = 1st half, 2 = 2nd half, 3+ = OT).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameState {
    Upcoming,
    Live,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameScore {
    pub team1: u32,
    pub team2: u32,
}

impl GameStatus {
    /// Create an upcoming game with no score data.
    pub fn upcoming(game_index: u8) -> Self {
        Self {
            game_index,
            status: GameState::Upcoming,
            score: None,
            winner: None,
            team1_win_probability: None,
            seconds_remaining: None,
            period: None,
        }
    }
}

/// Full tournament status — served by backend, updated via POST.
///
/// Contains game states only. Team reach probabilities are computed by the
/// forecaster and stored separately in the `mm:probs` Redis key.
///
/// For live games, `team1WinProbability` on the GameStatus provides in-game
/// win probability conditional on the current score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentStatus {
    pub games: Vec<GameStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

// ── Forecast Output ──────────────────────────────────────────────────

/// Forecast for a single bracket entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketForecast {
    /// Current score from decided games.
    pub current_score: u32,
    /// Maximum possible score if all remaining picks are correct.
    pub max_possible_score: u32,
    /// Expected final score (average over simulations).
    pub expected_score: f64,
    /// Probability of finishing with the highest score (winning the pool).
    pub win_probability: f64,
    /// Optional display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The full forecast file — address → BracketForecast.
pub type ForecastIndex = std::collections::BTreeMap<String, BracketForecast>;
