use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use eyre::bail;
use tracing::{info, warn};

use bracket_sim::Team;
use bracket_sim::live_resolver::GameModelResolver;
use seismic_march_madness::redis_keys::*;
use seismic_march_madness::{
    GameState, Pool, TeamAdvanceResults, TournamentData, TournamentStatus, build_reach_probs,
    get_teams_in_bracket_order, kenpom_csv, parse_bracket_hex,
    run_multi_pool_simulations_with_resolver, run_team_advance_simulations_with_resolver,
    tournament_json,
};

#[derive(Parser, Debug)]
#[command(
    name = "march-madness-forecaster",
    about = "Continuously simulate tournament outcomes and compute per-pool win probabilities"
)]
struct Cli {
    /// Path to the tournament status JSON file (overrides Redis — only for one-shot mode).
    #[arg(long = "status")]
    status_file: Option<PathBuf>,

    /// Path to the tournament data JSON (team names in bracket order).
    /// If not specified, uses the embedded tournament data for the given year.
    #[arg(long)]
    tournament_file: Option<PathBuf>,

    /// Path to write the forecast output JSON (in addition to Redis).
    #[arg(long)]
    output_file: Option<PathBuf>,

    /// Number of Monte Carlo simulations per iteration.
    #[arg(long, default_value = "50000")]
    simulations: u32,

    /// Tournament year (for loading embedded team data).
    #[arg(long, default_value = "2026")]
    year: u16,

    /// Print per-team advance probabilities for each round and exit (one-shot).
    #[arg(long)]
    team_advance: bool,

    /// Run once and exit instead of looping forever.
    #[arg(long)]
    once: bool,
}

/// Immutable context shared across iterations.
struct Context {
    team_names: Vec<String>,
    team_map: HashMap<String, Team>,
    resolver: GameModelResolver,
    simulations: u32,
    output_file: Option<PathBuf>,
}

fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Connect to Redis.
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let client = redis::Client::open(url.as_str())?;
    let mut conn = client.get_connection()?;

    // Load tournament structure (immutable across iterations).
    let tournament: TournamentData = match &cli.tournament_file {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => TournamentData::embedded(cli.year),
    };
    let team_names = get_teams_in_bracket_order(&tournament);

    // Load team metrics for live game simulation (immutable).
    let tj = tournament_json(cli.year)
        .unwrap_or_else(|| panic!("no embedded tournament data for year {}", cli.year));
    let kp = kenpom_csv(cli.year)
        .unwrap_or_else(|| panic!("no embedded KenPom data for year {}", cli.year));
    let teams = bracket_sim::team::load_teams_from_json_str(tj, kp)?;
    let team_map: HashMap<String, Team> = teams.into_iter().map(|t| (t.team.clone(), t)).collect();
    let resolver = GameModelResolver::new(&team_names, &team_map, bracket_sim::DEFAULT_PACE_D);

    let ctx = Context {
        team_names,
        team_map,
        resolver,
        simulations: cli.simulations,
        output_file: cli.output_file,
    };

    // --team-advance mode: one-shot, print table and exit.
    if cli.team_advance {
        let status = load_status(&mut conn, &cli.status_file)?;
        let reach = match &status.team_reach_probabilities {
            Some(reach_map) => build_reach_probs(&ctx.team_names, reach_map),
            None => bail!("tournament status missing teamReachProbabilities — cannot simulate"),
        };
        let resolver_opt = resolver_for_status(&ctx, &status);
        let results = run_team_advance_simulations_with_resolver(
            &status,
            &reach,
            ctx.simulations,
            resolver_opt,
        );
        results.print_table(&ctx.team_names, |name| {
            ctx.team_map.get(name).map(|t| t.seed).unwrap_or(0)
        });
        return Ok(());
    }

    // Main loop: reload Redis state each iteration.
    let mut iteration = 0u64;
    loop {
        iteration += 1;
        let start = Instant::now();

        match run_iteration(&mut conn, &ctx, &cli.status_file, iteration) {
            Ok(()) => {
                let elapsed = start.elapsed();
                info!(iteration, elapsed_ms = elapsed.as_millis() as u64, "done");
            }
            Err(e) => {
                warn!(iteration, error = %e, "iteration failed, will retry");
            }
        }

        if cli.once {
            break;
        }

        // Sleep briefly so we don't spin if iterations are very fast.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

/// One full forecasting iteration: read state, simulate, write results.
fn run_iteration(
    conn: &mut redis::Connection,
    ctx: &Context,
    status_file: &Option<PathBuf>,
    iteration: u64,
) -> eyre::Result<()> {
    // ── Load tournament status ────────────────────────────────────────
    let status = load_status(conn, status_file)?;
    let reach = match &status.team_reach_probabilities {
        Some(reach_map) => build_reach_probs(&ctx.team_names, reach_map),
        None => bail!("tournament status missing teamReachProbabilities"),
    };
    let resolver_opt = resolver_for_status(ctx, &status);

    let undecided = status
        .games
        .iter()
        .filter(|g| g.status != GameState::Final)
        .count();
    let live = status
        .games
        .iter()
        .filter(|g| g.status == GameState::Live)
        .count();
    info!(
        iteration,
        decided = 63 - undecided,
        live,
        undecided,
        simulations = ctx.simulations,
        "loaded status"
    );

    // ── Read entries, groups, mirrors from Redis ──────────────────────
    let raw_entries: HashMap<String, String> = redis::Commands::hgetall(conn, KEY_ENTRIES)?;
    let mut entry_brackets: HashMap<String, u64> = HashMap::new();
    for (addr, json) in &raw_entries {
        if let Ok(entry) = serde_json::from_str::<EntryData>(json)
            && let Some(hex) = &entry.bracket
            && let Some(bits) = parse_bracket_hex(hex)
        {
            entry_brackets.insert(addr.clone(), bits);
        }
    }

    let raw_group_members: HashMap<String, String> =
        redis::Commands::hgetall(conn, KEY_GROUP_MEMBERS)?;
    let group_members: HashMap<String, Vec<String>> = raw_group_members
        .into_iter()
        .filter_map(|(id, json)| {
            serde_json::from_str::<Vec<String>>(&json)
                .ok()
                .map(|members| (id, members))
        })
        .collect();

    let raw_groups: HashMap<String, String> = redis::Commands::hgetall(conn, KEY_GROUPS)?;
    let group_slugs: HashMap<String, String> = raw_groups
        .iter()
        .filter_map(|(id, json)| {
            serde_json::from_str::<GroupData>(json)
                .ok()
                .map(|g| (id.clone(), g.slug))
        })
        .collect();

    let raw_mirror_entries: HashMap<String, String> =
        redis::Commands::hgetall(conn, KEY_MIRROR_ENTRIES)?;
    let mut mirror_entries: HashMap<String, Vec<(String, u64)>> = HashMap::new();
    for (composite_key, bracket_hex) in &raw_mirror_entries {
        if let Some((mirror_id, entry_slug)) = composite_key.split_once(':')
            && let Some(bits) = parse_bracket_hex(bracket_hex)
        {
            mirror_entries
                .entry(mirror_id.to_string())
                .or_default()
                .push((entry_slug.to_string(), bits));
        }
    }

    // ── Build deduped bracket list and pools ──────────────────────────
    let mut bracket_dedup: HashMap<u64, usize> = HashMap::new();
    let mut brackets: Vec<u64> = Vec::new();

    let mut get_or_insert = |bits: u64| -> usize {
        if let Some(&idx) = bracket_dedup.get(&bits) {
            idx
        } else {
            let idx = brackets.len();
            brackets.push(bits);
            bracket_dedup.insert(bits, idx);
            idx
        }
    };

    let mm_members: Vec<(String, usize)> = entry_brackets
        .iter()
        .map(|(addr, &bits)| (addr.clone(), get_or_insert(bits)))
        .collect();

    let mut pools = vec![Pool {
        key: "mm".to_string(),
        members: mm_members,
    }];

    for (group_id, members) in &group_members {
        let group_pool_members: Vec<(String, usize)> = members
            .iter()
            .filter_map(|addr| {
                entry_brackets
                    .get(addr)
                    .map(|&bits| (addr.clone(), get_or_insert(bits)))
            })
            .collect();
        if !group_pool_members.is_empty() {
            let slug = group_slugs.get(group_id).map(|s| s.as_str()).unwrap_or("?");
            info!(group_id, slug, members = group_pool_members.len(), "pool");
            pools.push(Pool {
                key: format!("group:{group_id}"),
                members: group_pool_members,
            });
        }
    }

    for (mirror_id, entries) in &mirror_entries {
        let mirror_pool_members: Vec<(String, usize)> = entries
            .iter()
            .map(|(slug, bits)| (slug.clone(), get_or_insert(*bits)))
            .collect();
        if !mirror_pool_members.is_empty() {
            info!(mirror_id, entries = mirror_pool_members.len(), "pool");
            pools.push(Pool {
                key: format!("mirror:{mirror_id}"),
                members: mirror_pool_members,
            });
        }
    }

    info!(
        pools = pools.len(),
        unique_brackets = brackets.len(),
        entries = entry_brackets.len(),
        "built pools"
    );

    if brackets.is_empty() {
        info!("no valid brackets found, writing empty forecast");
        write_forecasts(conn, &[], &[], &ctx.output_file)?;
        return Ok(());
    }

    // ── Run simulations ──────────────────────────────────────────────
    let results = run_multi_pool_simulations_with_resolver(
        &brackets,
        &pools,
        &status,
        &reach,
        ctx.simulations,
        resolver_opt,
    );

    let advance_results =
        run_team_advance_simulations_with_resolver(&status, &reach, ctx.simulations, resolver_opt);
    write_team_probs(conn, &ctx.team_names, &advance_results)?;

    // ── Convert to basis points and write ─────────────────────────────
    let pool_forecasts: Vec<BTreeMap<String, u32>> = pools
        .iter()
        .enumerate()
        .map(|(pi, pool)| {
            let mut map = BTreeMap::new();
            for (mi, (key, _)) in pool.members.iter().enumerate() {
                let wins = results.pool_wins[pi][mi];
                let bps =
                    (wins as u64 * 10000 + results.num_sims as u64 / 2) / results.num_sims as u64;
                map.insert(key.clone(), bps as u32);
            }
            map
        })
        .collect();

    write_forecasts(conn, &pools, &pool_forecasts, &ctx.output_file)?;

    // Log top-3 from main pool.
    if let Some(mm_forecast) = pool_forecasts.first() {
        let mut sorted: Vec<(&String, &u32)> = mm_forecast.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (addr, bps) in sorted.iter().take(3) {
            info!(
                addr = addr.as_str(),
                pct = format!("{:.2}%", **bps as f64 / 100.0),
                "top"
            );
        }
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────

fn load_status(
    conn: &mut redis::Connection,
    status_file: &Option<PathBuf>,
) -> eyre::Result<TournamentStatus> {
    if let Some(path) = status_file {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    } else {
        let json: Option<String> = redis::Commands::get(conn, KEY_GAMES)?;
        let json =
            json.ok_or_else(|| eyre::eyre!("no tournament status in Redis (key: {KEY_GAMES})"))?;
        Ok(serde_json::from_str(&json)?)
    }
}

fn resolver_for_status<'a>(
    ctx: &'a Context,
    status: &TournamentStatus,
) -> Option<&'a dyn seismic_march_madness::LiveGameResolver> {
    let live = status
        .games
        .iter()
        .filter(|g| g.status == GameState::Live)
        .count();
    if live > 0 { Some(&ctx.resolver) } else { None }
}

fn write_team_probs(
    conn: &mut redis::Connection,
    team_names: &[String],
    results: &TeamAdvanceResults,
) -> eyre::Result<()> {
    let sims = results.num_sims as f64;
    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.del(KEY_TEAM_PROBS);
    for (idx, name) in team_names.iter().enumerate() {
        let probs: Vec<f64> = results.advance[idx]
            .iter()
            .map(|&count| count as f64 / sims)
            .collect();
        let json = serde_json::to_string(&probs)?;
        pipe.hset(KEY_TEAM_PROBS, name, &json);
    }
    let () = pipe.query(conn)?;
    info!(teams = team_names.len(), "team probs written");
    Ok(())
}

fn write_forecasts(
    conn: &mut redis::Connection,
    pools: &[Pool],
    pool_forecasts: &[BTreeMap<String, u32>],
    output_file: &Option<PathBuf>,
) -> eyre::Result<()> {
    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.del(KEY_FORECASTS);
    for (pi, pool) in pools.iter().enumerate() {
        let json = serde_json::to_string(&pool_forecasts[pi])?;
        pipe.hset(KEY_FORECASTS, &pool.key, &json);
    }
    let () = pipe.query(conn)?;
    info!(pools = pools.len(), "forecasts written");

    if let Some(path) = output_file {
        let mut all: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        for (pi, pool) in pools.iter().enumerate() {
            all.insert(pool.key.clone(), pool_forecasts[pi].clone());
        }
        let json = serde_json::to_string_pretty(&all)?;
        std::fs::write(path, &json)?;
        info!(output = %path.display(), "forecasts written to file");
    }

    Ok(())
}
