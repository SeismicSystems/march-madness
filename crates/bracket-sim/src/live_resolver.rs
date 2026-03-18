//! [`LiveGameResolver`] implementation using bracket-sim's game model.
//!
//! Simulates remaining possessions for live games using KenPom team metrics
//! instead of flipping a pre-computed coin.

use std::collections::HashMap;

use rand::{Rng, RngCore};

use seismic_march_madness::simulate::LiveGameResolver;
use seismic_march_madness::types::TournamentStatus;

use crate::game::Game;
use crate::team::Team;

/// Resolves live games by simulating remaining possessions with KenPom metrics.
pub struct GameModelResolver {
    /// Team names in bracket order (0-63).
    team_names: Vec<String>,
    /// Team name → Team data (metrics + goose).
    team_map: HashMap<String, Team>,
    /// Pace dispersion ratio for the simulation.
    pace_d: f64,
}

impl GameModelResolver {
    pub fn new(team_names: Vec<String>, team_map: HashMap<String, Team>, pace_d: f64) -> Self {
        Self {
            team_names,
            team_map,
            pace_d,
        }
    }
}

impl LiveGameResolver for GameModelResolver {
    fn resolve(
        &self,
        game_index: usize,
        team1_idx: usize,
        team2_idx: usize,
        status: &TournamentStatus,
        rng: &mut dyn RngCore,
    ) -> bool {
        let t1_name = &self.team_names[team1_idx];
        let t2_name = &self.team_names[team2_idx];

        let (t1, t2) = match (self.team_map.get(t1_name), self.team_map.get(t2_name)) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                // Fallback: coin flip with status probability
                let p = status.games[game_index]
                    .team1_win_probability
                    .unwrap_or(0.5);
                return rng.random::<f64>() < p;
            }
        };

        let game_model = Game::new(t1.clone(), t2.clone());
        let game_status = &status.games[game_index];

        if let (Some(score), Some(secs), Some(per)) = (
            &game_status.score,
            game_status.seconds_remaining,
            game_status.period,
        ) {
            // Simulate remaining possessions from current score
            let result = game_model.simulate_remaining(
                (score.team1, score.team2),
                secs,
                per,
                self.pace_d,
                rng,
            );
            result.team1_score > result.team2_score
        } else {
            // No time data — use pre-game probability from team metrics
            let prob = game_model.team1_win_probability();
            rng.random::<f64>() < prob
        }
    }
}
