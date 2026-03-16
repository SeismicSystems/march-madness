use crate::game::Game;
use crate::{assert_sentinel, set_sentinel, strip_sentinel};

#[derive(Debug, Clone)]
pub struct Bracket {
    pub picks: Vec<String>, // Team names that are picked to win
    pub score: u32,
}

impl Bracket {
    pub fn new(picks: Vec<String>) -> Self {
        Bracket { picks, score: 0 }
    }

    /// Encode this bracket as a ByteBracket u64 with the sentinel bit (bit 63) set.
    ///
    /// Bit encoding follows jimpo's ByteBracket Solidity contract (he's our boy):
    /// https://github.com/jimpo/march-madness-dapp/blob/master/contracts/ByteBracket.sol
    ///
    /// Bits 0-62: 63 game outcomes (1 = team1/top wins, 0 = team2/bottom wins).
    /// Bit 63: sentinel (always 1).
    /// Games are ordered round-by-round, top-to-bottom.
    pub fn to_byte_bracket_bb(&self, first_round_games: &[Game]) -> u64 {
        let mut bits: u64 = 0;
        let mut bit_idx = 0usize;

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
                    bits |= 1 << bit_idx;
                }
                next_round_winners.push(pick.as_str());
                bit_idx += 1;
                pick_idx += 1;
            }

            current_teams = next_round_winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0], c[1]))
                .collect();
        }

        set_sentinel(bits)
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
        let mut bit_idx = 0usize;

        let mut current_teams: Vec<(String, String)> = first_round_games
            .iter()
            .map(|g| (g.team1.team.clone(), g.team2.team.clone()))
            .collect();

        while !current_teams.is_empty() {
            let mut next_round_winners = Vec::new();

            for (t1, t2) in &current_teams {
                let winner = if (bits >> bit_idx) & 1 == 1 {
                    t1.clone()
                } else {
                    t2.clone()
                };
                picks.push(winner.clone());
                next_round_winners.push(winner);
                bit_idx += 1;
            }

            current_teams = next_round_winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0].clone(), c[1].clone()))
                .collect();
        }

        Bracket::new(picks)
    }

    /// Decode a hex string (with `0x` prefix and sentinel) into a Bracket.
    /// Panics if the sentinel bit is not set.
    pub fn from_byte_bracket(hex: &str, first_round_games: &[Game]) -> Self {
        let stripped = hex.strip_prefix("0x").unwrap_or(hex);
        assert!(
            stripped.len() == 16,
            "ByteBracket hex must be 16 hex digits, got '{}'",
            hex
        );
        let bb = u64::from_str_radix(stripped, 16).expect("Invalid hex in ByteBracket string");
        Self::from_byte_bracket_bb(bb, first_round_games)
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

        // A beats B (bit 0 = 1), D beats C (bit 1 = 0), A beats D (bit 2 = 1)
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
        // All top seeds win: A, C, A
        let bracket = Bracket::new(vec!["A".into(), "C".into(), "A".into()]);
        let hex = bracket.to_byte_bracket(&games);
        // bits 0,1,2 set = 0b111 = 7, plus sentinel bit 63
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
        // All bottom seeds win R1: B, D. Championship: B vs D, B is team1 -> bit 2 = 1
        let bracket = Bracket::new(vec!["B".into(), "D".into(), "B".into()]);
        let hex = bracket.to_byte_bracket(&games);
        // bits: 0,0,1 = 4, plus sentinel bit 63
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
        assert!(bb & crate::SENTINEL_BIT != 0, "sentinel must be set");
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
        // Raw bits without sentinel — should panic
        Bracket::from_byte_bracket_bb(0x0000000000000007, &games);
    }
}
