use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use clap::Parser;
use eyre::bail;
use tracing::info;

use bracket_sim::{DEFAULT_PACE_D, Game, Team};
use seismic_march_madness::{
    BracketForecast, EntryIndex, ForecastIndex, GameState, TournamentData, TournamentStatus,
    build_reach_probs, compute_current_score, compute_max_possible, get_teams_in_bracket_order,
    kenpom_csv, parse_bracket_hex, run_simulations, tournament_json,
};

/// Round start offsets (mirrors simulate.rs).
const ROUND_STARTS: [usize; 6] = [0, 32, 48, 56, 60, 62];
/// Number of Monte Carlo sims for computing each live game's conditional probability.
const LIVE_GAME_SIMS: u32 = 10_000;

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
}

/// For R64 game at index g (0-31), return the two team indices (0-63).
fn r64_teams(g: usize) -> (usize, usize) {
    (2 * g, 2 * g + 1)
}

/// Get the two feeder game indices for a game in rounds 1-5.
fn feeder_games(g: usize, round: usize) -> (usize, usize) {
    let offset_in_round = g - ROUND_STARTS[round];
    let prev_start = ROUND_STARTS[round - 1];
    (
        prev_start + 2 * offset_in_round,
        prev_start + 2 * offset_in_round + 1,
    )
}

/// Determine which round a game index belongs to.
fn game_round(g: usize) -> usize {
    for r in (0..6).rev() {
        if g >= ROUND_STARTS[r] {
            return r;
        }
    }
    0
}

/// Resolve the team index (0-63) that won a decided game, tracing back to R64.
/// Returns None if the game is not Final.
fn resolve_winner_team_idx(g: usize, status: &TournamentStatus) -> Option<usize> {
    let game = &status.games[g];
    if game.status != GameState::Final {
        return None;
    }
    let winner_is_team1 = game.winner?;

    let round = game_round(g);
    if round == 0 {
        let (t1, t2) = r64_teams(g);
        return Some(if winner_is_team1 { t1 } else { t2 });
    }

    let (f1, f2) = feeder_games(g, round);
    let team1_idx = resolve_winner_team_idx(f1, status)?;
    let team2_idx = resolve_winner_team_idx(f2, status)?;
    Some(if winner_is_team1 {
        team1_idx
    } else {
        team2_idx
    })
}

/// Resolve the two team indices playing in a game. For R64, this is direct.
/// For later rounds, trace feeder games (which must be Final for this game to be Live).
fn resolve_game_teams(g: usize, status: &TournamentStatus) -> Option<(usize, usize)> {
    let round = game_round(g);
    if round == 0 {
        return Some(r64_teams(g));
    }
    let (f1, f2) = feeder_games(g, round);
    let team1_idx = resolve_winner_team_idx(f1, status)?;
    let team2_idx = resolve_winner_team_idx(f2, status)?;
    Some((team1_idx, team2_idx))
}

/// Compute model-derived conditional win probabilities for live games that have
/// score + time data. Patches the probabilities into the status in-place.
fn compute_live_game_probabilities(
    status: &mut TournamentStatus,
    team_names: &[String],
    team_map: &HashMap<String, Team>,
) {
    let mut computed = 0u32;
    for g in 0..63 {
        let game = &status.games[g];
        if game.status != GameState::Live {
            continue;
        }

        // Need score + time data to compute conditional probability
        let score = match &game.score {
            Some(s) => (s.team1, s.team2),
            None => continue,
        };
        let seconds_remaining = match game.seconds_remaining {
            Some(s) => s,
            None => continue,
        };
        let period = match game.period {
            Some(p) => p,
            None => continue,
        };

        // Resolve the two teams playing
        let (t1_idx, t2_idx) = match resolve_game_teams(g, status) {
            Some(pair) => pair,
            None => continue,
        };
        let t1_name = &team_names[t1_idx];
        let t2_name = &team_names[t2_idx];
        let (t1, t2) = match (team_map.get(t1_name), team_map.get(t2_name)) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };

        let game_model = Game::new(t1.clone(), t2.clone());
        let prob = game_model.conditional_win_probability(
            score,
            seconds_remaining,
            period,
            DEFAULT_PACE_D,
            LIVE_GAME_SIMS,
        );

        info!(
            game_index = g,
            team1 = t1_name,
            team2 = t2_name,
            score = format!("{}-{}", score.0, score.1),
            period,
            seconds_remaining,
            computed_prob = format!("{:.3}", prob),
            "computed live game conditional probability"
        );

        status.games[g].team1_win_probability = Some(prob);
        computed += 1;
    }

    if computed > 0 {
        info!(computed, "patched live game probabilities from game model");
    }
}

fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let entries: EntryIndex = serde_json::from_str(&std::fs::read_to_string(&cli.entries_file)?)?;
    let mut status: TournamentStatus =
        serde_json::from_str(&std::fs::read_to_string(&cli.status_file)?)?;
    let tournament: TournamentData = match &cli.tournament_file {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => TournamentData::embedded(cli.year),
    };

    info!(
        entries = entries.len(),
        games = status.games.len(),
        simulations = cli.simulations,
        "loaded data"
    );

    // Build team names in bracket order
    let team_names = get_teams_in_bracket_order(&tournament);

    // Load team metrics for computing live game conditional probabilities
    let tj = tournament_json(cli.year)
        .unwrap_or_else(|| panic!("no embedded tournament data for year {}", cli.year));
    let kp = kenpom_csv(cli.year)
        .unwrap_or_else(|| panic!("no embedded KenPom data for year {}", cli.year));
    let teams = bracket_sim::team::load_teams_from_json_str(tj, kp)?;
    let team_map: HashMap<String, Team> = teams.into_iter().map(|t| (t.team.clone(), t)).collect();

    // Compute model-derived conditional probabilities for live games
    let live_count = status
        .games
        .iter()
        .filter(|g| g.status == GameState::Live)
        .count();
    if live_count > 0 {
        info!(
            live_games = live_count,
            "computing conditional probabilities for live games"
        );
        compute_live_game_probabilities(&mut status, &team_names, &team_map);
    }

    // Build reach probabilities from team names → bracket positions
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
