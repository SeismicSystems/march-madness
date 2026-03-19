//! Feed state management: tracks game state transitions and determines poll intervals.

use std::collections::HashMap;
use std::time::Duration;

use ncaa_api::Contest;
use seismic_march_madness::types::{GameScore, GameState, GameStatus};
use tracing::info;

use crate::mapper::GameMapper;

/// Adaptive poll intervals.
const PRE_GAME_INTERVAL: Duration = Duration::from_secs(60); // 1 min

/// Current feed phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedPhase {
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
    /// Fixed override interval (ignores adaptive polling when set).
    poll_interval_override: Option<Duration>,
}

impl FeedState {
    /// Create a new feed state, optionally seeded from an existing tournament-status.json.
    pub fn new(
        requests_per_sec: f64,
        poll_interval_override: Option<Duration>,
        existing: Option<&seismic_march_madness::TournamentStatus>,
    ) -> Self {
        let mut games = HashMap::new();

        if let Some(status) = existing {
            for game in &status.games {
                games.insert(game.game_index, game.clone());
            }
        }

        for i in 0..63u8 {
            games.entry(i).or_insert_with(|| GameStatus::upcoming(i));
        }

        Self {
            games,
            dirty: false,
            active_interval: Duration::from_secs_f64(1.0 / requests_per_sec),
            poll_interval_override,
        }
    }

    /// Update state from a batch of scoreboard contests.
    /// Filters to tournament games (those with seeded teams) internally.
    /// Returns the number of games that changed state.
    pub fn update_from_contests(&mut self, contests: &[Contest], mapper: &mut GameMapper) -> usize {
        let mut changes = 0;

        for contest in contests {
            // Skip non-tournament games.
            if !contest.teams.iter().any(|t| t.seed.is_some()) {
                continue;
            }

            let Some(game_index) = mapper.match_contest(contest) else {
                mapper.warn_if_partial_match(contest);
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

        let team1_idx = mapper.team1_contest_index(game_index, contest);

        let (new_status, seconds_remaining, period) = match &contest.state {
            ncaa_api::ContestState::Final(_) => (GameState::Final, None, None),
            ncaa_api::ContestState::Live {
                period: p,
                clock_seconds: c,
            } => (GameState::Live, *c, p.map(|p| p.as_number())),
            _ => (GameState::Upcoming, None, None),
        };

        // Don't downgrade final status, but allow score corrections.
        let (new_status, seconds_remaining, period) = if current.status == GameState::Final {
            (GameState::Final, None, None)
        } else {
            (new_status, seconds_remaining, period)
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

        let winner = if new_status == GameState::Final {
            score.as_ref().map(|s| s.team1 > s.team2)
        } else {
            None
        };

        let ncaa_game_id = if new_status == GameState::Live {
            Some(contest.contest_id)
        } else {
            None
        };

        let new_game = GameStatus {
            game_index,
            status: new_status,
            score,
            winner,
            team1_win_probability: None,
            seconds_remaining,
            period,
            ncaa_game_id,
        };

        let changed = current.status != new_game.status
            || current.score != new_game.score
            || current.seconds_remaining != new_game.seconds_remaining
            || current.period != new_game.period;

        if changed {
            if current.status == GameState::Final && new_game.status == GameState::Final {
                info!(
                    "game {game_index}: score correction {:?} → {:?}",
                    current.score, new_game.score
                );
            } else if current.status != new_game.status {
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

            mapper.record_winner_from_game(&new_game);
            self.games.insert(game_index, new_game);
        }

        changed
    }

    /// Determine the current feed phase and appropriate poll interval.
    pub fn poll_interval(&self) -> (FeedPhase, Duration) {
        let mut has_live = false;
        let mut final_count = 0u8;

        for game in self.games.values() {
            match game.status {
                GameState::Final => final_count += 1,
                GameState::Live => has_live = true,
                GameState::Upcoming => {}
            }
        }

        if final_count == 63 {
            return (FeedPhase::Complete, Duration::ZERO);
        }

        if let Some(override_interval) = self.poll_interval_override {
            let phase = if has_live {
                FeedPhase::Active
            } else {
                FeedPhase::PreGame
            };
            return (phase, override_interval);
        }

        if has_live {
            return (FeedPhase::Active, self.active_interval);
        }

        (FeedPhase::PreGame, PRE_GAME_INTERVAL)
    }

    /// Build the full tournament status for writing.
    pub fn to_tournament_status(&self) -> seismic_march_madness::TournamentStatus {
        let mut games: Vec<GameStatus> = self.games.values().cloned().collect();
        games.sort_by_key(|g| g.game_index);

        seismic_march_madness::TournamentStatus {
            games,
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
        let state = FeedState::new(1.0, None, None);
        assert_eq!(state.games.len(), 63);
        for game in state.games.values() {
            assert_eq!(game.status, GameState::Upcoming);
        }
    }

    #[test]
    fn test_poll_interval_phases() {
        let mut state = FeedState::new(1.0, None, None);

        let (phase, _) = state.poll_interval();
        assert_eq!(phase, FeedPhase::PreGame);

        state.games.get_mut(&0).unwrap().status = GameState::Live;
        let (phase, interval) = state.poll_interval();
        assert_eq!(phase, FeedPhase::Active);
        assert_eq!(interval, Duration::from_secs(1));

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
                ncaa_game_id: None,
            }],
            updated_at: None,
        };

        let state = FeedState::new(1.0, None, Some(&existing));
        assert_eq!(state.games[&0].status, GameState::Final);
        assert_eq!(state.games[&1].status, GameState::Upcoming);
    }
}
