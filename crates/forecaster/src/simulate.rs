/// Monte Carlo simulation engine.
///
/// For each simulation:
/// 1. Randomly resolve all undecided games using their team1WinProbability
/// 2. Build a complete 64-bit results bracket
/// 3. Score every bracket against those results using ByteBracket scoring
/// 4. Find the winner(s) — highest score
/// 5. Accumulate win counts and score totals
use march_madness_common::{TournamentStatus, get_scoring_mask, score_bracket_with_mask};
use rand::Rng;

pub struct SimulationResults {
    /// Number of times each bracket finished with the highest score.
    /// Ties split: if 3 brackets tie for first, each gets +1.
    pub wins: Vec<u32>,
    /// Sum of scores across all simulations (for computing expected score).
    pub expected_scores: Vec<f64>,
}

pub fn run_simulations(
    brackets: &[u64],
    _status: &TournamentStatus,
    undecided: &[(usize, f64)], // (gameIndex, team1WinProbability)
    partial_results: u64,
    num_sims: u32,
) -> SimulationResults {
    let n = brackets.len();
    let mut wins = vec![0u32; n];
    let mut expected_scores = vec![0.0f64; n];

    let mut rng = rand::rng();

    for _ in 0..num_sims {
        // Resolve undecided games randomly
        let mut results = partial_results;
        for &(game_idx, team1_prob) in undecided {
            let r: f64 = rng.random();
            if r < team1_prob {
                // team1 wins — set bit
                let bit_pos = 62 - game_idx as u32;
                results |= 1u64 << bit_pos;
            }
            // else team2 wins — bit stays 0
        }

        // Score all brackets against this simulated result
        let mask = get_scoring_mask(results);
        let mut best_score: u32 = 0;

        // First pass: compute scores and find best
        let scores: Vec<u32> = brackets
            .iter()
            .map(|&b| score_bracket_with_mask(b, results, mask))
            .collect();

        for &s in &scores {
            if s > best_score {
                best_score = s;
            }
        }

        // Second pass: accumulate wins and expected scores
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

#[cfg(test)]
mod tests {
    use super::*;
    use march_madness_common::{GameScore, GameState, GameStatus};

    fn make_status(decided: &[(u8, bool)], undecided_probs: &[(u8, f64)]) -> TournamentStatus {
        let mut games: Vec<GameStatus> = (0..63)
            .map(|i| GameStatus {
                game_index: i,
                status: GameState::Upcoming,
                score: None,
                winner: None,
                team1_win_probability: Some(0.5),
            })
            .collect();

        for &(idx, winner) in decided {
            games[idx as usize].status = GameState::Final;
            games[idx as usize].winner = Some(winner);
            games[idx as usize].score = Some(GameScore {
                team1: 70,
                team2: 60,
            });
            games[idx as usize].team1_win_probability = None;
        }

        for &(idx, prob) in undecided_probs {
            games[idx as usize].team1_win_probability = Some(prob);
        }

        TournamentStatus {
            games,
            team_reach_probabilities: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_all_decided_deterministic() {
        // All 63 games decided as team1 wins — chalky bracket should always win
        let decided: Vec<(u8, bool)> = (0..63).map(|i| (i, true)).collect();
        let status = make_status(&decided, &[]);
        let undecided: Vec<(usize, f64)> = vec![];

        let chalky = 0xFFFF_FFFF_FFFF_FFFEu64;
        let wrong = 0x8000_0000_0000_0001u64;
        let brackets = vec![chalky, wrong];

        let partial_results = chalky; // all team1 wins

        let results = run_simulations(&brackets, &status, &undecided, partial_results, 100);

        // Chalky bracket should win every simulation
        assert_eq!(results.wins[0], 100);
        assert_eq!(results.wins[1], 0);
    }

    #[test]
    fn test_undecided_games_are_random() {
        // 1 undecided game (game 0) with 50% probability
        let status = make_status(&[], &[(0, 0.5)]);
        let undecided = vec![(0usize, 0.5)];

        // Two brackets: one picks team1, other picks team2
        let team1_bracket = 0x8000_0000_0000_0000u64 | (1u64 << 62); // sentinel + game 0 = team1
        let team2_bracket = 0x8000_0000_0000_0000u64; // sentinel only, game 0 = team2

        let brackets = vec![team1_bracket, team2_bracket];
        let partial_results = 0x8000_0000_0000_0000u64;

        let results = run_simulations(&brackets, &status, &undecided, partial_results, 10000);

        // With 50/50, each should win roughly half
        // Allow wide margin for randomness
        assert!(
            results.wins[0] > 3000,
            "team1 bracket wins: {}",
            results.wins[0]
        );
        assert!(
            results.wins[1] > 3000,
            "team2 bracket wins: {}",
            results.wins[1]
        );
    }
}
