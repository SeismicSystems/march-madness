//! Forward Monte Carlo simulation engine.
//!
//! Simulates the tournament round-by-round. For each simulation:
//! 1. For decided games: use known winner
//! 2. For live games: flip coin using team1WinProbability (in-game conditional)
//! 3. For upcoming games: determine who's playing from earlier rounds in THIS sim,
//!    then derive P(A beats B) = reach[A][r+1] / (reach[A][r+1] + reach[B][r+1])
//! 4. Build complete 64-bit results, score all brackets, find winner(s)

use crate::scoring::{get_scoring_mask, score_bracket_with_mask};
use crate::types::{GameState, TournamentStatus};
use rand::Rng;

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
        // Championship — use the champion probability (reach[5])
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

pub fn run_simulations(
    brackets: &[u64],
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
) -> SimulationResults {
    let n = brackets.len();
    let mut wins = vec![0u32; n];
    let mut expected_scores = vec![0.0f64; n];

    let mut rng = rand::rng();

    for _ in 0..num_sims {
        // game_winner[g] = team index (0-63) that won game g
        let mut game_winner: [usize; 63] = [usize::MAX; 63];
        let mut results: u64 = 0x8000_0000_0000_0000; // sentinel

        // Forward simulate round by round
        for round in 0..6 {
            let start = ROUND_STARTS[round];
            let count = ROUND_SIZES[round];

            for i in 0..count {
                let g = start + i;
                let game = &status.games[g];

                // Determine team1 and team2 for this game
                let (team1, team2) = if round == 0 {
                    r64_teams(g)
                } else {
                    let (f1, f2) = feeder_games(g, round);
                    (game_winner[f1], game_winner[f2])
                };

                // Resolve the game
                let team1_wins = match game.status {
                    GameState::Final => {
                        // Already decided
                        game.winner.unwrap_or(true)
                    }
                    GameState::Live => {
                        // Use in-game conditional probability
                        let p = game.team1_win_probability.unwrap_or(0.5);
                        rng.random::<f64>() < p
                    }
                    GameState::Upcoming => {
                        // Derive from reach probabilities
                        let p = win_prob_from_reach(reach, team1, team2, round);
                        rng.random::<f64>() < p
                    }
                };

                let winner = if team1_wins { team1 } else { team2 };
                game_winner[g] = winner;

                // Set bit in results if team1 won
                if team1_wins {
                    let bit_pos = 62 - g as u32;
                    results |= 1u64 << bit_pos;
                }
            }
        }

        // Score all brackets against this simulated result
        let mask = get_scoring_mask(results);
        let mut best_score: u32 = 0;

        let scores: Vec<u32> = brackets
            .iter()
            .map(|&b| score_bracket_with_mask(b, results, mask))
            .collect();

        for &s in &scores {
            if s > best_score {
                best_score = s;
            }
        }

        for (i, &s) in scores.iter().enumerate() {
            expected_scores[i] += s as f64;
            if s == best_score {
                wins[i] += 1;
            }
        }
    }

    SimulationResults {
        wins,
        expected_scores,
    }
}

/// Per-team advance counts: `advance[team_idx][round]` = number of sims where
/// the team won their game in that round (i.e., advanced past it).
pub struct TeamAdvanceResults {
    /// 64 teams x 6 rounds. `advance[team][round]` = count of sims team won in round.
    pub advance: Vec<[u32; 6]>,
    pub num_sims: u32,
}

/// Run forward simulations tracking which teams advance to each round.
/// Uses the same game resolution logic as `run_simulations` (Final/Live/Upcoming).
pub fn run_team_advance_simulations(
    status: &TournamentStatus,
    reach: &ReachProbs,
    num_sims: u32,
) -> TeamAdvanceResults {
    let mut advance = vec![[0u32; 6]; 64];
    let mut rng = rand::rng();

    for _ in 0..num_sims {
        let mut game_winner: [usize; 63] = [usize::MAX; 63];

        for round in 0..6 {
            let start = ROUND_STARTS[round];
            let count = ROUND_SIZES[round];

            for i in 0..count {
                let g = start + i;
                let game = &status.games[g];

                let (team1, team2) = if round == 0 {
                    r64_teams(g)
                } else {
                    let (f1, f2) = feeder_games(g, round);
                    (game_winner[f1], game_winner[f2])
                };

                let team1_wins = match game.status {
                    GameState::Final => game.winner.unwrap_or(true),
                    GameState::Live => {
                        let p = game.team1_win_probability.unwrap_or(0.5);
                        rng.random::<f64>() < p
                    }
                    GameState::Upcoming => {
                        let p = win_prob_from_reach(reach, team1, team2, round);
                        rng.random::<f64>() < p
                    }
                };

                let winner = if team1_wins { team1 } else { team2 };
                game_winner[g] = winner;
                advance[winner][round] += 1;
            }
        }
    }

    TeamAdvanceResults { advance, num_sims }
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
        // Team 0 is much stronger
        reach[0] = [1.0, 0.95, 0.90, 0.80, 0.60, 0.40];
        reach
    }

    #[test]
    fn test_all_decided_deterministic() {
        // All 63 games decided as team1 wins
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
        // Game 0 is live with 90% chance team1 wins, rest decided as team1
        let decided: Vec<(u8, bool)> = (1..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[(0, 0.9)]);
        let reach = uniform_reach(0.5);

        // Bracket that picks team1 for game 0
        let team1_bracket = 0xFFFF_FFFF_FFFF_FFFEu64;
        // Bracket that picks team2 for game 0 (bit 62 = 0, rest = 1)
        let team2_bracket = 0xBFFF_FFFF_FFFF_FFFEu64;
        let brackets = vec![team1_bracket, team2_bracket];

        let results = run_simulations(&brackets, &status, &reach, 10000);
        // team1 bracket should win ~90% of the time
        assert!(
            results.wins[0] > 8000,
            "team1 bracket wins: {}",
            results.wins[0]
        );
    }

    #[test]
    fn test_forward_sim_uses_reach_probs() {
        // R64 game 0 is decided (team1 won), rest upcoming.
        // Team 0 is dominant — should advance far.
        // Test that expected score for a bracket picking team0 all the way
        // is higher than a bracket picking team1 all the way.
        let status = make_status(&[(0, true)], &[]);
        let reach = dominant_team0_reach();

        // Bracket: all team1 wins (picks the dominant team0 through the bracket)
        let all_team1 = 0xFFFF_FFFF_FFFF_FFFEu64;
        // Bracket: all team2 wins
        let all_team2 = 0x8000_0000_0000_0001u64;
        let brackets = vec![all_team1, all_team2];

        let results = run_simulations(&brackets, &status, &reach, 10000);
        // With dominant team 0 (team1 in every game), the all-team1 bracket
        // should have higher expected score since team0 advances more often
        let e1 = results.expected_scores[0] / 10000.0;
        let e2 = results.expected_scores[1] / 10000.0;
        assert!(
            e1 > e2,
            "all-team1 expected {:.1} should beat all-team2 {:.1}",
            e1,
            e2
        );
    }
}
