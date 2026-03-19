//! [`GameResolver`] implementation using bracket-sim's game model.
//!
//! Simulates games using KenPom team metrics:
//! - Live games: simulate remaining possessions from the current score
//! - Upcoming games: simulate the full game from scratch

use std::collections::HashMap;

use rand::{Rng, RngCore};

use seismic_march_madness::simulate::GameResolver;
use seismic_march_madness::types::TournamentStatus;

use crate::game::Game;
use crate::team::Team;

/// Resolves games by simulating with KenPom metrics.
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

impl GameResolver for GameModelResolver {
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
                // Fallback: coin flip with optional probability hint
                let p = status.games[game_index]
                    .team1_win_probability
                    .unwrap_or(0.5);
                return rng.random::<f64>() < p;
            }
        };

        let game_model = Game::new(t1.clone(), t2.clone());
        let game_status = &status.games[game_index];

        // If no score (upcoming game), simulate the full game from scratch.
        let score = match &game_status.score {
            Some(s) => s,
            None => {
                let result = game_model.simulate_remaining((0, 0), 1200, 1, self.pace_d, rng);
                return result.team1_score > result.team2_score;
            }
        };

        // Live game: resolve time data, falling back to estimates when the API
        // omits clock or period (e.g. during halftime the clock string is empty).
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

        let result =
            game_model.simulate_remaining((score.team1, score.team2), secs, per, self.pace_d, rng);
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
    fn resolve_win_rate(resolver: &GameModelResolver, status: &TournamentStatus, n: u32) -> f64 {
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
        let t1 = make_team("Team1", 8, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 8, 105.0, 105.0, 68.0);
        let resolver = make_resolver(&t1, &t2);

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
            period: Some(1),
        });

        let win_rate = resolve_win_rate(&resolver, &status, 10000);

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
    fn upcoming_game_uses_team_strength() {
        let strong = make_team("Strong", 1, 120.0, 90.0, 68.0);
        let weak = make_team("Weak", 16, 90.0, 120.0, 68.0);
        let resolver = make_resolver(&strong, &weak);

        let status = make_status_with_game0(GameStatus::upcoming(0));

        let win_rate = resolve_win_rate(&resolver, &status, 10000);

        assert!(
            win_rate > 0.80,
            "strong team should win >80% of upcoming games, got {:.1}%",
            win_rate * 100.0
        );
    }

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

    #[test]
    fn estimate_remaining_from_score_halftime() {
        let (secs, period) = estimate_remaining_from_score(70.0, 140.0);
        assert_eq!(period, 2);
        assert!((secs - 1200).abs() < 10, "expected ~1200 secs, got {secs}");
    }

    #[test]
    fn estimate_remaining_from_score_early_game() {
        let (secs, period) = estimate_remaining_from_score(30.0, 140.0);
        assert_eq!(period, 1);
        assert!(secs > 1200, "expected >1200 secs remaining, got {secs}");
    }
}
