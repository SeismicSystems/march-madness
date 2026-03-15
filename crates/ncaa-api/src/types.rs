//! NCAA API response types.
//!
//! Raw NCAA API responses are deserialized into `Scoreboard*`/`Schedule*` types,
//! then converted via `TryFrom` into strongly-typed `Contest`/`Team` types.
//! String fields from the API are parsed into enums, integers, and timestamps
//! at conversion time — callers never deal with raw strings.

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use crate::NcaaApiError;

// ── Strongly-typed output types ─────────────────────────────────────

/// A single team in an NCAA contest (strongly typed).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    /// Short display name (e.g. "Duke", "Michigan St").
    pub name_short: String,
    /// 6-character name (e.g. "DUKE", "MICHST").
    pub name_6char: String,
    /// SEO-friendly name slug.
    pub seoname: String,
    /// Current score (None for pre-game).
    pub score: Option<u32>,
    /// Tournament seed (None for non-tournament games).
    pub seed: Option<u32>,
    /// Whether this team won (only meaningful for final games).
    pub is_winner: bool,
    /// Whether this is the home team.
    pub is_home: bool,
}

/// State of an NCAA contest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContestState {
    /// Game hasn't started yet.
    Pre,
    /// Game is in progress.
    Live,
    /// Game is final (bool = went to overtime).
    Final(bool),
    /// Unknown state from the API.
    Other,
}

/// Current period of play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
    /// Regular half (1 or 2 for basketball).
    Half(u8),
    /// Overtime period (1 = first OT, 2 = second OT, etc.).
    Overtime(u8),
}

impl Period {
    /// Convert to the period number used in GameStatus (1, 2, 3=OT, 4=2OT, etc.).
    pub fn as_number(&self) -> u8 {
        match self {
            Period::Half(n) => *n,
            Period::Overtime(n) => 2 + n,
        }
    }
}

/// A single contest (game) from the NCAA scoreboard (strongly typed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    /// Unique contest identifier.
    pub contest_id: i64,
    /// The two teams.
    pub teams: Vec<Team>,
    /// Parsed game state.
    pub state: ContestState,
    /// Current period (None for pre-game or unknown).
    pub period: Option<Period>,
    /// Seconds remaining on the game clock (None for pre-game or halftime).
    pub clock_seconds: Option<i32>,
    /// Start time as Unix epoch seconds (None if unparseable).
    pub start_time_epoch: Option<i64>,
    /// Start date string (passed through from API).
    pub start_date: String,
    /// Start time display string (e.g. "12:00PM ET").
    pub start_time: String,
}

impl Contest {
    pub fn is_final(&self) -> bool {
        matches!(self.state, ContestState::Final(_))
    }

    pub fn is_live(&self) -> bool {
        self.state == ContestState::Live
    }

    /// Get scores for both teams. Returns (team0_score, team1_score).
    pub fn scores(&self) -> Option<(u32, u32)> {
        if self.teams.len() < 2 {
            return None;
        }
        Some((self.teams[0].score?, self.teams[1].score?))
    }
}

// ── Raw NCAA API types (deserialization only) ───────────────────────

/// Raw team from the NCAA GraphQL API.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawTeam {
    #[serde(default)]
    pub name_short: String,
    #[serde(default, rename = "name6Char")]
    pub name_6char: String,
    #[serde(default)]
    pub seoname: String,
    #[serde(default)]
    pub score: String,
    #[serde(default)]
    pub seed: String,
    #[serde(default)]
    pub is_winner: bool,
    #[serde(default)]
    pub is_home: bool,
}

/// Raw scoreboard response from the NCAA GraphQL API.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ScoreboardGqlResponse {
    pub data: Option<ScoreboardData>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ScoreboardData {
    #[serde(rename = "scoreboard")]
    pub scoreboard: Option<Vec<RawContest>>,
}

/// Raw contest from the NCAA GraphQL scoreboard.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawContest {
    #[serde(default)]
    pub contest_id: serde_json::Value,
    #[serde(default)]
    pub teams: Vec<RawTeam>,
    #[serde(default)]
    pub game_state: String,
    #[serde(default)]
    pub current_period: String,
    #[serde(default)]
    pub contest_clock: String,
    #[serde(default)]
    pub start_time_epoch: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub final_message: String,
}

impl TryFrom<RawContest> for Contest {
    type Error = NcaaApiError;

    fn try_from(raw: RawContest) -> Result<Self, NcaaApiError> {
        let contest_id = match &raw.contest_id {
            serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| {
                NcaaApiError::Parse(format!("contest_id not an i64: {}", raw.contest_id))
            })?,
            serde_json::Value::String(s) => s
                .parse()
                .map_err(|_| NcaaApiError::Parse(format!("contest_id not parseable: {s}")))?,
            _ => {
                return Err(NcaaApiError::Parse(format!(
                    "unexpected contest_id type: {}",
                    raw.contest_id
                )));
            }
        };

        let teams: Vec<Team> = raw.teams.into_iter().map(Team::from).collect();

        let is_overtime = raw.final_message.contains("OT");
        let state = match raw.game_state.as_str() {
            "F" => ContestState::Final(is_overtime),
            "P" => ContestState::Pre,
            "I" => ContestState::Live,
            _ => ContestState::Other,
        };

        let period = parse_period(&raw.current_period);
        let clock_seconds = parse_clock(&raw.contest_clock);

        let start_time_epoch = if raw.start_time_epoch.is_empty() {
            None
        } else {
            Some(raw.start_time_epoch.parse::<i64>().map_err(|_| {
                NcaaApiError::Parse(format!(
                    "start_time_epoch not parseable: {}",
                    raw.start_time_epoch
                ))
            })?)
        };

        Ok(Contest {
            contest_id,
            teams,
            state,
            period,
            clock_seconds,
            start_time_epoch,
            start_date: raw.start_date,
            start_time: raw.start_time,
        })
    }
}

impl From<RawTeam> for Team {
    fn from(raw: RawTeam) -> Self {
        Team {
            name_short: raw.name_short,
            name_6char: raw.name_6char,
            seoname: raw.seoname,
            score: raw.score.parse::<u32>().ok(),
            seed: raw.seed.parse::<u32>().ok(),
            is_winner: raw.is_winner,
            is_home: raw.is_home,
        }
    }
}

/// Schedule API response — used to get today's contest date.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ScheduleGqlResponse {
    pub data: Option<ScheduleData>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ScheduleData {
    #[serde(rename = "schedule")]
    pub schedule: Option<Vec<ScheduleEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScheduleEntry {
    #[serde(default)]
    pub contest_date: String,
    #[serde(default)]
    pub number_of_games: i32,
}

// ── Sport code ──────────────────────────────────────────────────────

/// Sport code for NCAA API queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SportCode {
    /// Men's basketball.
    Mbb,
    /// Women's basketball.
    Wbb,
}

impl SportCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SportCode::Mbb => "MBB",
            SportCode::Wbb => "WBB",
        }
    }
}

impl std::fmt::Display for SportCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SportCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mbb" => Ok(SportCode::Mbb),
            "wbb" => Ok(SportCode::Wbb),
            _ => Err(format!("unknown sport code: {s} (expected mbb or wbb)")),
        }
    }
}

// ── Parsing helpers ─────────────────────────────────────────────────

/// Parse NCAA period string into a Period enum.
fn parse_period(s: &str) -> Option<Period> {
    let p = s.trim();
    if p.is_empty() || p.eq_ignore_ascii_case("FINAL") {
        return None;
    }
    if let Ok(n) = p.parse::<u8>() {
        return Some(Period::Half(n));
    }
    if p.eq_ignore_ascii_case("HALF") {
        return Some(Period::Half(1));
    }
    if p.eq_ignore_ascii_case("OT") || p.eq_ignore_ascii_case("1OT") {
        return Some(Period::Overtime(1));
    }
    if let Some(num_str) = p.strip_suffix("OT")
        && let Ok(n) = num_str.parse::<u8>()
    {
        return Some(Period::Overtime(n));
    }
    None
}

/// Parse NCAA clock string "MM:SS" into total seconds remaining.
fn parse_clock(s: &str) -> Option<i32> {
    let clock = s.trim();
    if clock.is_empty() {
        return None;
    }
    if clock == "0:00" {
        return Some(0);
    }
    let (mins, secs) = clock.split_once(':')?;
    let mins = mins.parse::<i32>().ok()?;
    let secs = secs.parse::<i32>().ok()?;
    Some(mins * 60 + secs)
}

// ── NCAA contest date type ──────────────────────────────────────────

/// A date in NCAA API format (YYYY/MM/DD). Ensures valid format at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContestDate {
    date: chrono::NaiveDate,
}

impl ContestDate {
    /// Parse from "YYYY/MM/DD" format.
    pub fn parse(s: &str) -> Result<Self, NcaaApiError> {
        let date = chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d")
            .map_err(|e| NcaaApiError::Parse(format!("invalid contest date '{s}': {e}")))?;
        Ok(Self { date })
    }

    /// Create from a chrono NaiveDate.
    pub fn from_naive(date: chrono::NaiveDate) -> Self {
        Self { date }
    }

    /// Format as "YYYY/MM/DD" for the NCAA API.
    pub fn as_api_str(&self) -> String {
        self.date.format("%Y/%m/%d").to_string()
    }

    /// Get the underlying date.
    pub fn date(&self) -> chrono::NaiveDate {
        self.date
    }

    /// Compute NCAA season year for this date.
    /// NCAA season year = calendar year for dates Aug-Dec, calendar year - 1 for Jan-Jul.
    /// This means the 2025-2026 basketball season (including March Madness 2026) has
    /// season_year = 2025. The NCAA API expects this value.
    pub fn season_year(&self) -> i32 {
        let year = self.date.year();
        let month = self.date.month();
        if month < 7 { year - 1 } else { year }
    }
}

impl std::fmt::Display for ContestDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_api_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clock() {
        assert_eq!(parse_clock("15:42"), Some(15 * 60 + 42));
        assert_eq!(parse_clock("0:00"), Some(0));
        assert_eq!(parse_clock(""), None);
    }

    #[test]
    fn test_parse_period() {
        assert_eq!(parse_period("1"), Some(Period::Half(1)));
        assert_eq!(parse_period("2"), Some(Period::Half(2)));
        assert_eq!(parse_period("OT"), Some(Period::Overtime(1)));
        assert_eq!(parse_period("2OT"), Some(Period::Overtime(2)));
        assert_eq!(parse_period("FINAL"), None);
        assert_eq!(parse_period(""), None);
    }

    #[test]
    fn test_sport_code_roundtrip() {
        assert_eq!("mbb".parse::<SportCode>().unwrap(), SportCode::Mbb);
        assert_eq!("WBB".parse::<SportCode>().unwrap(), SportCode::Wbb);
        assert_eq!(SportCode::Mbb.as_str(), "MBB");
    }

    #[test]
    fn test_contest_date() {
        let d = ContestDate::parse("2026/03/15").unwrap();
        assert_eq!(d.as_api_str(), "2026/03/15");
        // March 2026 → 2025 season (season starts in fall)
        assert_eq!(d.season_year(), 2025);

        let d2 = ContestDate::parse("2026/11/15").unwrap();
        assert_eq!(d2.season_year(), 2026);

        assert!(ContestDate::parse("not-a-date").is_err());
    }

    #[test]
    fn test_contest_state_from_raw() {
        // Test the parsing through a minimal RawContest
        let raw = RawContest {
            contest_id: serde_json::json!(12345),
            teams: vec![],
            game_state: "F".into(),
            current_period: "FINAL".into(),
            contest_clock: "0:00".into(),
            start_time_epoch: "1742000000".into(),
            start_date: "2026-03-15".into(),
            start_time: "12:00PM ET".into(),
            final_message: "FINAL/OT".into(),
        };
        let contest = Contest::try_from(raw).unwrap();
        assert_eq!(contest.state, ContestState::Final(true)); // overtime
        assert_eq!(contest.contest_id, 12345);
        assert_eq!(contest.start_time_epoch, Some(1742000000));
    }

    #[test]
    fn test_team_score_parsing() {
        let raw = RawTeam {
            name_short: "Duke".into(),
            name_6char: "DUKE".into(),
            seoname: "duke".into(),
            score: "82".into(),
            seed: "1".into(),
            is_winner: true,
            is_home: false,
        };
        let team = Team::from(raw);
        assert_eq!(team.score, Some(82));
        assert_eq!(team.seed, Some(1));

        // Pre-game: empty score/seed
        let raw_pre = RawTeam {
            name_short: "Duke".into(),
            name_6char: "DUKE".into(),
            seoname: "duke".into(),
            score: String::new(),
            seed: String::new(),
            is_winner: false,
            is_home: true,
        };
        let team_pre = Team::from(raw_pre);
        assert_eq!(team_pre.score, None);
        assert_eq!(team_pre.seed, None);
    }
}
