use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;
use eyre::bail;
use tracing::info;

use seismic_march_madness::{
    BracketForecast, EntryIndex, ForecastIndex, GameState, TournamentData, TournamentStatus,
    build_reach_probs, compute_current_score, compute_max_possible, get_teams_in_bracket_order,
    parse_bracket_hex, run_simulations,
};

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
    #[arg(long, default_value = "data/2026/men/status.json")]
    status_file: PathBuf,

    /// Path to the tournament data JSON (team names in bracket order).
    #[arg(long, default_value = "data/2026/men/tournament.json")]
    tournament_file: PathBuf,

    /// Path to write the forecast output JSON.
    #[arg(long, default_value = "data/forecasts.json")]
    output_file: PathBuf,

    /// Number of Monte Carlo simulations to run.
    #[arg(long, default_value = "100000")]
    simulations: u32,
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let entries: EntryIndex = serde_json::from_str(&std::fs::read_to_string(&cli.entries_file)?)?;
    let status: TournamentStatus =
        serde_json::from_str(&std::fs::read_to_string(&cli.status_file)?)?;
    let tournament: TournamentData =
        serde_json::from_str(&std::fs::read_to_string(&cli.tournament_file)?)?;

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

    let sim_results = run_simulations(&bracket_bits, &status, &reach, cli.simulations);

    let total_sims = cli.simulations as f64;
    let mut forecast: ForecastIndex = BTreeMap::new();
    for (i, (address, _, name)) in brackets.iter().enumerate() {
        forecast.insert(
            address.clone(),
            BracketForecast {
                current_score: current_scores[i],
                max_possible_score: max_possible_scores[i],
                expected_score: sim_results.expected_scores[i] / total_sims,
                win_probability: sim_results.wins[i] as f64 / total_sims,
                name: name.clone(),
            },
        );
    }

    let output = serde_json::to_string_pretty(&forecast)?;
    std::fs::write(&cli.output_file, output)?;
    info!(output = %cli.output_file.display(), brackets = forecast.len(), "forecast written");

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
