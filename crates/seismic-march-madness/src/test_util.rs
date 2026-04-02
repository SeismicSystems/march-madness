//! Shared test utilities for building tournament status fixtures.

use crate::types::{GameScore, GameState, GameStatus, TournamentStatus};

/// Build a TournamentStatus from decided and live game specs.
///
/// - `decided`: `(game_index, team1_wins)` pairs for final games
/// - `live`: `(game_index, team1_win_probability)` pairs for live games
/// - All other games are set to Upcoming.
pub fn make_status(decided: &[(u8, bool)], live: &[(u8, f64)]) -> TournamentStatus {
    let mut games: Vec<GameStatus> = (0..63).map(GameStatus::upcoming).collect();

    for &(idx, winner) in decided {
        games[idx as usize].status = GameState::Final;
        games[idx as usize].winner = Some(winner);
        games[idx as usize].score = Some(GameScore {
            team1: 70,
            team2: 60,
        });
    }

    for &(idx, prob) in live {
        games[idx as usize].status = GameState::Live;
        games[idx as usize].team1_win_probability = Some(prob);
        games[idx as usize].score = Some(GameScore {
            team1: 40,
            team2: 38,
        });
    }

    TournamentStatus {
        games,
        updated_at: None,
    }
}

/// Build a fully-decided TournamentStatus from contract-correct results bits.
///
/// Each of the 63 game bits is read as: `(results >> game_index) & 1 == 1` → team1 wins.
pub fn fully_final_status(results: u64) -> TournamentStatus {
    let decided: Vec<(u8, bool)> = (0..63)
        .map(|game_index| {
            let winner = ((results >> game_index) & 1) == 1;
            (game_index as u8, winner)
        })
        .collect();
    make_status(&decided, &[])
}
