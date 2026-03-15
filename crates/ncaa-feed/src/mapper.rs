//! NCAA contest → bracket game index mapping.
//!
//! Maps NCAA scoreboard contests to game indices 0-62 in the bracket encoding.
//! Uses team names from `tournament.json` and resolves NCAA `nameShort` via alias map.

use std::collections::HashMap;

use ncaa_api::Contest;
use seismic_march_madness::tournament::{TournamentData, get_teams_in_bracket_order};
use tracing::{debug, warn};

/// Alias map: NCAA `nameShort` → tournament.json `name`.
/// Built by comparing real NCAA API output against our tournament data.
fn build_alias_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // NCAA nameShort → tournament.json name
        // Names that differ between NCAA API and our tournament data.
        ("Michigan St", "Michigan St."),
        ("Iowa St", "Iowa St."),
        ("Boise St", "Boise St."),
        ("Utah St", "Utah St."),
        ("Ohio St", "Ohio St."),
        ("Florida St", "Florida St."),
        ("San Diego St", "San Diego St."),
        ("N.C. State", "N.C. State"),
        ("St. John's (NY)", "St. John's"),
        ("UConn", "Connecticut"),
        ("Saint Mary's (CA)", "Saint Mary's"),
        ("St. Louis", "Saint Louis"),
        ("UCF", "UCF"),
        ("VCU", "VCU"),
        ("BYU", "BYU"),
        ("SMU", "SMU"),
        ("LSU", "LSU"),
        ("TCU", "TCU"),
        ("UCLA", "UCLA"),
        ("Miami (FL)", "Miami FL"),
        ("Texas A&M", "Texas A&M"),
    ])
}

/// Maps NCAA contests to bracket game indices.
pub struct GameMapper {
    /// 64 team names in bracket order.
    bracket_teams: Vec<String>,
    /// NCAA nameShort → tournament name.
    aliases: HashMap<String, String>,
    /// Game results: game_index → winner bracket position (for later-round matching).
    winners: HashMap<u8, usize>,
}

impl GameMapper {
    /// Create a new mapper from tournament data.
    pub fn new(tournament: &TournamentData) -> Self {
        let bracket_teams = get_teams_in_bracket_order(tournament);

        // Build alias map (both directions for flexibility).
        let static_aliases = build_alias_map();
        let mut aliases: HashMap<String, String> = HashMap::new();
        for (ncaa_name, tourn_name) in &static_aliases {
            aliases.insert(ncaa_name.to_string(), tourn_name.to_string());
        }
        // Also add identity mappings for all tournament names.
        for name in &bracket_teams {
            aliases.entry(name.clone()).or_insert_with(|| name.clone());
        }

        Self {
            bracket_teams,
            aliases,
            winners: HashMap::new(),
        }
    }

    /// Resolve an NCAA `nameShort` to a tournament team name.
    pub fn resolve_name(&self, ncaa_name: &str) -> Option<&str> {
        self.aliases.get(ncaa_name).map(|s| s.as_str()).or_else(|| {
            // Fallback: try exact match against bracket teams.
            self.bracket_teams
                .iter()
                .find(|t| t.as_str() == ncaa_name)
                .map(|s| s.as_str())
        })
    }

    /// Find the bracket position (0-63) of a team by name.
    pub fn team_position(&self, name: &str) -> Option<usize> {
        self.bracket_teams.iter().position(|t| t == name)
    }

    /// Record a game winner for later-round mapping.
    pub fn record_winner(&mut self, game_index: u8, winner_position: usize) {
        self.winners.insert(game_index, winner_position);
    }

    /// Get the two bracket positions that play in a given game.
    ///
    /// - R64 (games 0-31): positions `2*i` and `2*i+1`.
    /// - R32 (games 32-47): winners of games `2*(i-32)` and `2*(i-32)+1`.
    /// - Sweet 16 (games 48-55): winners of games `2*(i-48)+32` and `2*(i-48)+33`.
    /// - Elite 8 (games 56-59): winners of games `2*(i-56)+48` and `2*(i-56)+49`.
    /// - Final Four (games 60-61): winners of games `2*(i-60)+56` and `2*(i-60)+57`.
    /// - Championship (game 62): winners of games 60 and 61.
    pub fn game_team_positions(&self, game_index: u8) -> Option<(usize, usize)> {
        if game_index < 32 {
            // R64: direct bracket positions.
            let i = game_index as usize;
            Some((2 * i, 2 * i + 1))
        } else {
            // Later rounds: derive from feeder game winners.
            let (feeder1, feeder2) = feeder_games(game_index)?;
            let pos1 = self.winners.get(&feeder1)?;
            let pos2 = self.winners.get(&feeder2)?;
            Some((*pos1, *pos2))
        }
    }

    /// Try to match an NCAA contest to a game index.
    ///
    /// Returns `Some(game_index)` if both teams in the contest can be identified
    /// and matched to a specific game in the bracket.
    pub fn match_contest(&self, contest: &Contest) -> Option<u8> {
        if contest.teams.len() < 2 {
            return None;
        }

        let name0 = self.resolve_name(&contest.teams[0].name_short)?;
        let name1 = self.resolve_name(&contest.teams[1].name_short)?;

        let pos0 = self.team_position(name0)?;
        let pos1 = self.team_position(name1)?;

        // Search all 63 games for a match.
        for game_idx in 0..63u8 {
            if let Some((p1, p2)) = self.game_team_positions(game_idx)
                && ((p1 == pos0 && p2 == pos1) || (p1 == pos1 && p2 == pos0))
            {
                return Some(game_idx);
            }
        }

        debug!(
            "could not match contest: {} vs {} (positions {pos0}, {pos1})",
            name0, name1
        );
        None
    }

    /// Determine which team is "team1" (the higher-seeded / first in bracket order)
    /// for a given game index and contest.
    ///
    /// Returns the index into `contest.teams` that corresponds to team1 in bracket encoding.
    /// team1 is the team at the lower bracket position.
    pub fn team1_contest_index(&self, game_index: u8, contest: &Contest) -> Option<usize> {
        if contest.teams.len() < 2 {
            return None;
        }

        let (pos1, _pos2) = self.game_team_positions(game_index)?;
        let name0 = self.resolve_name(&contest.teams[0].name_short)?;
        let team0_pos = self.team_position(name0)?;

        if team0_pos == pos1 { Some(0) } else { Some(1) }
    }

    /// Log unmatched teams from a contest for debugging alias issues.
    pub fn warn_unmatched(&self, contest: &Contest) {
        for team in &contest.teams {
            if self.resolve_name(&team.name_short).is_none() {
                warn!(
                    "unresolved NCAA team name: '{}' (seed: {})",
                    team.name_short, team.seed
                );
            }
        }
    }
}

/// Get the two feeder game indices for a later-round game.
fn feeder_games(game_index: u8) -> Option<(u8, u8)> {
    if game_index < 32 {
        return None; // R64 has no feeder games.
    }
    // For game G in round R, the feeder games are at positions:
    // feeder1 = 2 * (G - round_start) + prev_round_start
    // feeder2 = feeder1 + 1
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

    fn test_tournament() -> TournamentData {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/2026/tournament.json"
        );
        let data = std::fs::read_to_string(path).expect("tournament.json not found");
        serde_json::from_str(&data).expect("invalid tournament.json")
    }

    #[test]
    fn test_r64_game_positions() {
        let mapper = GameMapper::new(&test_tournament());

        // Game 0: bracket positions 0 and 1 (Duke vs Washington).
        assert_eq!(mapper.game_team_positions(0), Some((0, 1)));
        assert_eq!(&mapper.bracket_teams[0], "Duke");
        assert_eq!(&mapper.bracket_teams[1], "Washington");

        // Game 31: last R64 game (positions 62, 63).
        assert_eq!(mapper.game_team_positions(31), Some((62, 63)));
    }

    #[test]
    fn test_feeder_games() {
        // R32 game 32: fed by R64 games 0 and 1.
        assert_eq!(feeder_games(32), Some((0, 1)));
        // R32 game 33: fed by R64 games 2 and 3.
        assert_eq!(feeder_games(33), Some((2, 3)));
        // S16 game 48: fed by R32 games 32 and 33.
        assert_eq!(feeder_games(48), Some((32, 33)));
        // E8 game 56: fed by S16 games 48 and 49.
        assert_eq!(feeder_games(56), Some((48, 49)));
        // F4 game 60: fed by E8 games 56 and 57.
        assert_eq!(feeder_games(60), Some((56, 57)));
        // Championship game 62: fed by F4 games 60 and 61.
        assert_eq!(feeder_games(62), Some((60, 61)));
    }

    #[test]
    fn test_alias_resolution() {
        let mapper = GameMapper::new(&test_tournament());

        // Direct match.
        assert_eq!(mapper.resolve_name("Duke"), Some("Duke"));
        // Alias match.
        assert_eq!(mapper.resolve_name("Michigan St"), Some("Michigan St."));
        assert_eq!(mapper.resolve_name("UConn"), Some("Connecticut"));
        assert_eq!(mapper.resolve_name("Miami (FL)"), Some("Miami FL"));
    }

    #[test]
    fn test_64_teams_in_bracket_order() {
        let mapper = GameMapper::new(&test_tournament());
        // Verify all 64 teams are mapped.
        for i in 0..32u8 {
            assert!(
                mapper.game_team_positions(i).is_some(),
                "game {i} should have positions"
            );
        }
    }
}
