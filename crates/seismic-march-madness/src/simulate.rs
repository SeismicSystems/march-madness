//! Forward Monte Carlo simulation engine.
//!
//! Simulates the tournament round-by-round. For each simulation:
//! 1. For decided games: use known winner
//! 2. For live/upcoming games: delegate to a [`GameResolver`] which simulates
//!    using KenPom team metrics (remaining possessions for live, full game for upcoming)
//! 3. Build complete 64-bit results, score all brackets, find winner(s)

use crate::scoring::{get_scoring_mask, score_bracket_with_mask};
use crate::types::{GameState, TournamentStatus};
use rand::RngCore;
use rayon::prelude::*;

/// Round start offsets: R64=0, R32=32, S16=48, E8=56, F4=60, Champ=62
pub const ROUND_STARTS: [usize; 6] = [0, 32, 48, 56, 60, 62];
pub const ROUND_SIZES: [usize; 6] = [32, 16, 8, 4, 2, 1];

pub struct SimulationResults {
    /// Number of times each bracket finished with the highest score.
    pub wins: Vec<u32>,
    /// Sum of scores across all simulations (for computing expected score).
    pub expected_scores: Vec<f64>,
}

/// Resolves game outcomes by simulating with KenPom team metrics.
///
/// For live games: simulates remaining possessions from the current score.
/// For upcoming games: simulates the full game from scratch.
pub trait GameResolver: Send + Sync {
    /// Simulate a game and return true if team1 wins.
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

/// Callback for processing each game result in a forward sim trial.
trait SimCallback {
    fn on_game(&mut self, game_index: usize, round: usize, team1_wins: bool, winner: usize);
    fn on_trial_end(&mut self, game_winner: &[usize; 63]);
}

/// Core forward simulation loop. Runs `num_sims` trials, calling the callback
/// for each game result and at the end of each trial.
fn run_forward_sim(
    status: &TournamentStatus,
    num_sims: u32,
    resolver: &dyn GameResolver,
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
                    GameState::Live | GameState::Upcoming => {
                        resolver.resolve(g, team1, team2, status, &mut rng)
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

struct BracketScoringCallback {
    brackets: Vec<u64>,
    wins: Vec<u32>,
    expected_scores: Vec<f64>,
    results: u64,
}

impl BracketScoringCallback {
    fn new(brackets: &[u64]) -> Self {
        let n = brackets.len();
        Self {
            brackets: brackets.to_vec(),
            wins: vec![0u32; n],
            expected_scores: vec![0.0f64; n],
            results: 0x8000_0000_0000_0000,
        }
    }
}

impl SimCallback for BracketScoringCallback {
    fn on_game(&mut self, game_index: usize, _round: usize, team1_wins: bool, _winner: usize) {
        if team1_wins {
            self.results |= 1u64 << game_index as u32;
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
    num_sims: u32,
    resolver: &dyn GameResolver,
) -> SimulationResults {
    let mut cb = BracketScoringCallback::new(brackets);
    run_forward_sim(status, num_sims, resolver, &mut cb);
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
    num_sims: u32,
    resolver: &dyn GameResolver,
) -> TeamAdvanceResults {
    let mut cb = TeamAdvanceCallback::new();
    run_forward_sim(status, num_sims, resolver, &mut cb);
    TeamAdvanceResults {
        advance: cb.advance,
        num_sims,
    }
}

// ── Multi-pool sim ─────────────────────────────────────────────────

/// A pool of brackets competing against each other.
pub struct Pool {
    /// Redis HASH field key: "mm", "group:3", "mirror:1".
    pub key: String,
    /// (member_key, bracket_index) pairs. member_key is an address or slug.
    /// bracket_index is an index into the global brackets array.
    pub members: Vec<(String, usize)>,
}

/// Results of a multi-pool simulation.
pub struct MultiPoolResults {
    /// pool_wins[pool_idx][member_idx] = sum of per-sim win shares.
    /// Tied top scores split `1 / count_at_max` across co-leaders.
    pub pool_wins: Vec<Vec<f64>>,
    /// score_sums[pool_idx][member_idx] = sum of scores across all sims (for computing expected score).
    pub score_sums: Vec<Vec<u64>>,
    pub num_sims: u32,
}

struct MultiPoolScoringCallback<'a> {
    brackets: Vec<u64>,
    pools: &'a [Pool],
    pool_wins: Vec<Vec<f64>>,
    score_sums: Vec<Vec<u64>>,
    results: u64,
}

impl<'a> MultiPoolScoringCallback<'a> {
    fn new(brackets: &[u64], pools: &'a [Pool]) -> Self {
        let pool_wins = pools
            .iter()
            .map(|pool| vec![0.0f64; pool.members.len()])
            .collect();
        let score_sums = pools
            .iter()
            .map(|pool| vec![0u64; pool.members.len()])
            .collect();
        Self {
            brackets: brackets.to_vec(),
            pools,
            pool_wins,
            score_sums,
            results: 0x8000_0000_0000_0000,
        }
    }
}

fn accumulate_pool_results(
    pool: &Pool,
    scores: &[u32],
    pool_wins: &mut [Vec<f64>],
    score_sums: &mut [Vec<u64>],
    pool_idx: usize,
) {
    let mut best = 0u32;
    let mut count_at_max = 0usize;
    for &(_, bracket_idx) in &pool.members {
        let s = scores[bracket_idx];
        if s > best {
            best = s;
            count_at_max = 1;
        } else if s == best {
            count_at_max += 1;
        }
    }

    let win_share = 1.0 / count_at_max as f64;
    for (member_idx, &(_, bracket_idx)) in pool.members.iter().enumerate() {
        let s = scores[bracket_idx];
        score_sums[pool_idx][member_idx] += s as u64;
        if s == best {
            pool_wins[pool_idx][member_idx] += win_share;
        }
    }
}

impl SimCallback for MultiPoolScoringCallback<'_> {
    fn on_game(&mut self, game_index: usize, _round: usize, team1_wins: bool, _winner: usize) {
        if team1_wins {
            self.results |= 1u64 << game_index as u32;
        }
    }

    fn on_trial_end(&mut self, _game_winner: &[usize; 63]) {
        let mask = get_scoring_mask(self.results);

        // Score all unique brackets once.
        let scores: Vec<u32> = self
            .brackets
            .iter()
            .map(|&b| score_bracket_with_mask(b, self.results, mask))
            .collect();

        // For each pool, find max score, increment winners, and accumulate score sums.
        for (pool_idx, pool) in self.pools.iter().enumerate() {
            accumulate_pool_results(
                pool,
                &scores,
                &mut self.pool_wins,
                &mut self.score_sums,
                pool_idx,
            );
        }

        self.results = 0x8000_0000_0000_0000;
    }
}

/// Run multi-pool simulations. All pools share the same forward sim trials,
/// so each pool sees the same simulated tournament outcomes. Uses rayon to
/// parallelize across chunks of simulations.
pub fn run_multi_pool_simulations(
    brackets: &[u64],
    pools: &[Pool],
    status: &TournamentStatus,
    num_sims: u32,
    resolver: &dyn GameResolver,
) -> MultiPoolResults {
    // Determine chunk size for parallelism. Each thread runs a chunk of sims
    // with its own callback, then we merge.
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = (num_sims as usize).div_ceil(num_threads);

    let chunks: Vec<u32> = (0..num_threads)
        .map(|i| {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(num_sims as usize);
            if start >= num_sims as usize {
                0
            } else {
                (end - start) as u32
            }
        })
        .filter(|&n| n > 0)
        .collect();

    // TODO: extract a type alias for (Vec<Vec<f64>>, Vec<Vec<u64>>) to reduce complexity
    #[allow(clippy::type_complexity)]
    let partial_results: Vec<(Vec<Vec<f64>>, Vec<Vec<u64>>)> = chunks
        .par_iter()
        .map(|&chunk_sims| {
            let mut cb = MultiPoolScoringCallback::new(brackets, pools);
            run_forward_sim(status, chunk_sims, resolver, &mut cb);
            (cb.pool_wins, cb.score_sums)
        })
        .collect();

    // Merge partial results.
    let mut pool_wins: Vec<Vec<f64>> = pools
        .iter()
        .map(|pool| vec![0.0f64; pool.members.len()])
        .collect();
    let mut score_sums: Vec<Vec<u64>> = pools
        .iter()
        .map(|pool| vec![0u64; pool.members.len()])
        .collect();

    for (partial_wins, partial_scores) in &partial_results {
        for (pi, pool_partial) in partial_wins.iter().enumerate() {
            for (mi, &count) in pool_partial.iter().enumerate() {
                pool_wins[pi][mi] += count;
            }
        }
        for (pi, pool_partial) in partial_scores.iter().enumerate() {
            for (mi, &sum) in pool_partial.iter().enumerate() {
                score_sums[pi][mi] += sum;
            }
        }
    }

    MultiPoolResults {
        pool_wins,
        score_sums,
        num_sims,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::{reverse_game_bits, score_bracket};
    use crate::types::{GameScore, GameState, GameStatus, TournamentStatus};
    use rand::Rng;

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
            updated_at: None,
        }
    }

    /// Build a fully-decided TournamentStatus from contract-correct results bits.
    fn fully_final_status(results: u64) -> TournamentStatus {
        let decided: Vec<(u8, bool)> = (0..63)
            .map(|game_index| {
                let winner = ((results >> game_index) & 1) == 1;
                (game_index as u8, winner)
            })
            .collect();
        make_status(&decided, &[])
    }

    /// A resolver that uses the team1_win_probability from game status as a coin flip.
    /// Used in tests that don't need full KenPom simulation.
    struct ProbabilityResolver;
    impl GameResolver for ProbabilityResolver {
        fn resolve(
            &self,
            game_index: usize,
            _team1_idx: usize,
            _team2_idx: usize,
            status: &TournamentStatus,
            rng: &mut dyn RngCore,
        ) -> bool {
            let p = status.games[game_index]
                .team1_win_probability
                .unwrap_or(0.5);
            rng.random::<f64>() < p
        }
    }

    #[test]
    fn test_all_decided_deterministic() {
        // All 63 games decided as team1 wins
        let decided: Vec<(u8, bool)> = (0..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[]);

        // Contract-correct: all team1 wins = all 63 game bits + sentinel
        let chalky = 0xFFFF_FFFF_FFFF_FFFFu64;
        // All team2 wins = only sentinel
        let wrong = 0x8000_0000_0000_0000u64;
        let brackets = vec![chalky, wrong];

        let results = run_simulations(&brackets, &status, 100, &ProbabilityResolver);
        assert_eq!(results.wins[0], 100);
        assert_eq!(results.wins[1], 0);
    }

    #[test]
    fn test_all_decided_matches_bytebracket_score() {
        // Use golden vector values, converting from legacy to contract-correct
        let legacy_results = 0xBFFF_FFFF_BFFF_BFBAu64; // cinderella_run
        let legacy_bracket = 0xD555_5555_5555_5555u64; // alternating_picks
        let results_bits = reverse_game_bits(legacy_results);
        let bracket_bits = reverse_game_bits(legacy_bracket);
        let status = fully_final_status(results_bits);

        let sim_results = run_simulations(&[bracket_bits], &status, 1, &ProbabilityResolver);

        assert_eq!(
            sim_results.expected_scores[0],
            score_bracket(bracket_bits, results_bits) as f64
        );
        assert_eq!(sim_results.wins[0], 1);
    }

    #[test]
    fn test_live_game_uses_resolver() {
        // Games 1-62 decided as team1 wins; game 0 is live with p=0.9
        let decided: Vec<(u8, bool)> = (1..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[(0, 0.9)]);

        // Contract-correct: team1_bracket picks team1 for all 63 games
        let team1_bracket = 0xFFFF_FFFF_FFFF_FFFFu64;
        // team2_bracket picks team2 for game 0, team1 for all others (bit 0 = 0)
        let team2_bracket = 0xFFFF_FFFF_FFFF_FFFEu64;
        let brackets = vec![team1_bracket, team2_bracket];

        let results = run_simulations(&brackets, &status, 10000, &ProbabilityResolver);
        assert!(
            results.wins[0] > 8000,
            "team1 bracket wins: {}",
            results.wins[0]
        );
    }

    #[test]
    fn test_resolver_overrides_live_game() {
        // Games 1-62 decided as team1 wins; game 0 is live with low p
        let decided: Vec<(u8, bool)> = (1..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[(0, 0.1)]);

        struct AlwaysTeam1;
        impl GameResolver for AlwaysTeam1 {
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

        // Picks team1 for all games — resolver forces team1 for game 0
        let team1_bracket = 0xFFFF_FFFF_FFFF_FFFFu64;
        let brackets = vec![team1_bracket];

        let results = run_simulations(&brackets, &status, 100, &AlwaysTeam1);
        assert_eq!(results.wins[0], 100);
    }

    #[test]
    fn test_multi_pool_ties_split_fractionally() {
        let pool = Pool {
            key: "mm".to_string(),
            members: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
                ("c".to_string(), 2),
            ],
        };
        let scores = vec![10, 10, 5];
        let mut pool_wins = vec![vec![0.0; 3]];
        let mut score_sums = vec![vec![0u64; 3]];

        accumulate_pool_results(&pool, &scores, &mut pool_wins, &mut score_sums, 0);

        assert_eq!(pool_wins[0], vec![0.5, 0.5, 0.0]);
        assert_eq!(score_sums[0], vec![10, 10, 5]);
    }
}
