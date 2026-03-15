use rand::Rng;
use rand_distr::{Binomial, Distribution, Gamma, Poisson};
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

    /// Sample a non-negative integer count with given `mean` and dispersion ratio
    /// `d = variance / mean`.
    ///
    /// - `d < 1`: underdispersed → Binomial(n, p) where p = 1 - d, n = mean / p
    /// - `d = 1`: Poisson(mean)
    /// - `d > 1`: overdispersed → Gamma-Poisson mixture (negative binomial)
    ///   with shape r = mean / (d - 1)
    pub fn sample_count(mean: f64, d: f64, rng: &mut impl Rng) -> f64 {
        if mean < 0.01 || !mean.is_finite() {
            return 0.0;
        }
        let d = d.max(0.01); // clamp to avoid division by zero

        if d < 1.0 {
            // Underdispersed: Binomial(n, p) with mean = np, variance = np(1-p) = mean*d
            // So 1-p = d, p = 1-d, n = mean/p = mean/(1-d)
            let p = 1.0 - d;
            let n = (mean / p).round() as u64;
            if n == 0 {
                return 0.0;
            }
            let p_actual = (mean / n as f64).clamp(0.0, 1.0);
            match Binomial::new(n, p_actual) {
                Ok(dist) => dist.sample(rng) as f64,
                Err(_) => mean.round(), // fallback: deterministic
            }
        } else if (d - 1.0).abs() < 1e-6 {
            match Poisson::new(mean) {
                Ok(dist) => dist.sample(rng),
                Err(_) => mean.round(),
            }
        } else {
            // Overdispersed: Gamma-Poisson (negative binomial)
            let r = mean / (d - 1.0);
            let scale = mean / r;
            let lambda = match Gamma::new(r, scale) {
                Ok(dist) => dist.sample(rng),
                Err(_) => return mean.round(),
            };
            if lambda < 0.01 {
                return 0.0;
            }
            match Poisson::new(lambda) {
                Ok(dist) => dist.sample(rng),
                Err(_) => lambda.round(),
            }
        }
    }

    pub fn simulate(t1_metrics: Metrics, pace_d: f64, rng: &mut impl Rng) -> GameResult {
        let actual_pace = Self::sample_count(t1_metrics.pace, pace_d, rng);
        Self::simulate_with_pace(t1_metrics, actual_pace, rng)
    }

    fn simulate_with_pace(t1_metrics: Metrics, actual_pace: f64, rng: &mut impl Rng) -> GameResult {
        let team1_expected = t1_metrics.ortg * actual_pace / 100.0;
        let team2_expected = t1_metrics.drtg * actual_pace / 100.0;

        let team1_score = if team1_expected < 0.01 {
            0
        } else {
            match Poisson::new(team1_expected) {
                Ok(dist) => dist.sample(rng) as u32,
                Err(_) => team1_expected.round() as u32,
            }
        };
        let team2_score = if team2_expected < 0.01 {
            0
        } else {
            match Poisson::new(team2_expected) {
                Ok(dist) => dist.sample(rng) as u32,
                Err(_) => team2_expected.round() as u32,
            }
        };

        GameResult {
            team1_score,
            team2_score,
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
    /// Uses the same pace distribution as regulation — the dispersion parameter
    /// naturally scales variance with the mean, so low-possession OT periods
    /// get appropriately tighter distributions without special-casing.
    fn resolve_overtime(&self, tied_score: u32, pace_d: f64, rng: &mut impl Rng) -> Option<&Team> {
        let base_metrics = self.expected_t1_metrics();
        let ot_metrics = Metrics {
            pace: base_metrics.pace * Self::OT_MINUTES / Self::REGULATION_MINUTES,
            ..base_metrics
        };

        let mut t1_total = tied_score;
        let mut t2_total = tied_score;
        for _ in 0..Self::MAX_OT {
            let ot = Game::simulate(ot_metrics, pace_d, rng);
            t1_total += ot.team1_score;
            t2_total += ot.team2_score;
            if let Some(w) = self.pick_by_score(t1_total, t2_total) {
                return Some(w);
            }
        }
        None
    }

    pub fn winner(&self, pace_d: f64, rng: &mut impl Rng) -> Option<&Team> {
        let result = self.result.as_ref()?;

        self.pick_by_score(result.team1_score, result.team2_score)
            .or_else(|| self.resolve_overtime(result.team1_score, pace_d, rng))
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
    use crate::DEFAULT_PACE_D;
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
        assert!(game.winner(DEFAULT_PACE_D, &mut rng).is_none());
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
        assert_eq!(game.winner(DEFAULT_PACE_D, &mut rng).unwrap().team, "Team1");

        game.result = Some(GameResult {
            team1_score: 70,
            team2_score: 80,
            pace: 68.0,
        });
        assert_eq!(game.winner(DEFAULT_PACE_D, &mut rng).unwrap().team, "Team2");
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
            let winner = game.winner(DEFAULT_PACE_D, &mut rng).unwrap();
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
            if game.winner(DEFAULT_PACE_D, &mut rng).unwrap().team == "Favorite" {
                fav_wins += 1;
            }
        }

        assert!(
            fav_wins > 600,
            "Favorite should win most OTs, got {}/1000",
            fav_wins
        );
    }

    #[test]
    fn sample_count_underdispersed() {
        let mut rng = StdRng::seed_from_u64(123);
        let mean = 68.0;
        let d = 0.5; // underdispersed: variance = 0.5 * mean = 34
        let n = 10_000;

        let samples: Vec<f64> = (0..n)
            .map(|_| Game::sample_count(mean, d, &mut rng))
            .collect();
        let sample_mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let sample_var: f64 = samples
            .iter()
            .map(|x| (x - sample_mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;

        // Variance should be roughly mean * d = 34 (less than Poisson's 68)
        assert!(
            sample_var < mean * 0.8,
            "Underdispersed variance ({:.1}) should be well below Poisson ({:.1})",
            sample_var,
            mean
        );
        assert!(
            (sample_mean - mean).abs() < 2.0,
            "Mean ({:.1}) should be close to target ({:.1})",
            sample_mean,
            mean
        );
    }

    #[test]
    fn sample_count_overdispersed() {
        let mut rng = StdRng::seed_from_u64(123);
        let mean = 68.0;
        let d = 1.68; // overdispersed: variance = 1.68 * mean ≈ 114
        let n = 10_000;

        let samples: Vec<f64> = (0..n)
            .map(|_| Game::sample_count(mean, d, &mut rng))
            .collect();
        let sample_mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let sample_var: f64 = samples
            .iter()
            .map(|x| (x - sample_mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;

        assert!(
            sample_var > mean * 1.2,
            "Overdispersed variance ({:.1}) should exceed Poisson ({:.1})",
            sample_var,
            mean
        );
        assert!(
            (sample_mean - mean).abs() < 2.0,
            "Mean ({:.1}) should be close to target ({:.1})",
            sample_mean,
            mean
        );
    }

    #[test]
    fn sample_count_poisson_baseline() {
        let mut rng = StdRng::seed_from_u64(123);
        let mean = 68.0;
        let d = 1.0; // Poisson
        let n = 10_000;

        let samples: Vec<f64> = (0..n)
            .map(|_| Game::sample_count(mean, d, &mut rng))
            .collect();
        let sample_mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let sample_var: f64 = samples
            .iter()
            .map(|x| (x - sample_mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;

        // Poisson: variance ≈ mean
        assert!(
            (sample_var - mean).abs() < mean * 0.1,
            "Poisson variance ({:.1}) should be close to mean ({:.1})",
            sample_var,
            mean
        );
    }

    #[test]
    fn ot_has_pace_variance() {
        let t1 = make_team("Team1", 1, 105.0, 105.0, 68.0);
        let t2 = make_team("Team2", 16, 105.0, 105.0, 68.0);
        let game = Game::new(t1, t2);
        let base_metrics = game.expected_t1_metrics();
        let ot_metrics = Metrics {
            pace: base_metrics.pace * Game::OT_MINUTES / Game::REGULATION_MINUTES,
            ..base_metrics
        };

        let mut rng = StdRng::seed_from_u64(456);
        let n = 1000;
        let paces: Vec<f64> = (0..n)
            .map(|_| Game::simulate(ot_metrics, DEFAULT_PACE_D, &mut rng).pace)
            .collect();

        let pace_mean: f64 = paces.iter().sum::<f64>() / n as f64;
        let pace_var: f64 =
            paces.iter().map(|x| (x - pace_mean).powi(2)).sum::<f64>() / (n - 1) as f64;

        // OT pace is ~8.5 possessions — should have some variance
        assert!(
            pace_var > 1.0,
            "OT pace should have variance > 1, got {:.2}",
            pace_var
        );
    }
}
