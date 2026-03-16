//! `seismic-march-madness` — types, scoring, simulation, and tournament helpers
//! for the March Madness on Seismic bracket contest.
//!
//! This crate is the shared library used by the server, indexer, forecaster,
//! and external 3rd-party data providers.

pub mod data;
pub mod scoring;
pub mod simulate;
pub mod tournament;
pub mod types;

pub use data::{KENPOM_CSV, KenpomRatings, TOURNAMENT_JSON};
pub use scoring::*;
pub use simulate::{ReachProbs, SimulationResults, run_simulations};
pub use tournament::*;
pub use types::*;
