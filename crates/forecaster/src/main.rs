mod simulate;

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;
use eyre::bail;
use serde::Deserialize;
use tracing::info;

use march_madness_common::{
    BracketForecast, EntryIndex, ForecastIndex, GameState, TournamentStatus, parse_bracket_hex,
};

use crate::simulate::{ReachProbs, run_simulations};

#[derive(Parser, Debug)]
#[command(
    name = "march-madness-forecaster",
    about = "Simulate tournament outcomes and compute win probabilities for each bracket"
)]
struct Cli {
    /// Path to the entries JSON file (from indexer).
    #[arg(long, default_value = "data/entries.json")]
    entries_file: PathBuf,

    /// Path to the tournament status JSON file.
    #[arg(long, default_value = "data/tournament-status.json")]
    status_file: PathBuf,

    /// Path to the tournament data JSON (team names in bracket order).
    #[arg(long, default_value = "data/mens-2026.json")]
    tournament_file: PathBuf,

    /// Path to write the forecast output JSON.
    #[arg(long, default_value = "data/forecasts.json")]
    output_file: PathBuf,

    /// Number of Monte Carlo simulations to run.
    #[arg(long, default_value = "100000")]
    simulations: u32,
}

/// Minimal tournament data — just enough to get team names in bracket order.
#[derive(Deserialize)]
struct TournamentData {
    regions: Vec<String>,
    teams: Vec<TeamData>,
}

#[derive(Deserialize)]
struct TeamData {
    name: String,
    seed: u32,
    region: String,
}

/// Seed order per region (matches bracket encoding).
const SEED_ORDER: [u32; 16] = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];

/// Get all 64 team names in bracket order (region by region, seed-ordered).
fn get_teams_in_bracket_order(data: &TournamentData) -> Vec<String> {
    let mut teams = Vec::with_capacity(64);
    for region in &data.regions {
        let region_teams: Vec<&TeamData> =
            data.teams.iter().filter(|t| t.region == *region).collect();
        for &seed in &SEED_ORDER {
            let team = region_teams
                .iter()
                .find(|t| t.seed == seed)
                .expect("missing team for seed");
            teams.push(team.name.clone());
        }
    }
    teams
}

/// Build reach probability array (64 teams × 6 rounds) from the name-keyed map.
/// Falls back to a default (flat 0.5 per round) for teams not in the map.
fn build_reach_probs(
    team_names: &[String],
    reach_map: &std::collections::HashMap<String, Vec<f64>>,
) -> ReachProbs {
    team_names
        .iter()
        .map(|name| {
            if let Some(probs) = reach_map.get(name) {
                let mut arr = [0.5; 6];
                for (i, &p) in probs.iter().enumerate().take(6) {
                    arr[i] = p;
                }
                arr
            } else {
                // Default: uniform low probability
                [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125]
            }
        })
        .collect()
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Load inputs
    let entries: EntryIndex = {
        let data = std::fs::read_to_string(&cli.entries_file)?;
        serde_json::from_str(&data)?
    };

    let status: TournamentStatus = {
        let data = std::fs::read_to_string(&cli.status_file)?;
        serde_json::from_str(&data)?
    };

    let tournament: TournamentData = {
        let data = std::fs::read_to_string(&cli.tournament_file)?;
        serde_json::from_str(&data)?
    };

    info!(
        entries = entries.len(),
        games = status.games.len(),
        simulations = cli.simulations,
        "loaded data"
    );

    // Build reach probabilities from team names → bracket positions
    let team_names = get_teams_in_bracket_order(&tournament);
    let reach = match &status.team_reach_probabilities {
        Some(reach_map) => build_reach_probs(&team_names, reach_map),
        None => bail!("tournament status missing teamReachProbabilities — cannot simulate"),
    };

    // Parse all valid brackets
    let mut brackets: Vec<(String, u64, Option<String>)> = Vec::new();
    for (address, entry) in &entries {
        if let Some(hex) = &entry.bracket
            && let Some(bits) = parse_bracket_hex(hex)
        {
            brackets.push((address.clone(), bits, entry.name.clone()));
        }
    }

    if brackets.is_empty() {
        info!("no valid brackets found, writing empty forecast");
        let empty: ForecastIndex = BTreeMap::new();
        std::fs::write(&cli.output_file, serde_json::to_string_pretty(&empty)?)?;
        return Ok(());
    }

    info!(valid_brackets = brackets.len(), "parsed brackets");

    let undecided_count = status
        .games
        .iter()
        .filter(|g| g.status != GameState::Final)
        .count();
    info!(
        undecided = undecided_count,
        decided = 63 - undecided_count,
        "game status"
    );

    let bracket_bits: Vec<u64> = brackets.iter().map(|(_, bits, _)| *bits).collect();

    let current_scores: Vec<u32> = bracket_bits
        .iter()
        .map(|&b| compute_current_score(b, &status))
        .collect();

    let max_possible_scores: Vec<u32> = bracket_bits
        .iter()
        .map(|&b| compute_max_possible(b, &status))
        .collect();

    // Run forward Monte Carlo simulations
    let sim_results = run_simulations(&bracket_bits, &status, &reach, cli.simulations);

    let expected_scores: Vec<f64> = sim_results
        .expected_scores
        .iter()
        .map(|&total| total / cli.simulations as f64)
        .collect();

    let total_sims = cli.simulations as f64;

    // Build forecast output
    let mut forecast: ForecastIndex = BTreeMap::new();
    for (i, (address, _, name)) in brackets.iter().enumerate() {
        forecast.insert(
            address.clone(),
            BracketForecast {
                current_score: current_scores[i],
                max_possible_score: max_possible_scores[i],
                expected_score: expected_scores[i],
                win_probability: sim_results.wins[i] as f64 / total_sims,
                name: name.clone(),
            },
        );
    }

    // Write output
    let output = serde_json::to_string_pretty(&forecast)?;
    std::fs::write(&cli.output_file, output)?;

    info!(
        output = %cli.output_file.display(),
        brackets = forecast.len(),
        "forecast written"
    );

    // Print summary
    let mut sorted: Vec<_> = forecast.iter().collect();
    sorted.sort_by(|a, b| {
        b.1.win_probability
            .partial_cmp(&a.1.win_probability)
            .unwrap()
    });
    println!("\n--- Forecast Summary ---");
    println!(
        "{:<44} {:>5} {:>5} {:>7} {:>8}",
        "Address", "Score", "Max", "E[Score]", "P(Win)"
    );
    for (addr, f) in sorted.iter().take(20) {
        let display = f.name.as_deref().unwrap_or(addr);
        println!(
            "{:<44} {:>5} {:>5} {:>7.1} {:>7.1}%",
            display,
            f.current_score,
            f.max_possible_score,
            f.expected_score,
            f.win_probability * 100.0,
        );
    }

    Ok(())
}

/// Compute current score from decided games only.
fn compute_current_score(bracket: u64, status: &TournamentStatus) -> u32 {
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize)
                && game.status == GameState::Final
                && let Some(winner) = game.winner
            {
                let bit_pos = 62 - game_idx as u32;
                let bracket_picked_team1 = (bracket >> bit_pos) & 1 == 1;
                if bracket_picked_team1 == winner {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}

/// Compute maximum possible score: current score + all remaining round points for
/// undecided games. This is optimistic (ignores elimination cascades).
fn compute_max_possible(bracket: u64, status: &TournamentStatus) -> u32 {
    let round_points: [u32; 6] = [1, 2, 4, 8, 16, 32];
    let mut score: u32 = 0;
    let mut game_idx = 0u8;
    let mut games_in_round = 32u8;

    for round in 0..6u8 {
        for _ in 0..games_in_round {
            if let Some(game) = status.games.get(game_idx as usize) {
                if game.status == GameState::Final {
                    if let Some(winner) = game.winner {
                        let bit_pos = 62 - game_idx as u32;
                        let bracket_picked_team1 = (bracket >> bit_pos) & 1 == 1;
                        if bracket_picked_team1 == winner {
                            score += round_points[round as usize];
                        }
                    }
                } else {
                    score += round_points[round as usize];
                }
            }
            game_idx += 1;
        }
        games_in_round /= 2;
    }

    score
}
