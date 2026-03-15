// src/lib.rs
// Organizes the library modules

mod bracket;
pub mod bracket_config;
pub mod calibration;
mod game;
mod metrics;
mod scoring;
pub mod team;
mod tournament;

pub use bracket::Bracket;
pub use bracket_config::BracketConfig;
pub use game::Game;
pub use scoring::ScoringSystem;
pub use team::Team;
pub use tournament::Tournament;

// Bracket structure constants (64-team single-elimination tournament)
pub const NUM_TEAMS: usize = 64;
pub const NUM_GAMES: usize = NUM_TEAMS - 1; // 63
pub const NUM_ROUNDS: usize = 6; // log2(64)

/// Cumulative game counts per round: R64 ends at 32, R32 at 48, S16 at 56, E8 at 60, F4 at 62, Championship at 63.
/// `ROUND_BOUNDARIES[r]` is the first game index of round `r`.
pub const ROUND_BOUNDARIES: [usize; NUM_ROUNDS] = {
    let mut b = [0usize; NUM_ROUNDS];
    let mut r = 0;
    let mut offset = 0;
    while r < NUM_ROUNDS {
        b[r] = offset;
        offset += NUM_TEAMS >> (r + 1); // 32, 16, 8, 4, 2, 1
        r += 1;
    }
    b
};

// Simulation constants
pub const AVERAGE_PACE: f64 = 68.0;
pub const AVERAGE_RATING: f64 = 105.0;

pub const MAX_PACE: f64 = 80.0;
pub const MIN_PACE: f64 = 55.0;

pub const MAX_RTG: f64 = 135.0;
pub const MIN_RTG: f64 = 75.0;

pub const UPDATE_FACTOR: f64 = 0.05;

/// Returns the `data/` directory at the workspace root.
/// Works because `CARGO_MANIFEST_DIR` is `crates/bracket-sim/` — two levels up.
pub fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root from CARGO_MANIFEST_DIR")
        .join("data")
}

/// Returns the `data/{year}/` directory for a given tournament year.
pub fn season_dir(year: u16) -> std::path::PathBuf {
    data_dir().join(year.to_string())
}

/// Load teams from the default data paths for a given year:
/// `data/{year}/tournament.json` + `data/{year}/kenpom.csv`.
/// If `input` is Some, loads from that combined CSV instead.
pub fn load_teams_for_year(
    input: Option<&std::path::Path>,
    year: u16,
) -> std::io::Result<Vec<Team>> {
    if let Some(path) = input {
        let p = path.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF-8 path")
        })?;
        return team::load_teams_from_combined_csv(p);
    }
    let dir = season_dir(year);
    let tournament_json = dir.join("tournament.json");
    let kenpom = dir.join("kenpom.csv");
    let kenpom_str = kenpom
        .to_str()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF-8 path"))?;
    team::load_teams_from_json(&tournament_json, kenpom_str)
}
