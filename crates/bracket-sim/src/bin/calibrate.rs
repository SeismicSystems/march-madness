use bracket_sim::Tournament;
use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::calibration::{self, CalibrationConfig};
use clap::Parser;
use std::io;
use std::path::PathBuf;
use tracing::{debug, info, trace, warn};

fn parse_nonzero_usize(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|e| format!("{e}"))?;
    if n == 0 {
        return Err("value must be at least 1".to_string());
    }
    Ok(n)
}

#[derive(Parser, Debug)]
#[command(name = "calibrate")]
#[command(version = "0.1.0")]
#[command(about = "Calibrate goose values to match target probabilities")]
struct CalibrateArgs {
    /// Tournament year (determines bracket structure / Final Four pairings)
    #[arg(short = 'y', long, default_value_t = DEFAULT_YEAR)]
    year: u16,

    /// Path to teams CSV (default: data/teams_{year}.csv)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Path to target odds CSV (default: data/{year}/targets_kalshi.csv)
    #[arg(short, long)]
    targets: Option<PathBuf>,

    /// Output path for calibrated teams CSV (default: overwrite kenpom.csv)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Simulations per calibration iteration
    #[arg(short = 'n', long, default_value_t = 10000, value_parser = parse_nonzero_usize)]
    sims_per_iter: usize,

    /// Maximum calibration iterations
    #[arg(short = 'm', long, default_value_t = 100)]
    max_iter: usize,

    /// Credible interval level for convergence (e.g. 0.99 = 99% CI)
    #[arg(short = 'c', long, default_value_t = 0.99)]
    credible_level: f64,

    /// Initial learning rate for goose adjustments
    #[arg(short = 'l', long, default_value_t = 1.0)]
    learning_rate: f64,

    /// Learning rate decay: lr = base_lr / (1 + iter * decay)
    #[arg(short = 'd', long, default_value_t = 0.3)]
    decay: f64,

    /// Renormalize target probabilities per bracket group (use when regions are approximate).
    /// Optional tolerance in percentage points: --renorm 5 only renorms groups within +/-5% of 100%,
    /// errors on groups outside that range. No value = renorm unconditionally.
    #[arg(long, num_args = 0..=1, default_missing_value = "100")]
    renorm: Option<f64>,
}

fn default_data_dir() -> PathBuf {
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
    let season_dir = data.join(year.to_string());
    let bracket = season_dir.join("bracket.csv");
    let kenpom = season_dir.join("kenpom.csv");
    Tournament::load_teams(path_to_str(&bracket)?, path_to_str(&kenpom)?)
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

    let args = CalibrateArgs::parse();
    let bracket_config = BracketConfig::for_year(args.year);
    let season_dir = default_data_dir().join(args.year.to_string());
    let targets_path = args
        .targets
        .unwrap_or_else(|| season_dir.join("targets_kalshi.csv"));
    let output = args.output.unwrap_or_else(|| season_dir.join("kenpom.csv"));

    info!(
        year = args.year,
        targets = %targets_path.display(),
        output = %output.display(),
        sims_per_iter = args.sims_per_iter,
        max_iter = args.max_iter,
        credible_level = format_args!("{:.0}%", args.credible_level * 100.0),
        "starting calibration"
    );

    let mut teams = load_teams(args.input, args.year)?;
    let mut targets =
        calibration::load_targets_from_csv(targets_path.to_str().expect("Invalid targets path"))?;

    if let Some(tolerance) = args.renorm {
        info!(tolerance, "renormalizing targets to bracket groups");
        calibration::renormalize_targets(&mut targets, &teams, &bracket_config, tolerance / 100.0);
    }

    if args.renorm.is_none() {
        let (errors, warnings) = calibration::validate_targets(&targets, &teams, &bracket_config);
        for w in &warnings {
            warn!("{}", w);
        }
        if !errors.is_empty() {
            for e in &errors {
                tracing::error!("{}", e);
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Target validation failed with {} error(s)", errors.len()),
            ));
        }
    }

    debug!(teams = teams.len(), targets = targets.len(), "loaded data");
    for t in &targets {
        trace!(
            team = %t.team,
            round = t.round,
            probability = format_args!("{:.1}%", t.probability * 100.0),
        );
    }

    let config = CalibrationConfig {
        max_iterations: args.max_iter,
        sims_per_iteration: args.sims_per_iter,
        credible_level: args.credible_level,
        base_learning_rate: args.learning_rate,
        decay_factor: args.decay,
        ..Default::default()
    };

    let result = calibration::calibrate(&mut teams, &targets, &config, &bracket_config);

    calibration::print_calibration_table(&result.final_errors);

    if result.converged {
        info!(iterations = result.iterations, "converged");
    } else {
        warn!(iterations = result.iterations, "did not converge");
    }

    if !result.goose_values.is_empty() {
        let mut sorted: Vec<_> = result.goose_values.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (team, goose) in sorted {
            debug!(team, goose = format_args!("{:+.2}", goose));
        }
    }

    for (team, round, target, observed) in &result.final_errors {
        trace!(
            team = %team,
            round,
            target = format_args!("{:.1}%", target * 100.0),
            observed = format_args!("{:.1}%", observed * 100.0),
            error = format_args!("{:+.1}%", (target - observed) * 100.0),
            "final error"
        );
    }

    bracket_sim::team::save_kenpom_csv(&teams, output.to_str().expect("Invalid output path"))?;
    info!(output = %output.display(), "saved calibrated teams");

    Ok(())
}
