//! NCAA contest → bracket game index mapping.
//!
//! Maps NCAA scoreboard contests to game indices 0-62 in the bracket encoding.
//! Derives name → bracket position from `tournament.json` (position = array index).

use std::collections::HashMap;
use std::path::Path;

use eyre::{Context, Result};
use ncaa_api::Contest;
use seismic_march_madness::types::{GameState, GameStatus};
use tracing::{debug, warn};

/// Tournament JSON format (just the fields we need).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TournamentTeam {
    name: String,
    #[serde(default)]
    first_four: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct TournamentJson {
    teams: Vec<TournamentTeam>,
}

/// Maps NCAA contests to bracket game indices.
pub struct GameMapper {
    /// NCAA nameShort → bracket position (0-63).
    name_to_position: HashMap<String, u8>,
    /// Game results: game_index → winner bracket position (for later-round matching).
    winners: HashMap<u8, u8>,
}

impl GameMapper {
    /// Load mapper from a tournament.json file.
    /// Position = index in the teams array. First Four entries map both
    /// individual team names to the same position.
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read {}", path.display()))?;
        Self::from_json(&json)
    }

    /// Load mapper from embedded tournament data for the given year.
    /// No filesystem access required.
    pub fn load_embedded(year: u16) -> Self {
        let json = seismic_march_madness::tournament_json(year)
            .unwrap_or_else(|| panic!("no embedded tournament data for year {year}"));
        Self::from_json(json)
            .unwrap_or_else(|e| panic!("failed to parse embedded tournament.json for {year}: {e}"))
    }

    /// Parse mapper from a JSON string.
    fn from_json(json: &str) -> Result<Self> {
        let tournament: TournamentJson =
            serde_json::from_str(json).wrap_err("failed to parse tournament JSON")?;

        let mut name_to_position = HashMap::new();
        for (i, team) in tournament.teams.iter().enumerate() {
            let pos = i as u8;
            if let Some(ref ff_names) = team.first_four {
                // First Four: map both individual names to this position.
                for ff_name in ff_names {
                    name_to_position.insert(ff_name.clone(), pos);
                }
            }
            // Also map the display name (e.g. "Texas/NC State" or normal name).
            name_to_position.insert(team.name.clone(), pos);
        }

        Ok(Self {
            name_to_position,
            winners: HashMap::new(),
        })
    }

    /// Get the bracket position (0-63) for an NCAA team name.
    pub fn team_position(&self, ncaa_name: &str) -> Option<u8> {
        self.name_to_position.get(ncaa_name).copied()
    }

    /// Record a game winner for later-round mapping.
    pub fn record_winner(&mut self, game_index: u8, winner_position: u8) {
        self.winners.insert(game_index, winner_position);
    }

    /// Get the two bracket positions that play in a given game.
    ///
    /// - R64 (games 0-31): positions `2*i` and `2*i+1`.
    /// - Later rounds: derived from feeder game winners.
    pub fn game_team_positions(&self, game_index: u8) -> Option<(u8, u8)> {
        if game_index < 32 {
            let i = game_index;
            Some((2 * i, 2 * i + 1))
        } else {
            let (feeder1, feeder2) = feeder_games(game_index)?;
            let pos1 = self.winners.get(&feeder1)?;
            let pos2 = self.winners.get(&feeder2)?;
            Some((*pos1, *pos2))
        }
    }

    /// Match an NCAA contest to a game index (0-62).
    pub fn match_contest(&self, contest: &Contest) -> Option<u8> {
        if contest.teams.len() < 2 {
            return None;
        }

        let pos0 = self.team_position(&contest.teams[0].name_short)?;
        let pos1 = self.team_position(&contest.teams[1].name_short)?;

        // R64 fast path: if teams are adjacent in bracket order, game index = min/2.
        let (lo, hi) = if pos0 < pos1 {
            (pos0, pos1)
        } else {
            (pos1, pos0)
        };
        if hi == lo + 1 && lo % 2 == 0 {
            return Some(lo / 2);
        }

        // Later rounds: scan decided games for a match.
        for game_idx in 32..63u8 {
            if let Some((p1, p2)) = self.game_team_positions(game_idx)
                && ((p1 == pos0 && p2 == pos1) || (p1 == pos1 && p2 == pos0))
            {
                return Some(game_idx);
            }
        }

        debug!(
            "could not match contest: {} vs {} (positions {pos0}, {pos1})",
            contest.teams[0].name_short, contest.teams[1].name_short
        );
        None
    }

    /// Determine which contest.teams index corresponds to team1 (lower bracket position).
    pub fn team1_contest_index(&self, game_index: u8, contest: &Contest) -> Option<usize> {
        if contest.teams.len() < 2 {
            return None;
        }

        let (pos1, _) = self.game_team_positions(game_index)?;
        let team0_pos = self.team_position(&contest.teams[0].name_short)?;

        if team0_pos == pos1 { Some(0) } else { Some(1) }
    }

    /// Record winner from a final GameStatus.
    pub fn record_winner_from_game(&mut self, game: &GameStatus) {
        if game.status == GameState::Final
            && let Some(winner) = game.winner
            && let Some((pos1, pos2)) = self.game_team_positions(game.game_index)
        {
            let winner_pos = if winner { pos1 } else { pos2 };
            self.record_winner(game.game_index, winner_pos);
            debug!(
                "recorded winner for game {}: position {winner_pos}",
                game.game_index
            );
        }
    }

    /// Log unmatched teams from a contest for debugging.
    pub fn warn_unmatched(&self, contest: &Contest) {
        for team in &contest.teams {
            if self.team_position(&team.name_short).is_none() {
                warn!(
                    "unresolved NCAA team name: '{}' (seed: {:?})",
                    team.name_short, team.seed
                );
            }
        }
    }
}

/// Get the two feeder game indices for a later-round game.
fn feeder_games(game_index: u8) -> Option<(u8, u8)> {
    if game_index < 32 {
        return None;
    }
    let (round_start, prev_round_start) = if game_index < 48 {
        (32u8, 0u8) // R32 fed by R64
    } else if game_index < 56 {
        (48, 32) // S16 fed by R32
    } else if game_index < 60 {
        (56, 48) // E8 fed by S16
    } else if game_index < 62 {
        (60, 56) // F4 fed by E8
    } else {
        (62, 60) // Championship fed by F4
    };

    let offset = game_index - round_start;
    let f1 = prev_round_start + 2 * offset;
    let f2 = f1 + 1;
    Some((f1, f2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mapper() -> GameMapper {
        GameMapper::load_embedded(2026)
    }

    #[test]
    fn test_r64_game_positions() {
        let mapper = test_mapper();

        // Game 0: positions 0 and 1 (Duke vs Siena — East 1 vs 16).
        assert_eq!(mapper.game_team_positions(0), Some((0, 1)));
        assert_eq!(mapper.team_position("Duke"), Some(0));
        assert_eq!(mapper.team_position("Siena"), Some(1));

        // Game 31: last R64 game (positions 62, 63).
        assert_eq!(mapper.game_team_positions(31), Some((62, 63)));
    }

    #[test]
    fn test_feeder_games() {
        assert_eq!(feeder_games(32), Some((0, 1)));
        assert_eq!(feeder_games(33), Some((2, 3)));
        assert_eq!(feeder_games(48), Some((32, 33)));
        assert_eq!(feeder_games(56), Some((48, 49)));
        assert_eq!(feeder_games(60), Some((56, 57)));
        assert_eq!(feeder_games(62), Some((60, 61)));
    }

    #[test]
    fn test_name_resolution() {
        let mapper = test_mapper();

        assert_eq!(mapper.team_position("Duke"), Some(0));
        assert_eq!(mapper.team_position("Michigan St."), Some(10));
        assert_eq!(mapper.team_position("UConn"), Some(14));
        assert_eq!(mapper.team_position("Iowa St."), Some(62));
        assert!(mapper.team_position("NONEXISTENT").is_none());
    }

    #[test]
    fn test_first_four_both_names_mapped() {
        let mapper = test_mapper();

        // "Texas/NC State" is a First Four slot — both individual names
        // and the combo name should map to the same position.
        let combo_pos = mapper.team_position("Texas/NC State");
        assert!(combo_pos.is_some());
        assert_eq!(mapper.team_position("Texas"), combo_pos);
        assert_eq!(mapper.team_position("NC State"), combo_pos);
    }

    #[test]
    fn test_all_64_teams_mapped() {
        let mapper = test_mapper();
        for i in 0..32u8 {
            let (p1, p2) = mapper.game_team_positions(i).unwrap();
            assert_eq!(p1, 2 * i);
            assert_eq!(p2, 2 * i + 1);
        }
    }
}
