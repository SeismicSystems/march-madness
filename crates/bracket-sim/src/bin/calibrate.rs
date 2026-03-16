use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::calibration_mm::{self, MmCalibrationConfig};
use bracket_sim::load_teams_for_year;
use clap::Parser;
use std::io;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use kalshi::orderbook;
use kalshi::rest::{self, KalshiRestClient};
use kalshi::team_names::{extract_team_name, load_team_name_map};
use kalshi::types::{MARKETS, TeamOrderbook};

fn parse_nonzero_usize(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|e| format!("{e}"))?;
    if n == 0 {
        return Err("value must be at least 1".to_string());
    }
    Ok(n)
}

#[derive(Parser, Debug)]
#[command(name = "calibrate")]
#[command(version = "1.0.0")]
#[command(about = "Calibrate goose values against Kalshi orderbooks")]
struct CalibrateArgs {
    /// Tournament year (determines bracket structure / Final Four pairings)
    #[arg(short = 'y', long, default_value_t = DEFAULT_YEAR)]
    year: u16,

    /// Path to combined teams CSV (overrides default JSON+KenPom loading)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output path for calibrated teams CSV (default: overwrite kenpom.csv)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Simulations per calibration iteration
    #[arg(short = 'n', long, default_value_t = 10000, value_parser = parse_nonzero_usize)]
    sims_per_iter: usize,

    /// Maximum calibration iterations
    #[arg(short = 'm', long, default_value_t = 100)]
    max_iter: usize,

    /// Initial learning rate for goose adjustments
    #[arg(short = 'l', long, default_value_t = 1.0)]
    learning_rate: f64,

    /// Learning rate decay: lr = base_lr / (1 + iter * decay)
    #[arg(short = 'd', long, default_value_t = 0.3)]
    decay: f64,

    /// Orderbook depth (levels per side)
    #[arg(long, default_value_t = 10)]
    depth: usize,

    /// Cache TTL in seconds
    #[arg(long, default_value_t = 21600)]
    cache_ttl: u64,

    /// Convergence threshold in dollars of total edge
    #[arg(long, default_value_t = 1.0)]
    edge_threshold: f64,

    /// Sleep between API requests in milliseconds
    #[arg(long, default_value_t = 300)]
    sleep_ms: u64,

    /// Max trades to display (omit for all, 0 for none)
    #[arg(long)]
    top_trades: Option<usize>,
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
    let season_dir = bracket_sim::season_dir(args.year);
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| season_dir.join("kenpom.csv"));

    info!(
        year = args.year,
        output = %output.display(),
        sims_per_iter = args.sims_per_iter,
        max_iter = args.max_iter,
        depth = args.depth,
        edge_threshold = format_args!("${:.2}", args.edge_threshold),
        "starting calibration"
    );

    // 1. Load teams
    let mut teams = load_teams_for_year(args.input.as_deref(), args.year)?;
    let team_names: Vec<String> = teams.iter().map(|t| t.team.clone()).collect();
    info!(teams = teams.len(), "loaded teams");

    // 2. Load team name map for Kalshi → canonical name resolution
    let name_map = load_team_name_map();

    // 3. Fetch markets and orderbooks per round
    let ttl = chrono::Duration::seconds(args.cache_ttl as i64);
    let mut client: Option<KalshiRestClient> = None;
    let mut all_team_orderbooks: Vec<TeamOrderbook> = Vec::new();

    for market_def in MARKETS {
        // Try market cache first (for the market list)
        let markets = match rest::load_cache(market_def, ttl) {
            Some(cached) => {
                info!(round = market_def.label, "using cached markets");
                cached.markets
            }
            None => {
                let c = client.get_or_insert_with(|| {
                    KalshiRestClient::new().expect("Failed to create Kalshi REST client")
                });
                info!(round = market_def.label, "fetching markets from Kalshi");
                let markets = c
                    .get_all_markets(market_def.event_ticker, args.sleep_ms)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                rest::save_cache(market_def, &markets)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                markets
            }
        };

        // Try orderbook cache
        let orderbooks = match rest::load_orderbook_cache(market_def, ttl) {
            Some(cached) => {
                info!(round = market_def.label, "using cached orderbooks");
                cached.orderbooks
            }
            None => {
                let c = client.get_or_insert_with(|| {
                    KalshiRestClient::new().expect("Failed to create Kalshi REST client")
                });
                info!(
                    round = market_def.label,
                    markets = markets.len(),
                    "fetching orderbooks from Kalshi"
                );
                let obs = c
                    .get_round_orderbooks(&markets, args.depth, args.sleep_ms)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                rest::save_orderbook_cache(market_def, &obs)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                obs
            }
        };

        // Resolve team names and build TeamOrderbook entries
        for (market, ob) in markets.iter().zip(orderbooks.into_iter()) {
            let raw_name = extract_team_name(market);
            let canonical = name_map.get(&raw_name).cloned().unwrap_or(raw_name.clone());

            // Only include teams that exist in our bracket data
            if !team_names.contains(&canonical) {
                debug!(
                    raw_name = %raw_name,
                    canonical = %canonical,
                    "skipping unknown team"
                );
                continue;
            }

            all_team_orderbooks.push(TeamOrderbook {
                team: canonical,
                round: market_def.round,
                ticker: market.ticker.clone(),
                orderbook: ob,
            });
        }
    }

    info!(
        orderbooks = all_team_orderbooks.len(),
        "loaded team orderbooks"
    );

    // 4. Run calibration
    let config = MmCalibrationConfig {
        max_iterations: args.max_iter,
        sims_per_iteration: args.sims_per_iter,
        edge_threshold: args.edge_threshold,
        base_learning_rate: args.learning_rate,
        decay_factor: args.decay,
        ..Default::default()
    };

    let result =
        calibration_mm::calibrate_mm(&mut teams, &all_team_orderbooks, &config, &bracket_config);

    // 5. Print results
    if result.converged {
        info!(
            iterations = result.iterations,
            total_edge = format_args!("${:.2}", result.final_total_edge),
            "converged"
        );
    } else {
        warn!(
            iterations = result.iterations,
            total_edge = format_args!("${:.2}", result.final_total_edge),
            "did not converge"
        );
    }

    // Edge summary by round
    orderbook::print_edge_summary(&result.final_edges, result.final_total_edge);

    // Profitable trades
    let mut trades = orderbook::all_trades(&result.final_edges);
    if let Some(n) = args.top_trades {
        trades.truncate(n);
    }
    orderbook::print_trade_log(&trades);

    // Goose values
    if !result.goose_values.is_empty() {
        let mut sorted: Vec<_> = result.goose_values.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        println!();
        println!("Top goose adjustments:");
        for (team, goose) in sorted.iter().take(20) {
            println!("  {:<20} {:+.2}", team, goose);
        }
    }

    // 6. Save calibrated teams
    bracket_sim::team::save_kenpom_csv(&teams, output.to_str().expect("Invalid output path"))?;
    info!(output = %output.display(), "saved calibrated teams");

    Ok(())
}
