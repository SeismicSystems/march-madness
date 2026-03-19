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

pub use data::{KenpomRatings, kenpom_csv, mappings_toml, tournament_json};
pub use scoring::*;
pub use simulate::{
    LiveGameResolver, MultiPoolResults, Pool, ROUND_SIZES, ROUND_STARTS, ReachProbs,
    SimulationResults, TeamAdvanceResults, run_multi_pool_simulations_with_resolver,
    run_simulations, run_simulations_with_resolver, run_team_advance_simulations,
    run_team_advance_simulations_with_resolver,
};
pub use tournament::*;
pub use types::*;
