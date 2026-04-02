use rand::Rng;
use rayon::prelude::*;

use crate::bracket_config::{BRACKET_SEED_ORDER, BracketConfig};
use crate::game::{Game, GameResult};
use crate::metrics::Metrics;
use crate::team::Team;
use crate::{Bracket, DEFAULT_KENPOM_UPDATE_FACTOR, DEFAULT_PACE_D, ScoringSystem};
use seismic_march_madness::types::{GameState, TournamentStatus};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
pub struct Tournament {
    teams: Vec<Team>,
    games: Vec<Game>,
    seeds: HashMap<String, u8>,
    /// Pace dispersion ratio (variance / mean). See [`DEFAULT_PACE_D`].
    pace_d: f64,
    /// KenPom-style Bayesian postgame metric adjustment factor.
    kenpom_update_factor: f64,
}

impl Default for Tournament {
    fn default() -> Self {
        Self::new()
    }
}

impl Tournament {
    pub fn new() -> Self {
        Tournament {
            teams: Vec::new(),
            games: Vec::new(),
            seeds: HashMap::new(),
            pace_d: DEFAULT_PACE_D,
            kenpom_update_factor: DEFAULT_KENPOM_UPDATE_FACTOR,
        }
    }

    /// Set the pace dispersion ratio. See [`DEFAULT_PACE_D`] for details.
    pub fn with_pace_d(mut self, pace_d: f64) -> Self {
        self.pace_d = pace_d;
        self
    }

    /// Set the KenPom-style Bayesian postgame metric adjustment factor.
    pub fn with_kenpom_update_factor(mut self, kenpom_update_factor: f64) -> Self {
        self.kenpom_update_factor = kenpom_update_factor;
        self
    }

    pub fn setup_tournament(&mut self, teams: Vec<Team>, config: &BracketConfig) {
        self.teams = teams;
        self.setup_first_round(config);
    }

    fn setup_first_round(&mut self, config: &BracketConfig) {
        let region_order = config.region_order();

        // Group teams by region and seed
        let mut region_teams: HashMap<String, HashMap<u8, Team>> = HashMap::new();
        for team in &self.teams {
            region_teams
                .entry(team.region.clone())
                .or_default()
                .insert(team.seed, team.clone());
            self.seeds.insert(team.team.clone(), team.seed);
        }

        // Validate all regions from config exist in the data
        for &region in &region_order {
            if !region_teams.contains_key(region) {
                panic!(
                    "Region '{}' from {} bracket config not found in teams. \
                     Available regions: {:?}",
                    region,
                    config.year,
                    region_teams.keys().collect::<Vec<_>>()
                );
            }
        }

        // Validate all regions in team data are covered by the config
        for region in region_teams.keys() {
            if !region_order.contains(&region.as_str()) {
                panic!(
                    "Region '{}' found in team data but not in bracket config. \
                     Config regions: {:?}",
                    region, region_order
                );
            }
        }

        // Validate each region has exactly seeds 1-16
        for &region in &region_order {
            let teams_by_seed = &region_teams[region];
            for seed in 1u8..=16 {
                if !teams_by_seed.contains_key(&seed) {
                    panic!(
                        "Region '{}' is missing seed {}. Present seeds: {:?}",
                        region,
                        seed,
                        {
                            let mut seeds: Vec<u8> = teams_by_seed.keys().copied().collect();
                            seeds.sort();
                            seeds
                        }
                    );
                }
            }
        }

        // Create games in bracket order: deterministic region ordering + S-curve seed matchups.
        // This ensures correct matchups through all rounds including Final Four.
        for &region in &region_order {
            let teams_by_seed = &region_teams[region];
            for &(seed_a, seed_b) in &BRACKET_SEED_ORDER {
                self.games.push(Game::new(
                    teams_by_seed[&seed_a].clone(),
                    teams_by_seed[&seed_b].clone(),
                ));
            }
        }

        assert_eq!(
            self.games.len(),
            32,
            "Expected 32 first-round games but got {}. Check bracket.csv for missing teams.",
            self.games.len()
        );
    }

    /// Simulate the full tournament, returning `(winner, loser)` name pairs.
    ///
    /// Unlike opponent bracket generation (which uses static pre-tournament
    /// metrics), this performs Bayesian `update_metrics` calls after each game
    /// so that later-round matchups reflect simulated tournament performance.
    pub fn simulate_tournament(&mut self, rng: &mut impl Rng) -> Vec<(String, String)> {
        let mut results = Vec::new();
        let mut current_round_games = self.games.clone();

        while !current_round_games.is_empty() {
            let (round_results, next_round_games) = self.simulate_round(current_round_games, rng);
            results.extend(round_results);
            current_round_games = next_round_games;
        }

        results
    }

    // Simulates a single round of the tournament
    // Returns Vec of (winner_name, loser_name) and next round games
    fn simulate_round(
        &self,
        games: Vec<Game>,
        rng: &mut impl Rng,
    ) -> (Vec<(String, String)>, Vec<Game>) {
        let mut round_results = Vec::new();
        let mut winners_for_next_round = Vec::new();

        for mut game in games {
            let t1_expected = game.expected_t1_metrics();
            game.result = Some(Game::simulate(t1_expected, self.pace_d, rng));

            if let Some(winner) = game.winner(self.pace_d, rng) {
                let result = game.result.as_ref().unwrap();
                let winner_is_t1 = winner.team == game.team1.team;

                // Observed metrics from team1's perspective (same as expected)
                let t1_observed = Metrics {
                    ortg: 100.0 * result.team1_score as f64 / result.pace,
                    drtg: 100.0 * result.team2_score as f64 / result.pace,
                    pace: result.pace,
                };

                // Update both teams from their own perspective
                let mut t1 = game.team1.clone();
                let mut t2 = game.team2.clone();
                t1.update_metrics(t1_expected, t1_observed, self.kenpom_update_factor);
                t2.update_metrics(
                    t1_expected.flip(),
                    t1_observed.flip(),
                    self.kenpom_update_factor,
                );

                let (winner_team, loser_name) = if winner_is_t1 {
                    (t1, t2.team)
                } else {
                    (t2, t1.team)
                };

                round_results.push((winner.team.clone(), loser_name));
                winners_for_next_round.push(winner_team);
            }
        }

        let next_round_games = self.create_next_round_matchups(winners_for_next_round);
        (round_results, next_round_games)
    }

    // Creates matchups for the next tournament round
    fn create_next_round_matchups(&self, winners: Vec<Team>) -> Vec<Game> {
        let mut next_round_games = Vec::new();

        // Pair up winners to create the next round's games
        for chunk in winners.chunks(2) {
            if chunk.len() == 2 {
                next_round_games.push(Game::new(chunk[0].clone(), chunk[1].clone()));
            }
        }

        next_round_games
    }

    pub fn get_teams(&self) -> &Vec<Team> {
        &self.teams
    }

    pub fn get_games(&self) -> &Vec<Game> {
        &self.games
    }

    pub fn get_seeds(&self) -> &HashMap<String, u8> {
        &self.seeds
    }

    pub fn calculate_team_win_probabilities(
        &self,
        num_simulations: usize,
    ) -> HashMap<String, Vec<f64>> {
        // Run simulations in parallel with rayon, each thread using its own RNG.
        // Each task returns a local HashMap of team advancement counts, then we
        // reduce by summing across all threads.
        let team_wins = (0..num_simulations)
            .into_par_iter()
            .map_init(rand::rng, |rng, _| {
                let mut tournament_clone = self.clone();
                let results = tournament_clone.simulate_tournament(rng);

                // Track how far each team advanced using game_index_to_round
                let mut teams_advanced: HashMap<String, usize> = HashMap::new();
                for (game_index, (winner, _loser)) in results.iter().enumerate() {
                    let round = self.game_index_to_round(game_index);
                    let prev = teams_advanced.get(winner).copied().unwrap_or(0);
                    if round + 1 > prev {
                        teams_advanced.insert(winner.clone(), round + 1);
                    }
                }

                // Build per-simulation counts
                let mut local_wins: HashMap<String, Vec<u32>> = HashMap::new();
                for (team_name, max_round) in teams_advanced {
                    let counts = local_wins.entry(team_name).or_insert_with(|| vec![0; 6]);
                    counts[max_round - 1] += 1;
                }

                local_wins
            })
            .reduce(HashMap::new, |mut acc, local| {
                for (team, counts) in local {
                    let entry = acc.entry(team).or_insert_with(|| vec![0; 6]);
                    for (i, c) in counts.iter().enumerate() {
                        entry[i] += c;
                    }
                }
                acc
            });

        // Convert counts to probabilities
        let num_sims_f64 = num_simulations as f64;
        team_wins
            .into_iter()
            .map(|(team_name, counts)| {
                let probs: Vec<f64> = counts.iter().map(|&c| c as f64 / num_sims_f64).collect();
                (team_name, probs)
            })
            .collect()
    }

    pub fn generate_bracket(&self, rng: &mut impl Rng) -> Bracket {
        let mut bracket_picks = Vec::new();
        let tournament_clone = self.clone();
        let mut current_round_games = tournament_clone.games.clone();

        while !current_round_games.is_empty() {
            let mut next_round_teams = Vec::new();

            for game in &current_round_games {
                // Pick winner based on probability
                let team1_win_prob = game.team1_win_probability();
                let winner = if rng.random::<f64>() < team1_win_prob {
                    &game.team1
                } else {
                    &game.team2
                };

                bracket_picks.push(winner.team.clone());
                next_round_teams.push(winner.clone());
            }

            // Create next round matchups
            current_round_games = Vec::new();
            for i in (0..next_round_teams.len()).step_by(2) {
                if i + 1 < next_round_teams.len() {
                    current_round_games.push(Game::new(
                        next_round_teams[i].clone(),
                        next_round_teams[i + 1].clone(),
                    ));
                }
            }
        }

        Bracket::new(bracket_picks)
    }

    pub fn score_bracket(
        &self,
        bracket: &Bracket,
        actual_results: &[(String, String)],
        scoring_system: ScoringSystem,
    ) -> u32 {
        let mut score = 0;
        let min_len = bracket.picks.len().min(actual_results.len());

        for (game_index, (actual_winner, actual_loser)) in
            actual_results.iter().enumerate().take(min_len)
        {
            let pick = &bracket.picks[game_index];

            if pick == actual_winner {
                let round_num = self.game_index_to_round(game_index);
                let team_seed = self.seeds.get(pick.as_str()).copied().unwrap_or(8);
                let opponent_seed = self.seeds.get(actual_loser.as_str()).copied().unwrap_or(8);
                score += scoring_system.calculate_points(round_num, team_seed, opponent_seed);
            }
        }

        score
    }

    /// Returns cumulative win probabilities: P(team advances at least through round R)
    /// Each team maps to a Vec<f64> of length 6 (rounds 1-6).
    pub fn cumulative_win_probabilities(
        &self,
        num_simulations: usize,
    ) -> HashMap<String, Vec<f64>> {
        let raw = self.calculate_team_win_probabilities(num_simulations);
        let mut cumulative: HashMap<String, Vec<f64>> = HashMap::new();
        for (team_name, probs) in raw {
            let cum: Vec<f64> = (0..6).map(|r| probs[r..].iter().sum::<f64>()).collect();
            cumulative.insert(team_name, cum);
        }
        cumulative
    }

    pub fn save_teams_to_csv(teams: &[Team], path: &str) -> io::Result<()> {
        let mut wtr = csv::Writer::from_path(path)?;
        // csv crate doesn't support #[serde(flatten)], so write manually
        wtr.write_record(["team", "seed", "region", "ortg", "drtg", "pace", "goose"])?;
        for team in teams {
            wtr.write_record([
                &team.team,
                &team.seed.to_string(),
                &team.region,
                &format!("{:.1}", team.metrics.ortg),
                &format!("{:.1}", team.metrics.drtg),
                &format!("{:.1}", team.metrics.pace),
                &format!("{:.2}", team.goose),
            ])?;
        }
        wtr.flush()?;
        Ok(())
    }

    /// Simulate the tournament, returning results as a ByteBracket u64.
    /// Same NB/Poisson simulation + Bayesian metric updates as `simulate_tournament`,
    /// but sets bits instead of collecting string pairs.
    pub fn simulate_tournament_bb(&mut self, rng: &mut impl Rng) -> u64 {
        let mut bits: u64 = crate::SENTINEL_BIT;
        let mut game_idx: usize = 0;
        let mut current_round_games = self.games.clone();

        while !current_round_games.is_empty() {
            let mut winners_for_next_round = Vec::new();

            for mut game in current_round_games {
                let t1_expected = game.expected_t1_metrics();
                game.result = Some(Game::simulate(t1_expected, self.pace_d, rng));

                if let Some(winner) = game.winner(self.pace_d, rng) {
                    let result = game.result.as_ref().unwrap();
                    let winner_is_t1 = winner.team == game.team1.team;

                    if winner_is_t1 {
                        bits |= crate::game_bit(game_idx);
                    }

                    let t1_observed = Metrics {
                        ortg: 100.0 * result.team1_score as f64 / result.pace,
                        drtg: 100.0 * result.team2_score as f64 / result.pace,
                        pace: result.pace,
                    };

                    let mut t1 = game.team1.clone();
                    let mut t2 = game.team2.clone();
                    t1.update_metrics(t1_expected, t1_observed, self.kenpom_update_factor);
                    t2.update_metrics(
                        t1_expected.flip(),
                        t1_observed.flip(),
                        self.kenpom_update_factor,
                    );

                    let winner_team = if winner_is_t1 { t1 } else { t2 };
                    winners_for_next_round.push(winner_team);
                }

                game_idx += 1;
            }

            current_round_games = self.create_next_round_matchups(winners_for_next_round);
        }

        bits
    }

    /// Simulate the tournament with live game overrides, returning results as a
    /// ByteBracket u64.
    ///
    /// Same Bayesian metric updates as [`simulate_tournament_bb`], but respects
    /// the current tournament state:
    /// - **Final** games: use the actual winner and score for Bayesian updates
    /// - **Live** games: simulate remaining possessions from the current score
    /// - **Upcoming** games: simulate the full game from scratch
    ///
    /// When all games are Upcoming this degenerates to `simulate_tournament_bb`.
    pub fn simulate_tournament_bb_with_status(
        &mut self,
        status: &TournamentStatus,
        rng: &mut impl Rng,
    ) -> u64 {
        let mut bits: u64 = crate::SENTINEL_BIT;
        let mut game_idx: usize = 0;
        let mut current_round_games = self.games.clone();

        while !current_round_games.is_empty() {
            let mut winners_for_next_round = Vec::new();

            for game in current_round_games {
                let t1_expected = game.expected_t1_metrics();
                let game_status = &status.games[game_idx];

                // Resolve the game outcome depending on its state.
                let (winner_is_t1, result) = match game_status.status {
                    GameState::Final => {
                        let t1_wins = game_status.winner.unwrap_or(true);
                        let result = if let Some(ref score) = game_status.score {
                            // Derive observed metrics from the actual score.
                            let total_expected = t1_expected.ortg + t1_expected.drtg;
                            let pace = if total_expected > 0.0 {
                                (score.team1 + score.team2) as f64 * 100.0 / total_expected
                            } else {
                                t1_expected.pace
                            };
                            GameResult {
                                team1_score: score.team1,
                                team2_score: score.team2,
                                pace,
                            }
                        } else {
                            // No score data — simulate to get plausible observed
                            // metrics for the Bayesian update. The winner is still
                            // forced to match the actual result below.
                            Game::simulate(t1_expected, self.pace_d, rng)
                        };
                        (t1_wins, result)
                    }
                    GameState::Live => {
                        let result = Self::resolve_live_game(&game, game_status, self.pace_d, rng);
                        let t1_wins = result.team1_score > result.team2_score;
                        (t1_wins, result)
                    }
                    GameState::Upcoming => {
                        let result = Game::simulate(t1_expected, self.pace_d, rng);
                        let t1_wins =
                            match game.pick_by_score(result.team1_score, result.team2_score) {
                                Some(w) => w.team == game.team1.team,
                                None => {
                                    // Tied — resolve via overtime
                                    game.resolve_overtime(result.team1_score, self.pace_d, rng)
                                        .map(|w| w.team == game.team1.team)
                                        .unwrap_or(rng.random::<bool>())
                                }
                            };
                        (t1_wins, result)
                    }
                };

                if winner_is_t1 {
                    bits |= crate::game_bit(game_idx);
                }

                // Bayesian metric update — same logic for all game states.
                let t1_observed = Metrics {
                    ortg: 100.0 * result.team1_score as f64 / result.pace,
                    drtg: 100.0 * result.team2_score as f64 / result.pace,
                    pace: result.pace,
                };

                let mut t1 = game.team1.clone();
                let mut t2 = game.team2.clone();
                t1.update_metrics(t1_expected, t1_observed, self.kenpom_update_factor);
                t2.update_metrics(
                    t1_expected.flip(),
                    t1_observed.flip(),
                    self.kenpom_update_factor,
                );

                let winner_team = if winner_is_t1 { t1 } else { t2 };
                winners_for_next_round.push(winner_team);

                game_idx += 1;
            }

            current_round_games = self.create_next_round_matchups(winners_for_next_round);
        }

        bits
    }

    /// Simulate the remaining portion of a live game using the live_resolver's
    /// clock/period estimation logic.
    fn resolve_live_game(
        game: &Game,
        game_status: &seismic_march_madness::types::GameStatus,
        pace_d: f64,
        rng: &mut impl Rng,
    ) -> GameResult {
        let score = match &game_status.score {
            Some(s) => s,
            None => {
                // No score yet (shouldn't happen for Live, but handle gracefully)
                return game.simulate_remaining((0, 0), 1200, 1, pace_d, rng);
            }
        };

        let (secs, per) = match (game_status.seconds_remaining, game_status.period) {
            (Some(s), Some(p)) => (s, p),
            (None, Some(p)) => (0, p),
            (Some(s), None) => {
                let total = (score.team1 + score.team2) as f64;
                let expected = game.estimate_total();
                let period = if expected > 0.0 && total / expected > 0.45 {
                    2
                } else {
                    1
                };
                (s, period)
            }
            (None, None) => {
                let total = (score.team1 + score.team2) as f64;
                let expected = game.estimate_total();
                let fraction = if expected > 0.0 {
                    (total / expected).clamp(0.05, 0.95)
                } else {
                    0.5
                };
                let remaining_secs = ((1.0 - fraction) * 2400.0) as i32;
                let period = if remaining_secs > 1200 { 1 } else { 2 };
                (remaining_secs, period)
            }
        };

        game.simulate_remaining((score.team1, score.team2), secs, per, pace_d, rng)
    }

    /// Convert game index to round number (0-indexed).
    ///
    /// Uses [`crate::ROUND_BOUNDARIES`] derived from [`crate::NUM_TEAMS`]:
    /// R64 [0..32), R32 [32..48), S16 [48..56), E8 [56..60), F4 [60..62), Champ [62..63).
    fn game_index_to_round(&self, game_index: usize) -> usize {
        use crate::{NUM_ROUNDS, ROUND_BOUNDARIES};
        for r in (0..NUM_ROUNDS).rev() {
            if game_index >= ROUND_BOUNDARIES[r] {
                return r;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bracket;
    use crate::bracket_config::BracketConfig;
    use seismic_march_madness::scoring::score_bracket;

    /// Build a minimal 4-team tournament (2 R1 games -> 1 championship = 3 total games).
    fn make_4team_tournament() -> (Tournament, Vec<Game>) {
        let teams = vec![
            Team {
                team: "A".into(),
                seed: 1,
                region: "W".into(),
                metrics: Metrics {
                    ortg: 120.0,
                    drtg: 90.0,
                    pace: 70.0,
                },
                goose: 0.0,
            },
            Team {
                team: "B".into(),
                seed: 2,
                region: "W".into(),
                metrics: Metrics {
                    ortg: 110.0,
                    drtg: 95.0,
                    pace: 70.0,
                },
                goose: 0.0,
            },
            Team {
                team: "C".into(),
                seed: 1,
                region: "X".into(),
                metrics: Metrics {
                    ortg: 115.0,
                    drtg: 92.0,
                    pace: 70.0,
                },
                goose: 0.0,
            },
            Team {
                team: "D".into(),
                seed: 2,
                region: "X".into(),
                metrics: Metrics {
                    ortg: 105.0,
                    drtg: 100.0,
                    pace: 70.0,
                },
                goose: 0.0,
            },
        ];

        let games = vec![
            Game::new(teams[0].clone(), teams[1].clone()), // game 0: A vs B
            Game::new(teams[2].clone(), teams[3].clone()), // game 1: C vs D
        ];

        let mut t = Tournament::new();
        t.teams = teams;
        t.games = games.clone();
        for team in &t.teams {
            t.seeds.insert(team.team.clone(), team.seed);
        }

        (t, games)
    }

    #[test]
    fn score_bracket_4team_all_correct() {
        let (t, _games) = make_4team_tournament();

        let actual = vec![
            ("A".into(), "B".into()),
            ("C".into(), "D".into()),
            ("A".into(), "C".into()),
        ];

        let bracket = Bracket::new(vec!["A".into(), "C".into(), "A".into()]);
        let score = t.score_bracket(&bracket, &actual, ScoringSystem::Base);
        assert_eq!(score, 3);
    }

    #[test]
    fn score_bracket_4team_all_wrong() {
        let (t, _games) = make_4team_tournament();

        let actual = vec![
            ("A".into(), "B".into()),
            ("C".into(), "D".into()),
            ("A".into(), "C".into()),
        ];

        let bracket = Bracket::new(vec!["B".into(), "D".into(), "B".into()]);
        let score = t.score_bracket(&bracket, &actual, ScoringSystem::Base);
        assert_eq!(score, 0);
    }

    #[test]
    fn score_bracket_4team_partial() {
        let (t, _games) = make_4team_tournament();

        let actual = vec![
            ("A".into(), "B".into()),
            ("C".into(), "D".into()),
            ("A".into(), "C".into()),
        ];

        let bracket = Bracket::new(vec!["A".into(), "D".into(), "A".into()]);
        let score = t.score_bracket(&bracket, &actual, ScoringSystem::Base);
        assert_eq!(score, 2);
    }

    #[test]
    #[should_panic(expected = "missing seed")]
    fn setup_first_round_panics_on_missing_seed() {
        let config = BracketConfig::for_year(2026);
        let kenpom_path = crate::data_dir().join("2026/men/kenpom.csv");
        if !kenpom_path.exists() {
            panic!("missing seed (test skipped: data file not found)");
        }

        let mut teams = match std::panic::catch_unwind(|| crate::load_teams_for_year(None, 2026)) {
            Ok(Ok(t)) => t,
            _ => panic!("missing seed (test skipped: kenpom data stale)"),
        };
        let region_order = config.region_order();
        let target_region = region_order[0];
        let remove_idx = teams
            .iter()
            .position(|t| t.region == target_region && t.seed == 5)
            .expect("should find seed 5 in first region");
        teams.remove(remove_idx);

        let mut tournament = Tournament::new();
        tournament.setup_tournament(teams, &config);
    }

    #[test]
    #[should_panic(expected = "found in team data but not in bracket config")]
    fn setup_first_round_panics_on_extra_region_in_data() {
        let config = BracketConfig {
            year: 9999,
            final_four: [
                ("East".to_string(), "West".to_string()),
                ("South".to_string(), "Midwest".to_string()),
            ],
        };

        let mut teams = Vec::new();
        for region in &["East", "West", "South", "Midwest"] {
            for seed in 1u8..=16 {
                teams.push(Team {
                    team: format!("{}-{}", region, seed),
                    seed,
                    region: region.to_string(),
                    metrics: Metrics {
                        ortg: 100.0,
                        drtg: 100.0,
                        pace: 70.0,
                    },
                    goose: 0.0,
                });
            }
        }
        for seed in 1u8..=16 {
            teams.push(Team {
                team: format!("Southeast-{}", seed),
                seed,
                region: "Southeast".to_string(),
                metrics: Metrics {
                    ortg: 100.0,
                    drtg: 100.0,
                    pace: 70.0,
                },
                goose: 0.0,
            });
        }

        let mut tournament = Tournament::new();
        tournament.setup_tournament(teams, &config);
    }

    // ── Golden vector tests (cross-language consistency with contract) ──

    fn load_vectors() -> serde_json::Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/test-vectors/bracket-vectors.json"
        );
        let data = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read test vectors at {}: {}", path, e));
        serde_json::from_str(&data).expect("Failed to parse test vectors JSON")
    }

    #[test]
    fn golden_vectors_self_score_192() {
        let vectors = load_vectors();
        let brackets = vectors["brackets"].as_array().unwrap();

        for v in brackets {
            let name = v["name"].as_str().unwrap();
            let hex = v["hex"].as_str().unwrap();
            let bb = crate::parse_bb(hex);
            let score = score_bracket(bb, bb);
            assert_eq!(
                score, 192,
                "Self-score should be 192 for '{}' (hex={})",
                name, hex
            );
        }
    }

    #[test]
    fn golden_vectors_scoring() {
        let vectors = load_vectors();
        let scoring_tests = vectors["scoringTests"].as_array().unwrap();

        for st in scoring_tests {
            let description = st["description"].as_str().unwrap();
            let bracket_hex = st["bracket"].as_str().unwrap();
            let results_hex = st["results"].as_str().unwrap();
            let expected_score = st["expectedScore"].as_u64().unwrap() as u32;

            let bracket = crate::parse_bb(bracket_hex);
            let results = crate::parse_bb(results_hex);
            let actual_score = score_bracket(bracket, results);

            assert_eq!(
                actual_score, expected_score,
                "Scoring mismatch for '{}': bracket={}, results={}",
                description, bracket_hex, results_hex
            );
        }
    }

    #[test]
    fn golden_vectors_encoding_parity() {
        use seismic_march_madness::reverse_game_bits;

        let vectors = load_vectors();
        let brackets = vectors["brackets"].as_array().unwrap();

        for v in brackets {
            let name = v["name"].as_str().unwrap();
            let legacy_hex = v["hex"].as_str().unwrap();
            let picks = v["picks"].as_array().unwrap();

            // Encode picks using contract-correct game_bit (game 0 → bit 0)
            let mut contract_bits: u64 = crate::SENTINEL_BIT;
            for (i, pick) in picks.iter().enumerate() {
                if pick.as_bool().unwrap() {
                    contract_bits |= crate::game_bit(i);
                }
            }

            // The JSON hex is legacy (game 0 → bit 62). Reversing the contract
            // encoding should recover the legacy hex.
            let legacy_bits = crate::parse_bb(legacy_hex);
            assert_eq!(
                reverse_game_bits(contract_bits),
                legacy_bits,
                "reverse(contract) != legacy for '{}': contract=0x{:016x}, legacy={}",
                name,
                contract_bits,
                legacy_hex
            );
        }
    }
}
