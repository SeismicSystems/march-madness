use chrono::Duration;
use clap::{Args, Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use kalshi::auth::workspace_root;
use kalshi::fair_value::{compute_fair_value_nbbo, normalize_round, normalize_teams, parse_f64};
use kalshi::rest::{self, KalshiRestClient};
use kalshi::team_names::{extract_team_name, load_team_name_map};
use kalshi::types::{MARKETS, YEAR};
use kalshi::ws::KalshiWs;

#[derive(Parser, Debug)]
#[command(name = "kalshi")]
#[command(version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Fetch via REST API (one-shot, with caching)
    Fetch(FetchArgs),
    /// Stream NBBO via WebSocket and periodically write CSV
    Watch(WatchArgs),
}

#[derive(Args, Debug)]
struct FetchArgs {
    /// Cache TTL in seconds (0 = always refetch)
    #[arg(long, default_value_t = 21600)]
    cache_ttl: u64,

    /// Output CSV path (default: data/{YEAR}/targets_kalshi.csv, or targets_kalshi_raw.csv with --raw)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Only include teams found in this teams CSV (default: data/{YEAR}/bracket.csv)
    #[arg(short, long)]
    teams: Option<PathBuf>,

    /// Skip normalization (output raw Kalshi probabilities)
    #[arg(long)]
    raw: bool,

    /// Sleep between paginated API requests (milliseconds)
    #[arg(long, default_value_t = 300)]
    sleep_ms: u64,
}

#[derive(Args, Debug)]
struct WatchArgs {
    /// How often to write CSV (seconds)
    #[arg(long, default_value_t = 60)]
    interval: u64,

    /// Output CSV path (default: data/{YEAR}/targets_kalshi.csv)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Only include teams found in this teams CSV (default: data/{YEAR}/bracket.csv)
    #[arg(short, long)]
    teams: Option<PathBuf>,

    /// Skip normalization (output raw Kalshi probabilities)
    #[arg(long)]
    raw: bool,

    /// Sleep between paginated REST requests when discovering tickers (milliseconds)
    #[arg(long, default_value_t = 300)]
    sleep_ms: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("kalshi=info")),
        )
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch(args) => run_fetch(args),
        Commands::Watch(args) => tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_watch(args)),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn default_data_dir() -> PathBuf {
    workspace_root().join("data").join(YEAR.to_string())
}

fn load_known_teams_from(path: &Path) -> Option<HashSet<String>> {
    Some(
        rest::load_known_teams(path)
            .unwrap_or_else(|e| {
                warn!("couldn't load teams from {}: {}", path.display(), e);
                Vec::new()
            })
            .into_iter()
            .collect(),
    )
}

/// Post-process targets: backfill, normalize, enforce monotonicity.
fn postprocess_targets(
    targets: &mut Vec<(String, usize, f64)>,
    known_teams: &Option<HashSet<String>>,
) {
    if let Some(known) = known_teams {
        for mdef in MARKETS {
            let teams_in_round: std::collections::HashSet<String> = targets
                .iter()
                .filter(|(_, r, _)| *r == mdef.round)
                .map(|(t, _, _)| t.clone())
                .collect();
            let mut added = 0;
            for t in known {
                if !teams_in_round.contains(t) {
                    targets.push((t.clone(), mdef.round, mdef.floor_prob));
                    added += 1;
                }
            }
            if added > 0 {
                debug!(
                    "backfilled {} teams into {} (floor={:.6})",
                    added, mdef.label, mdef.floor_prob
                );
            }
        }
    }

    // Normalize -> monotonicity -> re-normalize (5 passes).
    for _pass in 0..5 {
        for mdef in MARKETS {
            let max_iters = 50;
            for iter in 0..max_iters {
                let round_sum: f64 = targets
                    .iter()
                    .filter(|(_, r, _)| *r == mdef.round)
                    .map(|(_, _, p)| *p)
                    .sum();
                if (round_sum - mdef.expected_sum).abs() < 1e-9 || round_sum == 0.0 {
                    break;
                }
                let scale = mdef.expected_sum / round_sum;
                if _pass == 0 && iter == 0 {
                    debug!(
                        "normalization {}: sum={:.4} -> {:.1} (scale={:.6})",
                        mdef.label, round_sum, mdef.expected_sum, scale
                    );
                }
                let mut clamped_excess = 0.0_f64;
                let mut unclamped_sum = 0.0_f64;
                for (_, r, p) in targets.iter_mut() {
                    if *r == mdef.round {
                        let scaled = *p * scale;
                        if scaled > 1.0 {
                            clamped_excess += scaled - 1.0;
                            *p = 1.0;
                        } else {
                            *p = scaled;
                            unclamped_sum += scaled;
                        }
                    }
                }
                if clamped_excess < 1e-12 {
                    break;
                }
                if unclamped_sum > 0.0 {
                    let boost = 1.0 + clamped_excess / unclamped_sum;
                    for (_, r, p) in targets.iter_mut() {
                        if *r == mdef.round && *p < 1.0 {
                            *p = (*p * boost).min(1.0);
                        }
                    }
                }
            }
        }

        // Enforce monotonicity
        let mut by_team: HashMap<String, Vec<(usize, f64)>> = HashMap::new();
        for (team, round, prob) in targets.iter() {
            by_team
                .entry(team.clone())
                .or_default()
                .push((*round, *prob));
        }
        for rounds in by_team.values_mut() {
            rounds.sort_by_key(|(r, _)| *r);
            for i in (0..rounds.len().saturating_sub(1)).rev() {
                if rounds[i].1 < rounds[i + 1].1 {
                    rounds[i].1 = rounds[i + 1].1;
                }
            }
        }
        targets.clear();
        for (team, rounds) in &by_team {
            for (round, prob) in rounds {
                if *prob > 0.0 {
                    targets.push((team.clone(), *round, *prob));
                }
            }
        }
    }
}

fn write_csv(
    targets: &[(String, usize, f64)],
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write to temp file, then rename for atomicity
    let tmp_path = output_path.with_extension("csv.tmp");
    {
        let mut writer: Box<dyn Write> = Box::new(std::fs::File::create(&tmp_path)?);
        writeln!(writer, "team,round,probability")?;
        for (team, round, prob) in targets {
            writeln!(writer, "{},{},{:.6}", team, round, prob)?;
        }
    }
    std::fs::rename(&tmp_path, output_path)?;
    Ok(())
}

fn print_summary(targets: &[(String, usize, f64)]) {
    eprintln!("\n=== Summary ===");
    let mut round_sums: HashMap<usize, (usize, f64)> = HashMap::new();
    for (_, round, prob) in targets {
        let entry = round_sums.entry(*round).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += prob;
    }
    for mdef in MARKETS {
        if let Some((count, sum)) = round_sums.get(&mdef.round) {
            eprintln!(
                "  Round {} ({}): {} teams, sum={:.4}",
                mdef.round, mdef.label, count, sum
            );
        }
    }

    let mut by_team: HashMap<String, Vec<(usize, f64)>> = HashMap::new();
    for (team, round, prob) in targets {
        by_team
            .entry(team.clone())
            .or_default()
            .push((*round, *prob));
    }
    let mut violations = 0;
    for (team, rounds) in &by_team {
        let mut sorted: Vec<_> = rounds.clone();
        sorted.sort_by_key(|(r, _)| *r);
        for window in sorted.windows(2) {
            let (r_early, p_early) = window[0];
            let (r_late, p_late) = window[1];
            if p_late > p_early + 1e-9 {
                eprintln!(
                    "  MONOTONICITY VIOLATION: {} R{} ({:.0}%) < R{} ({:.0}%)",
                    team,
                    r_early,
                    p_early * 100.0,
                    r_late,
                    p_late * 100.0
                );
                violations += 1;
            }
        }
    }

    if violations > 0 {
        eprintln!("\n  {} monotonicity violation(s) found!", violations);
    } else {
        eprintln!("\n  Monotonicity OK");
    }
}

// ---------------------------------------------------------------------------
// Fetch (REST) subcommand
// ---------------------------------------------------------------------------

fn run_fetch(args: FetchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let ttl = Duration::seconds(args.cache_ttl as i64);
    let name_map = load_team_name_map();
    let mut client: Option<KalshiRestClient> = None;

    let teams_path = args
        .teams
        .unwrap_or_else(|| default_data_dir().join("bracket.csv"));
    let known_teams = load_known_teams_from(&teams_path);

    let mut targets: Vec<(String, usize, f64)> = Vec::new();

    info!(
        "Fetching Kalshi March Madness futures (cache TTL: {}s)",
        args.cache_ttl
    );

    for mdef in MARKETS {
        let markets = if let Some(cached) = rest::load_cache(mdef, ttl) {
            cached.markets
        } else {
            if client.is_some() {
                std::thread::sleep(std::time::Duration::from_millis(args.sleep_ms));
            }
            if client.is_none() {
                client = Some(KalshiRestClient::new()?);
            }
            info!("Fetching {} ({})", mdef.label, mdef.event_ticker);
            let fetched = client
                .as_ref()
                .unwrap()
                .get_all_markets(mdef.event_ticker, args.sleep_ms)?;
            rest::save_cache(mdef, &fetched)?;
            fetched
        };

        let team_probs = normalize_round(&markets, mdef, &name_map, args.raw);

        for (name, p) in &team_probs {
            if let Some(ref known) = known_teams
                && !known.contains(name)
            {
                continue;
            }
            if *p > 0.0 {
                targets.push((name.clone(), mdef.round, *p));
            }
        }
    }

    if !args.raw {
        postprocess_targets(&mut targets, &known_teams);
    }

    targets.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let output_path = args.output.unwrap_or_else(|| {
        let filename = if args.raw {
            "targets_kalshi_raw.csv"
        } else {
            "targets_kalshi.csv"
        };
        default_data_dir().join(filename)
    });

    write_csv(&targets, &output_path)?;
    print_summary(&targets);
    eprintln!("\nWritten to {}", output_path.display());

    Ok(())
}

// ---------------------------------------------------------------------------
// Watch (WebSocket) subcommand
// ---------------------------------------------------------------------------

/// Per-market ticker state: which round/team it represents, latest NBBO.
struct TickerState {
    team_name: String,
    round: usize,
    bid: f64,
    ask: f64,
}

async fn run_watch(args: WatchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let name_map = load_team_name_map();
    let teams_path = args
        .teams
        .unwrap_or_else(|| default_data_dir().join("bracket.csv"));
    let known_teams = load_known_teams_from(&teams_path);
    let output_path = args
        .output
        .unwrap_or_else(|| default_data_dir().join("targets_kalshi.csv"));

    // Step 1: Discover all market tickers via REST
    info!("discovering market tickers via REST");
    let rest_client = KalshiRestClient::new()?;
    let mut ticker_state: HashMap<String, TickerState> = HashMap::new();
    let mut all_tickers: Vec<String> = Vec::new();

    for mdef in MARKETS {
        info!("fetching {} ({})", mdef.label, mdef.event_ticker);
        let markets = rest_client.get_all_markets(mdef.event_ticker, args.sleep_ms)?;

        for m in &markets {
            let raw_name = extract_team_name(m);
            let team_name = name_map.get(&raw_name).cloned().unwrap_or(raw_name);

            let bid = parse_f64(m.yes_bid_dollars.as_deref());
            let ask = parse_f64(m.yes_ask_dollars.as_deref());

            all_tickers.push(m.ticker.clone());
            ticker_state.insert(
                m.ticker.clone(),
                TickerState {
                    team_name,
                    round: mdef.round,
                    bid,
                    ask,
                },
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(args.sleep_ms));
    }

    info!(
        "discovered {} market tickers across {} rounds",
        all_tickers.len(),
        MARKETS.len()
    );

    // Write initial CSV from REST data
    write_snapshot(&ticker_state, &known_teams, &output_path, args.raw)?;

    // Step 2: Connect WebSocket and subscribe
    let auth = kalshi::auth::KalshiAuth::from_env()?;
    let mut ws = KalshiWs::connect(&auth).await?;
    ws.subscribe_ticker(&all_tickers).await?;

    info!(
        interval = args.interval,
        "streaming NBBO, writing CSV every {}s", args.interval
    );

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(args.interval));
    interval.tick().await; // first tick is immediate, skip it
    let mut updates_since_write = 0u64;

    loop {
        tokio::select! {
            result = ws.next_ticker() => {
                match result {
                    Some(Ok((ticker, bid, ask))) => {
                        if let Some(state) = ticker_state.get_mut(&ticker) {
                            state.bid = bid;
                            state.ask = ask;
                            updates_since_write += 1;
                        }
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "WebSocket error, reconnecting...");
                        let auth = kalshi::auth::KalshiAuth::from_env()?;
                        ws = KalshiWs::connect(&auth).await?;
                        ws.subscribe_ticker(&all_tickers).await?;
                    }
                    None => {
                        warn!("WebSocket closed, reconnecting...");
                        let auth = kalshi::auth::KalshiAuth::from_env()?;
                        ws = KalshiWs::connect(&auth).await?;
                        ws.subscribe_ticker(&all_tickers).await?;
                    }
                }
            }
            _ = interval.tick() => {
                if updates_since_write > 0 {
                    info!(updates = updates_since_write, "writing CSV snapshot");
                    write_snapshot(
                        &ticker_state,
                        &known_teams,
                        &output_path,
                        args.raw,
                    )?;
                    updates_since_write = 0;
                } else {
                    debug!("no updates since last write, skipping");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down");
                // Final write
                write_snapshot(
                    &ticker_state,
                    &known_teams,
                    &output_path,
                    args.raw,
                )?;
                break;
            }
        }
    }

    Ok(())
}

fn write_snapshot(
    ticker_state: &HashMap<String, TickerState>,
    known_teams: &Option<HashSet<String>>,
    output_path: &Path,
    raw: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut targets: Vec<(String, usize, f64)> = Vec::new();

    // Group tickers by round, compute fair values
    let mut by_round: HashMap<usize, Vec<(&str, f64, f64)>> = HashMap::new();
    for state in ticker_state.values() {
        by_round
            .entry(state.round)
            .or_default()
            .push((&state.team_name, state.bid, state.ask));
    }

    for mdef in MARKETS {
        if let Some(entries) = by_round.get(&mdef.round) {
            let mut bid_teams: Vec<(String, f64)> = Vec::new();
            let mut no_bid_teams: Vec<(String, f64)> = Vec::new();

            for (team_name, bid, ask) in entries {
                let ob = compute_fair_value_nbbo(*bid, *ask);
                if ob.has_bid {
                    bid_teams.push((team_name.to_string(), ob.fair_value));
                } else {
                    no_bid_teams.push((team_name.to_string(), ob.ask));
                }
            }

            let team_probs = normalize_teams(bid_teams, no_bid_teams, mdef, raw);

            for (name, p) in &team_probs {
                if let Some(known) = known_teams
                    && !known.contains(name)
                {
                    continue;
                }
                if *p > 0.0 {
                    targets.push((name.clone(), mdef.round, *p));
                }
            }
        }
    }

    if !raw {
        postprocess_targets(&mut targets, known_teams);
    }

    targets.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    write_csv(&targets, output_path)?;
    print_summary(&targets);
    eprintln!("\nWritten to {}", output_path.display());

    Ok(())
}
