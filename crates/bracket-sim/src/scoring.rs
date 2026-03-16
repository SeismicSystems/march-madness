use clap::ValueEnum;

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum ScoringSystem {
    Base,
    SeedDifference,
    SeedTimesRound,
    SeedPlusRound,
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
