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

/// Estimate the remaining seconds from the current score total and expected game total.
///
/// Uses the ratio `actual_points / expected_total_points` as a proxy for game progress,
/// then converts to remaining regulation seconds (out of 2400).
fn estimate_remaining_from_score(score_total: f64, expected_total: f64) -> (i32, u8) {
    let fraction = if expected_total > 0.0 {
        (score_total / expected_total).clamp(0.05, 0.95)
    } else {
        0.5
    };
    let remaining_secs = ((1.0 - fraction) * 2400.0) as i32;
    let period = if remaining_secs > 1200 { 1 } else { 2 };
    (remaining_secs, period)
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

        // Score is required to incorporate live game state.
        let score = match &game_status.score {
            Some(s) => s,
            None => {
                let prob = game_model.team1_win_probability();
                return rng.random::<f64>() < prob;
            }
        };

        // Resolve time data, falling back to estimates when the API omits
        // clock or period (e.g. during halftime the clock string is empty).
        let (secs, per) = match (game_status.seconds_remaining, game_status.period) {
            (Some(s), Some(p)) => (s, p),
            // Clock unavailable (halftime): treat as 0 seconds left in current period.
            (None, Some(p)) => (0, p),
            // Period unavailable but clock present: estimate period from score.
            (Some(s), None) => {
                let total = (score.team1 + score.team2) as f64;
                let expected = game_model.estimate_total();
                let period = if expected > 0.0 && total / expected > 0.45 {
                    2
                } else {
                    1
                };
                (s, period)
            }
            // Neither clock nor period: estimate time remaining from score.
            (None, None) => {
                let total = (score.team1 + score.team2) as f64;
                let expected = game_model.estimate_total();
                let fraction = if expected > 0.0 {
                    (total / expected).clamp(0.05, 0.95)
                } else {
                    0.5
                };
                let remaining_secs = ((1.0 - fraction) * 2400.0) as i32;
                let period = if remaining_secs > 1200 { 1 } else { 2 };
                (remaining_secs, period)
            }
        };

        let result = game_model.simulate_remaining(
            (score.team1, score.team2),
            secs,
            per,
            self.pace_d,
            rng,
        );
        result.team1_score > result.team2_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metrics;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use seismic_march_madness::types::{GameScore, GameState, GameStatus};

    fn make_team(name: &str, seed: u8, ortg: f64, drtg: f64, pace: f64) -> Team {
        Team {
            team: name.to_string(),
            seed,
            region: "East".to_string(),
            metrics: Metrics { ortg, drtg, pace },
            goose: 0.0,
        }
    }

    /// Build a resolver with two teams at bracket positions 0 and 1.
    fn make_resolver(t1: &Team, t2: &Team) -> GameModelResolver {
        let names: Vec<String> = (0..64)
            .map(|i| {
                if i == 0 {
                    t1.team.clone()
                } else if i == 1 {
                    t2.team.clone()
                } else {
                    format!("Team{i}")
                }
            })
            .collect();
        let map: HashMap<String, Team> =
            [(t1.team.clone(), t1.clone()), (t2.team.clone(), t2.clone())]
                .into_iter()
                .collect();
        GameModelResolver::new(&names, &map, crate::DEFAULT_PACE_D)
    }

    fn make_status_with_game0(game: GameStatus) -> TournamentStatus {
        let mut games: Vec<GameStatus> = (0..63).map(GameStatus::upcoming).collect();
        games[0] = game;
        TournamentStatus {
            games,
            updated_at: None,
        }
    }

    /// Run the resolver N times and return team1 win fraction.
    fn resolve_win_rate(
        resolver: &GameModelResolver,
        status: &TournamentStatus,
        n: u32,
    ) -> f64 {
        let mut rng = StdRng::seed_from_u64(42);
        let mut wins = 0u32;
        for _ in 0..n {
            if resolver.resolve(0, 0, 1, status, &mut rng) {
                wins += 1;
            }
        }
        wins as f64 / n as f64
    }

    #[test]
    fn halftime_missing_clock_uses_score() {
        // Equal teams — isolates the score effect from team-strength asymmetry.
        let t1 = make_team("Team1", 8, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 8, 105.0, 105.0, 68.0);
        let resolver = make_resolver(&t1, &t2);

        // Halftime scenario: period=1 (from "HALF"), clock=None (empty string).
        // Team1 trails 25-40 (down 15).
        let status = make_status_with_game0(GameStatus {
            game_index: 0,
            status: GameState::Live,
            score: Some(GameScore {
                team1: 25,
                team2: 40,
            }),
            winner: None,
            team1_win_probability: None,
            seconds_remaining: None, // <-- halftime: clock is empty
            period: Some(1),
        });

        let win_rate = resolve_win_rate(&resolver, &status, 10000);

        // Equal teams, down 15 at half → should be well below 50%.
        // (Without the fix, this would be ~50% because pre-game prob ignores score.)
        assert!(
            win_rate < 0.25,
            "equal team down 15 at half should win <25%, got {:.1}%",
            win_rate * 100.0
        );
    }

    #[test]
    fn missing_clock_and_period_uses_score() {
        let t1 = make_team("Team1", 8, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 8, 105.0, 105.0, 68.0);
        let resolver = make_resolver(&t1, &t2);

        // Both clock and period are None, but we have a halftime-ish score.
        let status = make_status_with_game0(GameStatus {
            game_index: 0,
            status: GameState::Live,
            score: Some(GameScore {
                team1: 25,
                team2: 40,
            }),
            winner: None,
            team1_win_probability: None,
            seconds_remaining: None,
            period: None,
        });

        let win_rate = resolve_win_rate(&resolver, &status, 10000);

        // Equal teams, score suggests ~halftime, team1 down 15.
        assert!(
            win_rate < 0.25,
            "equal team down 15 mid-game should win <25%, got {:.1}%",
            win_rate * 100.0
        );
    }

    #[test]
    fn full_time_data_still_works() {
        let t1 = make_team("Team1", 1, 110.0, 100.0, 68.0);
        let t2 = make_team("Team2", 8, 100.0, 110.0, 68.0);
        let resolver = make_resolver(&t1, &t2);

        // Normal case: all data present. Team1 leads 60-40 with 5 min left.
        let status = make_status_with_game0(GameStatus {
            game_index: 0,
            status: GameState::Live,
            score: Some(GameScore {
                team1: 60,
                team2: 40,
            }),
            winner: None,
            team1_win_probability: None,
            seconds_remaining: Some(300),
            period: Some(2),
        });

        let win_rate = resolve_win_rate(&resolver, &status, 5000);

        assert!(
            win_rate > 0.90,
            "20-point lead with 5 min left should be >90%, got {:.1}%",
            win_rate * 100.0
        );
    }

    #[test]
    fn estimate_remaining_from_score_halftime() {
        // ~70 points at halftime, expected total ~140 → ~50% complete → ~1200 secs left
        let (secs, period) = estimate_remaining_from_score(70.0, 140.0);
        assert_eq!(period, 2); // second half
        assert!((secs - 1200).abs() < 10, "expected ~1200 secs, got {secs}");
    }

    #[test]
    fn estimate_remaining_from_score_early_game() {
        // ~30 points early in first half, expected total ~140 → ~21% complete
        let (secs, period) = estimate_remaining_from_score(30.0, 140.0);
        assert_eq!(period, 1); // still first half
        assert!(secs > 1200, "expected >1200 secs remaining, got {secs}");
    }
}
