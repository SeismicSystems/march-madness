//! NCAA API response types.

use serde::{Deserialize, Serialize};

/// A single team in an NCAA contest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    /// Short display name (e.g. "Duke", "Michigan St").
    #[serde(default)]
    pub name_short: String,

    /// 6-character name (e.g. "DUKE", "MICHST").
    #[serde(default, rename = "name6Char")]
    pub name_6char: String,

    /// SEO-friendly name slug.
    #[serde(default)]
    pub seoname: String,

    /// Current score (string in API, may be empty for pre-game).
    #[serde(default)]
    pub score: String,

    /// Tournament seed (string, may be empty for non-tournament games).
    #[serde(default)]
    pub seed: String,

    /// Whether this team won (only meaningful for final games).
    #[serde(default)]
    pub is_winner: bool,

    /// Whether this is the home team.
    #[serde(default)]
    pub is_home: bool,
}

/// A single contest (game) from the NCAA scoreboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contest {
    /// Unique contest identifier.
    #[serde(default)]
    pub contest_id: i64,

    /// The two teams. Index 0 = away, index 1 = home (typically).
    #[serde(default)]
    pub teams: Vec<Team>,

    /// Game state: "F" = final, "P" = pre-game, "I" = in-progress.
    #[serde(default)]
    pub game_state: String,

    /// Current period string (e.g. "1", "2", "OT", "FINAL").
    #[serde(default)]
    pub current_period: String,

    /// Game clock string (e.g. "15:42", "FINAL").
    #[serde(default)]
    pub contest_clock: String,

    /// Start time as Unix epoch string (seconds? milliseconds? varies).
    #[serde(default)]
    pub start_time_epoch: String,

    /// Start date string.
    #[serde(default)]
    pub start_date: String,

    /// Start time string (e.g. "12:00PM ET").
    #[serde(default)]
    pub start_time: String,

    /// Final message (e.g. "FINAL", "FINAL/OT").
    #[serde(default)]
    pub final_message: String,
}

impl Contest {
    /// Whether this game is final.
    pub fn is_final(&self) -> bool {
        self.game_state == "F"
    }

    /// Whether this game is in progress.
    pub fn is_live(&self) -> bool {
        self.game_state == "I"
    }

    /// Whether this game hasn't started.
    pub fn is_pre(&self) -> bool {
        self.game_state == "P"
    }

    /// Parse scores for both teams. Returns (team0_score, team1_score).
    pub fn scores(&self) -> Option<(u32, u32)> {
        if self.teams.len() < 2 {
            return None;
        }
        let s0 = self.teams[0].score.parse::<u32>().ok()?;
        let s1 = self.teams[1].score.parse::<u32>().ok()?;
        Some((s0, s1))
    }

    /// Parse the contest clock into total seconds remaining.
    /// Handles "MM:SS" format. Returns None if unparseable.
    pub fn clock_seconds(&self) -> Option<i32> {
        let clock = self.contest_clock.trim();
        if clock.is_empty() || clock == "0:00" {
            return Some(0);
        }
        let parts: Vec<&str> = clock.split(':').collect();
        if parts.len() == 2 {
            let mins = parts[0].parse::<i32>().ok()?;
            let secs = parts[1].parse::<i32>().ok()?;
            Some(mins * 60 + secs)
        } else {
            None
        }
    }

    /// Parse the current period as a number.
    /// "1" → 1, "2" → 2, "OT" → 3, "2OT" → 4, etc.
    pub fn period_number(&self) -> Option<u8> {
        let p = self.current_period.trim();
        if let Ok(n) = p.parse::<u8>() {
            return Some(n);
        }
        if p.eq_ignore_ascii_case("HALF") {
            return Some(1);
        }
        if p.eq_ignore_ascii_case("OT") || p.eq_ignore_ascii_case("1OT") {
            return Some(3);
        }
        // "2OT" → 4, "3OT" → 5, etc.
        if let Some(num_str) = p.strip_suffix("OT")
            && let Ok(n) = num_str.parse::<u8>()
        {
            return Some(2 + n);
        }
        if p.eq_ignore_ascii_case("FINAL") {
            return None; // not meaningful for final games
        }
        None
    }
}

/// Raw scoreboard response from the NCAA GraphQL API.
/// The actual structure is nested; we extract what we need.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoreboardGqlResponse {
    pub data: Option<ScoreboardData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoreboardData {
    #[serde(rename = "scoreboard")]
    pub scoreboard: Option<Vec<ScoreboardContest>>,
}

/// A contest entry in the GQL scoreboard response.
/// This matches the NCAA's GraphQL schema for scoreboard queries.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreboardContest {
    #[serde(default)]
    pub contest_id: serde_json::Value,
    #[serde(default)]
    pub teams: Vec<Team>,
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

impl From<ScoreboardContest> for Contest {
    fn from(sc: ScoreboardContest) -> Self {
        let contest_id = match &sc.contest_id {
            serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };
        Contest {
            contest_id,
            teams: sc.teams,
            game_state: sc.game_state,
            current_period: sc.current_period,
            contest_clock: sc.contest_clock,
            start_time_epoch: sc.start_time_epoch,
            start_date: sc.start_date,
            start_time: sc.start_time,
            final_message: sc.final_message,
        }
    }
}

/// Schedule API response — used to get today's contest date.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleGqlResponse {
    pub data: Option<ScheduleData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleData {
    #[serde(rename = "schedule")]
    pub schedule: Option<Vec<ScheduleEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntry {
    /// Date string in "YYYY/MM/DD" format.
    #[serde(default)]
    pub contest_date: String,
    /// Number of games on this date.
    #[serde(default)]
    pub number_of_games: i32,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contest_clock_parsing() {
        let mut c = Contest {
            contest_id: 0,
            teams: vec![],
            game_state: "I".into(),
            current_period: "1".into(),
            contest_clock: "15:42".into(),
            start_time_epoch: String::new(),
            start_date: String::new(),
            start_time: String::new(),
            final_message: String::new(),
        };
        assert_eq!(c.clock_seconds(), Some(15 * 60 + 42));

        c.contest_clock = "0:00".into();
        assert_eq!(c.clock_seconds(), Some(0));

        c.contest_clock = String::new();
        assert_eq!(c.clock_seconds(), Some(0));
    }

    #[test]
    fn test_period_number() {
        let make = |period: &str| Contest {
            contest_id: 0,
            teams: vec![],
            game_state: "I".into(),
            current_period: period.into(),
            contest_clock: String::new(),
            start_time_epoch: String::new(),
            start_date: String::new(),
            start_time: String::new(),
            final_message: String::new(),
        };
        assert_eq!(make("1").period_number(), Some(1));
        assert_eq!(make("2").period_number(), Some(2));
        assert_eq!(make("OT").period_number(), Some(3));
        assert_eq!(make("2OT").period_number(), Some(4));
        assert_eq!(make("FINAL").period_number(), None);
    }

    #[test]
    fn test_sport_code_roundtrip() {
        assert_eq!("mbb".parse::<SportCode>().unwrap(), SportCode::Mbb);
        assert_eq!("WBB".parse::<SportCode>().unwrap(), SportCode::Wbb);
        assert_eq!(SportCode::Mbb.as_str(), "MBB");
    }
}
