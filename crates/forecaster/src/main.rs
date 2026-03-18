use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use clap::Parser;
use eyre::bail;
use tracing::info;

use bracket_sim::Team;
use bracket_sim::live_resolver::GameModelResolver;
use seismic_march_madness::redis_keys::{DEFAULT_REDIS_URL, KEY_GAMES};
use seismic_march_madness::{
    BracketForecast, EntryIndex, ForecastIndex, GameState, TournamentData, TournamentStatus,
    build_reach_probs, compute_current_score, compute_max_possible, get_teams_in_bracket_order,
    kenpom_csv, parse_bracket_hex, run_simulations_with_resolver,
    run_team_advance_simulations_with_resolver, tournament_json,
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

    /// Read live tournament status from Redis instead of a file.
    #[arg(long)]
    live: bool,

    /// Path to the tournament status JSON file.
    /// Defaults to data/{year}/men/status.json. Ignored when --live is set.
    #[arg(long = "status")]
    status_file: Option<PathBuf>,

    /// Path to the tournament data JSON (team names in bracket order).
    /// If not specified, uses the embedded 2026 tournament data.
    #[arg(long)]
    tournament_file: Option<PathBuf>,

    /// Path to write the forecast output JSON.
    #[arg(long, default_value = "data/2026/men/forecasts.json")]
    output_file: PathBuf,

    /// Number of Monte Carlo simulations to run.
    #[arg(long, default_value = "100000")]
    simulations: u32,

    /// Tournament year (for loading embedded team data).
    #[arg(long, default_value = "2026")]
    year: u16,

    /// Print per-team advance probabilities for each round (no entries needed).
    #[arg(long)]
    team_advance: bool,
}

fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let status: TournamentStatus = if cli.live {
        info!("reading tournament status from Redis");
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
        let client = redis::Client::open(url.as_str())?;
        let mut conn = client.get_connection()?;
        let json: Option<String> = redis::Commands::get(&mut conn, KEY_GAMES)?;
        let json =
            json.ok_or_else(|| eyre::eyre!("no tournament status in Redis (key: {KEY_GAMES})"))?;
        serde_json::from_str(&json)?
    } else {
        let status_path = cli
            .status_file
            .unwrap_or_else(|| PathBuf::from(format!("data/{}/men/status.json", cli.year)));
        info!("reading tournament status from {}", status_path.display());
        serde_json::from_str(&std::fs::read_to_string(&status_path)?)?
    };
    let tournament: TournamentData = match &cli.tournament_file {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => TournamentData::embedded(cli.year),
    };

    // Build team names in bracket order
    let team_names = get_teams_in_bracket_order(&tournament);

    // Load team metrics for live game simulation
    let tj = tournament_json(cli.year)
        .unwrap_or_else(|| panic!("no embedded tournament data for year {}", cli.year));
    let kp = kenpom_csv(cli.year)
        .unwrap_or_else(|| panic!("no embedded KenPom data for year {}", cli.year));
    let teams = bracket_sim::team::load_teams_from_json_str(tj, kp)?;
    let team_map: HashMap<String, Team> = teams.into_iter().map(|t| (t.team.clone(), t)).collect();

    // Build resolver for live games — simulates remaining possessions directly
    let live_count = status
        .games
        .iter()
        .filter(|g| g.status == GameState::Live)
        .count();
    let resolver = GameModelResolver::new(&team_names, &team_map, bracket_sim::DEFAULT_PACE_D);
    let resolver_opt: Option<&dyn seismic_march_madness::LiveGameResolver> = if live_count > 0 {
        info!(
            live_games = live_count,
            "using game model resolver for live games"
        );
        Some(&resolver)
    } else {
        None
    };

    // Build reach probabilities from team names → bracket positions
    let reach = match &status.team_reach_probabilities {
        Some(reach_map) => build_reach_probs(&team_names, reach_map),
        None => bail!("tournament status missing teamReachProbabilities — cannot simulate"),
    };

    info!(
        games = status.games.len(),
        simulations = cli.simulations,
        "loaded data"
    );

    // --team-advance: print per-team advance probabilities and exit
    if cli.team_advance {
        let results = run_team_advance_simulations_with_resolver(
            &status,
            &reach,
            cli.simulations,
            resolver_opt,
        );
        results.print_table(&team_names, |name| {
            team_map.get(name).map(|t| t.seed).unwrap_or(0)
        });
        return Ok(());
    }

    // Normal forecast mode — needs entries
    let entries: EntryIndex = serde_json::from_str(&std::fs::read_to_string(&cli.entries_file)?)?;
    info!(entries = entries.len(), "loaded entries");

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

    let sim_results = run_simulations_with_resolver(
        &bracket_bits,
        &status,
        &reach,
        cli.simulations,
        resolver_opt,
    );

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
