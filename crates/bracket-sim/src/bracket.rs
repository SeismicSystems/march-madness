use crate::game::Game;

#[derive(Debug, Clone)]
pub struct Bracket {
    pub picks: Vec<String>, // Team names that are picked to win
    pub score: u32,
}

impl Bracket {
    pub fn new(picks: Vec<String>) -> Self {
        Bracket { picks, score: 0 }
    }

    /// Encode this bracket as a 16-character hex string (ByteBracket format).
    ///
    /// Bit encoding follows jimpo's ByteBracket Solidity contract (he's our boy):
    /// https://github.com/jimpo/march-madness-dapp/blob/master/contracts/ByteBracket.sol
    ///
    /// Each of the 63 games is one bit: 1 = team1 (top) wins, 0 = team2 (bottom) wins.
    /// Bit 63 is unused (set to 0). Games are ordered round-by-round, top-to-bottom.
    pub fn to_byte_bracket(&self, first_round_games: &[Game]) -> String {
        let mut bits: u64 = 0;
        let mut bit_idx = 0usize;

        // Walk through rounds, rebuilding matchups from picks
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
                // bit stays 0 if team2 won
                next_round_winners.push(pick.as_str());
                bit_idx += 1;
                pick_idx += 1;
            }

            // Pair winners for next round
            current_teams = next_round_winners
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0], c[1]))
                .collect();
        }

        format!("{:016X}", bits)
    }

    /// Decode a 16-character hex string (ByteBracket format) into a Bracket.
    /// Requires the first-round games to reconstruct team names.
    pub fn from_byte_bracket(hex: &str, first_round_games: &[Game]) -> Self {
        assert!(
            hex.len() == 16,
            "ByteBracket hex must be 16 characters, got {}",
            hex.len()
        );
        let bits = u64::from_str_radix(hex, 16).expect("Invalid hex in ByteBracket string");

        let mut picks = Vec::with_capacity(63);
        let mut bit_idx = 0usize;

        // Walk through rounds, advancing winners
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
        assert_eq!(hex, "0000000000000007"); // bits 0,1,2 set = 0b111 = 7
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
        assert_eq!(hex, "0000000000000004"); // bits: 0,0,1 = 4
        let decoded = Bracket::from_byte_bracket(&hex, &games);
        assert_eq!(bracket.picks, decoded.picks);
    }
}
