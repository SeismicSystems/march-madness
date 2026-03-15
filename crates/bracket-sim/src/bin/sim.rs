use bracket_sim::Tournament;
use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
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

    /// Path to teams CSV (default: data/teams_{year}.csv)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Number of tournament simulations to run
    #[arg(short, long, default_value_t = 10000)]
    n_sims: usize,
}

fn default_data_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/bracket-sim/ at compile time; workspace root is two levels up
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root from CARGO_MANIFEST_DIR")
        .join("data")
}

fn path_to_str(p: &std::path::Path) -> std::io::Result<&str> {
    p.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("non-UTF-8 path: {}", p.display()),
        )
    })
}

fn load_teams(input: Option<PathBuf>, year: u16) -> std::io::Result<Vec<bracket_sim::Team>> {
    if let Some(path) = input {
        return Tournament::load_teams_from_csv(path_to_str(&path)?);
    }
    let data = default_data_dir();
    let bracket_json = data.join(format!("mens-{}.json", year));
    let kenpom = data.join(year.to_string()).join("kenpom.csv");
    Tournament::load_teams_from_json(&bracket_json, path_to_str(&kenpom)?)
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
        "starting simulation"
    );

    let teams = load_teams(args.input, args.year)?;

    let mut tournament = Tournament::new();
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
