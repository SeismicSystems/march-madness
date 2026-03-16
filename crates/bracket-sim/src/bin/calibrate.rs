use bracket_sim::bracket_config::{BracketConfig, DEFAULT_YEAR};
use bracket_sim::calibration_mm::{self, MmCalibrationConfig};
use bracket_sim::load_teams_for_year;
use clap::Parser;
use std::io;
use std::path::PathBuf;
use tracing::{info, warn};

use kalshi::orderbook;
use kalshi::rest::{self, KalshiRestClient};
use kalshi::team_names::{extract_team_name, load_team_name_map};
use kalshi::types::{MARKETS, Orderbook, TeamOrderbook};

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

    // 1. Load teams and First Four mapping from tournament.json
    let mut teams = load_teams_for_year(args.input.as_deref(), args.year)?;
    let tournament_json_path = season_dir.join("tournament.json");
    let ff_to_slot = bracket_sim::team::build_first_four_map(&tournament_json_path)?;
    info!(
        teams = teams.len(),
        first_four = ff_to_slot.len() / 2,
        "loaded teams"
    );

    // Build team name lookup: maps canonical names to bracket slot names.
    // Individual FF names map to their slot name, but FF slots are excluded
    // from calibration (see below).
    let team_names: Vec<String> = teams.iter().map(|t| t.team.clone()).collect();
    let mut name_to_slot: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for name in &team_names {
        name_to_slot.insert(name.clone(), name.clone());
    }
    for (individual, slot) in &ff_to_slot {
        name_to_slot.insert(individual.clone(), slot.clone());
    }

    // First Four slot names (e.g. "Texas/NC State"). Kalshi has separate markets
    // for each individual FF team, not a joint market for the slot. Including them
    // would produce nonsense URLs and incorrect edge signals, so we skip them.
    let ff_slot_names: std::collections::HashSet<String> = ff_to_slot.values().cloned().collect();

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

        // Filter to only tournament teams before fetching orderbooks.
        // Also exclude First Four teams — Kalshi has separate individual markets
        // for each FF team, not a joint market for the bracket slot.
        let total_markets = markets.len();
        let mut ff_skipped = 0usize;
        let markets: Vec<_> = markets
            .into_iter()
            .filter(|m| {
                let raw_name = extract_team_name(m);
                let canonical = name_map.get(&raw_name).cloned().unwrap_or(raw_name);
                match name_to_slot.get(&canonical) {
                    Some(slot) if ff_slot_names.contains(slot) => {
                        ff_skipped += 1;
                        false
                    }
                    Some(_) => true,
                    None => false,
                }
            })
            .collect();
        let non_tournament = total_markets - markets.len() - ff_skipped;
        info!(
            round = market_def.label,
            kept = markets.len(),
            ff_skipped,
            non_tournament,
            "filtered markets"
        );

        // Build ticker set for filtered markets
        let market_tickers: std::collections::HashSet<String> =
            markets.iter().map(|m| m.ticker.clone()).collect();

        // Try orderbook cache, filtering to tournament teams
        let orderbooks = match rest::load_orderbook_cache(market_def, ttl) {
            Some(cached) => {
                // Cache may contain all teams — filter to only tournament tickers
                let obs: Vec<_> = cached
                    .orderbooks
                    .into_iter()
                    .filter(|ob| market_tickers.contains(&ob.ticker))
                    .collect();
                info!(
                    round = market_def.label,
                    count = obs.len(),
                    "using cached orderbooks (filtered)"
                );
                obs
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

        // Build ticker → orderbook lookup for matching (handles both cached and fresh)
        let ob_by_ticker: std::collections::HashMap<String, Orderbook> = orderbooks
            .into_iter()
            .map(|ob| (ob.ticker.clone(), ob))
            .collect();

        // Resolve team names and build TeamOrderbook entries.
        // FF teams were already filtered out above, but guard here too for safety.
        for market in &markets {
            let raw_name = extract_team_name(market);
            let canonical = name_map.get(&raw_name).cloned().unwrap_or(raw_name.clone());
            let slot_name = name_to_slot[&canonical].clone();

            if ff_slot_names.contains(&slot_name) {
                // Should not happen (filtered above), but guard anyway.
                continue;
            }

            let ob = match ob_by_ticker.get(&market.ticker) {
                Some(ob) => ob.clone(),
                None => {
                    warn!(ticker = %market.ticker, "missing orderbook");
                    continue;
                }
            };

            all_team_orderbooks.push(TeamOrderbook {
                team: slot_name,
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

    // 6. Save calibrated goose values back to kenpom.csv (preserving individual team metrics)
    let kenpom_path = season_dir.join("kenpom.csv");
    bracket_sim::team::save_kenpom_csv_with_goose(
        &teams,
        kenpom_path.to_str().expect("Invalid kenpom path"),
        output.to_str().expect("Invalid output path"),
        &ff_to_slot,
    )?;
    info!(output = %output.display(), "saved calibrated teams");

    Ok(())
}
