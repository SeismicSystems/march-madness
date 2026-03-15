//! Feed state management: tracks game state transitions and determines poll intervals.

use std::collections::HashMap;
use std::time::Duration;

use ncaa_api::Contest;
use seismic_march_madness::types::{GameScore, GameState, GameStatus};
use tracing::info;

use crate::mapper::GameMapper;

/// Adaptive poll intervals.
const PRE_GAME_INTERVAL: Duration = Duration::from_secs(60); // 1 min
const IDLE_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 min
const COMPLETE_SENTINEL: Duration = Duration::from_secs(0); // exit signal

/// Current feed phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedPhase {
    /// No games today or all far away.
    Idle,
    /// Games today but none started yet.
    PreGame,
    /// At least one game is live.
    Active,
    /// All 63 games are final (tournament complete).
    Complete,
}

/// Tracks the state of all 63 bracket games.
pub struct FeedState {
    /// Game statuses indexed by game_index (0-62).
    pub games: HashMap<u8, GameStatus>,
    /// Whether state changed since last write.
    pub dirty: bool,
    /// Active poll interval (1/requests_per_sec).
    active_interval: Duration,
}

impl FeedState {
    /// Create a new feed state, optionally seeded from an existing tournament-status.json.
    pub fn new(
        requests_per_sec: f64,
        existing: Option<&seismic_march_madness::TournamentStatus>,
    ) -> Self {
        let mut games = HashMap::new();

        // Seed from existing status.
        if let Some(status) = existing {
            for game in &status.games {
                games.insert(game.game_index, game.clone());
            }
        }

        // Fill in any missing games as upcoming.
        for i in 0..63u8 {
            games.entry(i).or_insert_with(|| GameStatus::upcoming(i));
        }

        let active_interval = Duration::from_secs_f64(1.0 / requests_per_sec);

        Self {
            games,
            dirty: false,
            active_interval,
        }
    }

    /// Update state from a batch of scoreboard contests.
    /// Returns the number of games that changed state.
    pub fn update_from_contests(&mut self, contests: &[Contest], mapper: &mut GameMapper) -> usize {
        let mut changes = 0;

        for contest in contests {
            let Some(game_index) = mapper.match_contest(contest) else {
                // Only warn if this looks like a tournament game (has seeds).
                if contest.teams.iter().any(|t| !t.seed.is_empty()) {
                    mapper.warn_unmatched(contest);
                }
                continue;
            };

            if self.update_game(game_index, contest, mapper) {
                changes += 1;
            }
        }

        if changes > 0 {
            self.dirty = true;
        }

        changes
    }

    /// Update a single game from a contest. Returns true if state changed.
    fn update_game(&mut self, game_index: u8, contest: &Contest, mapper: &mut GameMapper) -> bool {
        let current = self
            .games
            .get(&game_index)
            .cloned()
            .unwrap_or_else(|| GameStatus::upcoming(game_index));

        // Don't downgrade final games.
        if current.status == GameState::Final {
            return false;
        }

        let team1_idx = mapper.team1_contest_index(game_index, contest);

        let new_status = if contest.is_final() {
            GameState::Final
        } else if contest.is_live() {
            GameState::Live
        } else {
            GameState::Upcoming
        };

        // Build score with correct team ordering.
        let score = contest.scores().and_then(|scores| {
            let idx = team1_idx?;
            if idx == 0 {
                Some(GameScore {
                    team1: scores.0,
                    team2: scores.1,
                })
            } else {
                Some(GameScore {
                    team1: scores.1,
                    team2: scores.0,
                })
            }
        });

        // Determine winner for final games.
        let winner = if new_status == GameState::Final {
            score.as_ref().map(|s| s.team1 > s.team2)
        } else {
            None
        };

        // Clock and period for live games.
        let (seconds_remaining, period) = if new_status == GameState::Live {
            (contest.clock_seconds(), contest.period_number())
        } else {
            (None, None)
        };

        let new_game = GameStatus {
            game_index,
            status: new_status,
            score,
            winner,
            team1_win_probability: None, // computed externally
            seconds_remaining,
            period,
        };

        // Check if anything actually changed.
        let changed = current.status != new_game.status
            || current.score != new_game.score
            || current.seconds_remaining != new_game.seconds_remaining
            || current.period != new_game.period;

        if changed {
            if current.status != new_game.status {
                info!(
                    "game {game_index}: {:?} → {:?}{}",
                    current.status,
                    new_game.status,
                    if new_game.status == GameState::Final {
                        format!(
                            " ({})",
                            new_game
                                .score
                                .as_ref()
                                .map(|s| format!("{}-{}", s.team1, s.team2))
                                .unwrap_or_default()
                        )
                    } else {
                        String::new()
                    }
                );
            }

            // Record winner for later-round mapping.
            mapper.record_winner_from_game(&new_game);

            self.games.insert(game_index, new_game);
        }

        changed
    }

    /// Determine the current feed phase and appropriate poll interval.
    pub fn poll_interval(&self) -> (FeedPhase, Duration) {
        let mut has_live = false;
        let mut has_non_final = false;
        let mut final_count = 0u8;

        for game in self.games.values() {
            match game.status {
                GameState::Final => final_count += 1,
                GameState::Live => has_live = true,
                GameState::Upcoming => has_non_final = true,
            }
        }

        if final_count == 63 {
            return (FeedPhase::Complete, COMPLETE_SENTINEL);
        }

        if has_live {
            return (FeedPhase::Active, self.active_interval);
        }

        if has_non_final {
            return (FeedPhase::PreGame, PRE_GAME_INTERVAL);
        }

        (FeedPhase::Idle, IDLE_INTERVAL)
    }

    /// Build the full tournament status for writing.
    pub fn to_tournament_status(&self) -> seismic_march_madness::TournamentStatus {
        let mut games: Vec<GameStatus> = self.games.values().cloned().collect();
        games.sort_by_key(|g| g.game_index);

        seismic_march_madness::TournamentStatus {
            games,
            team_reach_probabilities: None,
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Mark state as clean (after writing).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_all_upcoming() {
        let state = FeedState::new(1.0, None);
        assert_eq!(state.games.len(), 63);
        for game in state.games.values() {
            assert_eq!(game.status, GameState::Upcoming);
        }
    }

    #[test]
    fn test_poll_interval_phases() {
        let mut state = FeedState::new(1.0, None);

        // All upcoming → pre-game.
        let (phase, _) = state.poll_interval();
        assert_eq!(phase, FeedPhase::PreGame);

        // Set one game to live.
        state.games.get_mut(&0).unwrap().status = GameState::Live;
        let (phase, interval) = state.poll_interval();
        assert_eq!(phase, FeedPhase::Active);
        assert_eq!(interval, Duration::from_secs(1));

        // Set all to final.
        for i in 0..63u8 {
            state.games.get_mut(&i).unwrap().status = GameState::Final;
        }
        let (phase, _) = state.poll_interval();
        assert_eq!(phase, FeedPhase::Complete);
    }

    #[test]
    fn test_seed_from_existing() {
        let existing = seismic_march_madness::TournamentStatus {
            games: vec![GameStatus {
                game_index: 0,
                status: GameState::Final,
                score: Some(GameScore {
                    team1: 82,
                    team2: 55,
                }),
                winner: Some(true),
                team1_win_probability: None,
                seconds_remaining: None,
                period: None,
            }],
            team_reach_probabilities: None,
            updated_at: None,
        };

        let state = FeedState::new(1.0, Some(&existing));
        assert_eq!(state.games[&0].status, GameState::Final);
        assert_eq!(state.games[&1].status, GameState::Upcoming);
    }
}
