use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::live_resolver::GameModelResolver;
use bracket_sim::{DEFAULT_PACE_D, Tournament, load_teams_for_year};
use clap::Parser;
use tracing::info;

use seismic_march_madness::redis_keys::{DEFAULT_REDIS_URL, KEY_GAMES};
use seismic_march_madness::{
    GameState, TournamentData, TournamentStatus, build_reach_probs, get_teams_in_bracket_order,
    run_team_advance_simulations_with_resolver,
};

#[derive(Parser, Debug)]
#[command(name = "sim")]
#[command(version = "0.1.0")]
#[command(about = "Simulate tournament and print round-by-round advancement probabilities")]
struct SimArgs {
    /// Tournament year (determines bracket structure / Final Four pairings)
    #[arg(short = 'y', long, default_value_t = DEFAULT_YEAR)]
    year: u16,

    /// Path to combined teams CSV (overrides default JSON+KenPom loading)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Number of tournament simulations to run
    #[arg(short, long, default_value_t = 10000)]
    n_sims: usize,

    /// Pace dispersion ratio (variance / mean).
    /// <1 = underdispersed (binomial), 1 = Poisson, >1 = overdispersed (NB).
    #[arg(long, default_value_t = DEFAULT_PACE_D)]
    pace_d: f64,

    /// Condition on live tournament status from Redis.
    #[arg(long, conflicts_with = "status")]
    live: bool,

    /// Condition on game state from a status JSON file.
    /// Omit both --live and --status to simulate from scratch.
    #[arg(long = "status")]
    status: Option<PathBuf>,
}

fn print_table(
    team_names: &[String],
    team_map: &HashMap<String, bracket_sim::Team>,
    probs: &[[f64; 6]],
) {
    println!(
        "\n{:<25} {:>4}  {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "Team", "Seed", "R64", "R32", "S16", "E8", "F4", "Champ"
    );
    println!("{}", "-".repeat(82));

    let mut indices: Vec<usize> = (0..64).collect();
    indices.sort_by(|&a, &b| {
        probs[b][5]
            .partial_cmp(&probs[a][5])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for &idx in &indices {
        let name = &team_names[idx];
        let seed = team_map.get(name).map(|t| t.seed).unwrap_or(0);
        println!(
            "{:<25} {:>4}  {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}% {:>6.1}%",
            name,
            seed,
            probs[idx][0] * 100.0,
            probs[idx][1] * 100.0,
            probs[idx][2] * 100.0,
            probs[idx][3] * 100.0,
            probs[idx][4] * 100.0,
            probs[idx][5] * 100.0,
        );
    }
}

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .init();

    let args = SimArgs::parse();
    let bracket_config = BracketConfig::for_year(args.year);

    let teams = load_teams_for_year(args.input.as_deref(), args.year)?;
    let team_map: HashMap<String, bracket_sim::Team> =
        teams.iter().map(|t| (t.team.clone(), t.clone())).collect();

    // Get team names in bracket order
    let tournament_data = TournamentData::embedded(args.year);
    let team_names = get_teams_in_bracket_order(&tournament_data);

    // Load tournament status: --live (Redis), --status <path> (file), or none (unconditioned)
    let status: Option<TournamentStatus> = if args.live {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
        let client = redis::Client::open(url.as_str()).map_err(io::Error::other)?;
        let mut conn = client.get_connection().map_err(io::Error::other)?;
        let json: Option<String> =
            redis::Commands::get(&mut conn, KEY_GAMES).map_err(io::Error::other)?;
        let json = json.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no tournament status in Redis (key: {KEY_GAMES})"),
            )
        })?;
        info!("loaded tournament status from Redis");
        Some(serde_json::from_str(&json).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Redis status: {e}"))
        })?)
    } else if let Some(status_path) = &args.status {
        let status_str = std::fs::read_to_string(status_path)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", status_path.display(), e)))?;
        info!("loaded tournament status from {}", status_path.display());
        Some(serde_json::from_str(&status_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: {}", status_path.display(), e),
            )
        })?)
    } else {
        None
    };

    if let Some(ref status) = status {
        info!(
            year = args.year,
            n_sims = args.n_sims,
            pace_d = args.pace_d,
            "conditioned simulation"
        );

        // Build resolver for live games
        let live_count = status
            .games
            .iter()
            .filter(|g| g.status == GameState::Live)
            .count();
        let resolver = GameModelResolver::new(&team_names, &team_map, args.pace_d);
        let resolver_opt: Option<&dyn seismic_march_madness::LiveGameResolver> = if live_count > 0 {
            info!(live_games = live_count, "using game model resolver");
            Some(&resolver)
        } else {
            None
        };

        // Build reach probs: use status file's if present, otherwise compute from full tournament sim
        let reach = if let Some(reach_map) = &status.team_reach_probabilities {
            if !reach_map.is_empty() {
                info!("using reach probs from status");
                build_reach_probs(&team_names, reach_map)
            } else {
                compute_reach_probs(&teams, &bracket_config, &team_names, args.n_sims)
            }
        } else {
            compute_reach_probs(&teams, &bracket_config, &team_names, args.n_sims)
        };

        let results = run_team_advance_simulations_with_resolver(
            status,
            &reach,
            args.n_sims as u32,
            resolver_opt,
        );
        results.print_table(&team_names, |name| {
            team_map.get(name).map(|t| t.seed).unwrap_or(0)
        });
    } else {
        // Unconditioned mode: full Poisson tournament sim (original behavior)
        info!(
            year = args.year,
            n_sims = args.n_sims,
            pace_d = args.pace_d,
            "unconditioned simulation"
        );

        let mut tournament = Tournament::new().with_pace_d(args.pace_d);
        tournament.setup_tournament(teams, &bracket_config);
        let win_probs = tournament.calculate_team_win_probabilities(args.n_sims);

        // Convert to bracket-order array
        let probs: Vec<[f64; 6]> = team_names
            .iter()
            .map(|name| {
                if let Some(raw) = win_probs.get(name) {
                    let mut cum = [0.0; 6];
                    for (r, val) in cum.iter_mut().enumerate() {
                        *val = raw[r..].iter().sum::<f64>();
                    }
                    cum
                } else {
                    [0.0; 6]
                }
            })
            .collect();

        print_table(&team_names, &team_map, &probs);
    }

    Ok(())
}

/// Compute reach probs from a full Poisson tournament sim.
fn compute_reach_probs(
    teams: &[bracket_sim::Team],
    bracket_config: &BracketConfig,
    team_names: &[String],
    n_sims: usize,
) -> seismic_march_madness::ReachProbs {
    info!(n_sims, "computing reach probs from full tournament sim");
    let mut tournament = Tournament::new();
    tournament.setup_tournament(teams.to_vec(), bracket_config);
    let cum_probs = tournament.cumulative_win_probabilities(n_sims);
    let reach_map: HashMap<String, Vec<f64>> = cum_probs.into_iter().collect();
    build_reach_probs(team_names, &reach_map)
}
