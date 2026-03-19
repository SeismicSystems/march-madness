use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use clap::Parser;
use eyre::bail;
use tracing::info;

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
    about = "Simulate tournament outcomes and compute per-pool win probabilities"
)]
struct Cli {
    /// Path to the tournament status JSON file (overrides Redis).
    #[arg(long = "status")]
    status_file: Option<PathBuf>,

    /// Path to the tournament data JSON (team names in bracket order).
    /// If not specified, uses the embedded tournament data for the given year.
    #[arg(long)]
    tournament_file: Option<PathBuf>,

    /// Path to write the forecast output JSON (in addition to Redis).
    #[arg(long)]
    output_file: Option<PathBuf>,

    /// Number of Monte Carlo simulations to run.
    #[arg(long, default_value = "50000")]
    simulations: u32,

    /// Tournament year (for loading embedded team data).
    #[arg(long, default_value = "2026")]
    year: u16,

    /// Print per-team advance probabilities for each round (no entries needed).
    #[arg(long)]
    team_advance: bool,
}

fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Connect to Redis.
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let client = redis::Client::open(url.as_str())?;
    let mut conn = client.get_connection()?;

    // Load tournament status: from file override or Redis.
    let status: TournamentStatus = if let Some(path) = &cli.status_file {
        info!("reading tournament status from {}", path.display());
        serde_json::from_str(&std::fs::read_to_string(path)?)?
    } else {
        info!("reading tournament status from Redis");
        let json: Option<String> = redis::Commands::get(&mut conn, KEY_GAMES)?;
        let json =
            json.ok_or_else(|| eyre::eyre!("no tournament status in Redis (key: {KEY_GAMES})"))?;
        serde_json::from_str(&json)?
    };

    let tournament: TournamentData = match &cli.tournament_file {
        Some(path) => serde_json::from_str(&std::fs::read_to_string(path)?)?,
        None => TournamentData::embedded(cli.year),
    };

    // Build team names in bracket order.
    let team_names = get_teams_in_bracket_order(&tournament);

    // Load team metrics for live game simulation.
    let tj = tournament_json(cli.year)
        .unwrap_or_else(|| panic!("no embedded tournament data for year {}", cli.year));
    let kp = kenpom_csv(cli.year)
        .unwrap_or_else(|| panic!("no embedded KenPom data for year {}", cli.year));
    let teams = bracket_sim::team::load_teams_from_json_str(tj, kp)?;
    let team_map: HashMap<String, Team> = teams.into_iter().map(|t| (t.team.clone(), t)).collect();

    // Build resolver for live games.
    let live_count = status
        .games
        .iter()
        .filter(|g| g.status == GameState::Live)
        .count();
    let resolver = GameModelResolver::new(&team_names, &team_map, bracket_sim::DEFAULT_PACE_D);
    let resolver_opt: Option<&dyn seismic_march_madness::LiveGameResolver> = if live_count > 0 {
        info!(
            live_games = live_count,
            "using game model resolver for live games"
        );
        Some(&resolver)
    } else {
        None
    };

    // Build reach probabilities.
    let reach = match &status.team_reach_probabilities {
        Some(reach_map) => build_reach_probs(&team_names, reach_map),
        None => bail!("tournament status missing teamReachProbabilities — cannot simulate"),
    };

    let undecided_count = status
        .games
        .iter()
        .filter(|g| g.status != GameState::Final)
        .count();
    info!(
        games = status.games.len(),
        decided = 63 - undecided_count,
        undecided = undecided_count,
        simulations = cli.simulations,
        "loaded tournament data"
    );

    // --team-advance mode: print per-team advance probabilities and exit.
    if cli.team_advance {
        let results = run_team_advance_simulations_with_resolver(
            &status,
            &reach,
            cli.simulations,
            resolver_opt,
        );
        results.print_table(&team_names, |name| {
            team_map.get(name).map(|t| t.seed).unwrap_or(0)
        });
        return Ok(());
    }

    // ── Read all data from Redis ──────────────────────────────────────

    // 1. Entries: address → EntryData
    info!("reading entries from Redis");
    let raw_entries: HashMap<String, String> = redis::Commands::hgetall(&mut conn, KEY_ENTRIES)?;
    let mut entry_brackets: HashMap<String, u64> = HashMap::new();
    for (addr, json) in &raw_entries {
        if let Ok(entry) = serde_json::from_str::<EntryData>(json)
            && let Some(hex) = &entry.bracket
            && let Some(bits) = parse_bracket_hex(hex)
        {
            entry_brackets.insert(addr.clone(), bits);
        }
    }
    info!(
        total = raw_entries.len(),
        valid = entry_brackets.len(),
        "entries loaded"
    );

    // 2. Group members: groupId → [addresses]
    let raw_group_members: HashMap<String, String> =
        redis::Commands::hgetall(&mut conn, KEY_GROUP_MEMBERS)?;
    let group_members: HashMap<String, Vec<String>> = raw_group_members
        .into_iter()
        .filter_map(|(id, json)| {
            serde_json::from_str::<Vec<String>>(&json)
                .ok()
                .map(|members| (id, members))
        })
        .collect();

    // 3. Groups metadata (for logging).
    let raw_groups: HashMap<String, String> = redis::Commands::hgetall(&mut conn, KEY_GROUPS)?;
    let group_slugs: HashMap<String, String> = raw_groups
        .iter()
        .filter_map(|(id, json)| {
            serde_json::from_str::<GroupData>(json)
                .ok()
                .map(|g| (id.clone(), g.slug))
        })
        .collect();

    // 4. Mirror entries: "mirrorId:entrySlug" → bracket_hex
    let raw_mirror_entries: HashMap<String, String> =
        redis::Commands::hgetall(&mut conn, KEY_MIRROR_ENTRIES)?;
    // Parse into mirrorId → [(slug, bracket_bits)]
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

    // Main pool.
    let mm_members: Vec<(String, usize)> = entry_brackets
        .iter()
        .map(|(addr, &bits)| (addr.clone(), get_or_insert(bits)))
        .collect();

    let mut pools = vec![Pool {
        key: "mm".to_string(),
        members: mm_members,
    }];

    // Per-group pools.
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
            info!(
                group_id,
                slug,
                members = group_pool_members.len(),
                "group pool"
            );
            pools.push(Pool {
                key: format!("group:{group_id}"),
                members: group_pool_members,
            });
        }
    }

    // Per-mirror pools.
    for (mirror_id, entries) in &mirror_entries {
        let mirror_pool_members: Vec<(String, usize)> = entries
            .iter()
            .map(|(slug, bits)| (slug.clone(), get_or_insert(*bits)))
            .collect();
        if !mirror_pool_members.is_empty() {
            info!(
                mirror_id,
                entries = mirror_pool_members.len(),
                "mirror pool"
            );
            pools.push(Pool {
                key: format!("mirror:{mirror_id}"),
                members: mirror_pool_members,
            });
        }
    }

    info!(
        pools = pools.len(),
        unique_brackets = brackets.len(),
        "built pools"
    );

    if brackets.is_empty() {
        info!("no valid brackets found, writing empty forecast");
        write_forecasts(&mut conn, &[], &[], &cli.output_file)?;
        return Ok(());
    }

    // ── Run simulations ───────────────────────────────────────────────

    let results = run_multi_pool_simulations_with_resolver(
        &brackets,
        &pools,
        &status,
        &reach,
        cli.simulations,
        resolver_opt,
    );

    // Also run team advance simulations and write to mm:probs.
    let advance_results =
        run_team_advance_simulations_with_resolver(&status, &reach, cli.simulations, resolver_opt);
    write_team_probs(&mut conn, &team_names, &advance_results)?;

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

    write_forecasts(&mut conn, &pools, &pool_forecasts, &cli.output_file)?;

    // Print summary for main pool.
    if let Some(mm_forecast) = pool_forecasts.first() {
        let mut sorted: Vec<(&String, &u32)> = mm_forecast.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        println!("\n--- Main Pool Forecast (top 20) ---");
        println!("{:<44} {:>8}", "Address", "P(Win)");
        for (addr, bps) in sorted.iter().take(20) {
            println!("{:<44} {:>7.2}%", addr, **bps as f64 / 100.0);
        }
    }

    // Print pool summary.
    for (pi, pool) in pools.iter().enumerate().skip(1) {
        let forecast = &pool_forecasts[pi];
        let mut sorted: Vec<(&String, &u32)> = forecast.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        println!("\n--- {} (top 5) ---", pool.key);
        for (key, bps) in sorted.iter().take(5) {
            println!("  {:<40} {:>7.2}%", key, **bps as f64 / 100.0);
        }
    }

    Ok(())
}

/// Write per-team advance probabilities to Redis HASH `mm:probs`.
/// Each field is a team name → JSON array of 6 probabilities [R64, R32, S16, E8, F4, Champ].
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
    info!(teams = team_names.len(), "team probs written to Redis");
    Ok(())
}

/// Write forecasts to Redis HASH and optionally to a file.
fn write_forecasts(
    conn: &mut redis::Connection,
    pools: &[Pool],
    pool_forecasts: &[BTreeMap<String, u32>],
    output_file: &Option<PathBuf>,
) -> eyre::Result<()> {
    // Delete old key and write all fields atomically via pipeline.
    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.del(KEY_FORECASTS);
    for (pi, pool) in pools.iter().enumerate() {
        let json = serde_json::to_string(&pool_forecasts[pi])?;
        pipe.hset(KEY_FORECASTS, &pool.key, &json);
    }
    let () = pipe.query(conn)?;
    info!(pools = pools.len(), "forecasts written to Redis");

    // Optionally write to file.
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
