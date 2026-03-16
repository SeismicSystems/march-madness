//! Embedded tournament data for NCAA tournaments.
//!
//! Data files are baked into the binary at compile time via `include_str!`.
//! Downstream crates call `TournamentData::embedded(year)` and
//! `KenpomRatings::embedded(year)` to get parsed data without filesystem access.

use serde::Deserialize;
use std::collections::HashMap;

// ── Embedded raw strings ────────────────────────────────────────────

const TOURNAMENT_JSON_2025: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2025/men/tournament.json"
));
const KENPOM_CSV_2025: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2025/men/kenpom.csv"
));

const TOURNAMENT_JSON_2026: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2026/men/tournament.json"
));
const KENPOM_CSV_2026: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2026/men/kenpom.csv"
));

/// Return the embedded tournament JSON string for a given year, if available.
pub fn tournament_json(year: u16) -> Option<&'static str> {
    match year {
        2025 => Some(TOURNAMENT_JSON_2025),
        2026 => Some(TOURNAMENT_JSON_2026),
        _ => None,
    }
}

/// Return the embedded KenPom CSV string for a given year, if available.
pub fn kenpom_csv(year: u16) -> Option<&'static str> {
    match year {
        2025 => Some(KENPOM_CSV_2025),
        2026 => Some(KENPOM_CSV_2026),
        _ => None,
    }
}

// ── Tournament data ─────────────────────────────────────────────────

impl crate::TournamentData {
    /// Load embedded tournament data for the given year.
    ///
    /// Panics if the year is not embedded at compile time.
    pub fn embedded(year: u16) -> Self {
        let json = tournament_json(year)
            .unwrap_or_else(|| panic!("no embedded tournament data for year {year}"));
        serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("failed to parse embedded tournament.json for {year}: {e}"))
    }
}

// ── KenPom ratings ──────────────────────────────────────────────────

/// A single team's KenPom ratings.
#[derive(Debug, Clone)]
pub struct KenpomEntry {
    pub team: String,
    pub ortg: f64,
    pub drtg: f64,
    pub pace: f64,
    pub goose: f64,
}

/// Row for CSV deserialization.
#[derive(Debug, Deserialize)]
struct KenpomRow {
    team: String,
    ortg: f64,
    drtg: f64,
    pace: f64,
    #[serde(default)]
    goose: f64,
}

/// Parsed KenPom ratings for all tournament teams.
#[derive(Debug, Clone)]
pub struct KenpomRatings {
    /// All team entries, in file order (sorted by net rating, best first).
    pub teams: Vec<KenpomEntry>,
}

impl KenpomRatings {
    /// Load embedded KenPom ratings for the given year.
    ///
    /// Panics if the year is not embedded at compile time.
    pub fn embedded(year: u16) -> Self {
        let csv =
            kenpom_csv(year).unwrap_or_else(|| panic!("no embedded KenPom data for year {year}"));
        Self::from_csv(csv)
            .unwrap_or_else(|e| panic!("failed to parse embedded kenpom.csv for {year}: {e}"))
    }

    /// Parse KenPom ratings from a CSV string.
    pub fn from_csv(csv_content: &str) -> eyre::Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_content.as_bytes());

        let mut teams = Vec::new();
        for row in reader.deserialize() {
            let r: KenpomRow = row?;
            teams.push(KenpomEntry {
                team: r.team,
                ortg: r.ortg,
                drtg: r.drtg,
                pace: r.pace,
                goose: r.goose,
            });
        }
        Ok(KenpomRatings { teams })
    }

    /// Build a lookup map: team name -> (ortg, drtg, pace, goose).
    pub fn as_map(&self) -> HashMap<String, (f64, f64, f64, f64)> {
        self.teams
            .iter()
            .map(|e| (e.team.clone(), (e.ortg, e.drtg, e.pace, e.goose)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tournament_data_2026() {
        let data = crate::TournamentData::embedded(2026);
        assert_eq!(data.regions.len(), 4);
        assert!(data.teams.len() >= 64, "should have at least 64 teams");
    }

    #[test]
    fn tournament_data_2025() {
        let data = crate::TournamentData::embedded(2025);
        assert_eq!(data.regions.len(), 4);
        assert!(data.teams.len() >= 64, "should have at least 64 teams");
    }

    #[test]
    #[should_panic(expected = "no embedded tournament data for year 2020")]
    fn tournament_data_missing_year() {
        crate::TournamentData::embedded(2020);
    }

    #[test]
    fn kenpom_ratings_2026() {
        let ratings = KenpomRatings::embedded(2026);
        assert!(ratings.teams.len() >= 64, "should have at least 64 teams");
        let first = &ratings.teams[0];
        assert!(first.ortg > 80.0 && first.ortg < 150.0);
        assert!(first.drtg > 80.0 && first.drtg < 150.0);
        assert!(first.pace > 50.0 && first.pace < 90.0);
    }

    #[test]
    fn kenpom_ratings_2025() {
        let ratings = KenpomRatings::embedded(2025);
        assert!(ratings.teams.len() >= 64, "should have at least 64 teams");
    }

    #[test]
    #[should_panic(expected = "no embedded KenPom data for year 2020")]
    fn kenpom_ratings_missing_year() {
        KenpomRatings::embedded(2020);
    }

    #[test]
    fn kenpom_map_works() {
        let ratings = KenpomRatings::embedded(2026);
        let map = ratings.as_map();
        assert!(map.len() >= 64);
        assert!(map.contains_key("Duke"), "should contain Duke");
    }

    #[test]
    fn raw_strings_are_nonempty() {
        assert!(tournament_json(2025).unwrap().len() > 100);
        assert!(tournament_json(2026).unwrap().len() > 100);
        assert!(kenpom_csv(2025).unwrap().len() > 100);
        assert!(kenpom_csv(2026).unwrap().len() > 100);
    }
}
