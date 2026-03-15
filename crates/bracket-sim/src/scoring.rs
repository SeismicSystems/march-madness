use clap::ValueEnum;

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum ScoringSystem {
    Base,
    SeedDifference,
    SeedTimesRound,
    SeedPlusRound,
}

/// Score a bracket against results using Base scoring, both in ByteBracket (u64) format.
///
/// Bit encoding follows jimpo's ByteBracket Solidity contract (he's our boy):
/// https://github.com/jimpo/march-madness-dapp/blob/master/contracts/ByteBracket.sol
///
/// Each bit represents a game outcome: 1 = team1 (top) wins, 0 = team2 (bottom) wins.
/// Bits 0-31 = Round 1 (32 games), 32-47 = Round 2, ..., 62 = Championship.
///
/// A pick is correct only if the bit matches AND (for rounds 1+) the feeder game
/// that produced the picked team was also correctly picked. This ensures we don't
/// award points when the same bit value corresponds to different teams due to
/// earlier-round disagreements.
///
/// Returns total points under Base scoring (1, 2, 4, 8, 16, 32 per round).
#[cfg(test)]
pub(crate) fn score_base_bb(bracket: u64, results: u64) -> u32 {
    let matching = !(bracket ^ results); // bit i set iff bracket[i] == results[i]

    // Round 0 (bits 0-31): no feeder check needed
    let mut correct: u64 = matching & 0xFFFF_FFFF;
    let mut score = (correct as u32).count_ones(); // x 1

    let round_sizes: [u32; 6] = [32, 16, 8, 4, 2, 1];
    let mut prev_offset: u32 = 0;

    for round in 1..6u32 {
        let offset = prev_offset + round_sizes[round as usize - 1];
        let n_games = round_sizes[round as usize];
        let points = 1u32 << round;

        for g in 0..n_games {
            let bit_pos = offset + g;
            let match_bit = (matching >> bit_pos) & 1;

            // The result bit tells us which feeder produced the winner:
            // 1 -> left feeder (game 2g at prev round), 0 -> right feeder (game 2g+1)
            let result_bit = (results >> bit_pos) & 1;
            let feeder_pos = prev_offset + 2 * g + (1 - result_bit as u32);
            let feeder_ok = (correct >> feeder_pos) & 1;

            if match_bit & feeder_ok == 1 {
                correct |= 1u64 << bit_pos;
                score += points;
            }
        }

        prev_offset = offset;
    }

    score
}

impl ScoringSystem {
    // Calculate points for a correctly picked game
    pub fn calculate_points(
        &self,
        round_num: usize, // 0-indexed round number (0=First Round, 1=Second Round, etc)
        team_seed: u8,    // Seed of the team that was picked correctly (1-16)
        opponent_seed: u8, // Optional opponent seed if known
    ) -> u32 {
        // Base points (1, 2, 4, 8, 16, 32)
        let base_points = 1 << round_num;
        base_points + self.bonus(round_num, team_seed, opponent_seed)
    }

    fn bonus(&self, round_num: usize, team_seed: u8, opponent_seed: u8) -> u32 {
        match self {
            ScoringSystem::Base => {
                // Just return base points
                0
            }
            ScoringSystem::SeedDifference => {
                // Add seed difference bonus for upset picks
                if team_seed > opponent_seed {
                    // Upset bonus (higher seed beats lower seed)
                    (team_seed - opponent_seed) as u32
                } else {
                    // No upset bonus
                    0
                }
            }
            ScoringSystem::SeedTimesRound => {
                // Add bonus based on seed times round number
                team_seed as u32 * (round_num as u32 + 1)
            }
            ScoringSystem::SeedPlusRound => {
                // Add bonus based on seed plus round number
                team_seed as u32 + round_num as u32
            }
        }
    }

    // Helper function to get a descriptive name
    pub fn name(&self) -> &'static str {
        match self {
            ScoringSystem::Base => "Base Scoring",
            ScoringSystem::SeedDifference => "Seed Difference Bonus",
            ScoringSystem::SeedTimesRound => "Seed Times Round Bonus",
            ScoringSystem::SeedPlusRound => "Seed Plus Round Bonus",
        }
    }
}
