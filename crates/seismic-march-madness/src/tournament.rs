//! Tournament data loading and bracket-order helpers.

use serde::Deserialize;

/// Seed order per region (matches bracket encoding).
pub const SEED_ORDER: [u32; 16] = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];

/// Tournament data from the JSON file (e.g. `data/mens-2026.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct TournamentData {
    pub name: String,
    pub regions: Vec<String>,
    pub teams: Vec<TeamData>,
}

/// A single team entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamData {
    /// Team name. None for First Four slots.
    #[serde(default)]
    pub name: Option<String>,
    pub seed: u32,
    pub region: String,
    #[serde(default)]
    pub abbrev: Option<String>,
    #[serde(default)]
    pub first_four: Option<FirstFourData>,
}

/// First Four entry with individual teams and optional winner.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstFourData {
    pub teams: Vec<FirstFourTeamData>,
    #[serde(default)]
    pub winner: Option<String>,
}

/// A team within a First Four entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstFourTeamData {
    pub name: String,
    pub abbrev: String,
}

impl TeamData {
    /// Resolved display name: the team name, the FF winner, or a "A/B" combo.
    pub fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            return name.clone();
        }
        if let Some(ref ff) = self.first_four {
            if let Some(ref winner) = ff.winner {
                return winner.clone();
            }
            if ff.teams.len() == 2 {
                return format!("{}/{}", ff.teams[0].name, ff.teams[1].name);
            }
        }
        "TBD".to_string()
    }
}

/// Get all 64 team names in bracket order (region by region, seed-ordered).
pub fn get_teams_in_bracket_order(data: &TournamentData) -> Vec<String> {
    let mut teams = Vec::with_capacity(64);
    for region in &data.regions {
        let region_teams: Vec<&TeamData> =
            data.teams.iter().filter(|t| t.region == *region).collect();
        for &seed in &SEED_ORDER {
            let team = region_teams
                .iter()
                .find(|t| t.seed == seed)
                .expect("missing team for seed");
            teams.push(team.display_name());
        }
    }
    teams
}

/// Compute current score from decided games only.
///
/// Expects `bracket` in contract-correct encoding (game 0 → bit 0).
pub fn compute_current_score(bracket: u64, status: &crate::TournamentStatus) -> u32 {
    use crate::types::GameState;
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize)
                && game.status == GameState::Final
                && let Some(winner) = game.winner
            {
                let bracket_picked_team1 = (bracket >> game_idx as u32) & 1 == 1;
                if bracket_picked_team1 == winner {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}

/// Compute maximum possible score (optimistic — ignores elimination cascades).
///
/// Expects `bracket` in contract-correct encoding (game 0 → bit 0).
pub fn compute_max_possible(bracket: u64, status: &crate::TournamentStatus) -> u32 {
    use crate::types::GameState;
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize) {
                if game.status == GameState::Final {
                    if let Some(winner) = game.winner {
                        let bracket_picked_team1 = (bracket >> game_idx as u32) & 1 == 1;
                        if bracket_picked_team1 == winner {
                            score += round_points[round as usize];
                        }
                    }
                } else {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::score_bracket;
    use crate::test_util::fully_final_status;

    #[test]
    fn display_name_regular_team() {
        let team = TeamData {
            name: Some("Duke".to_string()),
            seed: 1,
            region: "East".to_string(),
            abbrev: None,
            first_four: None,
        };
        assert_eq!(team.display_name(), "Duke");
    }

    #[test]
    fn display_name_ff_pending() {
        let team = TeamData {
            name: None,
            seed: 16,
            region: "South".to_string(),
            abbrev: None,
            first_four: Some(FirstFourData {
                teams: vec![
                    FirstFourTeamData {
                        name: "Prairie View A&M".to_string(),
                        abbrev: "PV A&M".to_string(),
                    },
                    FirstFourTeamData {
                        name: "Lehigh".to_string(),
                        abbrev: "Lehigh".to_string(),
                    },
                ],
                winner: None,
            }),
        };
        assert_eq!(team.display_name(), "Prairie View A&M/Lehigh");
    }

    #[test]
    fn display_name_ff_decided() {
        let team = TeamData {
            name: None,
            seed: 16,
            region: "Midwest".to_string(),
            abbrev: None,
            first_four: Some(FirstFourData {
                teams: vec![
                    FirstFourTeamData {
                        name: "UMBC".to_string(),
                        abbrev: "UMBC".to_string(),
                    },
                    FirstFourTeamData {
                        name: "Howard".to_string(),
                        abbrev: "Howard".to_string(),
                    },
                ],
                winner: Some("Howard".to_string()),
            }),
        };
        assert_eq!(team.display_name(), "Howard");
    }

    #[test]
    fn deserialize_null_name_ff() {
        let json = r#"{"name": null, "seed": 16, "region": "S", "firstFour": {"teams": [{"name": "A", "abbrev": "A"}, {"name": "B", "abbrev": "B"}]}}"#;
        let team: TeamData = serde_json::from_str(json).unwrap();
        assert!(team.name.is_none());
        assert!(team.first_four.is_some());
        assert_eq!(team.display_name(), "A/B");
    }

    #[test]
    fn deserialize_mixed_tournament() {
        let json = r#"{
            "name": "Test", "regions": ["East"],
            "teams": [
                {"name": "Duke", "seed": 1, "region": "East"},
                {"name": null, "seed": 16, "region": "East",
                 "firstFour": {"teams": [{"name": "A", "abbrev": "A"}, {"name": "B", "abbrev": "B"}]}},
                {"name": "Kansas", "seed": 8, "region": "East"},
                {"name": null, "seed": 9, "region": "East",
                 "firstFour": {"teams": [{"name": "X", "abbrev": "X"}, {"name": "Y", "abbrev": "Y"}], "winner": "X"}}
            ]
        }"#;
        let data: TournamentData = serde_json::from_str(json).unwrap();
        assert_eq!(data.teams[0].display_name(), "Duke");
        assert_eq!(data.teams[1].display_name(), "A/B");
        assert_eq!(data.teams[2].display_name(), "Kansas");
        assert_eq!(data.teams[3].display_name(), "X");
    }

    #[test]
    fn contract_correct_score_matches_bytebracket() {
        // Jimpo contract test vector: bracket has bit 62 set (championship team1 wins),
        // results has all upsets. Contract scores this as 160.
        let results = 0x8000_0000_0000_0000u64;
        let bracket = 0xC000_0000_0000_0000u64;
        let status = fully_final_status(results);

        let contract_score = score_bracket(bracket, results);
        let current_score = compute_current_score(bracket, &status);
        assert_eq!(contract_score, 160);
        assert_eq!(
            current_score, contract_score,
            "compute_current_score must agree with ByteBracket scorer"
        );
    }

    #[test]
    fn contract_correct_perfect_bracket() {
        let bracket = 0xFFFF_FFFF_FFFF_FFFFu64;
        let results = 0xFFFF_FFFF_FFFF_FFFFu64;
        let status = fully_final_status(results);

        assert_eq!(score_bracket(bracket, results), 192);
        assert_eq!(compute_current_score(bracket, &status), 192);
        assert_eq!(compute_max_possible(bracket, &status), 192);
    }
}
