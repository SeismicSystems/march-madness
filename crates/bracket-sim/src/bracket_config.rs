use crate::team::Team;
use serde::Deserialize;

pub const DEFAULT_YEAR: u16 = 2026;

/// Standard bracket seed matchup ordering within each region (S-curve).
/// Ensures correct bracket structure: 1-seed faces 8/9 winner in R2, not 2/15 winner.
pub const BRACKET_SEED_ORDER: [(u8, u8); 8] = [
    (1, 16),
    (8, 9),
    (5, 12),
    (4, 13),
    (6, 11),
    (3, 14),
    (7, 10),
    (2, 15),
];

#[derive(Debug, Clone)]
pub struct BracketConfig {
    pub year: u16,
    /// Final Four semifinal pairings: [(region_a, region_b), (region_c, region_d)]
    pub final_four: [(String, String); 2],
}

/// Tournament JSON: regions array encodes Final Four pairings —
/// [0] vs [1] is semi 1, [2] vs [3] is semi 2.
#[derive(Debug, Deserialize)]
struct TournamentJson {
    regions: [String; 4],
}

impl BracketConfig {
    /// Load bracket config for a given year from data/{year}/tournament.json.
    /// The JSON `regions` array encodes Final Four pairings: [0] vs [1], [2] vs [3].
    pub fn for_year(year: u16) -> Self {
        let path = crate::season_dir(year).join("tournament.json");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
        let tournament: TournamentJson = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));

        let r = tournament.regions;
        BracketConfig {
            year,
            final_four: [(r[0].clone(), r[1].clone()), (r[2].clone(), r[3].clone())],
        }
    }

    /// Returns regions ordered so that Final Four pairings are adjacent.
    /// Games are laid out: [semi1_a, semi1_b, semi2_a, semi2_b] so that
    /// halving the bracket at each round produces correct matchups through
    /// the championship.
    pub fn region_order(&self) -> [&str; 4] {
        [
            &self.final_four[0].0,
            &self.final_four[0].1,
            &self.final_four[1].0,
            &self.final_four[1].1,
        ]
    }
}

/// Returns bracket groups for each round level. Each group is a set of teams
/// competing for one advancement slot in the next round.
///
/// Returns 6 rounds: R1 (32 groups of 2), R2 (16 groups of 4), Sweet16 (8 groups of 8),
/// Elite8 (4 groups of 16), Final4 (2 groups of 32), Championship (1 group of 64).
///
/// Used for validating target probability consistency.
pub fn bracket_groups(teams: &[Team], config: &BracketConfig) -> Vec<Vec<Vec<String>>> {
    let region_order = config.region_order();

    // Build team lookup by (region, seed)
    let mut by_region_seed: std::collections::HashMap<(&str, u8), String> =
        std::collections::HashMap::new();
    for team in teams {
        for &region in &region_order {
            if team.region == region {
                by_region_seed.insert((region, team.seed), team.team.clone());
            }
        }
    }

    // Build flat bracket order: teams listed in game order
    let mut bracket_order: Vec<String> = Vec::with_capacity(64);
    for &region in &region_order {
        for &(seed_a, seed_b) in &BRACKET_SEED_ORDER {
            if let Some(name) = by_region_seed.get(&(region, seed_a)) {
                bracket_order.push(name.clone());
            }
            if let Some(name) = by_region_seed.get(&(region, seed_b)) {
                bracket_order.push(name.clone());
            }
        }
    }

    // Build groups at each round level by chunking with increasing size
    let group_sizes = [2, 4, 8, 16, 32, 64];
    group_sizes
        .iter()
        .map(|&size| {
            bracket_order
                .chunks(size)
                .map(|chunk| chunk.to_vec())
                .collect()
        })
        .collect()
}
