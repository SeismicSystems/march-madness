//! `seismic-march-madness` — types, scoring, simulation, and tournament helpers
//! for the March Madness on Seismic bracket contest.
//!
//! This crate is the shared library used by the server, indexer, forecaster,
//! and external 3rd-party data providers.

pub mod data;
pub mod redis_keys;
pub mod scoring;
pub mod simulate;
pub mod tournament;
pub mod types;

pub use data::{KenpomRatings, kenpom_csv, tournament_json};
pub use scoring::*;
pub use simulate::{
    ROUND_SIZES, ROUND_STARTS, ReachProbs, SimulationResults, TeamAdvanceResults, run_simulations,
    run_team_advance_simulations,
};
pub use tournament::*;
pub use types::*;
