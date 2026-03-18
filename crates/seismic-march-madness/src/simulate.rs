//! Forward Monte Carlo simulation engine.
//!
//! Simulates the tournament round-by-round. For each simulation:
//! 1. For decided games: use known winner
//! 2. For live games: simulate remaining possessions via `LiveGameResolver`
//!    (falls back to coin flip with team1WinProbability if no resolver provided)
//! 3. For upcoming games: determine who's playing from earlier rounds in THIS sim,
//!    then derive P(A beats B) = reach[A][r+1] / (reach[A][r+1] + reach[B][r+1])
//! 4. Build complete 64-bit results, score all brackets, find winner(s)

use crate::scoring::{get_scoring_mask, score_bracket_with_mask};
use crate::types::{GameState, TournamentStatus};
use rand::{Rng, RngCore};

/// Round start offsets: R64=0, R32=32, S16=48, E8=56, F4=60, Champ=62
pub const ROUND_STARTS: [usize; 6] = [0, 32, 48, 56, 60, 62];
pub const ROUND_SIZES: [usize; 6] = [32, 16, 8, 4, 2, 1];

pub struct SimulationResults {
    /// Number of times each bracket finished with the highest score.
    pub wins: Vec<u32>,
    /// Sum of scores across all simulations (for computing expected score).
    pub expected_scores: Vec<f64>,
}

/// Reach probabilities indexed by bracket position (0-63), one per round (6 values).
/// reach_by_team[team_idx][round] = P(team reaches that round).
/// round 0 = R64 (always 1.0), round 5 = champion.
pub type ReachProbs = Vec<[f64; 6]>;

/// Resolves live game outcomes by simulating remaining possessions.
///
/// Implementations have access to team metrics and game state, and return
/// true if team1 wins for a given simulation trial.
pub trait LiveGameResolver {
    /// Simulate the remaining portion of a live game and return true if team1 wins.
    ///
    /// - `game_index`: 0-62 game index
    /// - `team1_idx`, `team2_idx`: bracket position indices (0-63)
    /// - `status`: full tournament status (for reading score/time/period)
    /// - `rng`: random number generator for this trial
    fn resolve(
        &self,
        game_index: usize,
        team1_idx: usize,
        team2_idx: usize,
        status: &TournamentStatus,
        rng: &mut dyn RngCore,
    ) -> bool;
}

/// Get the two feeder game indices for a game in rounds 1-5.
/// For game `g` in round `r`: feeders are in the previous round.
fn feeder_games(g: usize, round: usize) -> (usize, usize) {
    let offset_in_round = g - ROUND_STARTS[round];
    let prev_start = ROUND_STARTS[round - 1];
    (
        prev_start + 2 * offset_in_round,
        prev_start + 2 * offset_in_round + 1,
    )
}

/// For R64 game at index g (0-31), return the two team indices (0-63).
fn r64_teams(g: usize) -> (usize, usize) {
    (2 * g, 2 * g + 1)
}

/// Derive P(team_a beats team_b) in round `round` from reach probabilities.
/// Uses: P(A wins) = reach[A][round+1] / (reach[A][round+1] + reach[B][round+1])
/// Falls back to 0.5 if both are 0.
fn win_prob_from_reach(reach: &ReachProbs, team_a: usize, team_b: usize, round: usize) -> f64 {
    if round >= 5 {
        let pa = reach[team_a][5];
        let pb = reach[team_b][5];
        if pa + pb == 0.0 {
            return 0.5;
        }
        return pa / (pa + pb);
    }
    let next_round = round + 1;
    let pa = reach[team_a][next_round];
    let pb = reach[team_b][next_round];
    if pa + pb == 0.0 {
        return 0.5;
    }
    pa / (pa + pb)
}

/// Callback for processing each game result in a forward sim trial.
trait SimCallback {
    fn on_game(&mut self, game_index: usize, round: usize, team1_wins: bool, winner: usize);
    fn on_trial_end(&mut self, game_winner: &[usize; 63]);
}

/// Core forward simulation loop. Runs `num_sims` trials, calling the callback
/// for each game result and at the end of each trial.
fn run_forward_sim(
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
    resolver: Option<&dyn LiveGameResolver>,
    callback: &mut dyn SimCallback,
) {
    let mut rng = rand::rng();

    for _ in 0..num_sims {
        let mut game_winner: [usize; 63] = [usize::MAX; 63];

        for round in 0..6 {
            let start = ROUND_STARTS[round];
            let count = ROUND_SIZES[round];

            for i in 0..count {
                let g = start + i;

                let (team1, team2) = if round == 0 {
                    r64_teams(g)
                } else {
                    let (f1, f2) = feeder_games(g, round);
                    (game_winner[f1], game_winner[f2])
                };

                let game = &status.games[g];
                let team1_wins = match game.status {
                    GameState::Final => game.winner.unwrap_or(true),
                    GameState::Live => {
                        if let Some(res) = resolver {
                            res.resolve(g, team1, team2, status, &mut rng)
                        } else {
                            let p = game.team1_win_probability.unwrap_or(0.5);
                            rng.random::<f64>() < p
                        }
                    }
                    GameState::Upcoming => {
                        let p = win_prob_from_reach(reach, team1, team2, round);
                        rng.random::<f64>() < p
                    }
                };
                let winner = if team1_wins { team1 } else { team2 };
                game_winner[g] = winner;

                callback.on_game(g, round, team1_wins, winner);
            }
        }

        callback.on_trial_end(&game_winner);
    }
}

// ── Bracket scoring sim ─────────────────────────────────────────────

struct BracketScoringCallback<'a> {
    brackets: &'a [u64],
    wins: Vec<u32>,
    expected_scores: Vec<f64>,
    results: u64,
}

impl<'a> BracketScoringCallback<'a> {
    fn new(brackets: &'a [u64]) -> Self {
        let n = brackets.len();
        Self {
            brackets,
            wins: vec![0u32; n],
            expected_scores: vec![0.0f64; n],
            results: 0x8000_0000_0000_0000,
        }
    }
}

impl SimCallback for BracketScoringCallback<'_> {
    fn on_game(&mut self, game_index: usize, _round: usize, team1_wins: bool, _winner: usize) {
        if team1_wins {
            let bit_pos = 62 - game_index as u32;
            self.results |= 1u64 << bit_pos;
        }
    }

    fn on_trial_end(&mut self, _game_winner: &[usize; 63]) {
        let mask = get_scoring_mask(self.results);
        let mut best_score: u32 = 0;

        let scores: Vec<u32> = self
            .brackets
            .iter()
            .map(|&b| score_bracket_with_mask(b, self.results, mask))
            .collect();

        for &s in &scores {
            if s > best_score {
                best_score = s;
            }
        }

        for (i, &s) in scores.iter().enumerate() {
            self.expected_scores[i] += s as f64;
            if s == best_score {
                self.wins[i] += 1;
            }
        }

        // Reset for next trial
        self.results = 0x8000_0000_0000_0000;
    }
}

pub fn run_simulations(
    brackets: &[u64],
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
) -> SimulationResults {
    run_simulations_with_resolver(brackets, status, reach, num_sims, None)
}

pub fn run_simulations_with_resolver(
    brackets: &[u64],
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
    resolver: Option<&dyn LiveGameResolver>,
) -> SimulationResults {
    let mut cb = BracketScoringCallback::new(brackets);
    run_forward_sim(status, reach, num_sims, resolver, &mut cb);
    SimulationResults {
        wins: cb.wins,
        expected_scores: cb.expected_scores,
    }
}

// ── Team advance sim ────────────────────────────────────────────────

/// Per-team advance counts: `advance[team_idx][round]` = number of sims where
/// the team won their game in that round (i.e., advanced past it).
pub struct TeamAdvanceResults {
    /// 64 teams x 6 rounds. `advance[team][round]` = count of sims team won in round.
    pub advance: Vec<[u32; 6]>,
    pub num_sims: u32,
}

impl TeamAdvanceResults {
    /// Print a formatted table of team advance probabilities sorted by championship odds.
    ///
    /// `team_names`: 64 names in bracket order.
    /// `get_seed`: returns the seed for a team name (e.g., from a team map).
    pub fn print_table(&self, team_names: &[String], get_seed: impl Fn(&str) -> u8) {
        let sims = self.num_sims as f64;

        println!(
            "\n{:<25} {:>4}  {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
            "Team", "Seed", "R64", "R32", "S16", "E8", "F4", "Champ"
        );
        println!("{}", "-".repeat(82));

        let mut indices: Vec<usize> = (0..64).collect();
        indices.sort_by(|&a, &b| {
            self.advance[b][5]
                .partial_cmp(&self.advance[a][5])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for &idx in &indices {
            let name = &team_names[idx];
            let seed = get_seed(name);
            println!(
                "{:<25} {:>4}  {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}%",
                name,
                seed,
                self.advance[idx][0] as f64 / sims * 100.0,
                self.advance[idx][1] as f64 / sims * 100.0,
                self.advance[idx][2] as f64 / sims * 100.0,
                self.advance[idx][3] as f64 / sims * 100.0,
                self.advance[idx][4] as f64 / sims * 100.0,
                self.advance[idx][5] as f64 / sims * 100.0,
            );
        }
    }
}

struct TeamAdvanceCallback {
    advance: Vec<[u32; 6]>,
}

impl TeamAdvanceCallback {
    fn new() -> Self {
        Self {
            advance: vec![[0u32; 6]; 64],
        }
    }
}

impl SimCallback for TeamAdvanceCallback {
    fn on_game(&mut self, _game_index: usize, round: usize, _team1_wins: bool, winner: usize) {
        self.advance[winner][round] += 1;
    }

    fn on_trial_end(&mut self, _game_winner: &[usize; 63]) {}
}

pub fn run_team_advance_simulations(
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
) -> TeamAdvanceResults {
    run_team_advance_simulations_with_resolver(status, reach, num_sims, None)
}

pub fn run_team_advance_simulations_with_resolver(
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
    resolver: Option<&dyn LiveGameResolver>,
) -> TeamAdvanceResults {
    let mut cb = TeamAdvanceCallback::new();
    run_forward_sim(status, reach, num_sims, resolver, &mut cb);
    TeamAdvanceResults {
        advance: cb.advance,
        num_sims,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GameScore, GameState, GameStatus, TournamentStatus};

    fn make_status(decided: &[(u8, bool)], live: &[(u8, f64)]) -> TournamentStatus {
        let mut games: Vec<GameStatus> = (0..63).map(GameStatus::upcoming).collect();

        for &(idx, winner) in decided {
            games[idx as usize].status = GameState::Final;
            games[idx as usize].winner = Some(winner);
            games[idx as usize].score = Some(GameScore {
                team1: 70,
                team2: 60,
            });
        }

        for &(idx, prob) in live {
            games[idx as usize].status = GameState::Live;
            games[idx as usize].team1_win_probability = Some(prob);
            games[idx as usize].score = Some(GameScore {
                team1: 40,
                team2: 38,
            });
        }

        TournamentStatus {
            games,
            team_reach_probabilities: None,
            updated_at: None,
        }
    }

    /// Uniform reach probs — every team has equal chance at every round.
    fn uniform_reach(p: f64) -> ReachProbs {
        (0..64).map(|_| [1.0, p, p, p, p, p]).collect()
    }

    /// Reach probs where team 0 is dominant.
    fn dominant_team0_reach() -> ReachProbs {
        let mut reach: ReachProbs = (0..64)
            .map(|_| [1.0, 0.3, 0.1, 0.03, 0.01, 0.003])
            .collect();
        reach[0] = [1.0, 0.95, 0.90, 0.80, 0.60, 0.40];
        reach
    }

    #[test]
    fn test_all_decided_deterministic() {
        let decided: Vec<(u8, bool)> = (0..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[]);
        let reach = uniform_reach(0.5);

        let chalky = 0xFFFF_FFFF_FFFF_FFFEu64;
        let wrong = 0x8000_0000_0000_0001u64;
        let brackets = vec![chalky, wrong];

        let results = run_simulations(&brackets, &status, &reach, 100);
        assert_eq!(results.wins[0], 100);
        assert_eq!(results.wins[1], 0);
    }

    #[test]
    fn test_live_game_uses_probability() {
        let decided: Vec<(u8, bool)> = (1..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[(0, 0.9)]);
        let reach = uniform_reach(0.5);

        let team1_bracket = 0xFFFF_FFFF_FFFF_FFFEu64;
        let team2_bracket = 0xBFFF_FFFF_FFFF_FFFEu64;
        let brackets = vec![team1_bracket, team2_bracket];

        let results = run_simulations(&brackets, &status, &reach, 10000);
        assert!(
            results.wins[0] > 8000,
            "team1 bracket wins: {}",
            results.wins[0]
        );
    }

    #[test]
    fn test_forward_sim_uses_reach_probs() {
        let status = make_status(&[(0, true)], &[]);
        let reach = dominant_team0_reach();

        let all_team1 = 0xFFFF_FFFF_FFFF_FFFEu64;
        let all_team2 = 0x8000_0000_0000_0001u64;
        let brackets = vec![all_team1, all_team2];

        let results = run_simulations(&brackets, &status, &reach, 10000);
        let e1 = results.expected_scores[0] / 10000.0;
        let e2 = results.expected_scores[1] / 10000.0;
        assert!(
            e1 > e2,
            "all-team1 expected {:.1} should beat all-team2 {:.1}",
            e1,
            e2
        );
    }

    #[test]
    fn test_resolver_overrides_live_game() {
        let decided: Vec<(u8, bool)> = (1..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[(0, 0.1)]);
        let reach = uniform_reach(0.5);

        struct AlwaysTeam1;
        impl LiveGameResolver for AlwaysTeam1 {
            fn resolve(
                &self,
                _game_index: usize,
                _team1_idx: usize,
                _team2_idx: usize,
                _status: &TournamentStatus,
                _rng: &mut dyn RngCore,
            ) -> bool {
                true
            }
        }

        let team1_bracket = 0xFFFF_FFFF_FFFF_FFFEu64;
        let brackets = vec![team1_bracket];

        let results =
            run_simulations_with_resolver(&brackets, &status, &reach, 100, Some(&AlwaysTeam1));
        assert_eq!(results.wins[0], 100);
    }
}
