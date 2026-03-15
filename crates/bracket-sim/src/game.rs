use rand::Rng;
use rand_distr::{Distribution, Poisson};
use statrs::distribution::{ContinuousCDF, Normal};

use crate::{AVERAGE_PACE, AVERAGE_RATING, metrics::Metrics, team::Team};

#[derive(Debug, Clone)]
pub struct GameResult {
    pub team1_score: u32,
    pub team2_score: u32,
    pub pace: f64,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub team1: Team,
    pub team2: Team,
    pub result: Option<GameResult>,
}

impl Game {
    pub fn new(team1: Team, team2: Team) -> Self {
        Game {
            team1,
            team2,
            result: None,
        }
    }

    fn possessions(&self) -> f64 {
        (self.team1.metrics.pace * self.team2.metrics.pace) / AVERAGE_PACE
    }

    pub fn team1_ortg(&self) -> f64 {
        // Goose splits evenly: half boosts offense, half improves defense
        let t1_ortg = self.team1.metrics.ortg + self.team1.goose / 2.0;
        let t2_drtg = self.team2.metrics.drtg - self.team2.goose / 2.0;
        (t1_ortg * t2_drtg) / AVERAGE_RATING
    }

    pub fn team2_ortg(&self) -> f64 {
        let t2_ortg = self.team2.metrics.ortg + self.team2.goose / 2.0;
        let t1_drtg = self.team1.metrics.drtg - self.team1.goose / 2.0;
        (t2_ortg * t1_drtg) / AVERAGE_RATING
    }

    pub fn estimate_spread(&self) -> f64 {
        let possessions = self.possessions();
        let team1_points = self.team1_ortg() * possessions / 100.0;
        let team2_points = self.team2_ortg() * possessions / 100.0;
        team1_points - team2_points
    }

    pub fn estimate_total(&self) -> f64 {
        let possessions = self.possessions();
        let team1_points = self.team1_ortg() * possessions / 100.0;
        let team2_points = self.team2_ortg() * possessions / 100.0;
        team1_points + team2_points
    }

    pub fn team1_win_probability(&self) -> f64 {
        let spread = self.estimate_spread();
        let total = self.estimate_total();
        let normal = Normal::new(spread, total.sqrt()).unwrap();
        1.0 - normal.cdf(0.0)
    }

    pub fn expected_t1_metrics(&self) -> Metrics {
        Metrics {
            ortg: self.team1_ortg(),
            drtg: self.team2_ortg(),
            pace: self.possessions(),
        }
    }

    pub fn simulate(t1_metrics: Metrics, rng: &mut impl Rng) -> GameResult {
        let pace_poisson = Poisson::new(t1_metrics.pace).unwrap();
        let actual_pace = pace_poisson.sample(rng);
        Self::simulate_with_pace(t1_metrics, actual_pace, rng)
    }

    /// Simulate with a fixed pace (no Poisson on possessions).
    /// Used for OT where the low possession count makes Poisson a poor fit.
    fn simulate_fixed_pace(t1_metrics: Metrics, rng: &mut impl Rng) -> GameResult {
        Self::simulate_with_pace(t1_metrics, t1_metrics.pace, rng)
    }

    fn simulate_with_pace(t1_metrics: Metrics, actual_pace: f64, rng: &mut impl Rng) -> GameResult {
        let team1_expected = t1_metrics.ortg * actual_pace / 100.0;
        let team2_expected = t1_metrics.drtg * actual_pace / 100.0;

        let team1_score = Poisson::new(team1_expected).unwrap().sample(rng);
        let team2_score = Poisson::new(team2_expected).unwrap().sample(rng);

        GameResult {
            team1_score: team1_score as u32,
            team2_score: team2_score as u32,
            pace: actual_pace,
        }
    }

    const MAX_OT: u32 = 10;
    const OT_MINUTES: f64 = 5.0;
    const REGULATION_MINUTES: f64 = 40.0;

    fn pick_by_score(&self, t1_score: u32, t2_score: u32) -> Option<&Team> {
        match t1_score.cmp(&t2_score) {
            std::cmp::Ordering::Greater => Some(&self.team1),
            std::cmp::Ordering::Less => Some(&self.team2),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// Simulate up to MAX_OT overtime periods (5 min each). Returns the winner,
    /// or None if still tied after all OT periods.
    fn resolve_overtime(&self, tied_score: u32, rng: &mut impl Rng) -> Option<&Team> {
        let base_metrics = self.expected_t1_metrics();
        let ot_metrics = Metrics {
            pace: base_metrics.pace * Self::OT_MINUTES / Self::REGULATION_MINUTES,
            ..base_metrics
        };

        let mut t1_total = tied_score;
        let mut t2_total = tied_score;
        for _ in 0..Self::MAX_OT {
            let ot = Game::simulate_fixed_pace(ot_metrics, rng);
            t1_total += ot.team1_score;
            t2_total += ot.team2_score;
            if let Some(w) = self.pick_by_score(t1_total, t2_total) {
                return Some(w);
            }
        }
        None
    }

    pub fn winner(&self, rng: &mut impl Rng) -> Option<&Team> {
        let result = self.result.as_ref()?;

        self.pick_by_score(result.team1_score, result.team2_score)
            .or_else(|| self.resolve_overtime(result.team1_score, rng))
            .or_else(|| {
                // Coin flip after MAX_OT
                if rng.random::<bool>() {
                    Some(&self.team1)
                } else {
                    Some(&self.team2)
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn make_team(name: &str, seed: u8, ortg: f64, drtg: f64, pace: f64) -> Team {
        Team {
            team: name.to_string(),
            seed,
            region: "East".to_string(),
            metrics: Metrics { ortg, drtg, pace },
            goose: 0.0,
        }
    }

    fn make_equal_game() -> Game {
        let t1 = make_team("Team1", 1, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 16, 105.0, 105.0, 68.0);
        Game::new(t1, t2)
    }

    #[test]
    fn winner_returns_none_without_result() {
        let game = make_equal_game();
        let mut rng = StdRng::seed_from_u64(0);
        assert!(game.winner(&mut rng).is_none());
    }

    #[test]
    fn winner_returns_team_with_higher_score() {
        let mut game = make_equal_game();
        game.result = Some(GameResult {
            team1_score: 80,
            team2_score: 70,
            pace: 68.0,
        });
        let mut rng = StdRng::seed_from_u64(0);
        assert_eq!(game.winner(&mut rng).unwrap().team, "Team1");

        game.result = Some(GameResult {
            team1_score: 70,
            team2_score: 80,
            pace: 68.0,
        });
        assert_eq!(game.winner(&mut rng).unwrap().team, "Team2");
    }

    #[test]
    fn overtime_resolves_ties_without_always_favoring_team1() {
        let t1 = make_team("Team1", 1, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 16, 105.0, 105.0, 68.0);

        let mut rng = StdRng::seed_from_u64(42);
        let mut t1_wins = 0u32;
        let mut t2_wins = 0u32;
        let trials = 1000;

        for _ in 0..trials {
            let mut game = Game::new(t1.clone(), t2.clone());
            game.result = Some(GameResult {
                team1_score: 75,
                team2_score: 75,
                pace: 68.0,
            });
            let winner = game.winner(&mut rng).unwrap();
            if winner.team == "Team1" {
                t1_wins += 1;
            } else {
                t2_wins += 1;
            }
        }

        assert!(
            t1_wins > 100 && t2_wins > 100,
            "Expected both teams to win often, got t1={} t2={}",
            t1_wins,
            t2_wins
        );
    }

    #[test]
    fn overtime_favors_stronger_team() {
        let t1 = make_team("Favorite", 1, 120.0, 95.0, 68.0);
        let t2 = make_team("Underdog", 8, 95.0, 120.0, 68.0);

        let mut rng = StdRng::seed_from_u64(99);
        let mut fav_wins = 0u32;
        let trials = 1000;

        for _ in 0..trials {
            let mut game = Game::new(t1.clone(), t2.clone());
            game.result = Some(GameResult {
                team1_score: 75,
                team2_score: 75,
                pace: 68.0,
            });
            if game.winner(&mut rng).unwrap().team == "Favorite" {
                fav_wins += 1;
            }
        }

        assert!(
            fav_wins > 600,
            "Favorite should win most OTs, got {}/1000",
            fav_wins
        );
    }
}
