//! Tournament data loading and bracket-order helpers.

use serde::Deserialize;
use std::collections::HashMap;

use crate::simulate::ReachProbs;

/// Seed order per region (matches bracket encoding).
pub const SEED_ORDER: [u32; 16] = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];

/// Tournament data from the JSON file (e.g. `data/mens-2026.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct TournamentData {
    pub name: String,
    pub regions: Vec<String>,
    pub teams: Vec<TeamData>,
}

/// A single team entry.
#[derive(Debug, Clone, Deserialize)]
pub struct TeamData {
    pub name: String,
    pub seed: u32,
    pub region: String,
    #[serde(default)]
    pub abbrev: String,
}

/// Get all 64 team names in bracket order (region by region, seed-ordered).
pub fn get_teams_in_bracket_order(data: &TournamentData) -> Vec<String> {
    let mut teams = Vec::with_capacity(64);
    for region in &data.regions {
        let region_teams: Vec<&TeamData> =
            data.teams.iter().filter(|t| t.region == *region).collect();
        for &seed in &SEED_ORDER {
            let team = region_teams
                .iter()
                .find(|t| t.seed == seed)
                .expect("missing team for seed");
            teams.push(team.name.clone());
        }
    }
    teams
}

/// Build reach probability array (64 teams x 6 rounds) from the name-keyed map.
/// Falls back to a default for teams not in the map.
pub fn build_reach_probs(
    team_names: &[String],
    reach_map: &HashMap<String, Vec<f64>>,
) -> ReachProbs {
    team_names
        .iter()
        .map(|name| {
            if let Some(probs) = reach_map.get(name) {
                let mut arr = [0.5; 6];
                for (i, &p) in probs.iter().enumerate().take(6) {
                    arr[i] = p;
                }
                arr
            } else {
                [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125]
            }
        })
        .collect()
}

/// Compute current score from decided games only.
pub fn compute_current_score(bracket: u64, status: &crate::TournamentStatus) -> u32 {
    use crate::types::GameState;
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize)
                && game.status == GameState::Final
                && let Some(winner) = game.winner
            {
                let bit_pos = 62 - game_idx as u32;
                let bracket_picked_team1 = (bracket >> bit_pos) & 1 == 1;
                if bracket_picked_team1 == winner {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}

/// Compute maximum possible score (optimistic — ignores elimination cascades).
pub fn compute_max_possible(bracket: u64, status: &crate::TournamentStatus) -> u32 {
    use crate::types::GameState;
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize) {
                if game.status == GameState::Final {
                    if let Some(winner) = game.winner {
                        let bit_pos = 62 - game_idx as u32;
                        let bracket_picked_team1 = (bracket >> bit_pos) & 1 == 1;
                        if bracket_picked_team1 == winner {
                            score += round_points[round as usize];
                        }
                    }
                } else {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}
