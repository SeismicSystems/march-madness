use rand::Rng;

use crate::bracket_config::{BRACKET_SEED_ORDER, BracketConfig};
use crate::game::Game;
use crate::metrics::Metrics;
use crate::team::{self, Team};
use crate::{Bracket, ScoringSystem};
use std::collections::HashMap;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Tournament {
    teams: Vec<Team>,
    games: Vec<Game>,
    seeds: HashMap<String, u8>,
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
        }
    }

    /// Load teams by joining a tournament JSON (data/mens-{year}.json) with a KenPom CSV.
    pub fn load_teams_from_json(json_path: &Path, kenpom_path: &str) -> io::Result<Vec<Team>> {
        team::load_teams_from_json(json_path, kenpom_path)
    }

    /// Load teams by joining bracket CSV (team,seed,region) with KenPom CSV (team,ortg,drtg,pace).
    /// Panics if any bracket team is missing from KenPom data.
    pub fn load_teams(bracket_path: &str, kenpom_path: &str) -> io::Result<Vec<Team>> {
        team::load_teams(bracket_path, kenpom_path)
    }

    /// Load teams from a single combined CSV (legacy format: team,seed,region,ortg,drtg,pace[,goose]).
    pub fn load_teams_from_csv(path: &str) -> io::Result<Vec<Team>> {
        team::load_teams_from_combined_csv(path)
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
            game.result = Some(Game::simulate(t1_expected, rng));

            if let Some(winner) = game.winner(rng) {
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
                t1.update_metrics(t1_expected, t1_observed);
                t2.update_metrics(t1_expected.flip(), t1_observed.flip());

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
        let mut rng = rand::rng();
        let mut team_wins: HashMap<String, Vec<u32>> = HashMap::new();

        // Initialize counts for each team at each round (6 rounds)
        for team in &self.teams {
            team_wins.insert(team.team.clone(), vec![0; 6]);
        }

        // Run simulations
        for _ in 0..num_simulations {
            let mut tournament_clone = self.clone();
            let results = tournament_clone.simulate_tournament(&mut rng);

            // Track how far each team advanced using game_index_to_round
            let mut teams_advanced: HashMap<String, usize> = HashMap::new();

            for (game_index, (winner, _loser)) in results.iter().enumerate() {
                let round = self.game_index_to_round(game_index);
                let prev = teams_advanced.get(winner).copied().unwrap_or(0);
                if round + 1 > prev {
                    teams_advanced.insert(winner.clone(), round + 1);
                }
            }

            // Update team win counts
            for (team_name, max_round) in teams_advanced {
                if let Some(counts) = team_wins.get_mut(&team_name) {
                    counts[max_round - 1] += 1;
                }
            }
        }

        // Convert counts to probabilities
        let mut probabilities: HashMap<String, Vec<f64>> = HashMap::new();
        let num_sims_f64 = num_simulations as f64;

        for (team_name, counts) in team_wins {
            let probs: Vec<f64> = counts.iter().map(|&c| c as f64 / num_sims_f64).collect();
            probabilities.insert(team_name, probs);
        }

        probabilities
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
    /// Same Poisson simulation + Bayesian metric updates as `simulate_tournament`,
    /// but sets bits instead of collecting string pairs.
    pub fn simulate_tournament_bb(&mut self, rng: &mut impl Rng) -> u64 {
        let mut bits: u64 = 0;
        let mut bit_idx: u32 = 0;
        let mut current_round_games = self.games.clone();

        while !current_round_games.is_empty() {
            let mut winners_for_next_round = Vec::new();

            for mut game in current_round_games {
                let t1_expected = game.expected_t1_metrics();
                game.result = Some(Game::simulate(t1_expected, rng));

                if let Some(winner) = game.winner(rng) {
                    let result = game.result.as_ref().unwrap();
                    let winner_is_t1 = winner.team == game.team1.team;

                    if winner_is_t1 {
                        bits |= 1u64 << bit_idx;
                    }

                    let t1_observed = Metrics {
                        ortg: 100.0 * result.team1_score as f64 / result.pace,
                        drtg: 100.0 * result.team2_score as f64 / result.pace,
                        pace: result.pace,
                    };

                    let mut t1 = game.team1.clone();
                    let mut t2 = game.team2.clone();
                    t1.update_metrics(t1_expected, t1_observed);
                    t2.update_metrics(t1_expected.flip(), t1_observed.flip());

                    let winner_team = if winner_is_t1 { t1 } else { t2 };
                    winners_for_next_round.push(winner_team);
                }

                bit_idx += 1;
            }

            current_round_games = self.create_next_round_matchups(winners_for_next_round);
        }

        bits
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
    use crate::scoring::score_base_bb;

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
        let config = BracketConfig::for_year(2025);
        let teams_path = "../../data/2025/teams.csv";
        if !std::path::Path::new(teams_path).exists() {
            panic!("missing seed (test skipped: data file not found)");
        }

        let mut teams = Tournament::load_teams_from_csv(teams_path).unwrap();
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

    #[test]
    fn score_base_bb_identical() {
        let results: u64 = 0x5A5A_5A5A_5A5A_5A5A & ((1u64 << 63) - 1);
        let score = score_base_bb(results, results);
        assert_eq!(score, 192);
    }

    #[test]
    fn score_base_bb_all_different() {
        let bracket: u64 = 0;
        let results: u64 = (1u64 << 63) - 1;
        let score = score_base_bb(bracket, results);
        assert_eq!(score, 0);
    }

    #[test]
    fn score_base_bb_r0_only() {
        let bracket: u64 = 0xFFFF_FFFF;
        let results: u64 = 0x7FFF_FFFF_FFFF_FFFF;
        let score = score_base_bb(bracket, results);
        assert_eq!(score, 32);
    }

    #[test]
    fn score_base_bb_cross_validate_with_string_scoring() {
        let config = BracketConfig::for_year(2025);
        let teams_path = "../../data/2025/teams.csv";

        if !std::path::Path::new(teams_path).exists() {
            return;
        }

        let teams = Tournament::load_teams_from_csv(teams_path).unwrap();
        let mut tournament = Tournament::new();
        tournament.setup_tournament(teams, &config);

        let mut rng = rand::rng();

        for _ in 0..20 {
            let bracket = tournament.generate_bracket(&mut rng);
            let mut tourn_clone = tournament.clone();
            let actual_results = tourn_clone.simulate_tournament(&mut rng);

            let string_score =
                tournament.score_bracket(&bracket, &actual_results, ScoringSystem::Base);

            let first_round_games = tournament.get_games();
            let bracket_bb =
                u64::from_str_radix(&bracket.to_byte_bracket(first_round_games), 16).unwrap();

            let results_bracket = {
                let picks: Vec<String> = actual_results.iter().map(|(w, _)| w.clone()).collect();
                Bracket::new(picks)
            };
            let results_bb =
                u64::from_str_radix(&results_bracket.to_byte_bracket(first_round_games), 16)
                    .unwrap();

            let bb_score = score_base_bb(bracket_bb, results_bb);

            assert_eq!(
                string_score, bb_score,
                "String score {} != BB score {} for bracket {:016X} vs results {:016X}",
                string_score, bb_score, bracket_bb, results_bb
            );
        }
    }
}
