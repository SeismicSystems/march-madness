use crate::game::Game;
use crate::{SENTINEL_BIT, assert_sentinel, game_bit, strip_sentinel};

#[derive(Debug, Clone)]
pub struct Bracket {
    pub picks: Vec<String>, // Team names that are picked to win
    pub score: u32,
}

impl Bracket {
    pub fn new(picks: Vec<String>) -> Self {
        Bracket { picks, score: 0 }
    }

    /// Encode this bracket as a ByteBracket u64 with sentinel.
    ///
    /// Contract-correct encoding (matching Solidity ByteBracket.getBracketScore):
    ///   Bit 63 = sentinel (always 1)
    ///   Bit 0 = game 0 (first R64 game), ..., bit 62 = game 62 (championship)
    ///
    /// 1 = team1 (top/higher seed) wins, 0 = team2 (bottom/lower seed) wins.
    pub fn to_byte_bracket_bb(&self, first_round_games: &[Game]) -> u64 {
        let mut bits: u64 = SENTINEL_BIT;
        let mut game_idx = 0usize;

        let mut current_teams: Vec<(&str, &str)> = first_round_games
            .iter()
            .map(|g| (g.team1.team.as_str(), g.team2.team.as_str()))
            .collect();

        let mut pick_idx = 0usize;

        while !current_teams.is_empty() {
            let mut next_round_winners = Vec::new();

            for &(t1, _t2) in &current_teams {
                let pick = &self.picks[pick_idx];
                if pick == t1 {
                    bits |= game_bit(game_idx);
                }
                next_round_winners.push(pick.as_str());
                game_idx += 1;
                pick_idx += 1;
            }

            current_teams = next_round_winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0], c[1]))
                .collect();
        }

        bits
    }

    /// Encode this bracket as a `0x`-prefixed lowercase hex string with sentinel.
    pub fn to_byte_bracket(&self, first_round_games: &[Game]) -> String {
        crate::format_bb(self.to_byte_bracket_bb(first_round_games))
    }

    /// Decode a ByteBracket u64 (with sentinel) into a Bracket.
    /// Panics if the sentinel bit is not set.
    pub fn from_byte_bracket_bb(bb: u64, first_round_games: &[Game]) -> Self {
        assert_sentinel(bb);
        let bits = strip_sentinel(bb);

        let mut picks = Vec::with_capacity(63);
        let mut game_idx = 0usize;

        let mut current_teams: Vec<(String, String)> = first_round_games
            .iter()
            .map(|g| (g.team1.team.clone(), g.team2.team.clone()))
            .collect();

        while !current_teams.is_empty() {
            let mut next_round_winners = Vec::new();

            for (t1, t2) in &current_teams {
                let winner = if bits & (1u64 << game_idx) != 0 {
                    t1.clone()
                } else {
                    t2.clone()
                };
                picks.push(winner.clone());
                next_round_winners.push(winner);
                game_idx += 1;
            }

            current_teams = next_round_winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0].clone(), c[1].clone()))
                .collect();
        }

        Bracket::new(picks)
    }

    /// Decode a hex string (with optional `0x` prefix) into a Bracket.
    /// Panics if the sentinel bit is not set.
    pub fn from_byte_bracket(hex: &str, first_round_games: &[Game]) -> Self {
        Self::from_byte_bracket_bb(crate::parse_bb(hex), first_round_games)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metrics;
    use crate::team::Team;

    fn make_team(name: &str) -> Team {
        Team {
            team: name.to_string(),
            seed: 1,
            region: "Test".to_string(),
            metrics: Metrics {
                ortg: 100.0,
                drtg: 100.0,
                pace: 70.0,
            },
            goose: 0.0,
        }
    }

    /// Build a tiny 4-team bracket (2 first-round games -> 3 total games).
    #[test]
    fn byte_bracket_roundtrip_small() {
        let games = vec![
            Game::new(make_team("A"), make_team("B")),
            Game::new(make_team("C"), make_team("D")),
        ];

        // A beats B (game 0), D beats C (game 1), A beats D (game 2)
        let bracket = Bracket::new(vec!["A".into(), "D".into(), "A".into()]);
        let hex = bracket.to_byte_bracket(&games);
        let decoded = Bracket::from_byte_bracket(&hex, &games);
        assert_eq!(bracket.picks, decoded.picks);
    }

    #[test]
    fn byte_bracket_roundtrip_all_top() {
        let games = vec![
            Game::new(make_team("A"), make_team("B")),
            Game::new(make_team("C"), make_team("D")),
        ];
        // All top seeds win: A, C, A — games 0,1,2 all team1 wins
        // Contract-correct: sentinel(bit63) | game0(bit0) | game1(bit1) | game2(bit2)
        // = 0x8000000000000007
        let bracket = Bracket::new(vec!["A".into(), "C".into(), "A".into()]);
        let hex = bracket.to_byte_bracket(&games);
        assert_eq!(hex, "0x8000000000000007");
        let decoded = Bracket::from_byte_bracket(&hex, &games);
        assert_eq!(bracket.picks, decoded.picks);
    }

    #[test]
    fn byte_bracket_roundtrip_all_bottom() {
        let games = vec![
            Game::new(make_team("A"), make_team("B")),
            Game::new(make_team("C"), make_team("D")),
        ];
        // All bottom seeds win R1: B, D. Championship: B vs D, B is team1 -> game2=1
        // Contract-correct: sentinel(bit63) | game0=0 | game1=0 | game2=1(bit2)
        // = 0x8000000000000004
        let bracket = Bracket::new(vec!["B".into(), "D".into(), "B".into()]);
        let hex = bracket.to_byte_bracket(&games);
        assert_eq!(hex, "0x8000000000000004");
        let decoded = Bracket::from_byte_bracket(&hex, &games);
        assert_eq!(bracket.picks, decoded.picks);
    }

    #[test]
    fn bb_roundtrip_via_u64() {
        let games = vec![
            Game::new(make_team("A"), make_team("B")),
            Game::new(make_team("C"), make_team("D")),
        ];
        let bracket = Bracket::new(vec!["A".into(), "D".into(), "A".into()]);
        let bb = bracket.to_byte_bracket_bb(&games);
        assert!(bb & SENTINEL_BIT != 0, "sentinel must be set");
        let decoded = Bracket::from_byte_bracket_bb(bb, &games);
        assert_eq!(bracket.picks, decoded.picks);
    }

    #[test]
    #[should_panic(expected = "missing sentinel bit")]
    fn from_bb_panics_without_sentinel() {
        let games = vec![
            Game::new(make_team("A"), make_team("B")),
            Game::new(make_team("C"), make_team("D")),
        ];
        Bracket::from_byte_bracket_bb(0x7000000000000000, &games);
    }
}
