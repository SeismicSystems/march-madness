use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::{Tournament, load_teams_for_year};
use clap::Parser;
use std::io;
use std::path::PathBuf;
use tracing::info;

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
    #[arg(long, default_value_t = bracket_sim::DEFAULT_PACE_D)]
    pace_d: f64,
}

fn main() -> io::Result<()> {
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

    info!(
        year = args.year,
        n_sims = args.n_sims,
        pace_d = args.pace_d,
        "starting simulation"
    );

    let teams = load_teams_for_year(args.input.as_deref(), args.year)?;

    let mut tournament = Tournament::new().with_pace_d(args.pace_d);
    tournament.setup_tournament(teams, &bracket_config);
    let win_probs = tournament.calculate_team_win_probabilities(args.n_sims);

    println!(
        "\n{:<20} {:<5} {:<8} {:<8} {:<8} {:<8} {:<8} {:<8}",
        "Team", "Seed", "Rd1", "Rd2", "Sweet16", "Elite8", "Final4", "Champ"
    );

    let mut sorted_teams: Vec<_> = tournament.get_teams().clone();
    sorted_teams.sort_by(|a, b| a.seed.cmp(&b.seed).then(a.region.cmp(&b.region)));

    for team in &sorted_teams {
        if let Some(probs) = win_probs.get(&team.team) {
            let mut cumulative = [0.0f64; 6];
            for r in 0..6 {
                cumulative[r] = probs[r..].iter().sum::<f64>();
            }
            println!(
                "{:<20} {:<5} {:<8.1} {:<8.1} {:<8.1} {:<8.1} {:<8.1} {:<8.1}",
                team.team,
                team.seed,
                cumulative[0] * 100.0,
                cumulative[1] * 100.0,
                cumulative[2] * 100.0,
                cumulative[3] * 100.0,
                cumulative[4] * 100.0,
                cumulative[5] * 100.0,
            );
        }
    }

    Ok(())
}
