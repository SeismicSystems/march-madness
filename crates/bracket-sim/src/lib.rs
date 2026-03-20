// src/lib.rs
// Organizes the library modules

mod bracket;
pub mod bracket_config;
pub mod calibration_mm;
pub mod constants;
mod game;
pub mod live_resolver;
pub mod metrics;
pub mod scoring;
pub mod team;
mod tournament;

pub use bracket::Bracket;
pub use bracket_config::BracketConfig;
pub use constants::{
    AVERAGE_PACE, AVERAGE_RATING, DEFAULT_KENPOM_UPDATE_FACTOR, DEFAULT_PACE_D, MAX_PACE, MAX_RTG,
    MIN_PACE, MIN_RTG,
};
pub use game::Game;
pub use scoring::ScoringSystem;
pub use team::Team;
pub use tournament::Tournament;

// Bracket structure constants (64-team single-elimination tournament)
pub const NUM_TEAMS: usize = 64;
pub const NUM_GAMES: usize = NUM_TEAMS - 1; // 63

/// Sentinel bit (MSB, bit 63) that must be set on every valid ByteBracket u64.
/// Distinguishes submitted brackets from uninitialized (zero) storage on-chain.
pub const SENTINEL_BIT: u64 = 1u64 << 63;

/// Set the sentinel bit on a raw 63-bit bracket value.
#[inline]
pub fn set_sentinel(bb: u64) -> u64 {
    bb | SENTINEL_BIT
}

/// Return the 63 game bits with the sentinel stripped.
#[inline]
pub fn strip_sentinel(bb: u64) -> u64 {
    bb & !SENTINEL_BIT
}

/// Panic if the sentinel bit is not set.
#[inline]
pub fn assert_sentinel(bb: u64) {
    assert!(
        bb & SENTINEL_BIT != 0,
        "ByteBracket missing sentinel bit (MSB): 0x{:016x}",
        bb
    );
}

/// Format a ByteBracket u64 (with sentinel) as a `0x`-prefixed lowercase hex string.
pub fn format_bb(bb: u64) -> String {
    assert_sentinel(bb);
    format!("0x{:016x}", bb)
}

/// Convert a game index (0-62) to its bit position in ByteBracket format.
///
/// Contract encoding (MSB-first): game 0 → bit 62, game 62 → bit 0.
/// Bit 63 is the sentinel.
#[inline]
pub fn game_bit(game_index: usize) -> u64 {
    1u64 << (62 - game_index)
}

/// Parse a hex string (0x-prefixed or bare) into a ByteBracket u64.
/// Panics if sentinel bit is not set.
pub fn parse_bb(hex: &str) -> u64 {
    let stripped = hex.strip_prefix("0x").unwrap_or(hex);
    let bb = u64::from_str_radix(stripped, 16)
        .unwrap_or_else(|e| panic!("Invalid ByteBracket hex '{}': {}", hex, e));
    assert_sentinel(bb);
    bb
}

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
/// Returns the `data/` directory at the workspace root.
/// Works because `CARGO_MANIFEST_DIR` is `crates/bracket-sim/` — two levels up.
pub fn data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root from CARGO_MANIFEST_DIR")
        .join("data")
}

/// Returns the `data/{year}/men/` directory for a given tournament year.
pub fn season_dir(year: u16) -> std::path::PathBuf {
    data_dir().join(year.to_string()).join("men")
}

/// Load teams from the default data paths for a given year:
/// `data/{year}/men/tournament.json` + `data/{year}/men/kenpom.csv`.
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
