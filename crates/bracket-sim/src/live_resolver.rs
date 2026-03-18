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
///
/// Pre-caches teams by bracket position (0-63) at construction time to avoid
/// HashMap lookups and Team clones in the hot simulation loop.
pub struct GameModelResolver {
    /// Teams indexed by bracket position (0-63). None if team name not found.
    teams_by_idx: Vec<Option<Team>>,
    /// Pace dispersion ratio for the simulation.
    pace_d: f64,
}

impl GameModelResolver {
    pub fn new(team_names: &[String], team_map: &HashMap<String, Team>, pace_d: f64) -> Self {
        let teams_by_idx = team_names
            .iter()
            .map(|name| team_map.get(name).cloned())
            .collect();
        Self {
            teams_by_idx,
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
        let (t1, t2) = match (&self.teams_by_idx[team1_idx], &self.teams_by_idx[team2_idx]) {
            (Some(a), Some(b)) => (a, b),
            _ => {
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
            let result = game_model.simulate_remaining(
                (score.team1, score.team2),
                secs,
                per,
                self.pace_d,
                rng,
            );
            result.team1_score > result.team2_score
        } else {
            let prob = game_model.team1_win_probability();
            rng.random::<f64>() < prob
        }
    }
}
