//! Score distribution calibration tool.
//!
//! Sweeps pace dispersion values and reports game-level statistics
//! (mean total, margin spread, OT frequency, etc.) so you can compare
//! against empirical NCAA tournament data and pick the best-fit parameter.
//!
//! Known NCAA tournament empirical targets (approximate):
//!   - Average total score:      ~140-145 points
//!   - Average margin:           ~10-12 points (unsigned)
//!   - Margin stddev:            ~12-13 points
//!   - OT frequency:             ~5-7% of games
//!   - Total stddev:             ~18-20 points

use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::load_teams_for_year;
use bracket_sim::{Game, Tournament};
use clap::Parser;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "score-dist")]
#[command(about = "Sweep pace dispersion and report game score distributions")]
struct Args {
    /// Tournament year
    #[arg(short = 'y', long, default_value_t = DEFAULT_YEAR)]
    year: u16,

    /// Path to combined teams CSV
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Games to simulate per dispersion value
    #[arg(short, long, default_value_t = 50_000)]
    n_games: usize,

    /// Dispersion values to sweep (comma-separated)
    #[arg(short, long, default_value = "0.3,0.5,0.7,0.8,0.9,1.0,1.2,1.5,2.0")]
    d_values: String,

    /// RNG seed for reproducibility
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

struct GameStats {
    total: f64,
    margin: f64,
    is_tie: bool,
    pace: f64,
}

fn simulate_games(
    games: &[Game],
    pace_d: f64,
    n_games: usize,
    rng: &mut impl Rng,
) -> Vec<GameStats> {
    let mut stats = Vec::with_capacity(n_games);
    let n_matchups = games.len();

    for i in 0..n_games {
        let game = &games[i % n_matchups];
        let metrics = game.expected_t1_metrics();
        let result = Game::simulate(metrics, pace_d, rng);

        stats.push(GameStats {
            total: (result.team1_score + result.team2_score) as f64,
            margin: (result.team1_score as f64 - result.team2_score as f64).abs(),
            is_tie: result.team1_score == result.team2_score,
            pace: result.pace,
        });
    }

    stats
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let bracket_config = BracketConfig::for_year(args.year);
    let teams = load_teams_for_year(args.input.as_deref(), args.year)?;

    let mut tournament = Tournament::new();
    tournament.setup_tournament(teams, &bracket_config);
    let games = tournament.get_games().clone();

    let d_values: Vec<f64> = args
        .d_values
        .split(',')
        .map(|s| s.trim().parse::<f64>().expect("invalid dispersion value"))
        .collect();

    // Header
    println!(
        "\n{:>6}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}",
        "d", "AvgTot", "TotSD", "AvgMgn", "MgnSD", "OT%", "AvgPace", "PaceSD", "P(0pts)"
    );
    println!("{}", "-".repeat(82));

    // Empirical targets row
    println!(
        "{:>6}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}  {:>7}",
        "REAL", "~142", "~19", "~11", "~12", "~6%", "~68", "?", "~0%"
    );
    println!("{}", "-".repeat(82));

    for &d in &d_values {
        let mut rng = StdRng::seed_from_u64(args.seed);
        let stats = simulate_games(&games, d, args.n_games, &mut rng);
        let n = stats.len() as f64;

        let avg_total: f64 = stats.iter().map(|s| s.total).sum::<f64>() / n;
        let total_sd: f64 = (stats
            .iter()
            .map(|s| (s.total - avg_total).powi(2))
            .sum::<f64>()
            / (n - 1.0))
            .sqrt();

        let avg_margin: f64 = stats.iter().map(|s| s.margin).sum::<f64>() / n;
        let margin_sd: f64 = (stats
            .iter()
            .map(|s| (s.margin - avg_margin).powi(2))
            .sum::<f64>()
            / (n - 1.0))
            .sqrt();

        let ot_pct: f64 = stats.iter().filter(|s| s.is_tie).count() as f64 / n * 100.0;

        let avg_pace: f64 = stats.iter().map(|s| s.pace).sum::<f64>() / n;
        let pace_sd: f64 = (stats
            .iter()
            .map(|s| (s.pace - avg_pace).powi(2))
            .sum::<f64>()
            / (n - 1.0))
            .sqrt();

        let zero_pts_pct: f64 = stats
            .iter()
            .filter(|s| s.total < 0.5) // both teams scored 0
            .count() as f64
            / n
            * 100.0;

        println!(
            "{:>6.2}  {:>7.1}  {:>7.1}  {:>7.1}  {:>7.1}  {:>6.1}%  {:>7.1}  {:>7.1}  {:>6.2}%",
            d, avg_total, total_sd, avg_margin, margin_sd, ot_pct, avg_pace, pace_sd, zero_pts_pct
        );
    }

    println!("\n(Empirical targets: NCAA tournament 2010-2024 averages, approximate)");

    Ok(())
}
