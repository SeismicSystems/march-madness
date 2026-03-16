//! Embedded tournament data for the 2026 Men's NCAA tournament.
//!
//! Data files are baked into the binary at compile time via `include_str!`.
//! Downstream crates can call `TournamentData::load()` and `KenpomRatings::load()`
//! to get parsed data without any filesystem access.

use serde::Deserialize;
use std::collections::HashMap;

// ── Embedded raw strings ────────────────────────────────────────────

/// Raw JSON content of `data/2026/men/tournament.json`.
pub const TOURNAMENT_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2026/men/tournament.json"
));

/// Raw CSV content of `data/2026/men/kenpom.csv`.
pub const KENPOM_CSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/2026/men/kenpom.csv"
));

// ── Tournament data ─────────────────────────────────────────────────

/// Parsed tournament data (team names, seeds, regions, Final Four pairings).
///
/// This is the same schema as the `TournamentData` in the `tournament` module,
/// re-exported here with a `load()` constructor for convenience.
impl crate::TournamentData {
    /// Load the embedded 2026 Men's tournament data.
    ///
    /// This parses the compile-time-embedded `tournament.json` — no filesystem
    /// access required.
    pub fn load() -> Self {
        serde_json::from_str(TOURNAMENT_JSON)
            .expect("embedded tournament.json should always parse correctly")
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
    /// Load the embedded 2026 Men's KenPom ratings.
    ///
    /// This parses the compile-time-embedded `kenpom.csv` — no filesystem
    /// access required.
    pub fn load() -> Self {
        Self::from_csv(KENPOM_CSV).expect("embedded kenpom.csv should always parse correctly")
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
    fn tournament_data_loads() {
        let data = crate::TournamentData::load();
        assert_eq!(data.regions.len(), 4);
        assert!(data.teams.len() >= 64, "should have at least 64 teams");
    }

    #[test]
    fn kenpom_ratings_load() {
        let ratings = KenpomRatings::load();
        assert!(ratings.teams.len() >= 64, "should have at least 64 teams");
        // Check that the first team has plausible values
        let first = &ratings.teams[0];
        assert!(first.ortg > 80.0 && first.ortg < 150.0);
        assert!(first.drtg > 80.0 && first.drtg < 150.0);
        assert!(first.pace > 50.0 && first.pace < 90.0);
    }

    #[test]
    fn kenpom_map_works() {
        let ratings = KenpomRatings::load();
        let map = ratings.as_map();
        assert!(map.len() >= 64);
        // Spot-check a known team
        assert!(map.contains_key("Duke"), "should contain Duke");
    }

    #[test]
    fn raw_strings_are_nonempty() {
        assert!(!TOURNAMENT_JSON.is_empty());
        assert!(!KENPOM_CSV.is_empty());
    }
}
