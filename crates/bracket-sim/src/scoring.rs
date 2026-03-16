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
/// Delegates to `seismic_march_madness::scoring::score_bracket` which is a direct
/// port of the on-chain `ByteBracket.sol` scoring logic (MSB-first encoding).
///
/// Returns total points under Base scoring (1, 2, 4, 8, 16, 32 per round). Max 192.
pub fn score_base_bb(bracket: u64, results: u64) -> u32 {
    seismic_march_madness::scoring::score_bracket(bracket, results)
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
            ScoringSystem::Base => 0,
            ScoringSystem::SeedDifference => {
                if team_seed > opponent_seed {
                    (team_seed - opponent_seed) as u32
                } else {
                    0
                }
            }
            ScoringSystem::SeedTimesRound => team_seed as u32 * (round_num as u32 + 1),
            ScoringSystem::SeedPlusRound => team_seed as u32 + round_num as u32,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ScoringSystem::Base => "Base Scoring",
            ScoringSystem::SeedDifference => "Seed Difference Bonus",
            ScoringSystem::SeedTimesRound => "Seed Times Round Bonus",
            ScoringSystem::SeedPlusRound => "Seed Plus Round Bonus",
        }
    }
}
