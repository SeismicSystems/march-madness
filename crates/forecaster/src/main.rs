use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use bracket_sim::bracket_config::BracketConfig;
use bracket_sim::live_resolver::GameModelResolver;
use bracket_sim::team::load_teams_from_json_str;
use bracket_sim::{Team, Tournament};
use clap::Parser;
use eyre::{Result, bail, eyre};
use tracing::{info, warn};

use seismic_march_madness::redis_keys::*;
use seismic_march_madness::{
    GameState, GameStatus, Pool, ReachProbs, TeamAdvanceResults, TournamentData, TournamentStatus,
    build_reach_probs, get_teams_in_bracket_order, kenpom_csv, parse_bracket_hex,
    run_multi_pool_simulations_with_resolver, run_team_advance_simulations_with_resolver,
    tournament_json,
};

#[derive(Parser, Debug)]
#[command(
    name = "march-madness-forecaster",
    about = "Continuously simulate tournament outcomes and compute per-pool win probabilities"
)]
struct Cli {
    /// Path to the tournament data JSON (team names in bracket order).
    /// If not specified, uses the embedded tournament data for the given year.
    #[arg(long)]
    tournament_file: Option<PathBuf>,

    /// Path to write the forecast output JSON (in addition to Redis).
    #[arg(long)]
    output_file: Option<PathBuf>,

    /// Number of Monte Carlo simulations per iteration.
    #[arg(long, default_value = "50000", value_parser = clap::value_parser!(u32).range(1..))]
    simulations: u32,

    /// Tournament year (for loading embedded team data).
    #[arg(long, default_value = "2026")]
    year: u16,

    /// Ignore current game state and simulate from pre-tournament probabilities.
    #[arg(long)]
    pre_lock: bool,

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
    pre_lock: Option<PreLockContext>,
}

struct PreLockContext {
    status: TournamentStatus,
    reach: ReachProbs,
}

struct ForecastInputs {
    entry_brackets: HashMap<String, u64>,
    group_members: HashMap<String, Vec<String>>,
    group_slugs: HashMap<String, String>,
    mirror_entries: HashMap<String, Vec<(String, u64)>>,
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let client = redis::Client::open(url.as_str())?;
    let mut conn = client.get_connection()?;

    let ctx = build_context(&cli)?;

    if cli.team_advance {
        let (status, reach, resolver_opt) = load_simulation_state(&mut conn, &ctx)?;
        let results = run_team_advance_simulations_with_resolver(
            &status,
            &reach,
            ctx.simulations,
            resolver_opt,
        );
        results.print_table(&ctx.team_names, |name| {
            ctx.team_map.get(name).map(|team| team.seed).unwrap_or(0)
        });
        return Ok(());
    }

    let mut iteration = 0u64;
    loop {
        iteration += 1;
        let start = Instant::now();

        match run_iteration(&mut conn, &ctx, iteration) {
            Ok(()) => {
                let elapsed = start.elapsed();
                info!(iteration, elapsed_ms = elapsed.as_millis() as u64, "done");
            }
            Err(error) => {
                warn!(iteration, error = %error, "iteration failed, will retry");
            }
        }

        if cli.once {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn build_context(cli: &Cli) -> Result<Context> {
    let tournament_json = load_tournament_json(cli)?;
    let tournament: TournamentData = serde_json::from_str(&tournament_json)?;
    let team_names = get_teams_in_bracket_order(&tournament);

    let kenpom = kenpom_csv(cli.year)
        .ok_or_else(|| eyre!("no embedded KenPom data for year {}", cli.year))?;
    let teams = load_teams_from_json_str(&tournament_json, kenpom)?;
    let team_map: HashMap<String, Team> = teams
        .iter()
        .cloned()
        .map(|team| (team.team.clone(), team))
        .collect();
    let resolver = GameModelResolver::new(&team_names, &team_map, bracket_sim::DEFAULT_PACE_D);

    let pre_lock = if cli.pre_lock {
        Some(build_pre_lock_context(
            cli.year,
            cli.simulations,
            &team_names,
            &teams,
        ))
    } else {
        None
    };

    Ok(Context {
        team_names,
        team_map,
        resolver,
        simulations: cli.simulations,
        output_file: cli.output_file.clone(),
        pre_lock,
    })
}

fn build_pre_lock_context(
    year: u16,
    simulations: u32,
    team_names: &[String],
    teams: &[Team],
) -> PreLockContext {
    let bracket_config = BracketConfig::for_year(year);
    let mut tournament = Tournament::new().with_pace_d(bracket_sim::DEFAULT_PACE_D);
    tournament.setup_tournament(teams.to_vec(), &bracket_config);
    let reach_map = tournament.cumulative_win_probabilities(simulations as usize);

    PreLockContext {
        status: pre_lock_status(),
        reach: build_reach_probs(team_names, &reach_map),
    }
}

fn load_tournament_json(cli: &Cli) -> Result<String> {
    match &cli.tournament_file {
        Some(path) => Ok(std::fs::read_to_string(path)?),
        None => Ok(tournament_json(cli.year)
            .ok_or_else(|| eyre!("no embedded tournament data for year {}", cli.year))?
            .to_string()),
    }
}

fn pre_lock_status() -> TournamentStatus {
    TournamentStatus {
        games: (0..63).map(GameStatus::upcoming).collect(),
        team_reach_probabilities: None,
        updated_at: None,
    }
}

fn run_iteration(conn: &mut redis::Connection, ctx: &Context, iteration: u64) -> Result<()> {
    let (status, reach, resolver_opt) = load_simulation_state(conn, ctx)?;
    let undecided = status
        .games
        .iter()
        .filter(|game| game.status != GameState::Final)
        .count();
    let live = status
        .games
        .iter()
        .filter(|game| game.status == GameState::Live)
        .count();
    let mode = if ctx.pre_lock.is_some() {
        "pre-lock"
    } else {
        "live"
    };
    info!(
        iteration,
        mode,
        decided = 63 - undecided,
        live,
        undecided,
        simulations = ctx.simulations,
        "loaded simulation state"
    );

    let inputs = load_forecast_inputs(conn)?;
    let (brackets, pools) = build_brackets_and_pools(&inputs);
    info!(
        pools = pools.len(),
        unique_brackets = brackets.len(),
        entries = inputs.entry_brackets.len(),
        "built pools"
    );

    let advance_results =
        run_team_advance_simulations_with_resolver(&status, &reach, ctx.simulations, resolver_opt);
    write_team_probs(conn, &ctx.team_names, &advance_results)?;

    if brackets.is_empty() {
        let empty_forecasts: Vec<BTreeMap<String, u32>> =
            pools.iter().map(|_| BTreeMap::new()).collect();
        info!("no valid brackets found, writing empty forecasts");
        write_forecasts(conn, &pools, &empty_forecasts, &ctx.output_file)?;
        return Ok(());
    }

    let results = run_multi_pool_simulations_with_resolver(
        &brackets,
        &pools,
        &status,
        &reach,
        ctx.simulations,
        resolver_opt,
    );

    let pool_forecasts: Vec<BTreeMap<String, u32>> = pools
        .iter()
        .enumerate()
        .map(|(pool_idx, pool)| {
            pool.member_keys
                .iter()
                .enumerate()
                .map(|(member_idx, member_key)| {
                    let wins = results.pool_wins[pool_idx][member_idx];
                    (
                        member_key.clone(),
                        wins_to_basis_points(wins, results.num_sims),
                    )
                })
                .collect()
        })
        .collect();

    write_forecasts(conn, &pools, &pool_forecasts, &ctx.output_file)?;

    if let Some(main_pool) = pool_forecasts.first() {
        let mut sorted: Vec<(&String, &u32)> = main_pool.iter().collect();
        sorted.sort_by(|left, right| right.1.cmp(left.1));
        for (address, bps) in sorted.iter().take(3) {
            info!(
                addr = address.as_str(),
                pct = format!("{:.2}%", **bps as f64 / 100.0),
                "top"
            );
        }
    }

    Ok(())
}

fn load_simulation_state<'a>(
    conn: &mut redis::Connection,
    ctx: &'a Context,
) -> Result<(
    TournamentStatus,
    ReachProbs,
    Option<&'a dyn seismic_march_madness::LiveGameResolver>,
)> {
    if let Some(pre_lock) = &ctx.pre_lock {
        return Ok((pre_lock.status.clone(), pre_lock.reach.clone(), None));
    }

    let status = load_status(conn)?;
    let reach = match &status.team_reach_probabilities {
        Some(reach_map) => build_reach_probs(&ctx.team_names, reach_map),
        None => bail!("tournament status missing teamReachProbabilities"),
    };
    let resolver_opt = resolver_for_status(ctx, &status);

    Ok((status, reach, resolver_opt))
}

fn load_forecast_inputs(conn: &mut redis::Connection) -> Result<ForecastInputs> {
    let raw_entries: HashMap<String, String> = redis::Commands::hgetall(conn, KEY_ENTRIES)?;
    let mut entry_brackets = HashMap::new();
    for (address, json) in &raw_entries {
        match serde_json::from_str::<EntryData>(json) {
            Ok(entry) => {
                if let Some(bracket_hex) = &entry.bracket
                    && let Some(bits) = parse_bracket_hex(bracket_hex)
                {
                    entry_brackets.insert(address.clone(), bits);
                }
            }
            Err(error) => warn!(address, error = %error, "skipping corrupt entry"),
        }
    }

    let raw_group_members: HashMap<String, String> =
        redis::Commands::hgetall(conn, KEY_GROUP_MEMBERS)?;
    let group_members = raw_group_members
        .into_iter()
        .filter_map(
            |(group_id, json)| match serde_json::from_str::<Vec<String>>(&json) {
                Ok(members) => Some((group_id, members)),
                Err(error) => {
                    warn!(group_id, error = %error, "skipping corrupt group member list");
                    None
                }
            },
        )
        .collect();

    let raw_groups: HashMap<String, String> = redis::Commands::hgetall(conn, KEY_GROUPS)?;
    let group_slugs = raw_groups
        .iter()
        .filter_map(
            |(group_id, json)| match serde_json::from_str::<GroupData>(json) {
                Ok(group) => Some((group_id.clone(), group.slug)),
                Err(error) => {
                    warn!(group_id, error = %error, "skipping corrupt group metadata");
                    None
                }
            },
        )
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

    Ok(ForecastInputs {
        entry_brackets,
        group_members,
        group_slugs,
        mirror_entries,
    })
}

fn build_brackets_and_pools(inputs: &ForecastInputs) -> (Vec<u64>, Vec<Pool>) {
    let mut deduped_indices = HashMap::new();
    let mut brackets = Vec::new();

    let mut get_or_insert = |bits: u64| -> usize {
        if let Some(&index) = deduped_indices.get(&bits) {
            index
        } else {
            let index = brackets.len();
            brackets.push(bits);
            deduped_indices.insert(bits, index);
            index
        }
    };

    let mm_members = inputs
        .entry_brackets
        .iter()
        .map(|(address, &bits)| (address.clone(), get_or_insert(bits)))
        .collect();

    let mut pools = vec![make_pool("mm", mm_members)];

    for (group_id, members) in &inputs.group_members {
        let pool_members: Vec<(String, usize)> = members
            .iter()
            .filter_map(|address| {
                inputs
                    .entry_brackets
                    .get(address)
                    .map(|&bits| (address.clone(), get_or_insert(bits)))
            })
            .collect();
        if !pool_members.is_empty() {
            let slug = inputs
                .group_slugs
                .get(group_id)
                .map(String::as_str)
                .unwrap_or("?");
            info!(group_id, slug, members = pool_members.len(), "pool");
            pools.push(make_pool(format!("group:{group_id}"), pool_members));
        }
    }

    for (mirror_id, entries) in &inputs.mirror_entries {
        let pool_members: Vec<(String, usize)> = entries
            .iter()
            .map(|(entry_slug, bits)| (entry_slug.clone(), get_or_insert(*bits)))
            .collect();
        if !pool_members.is_empty() {
            info!(mirror_id, entries = pool_members.len(), "pool");
            pools.push(make_pool(format!("mirror:{mirror_id}"), pool_members));
        }
    }

    (brackets, pools)
}

fn make_pool(key: impl Into<String>, members: Vec<(String, usize)>) -> Pool {
    let (member_keys, bracket_indices): (Vec<_>, Vec<_>) = members.into_iter().unzip();
    Pool {
        key: key.into(),
        member_keys,
        bracket_indices,
    }
}

fn wins_to_basis_points(wins: u32, num_sims: u32) -> u32 {
    ((wins as u64 * 10_000 + num_sims as u64 / 2) / num_sims as u64) as u32
}

fn load_status(conn: &mut redis::Connection) -> Result<TournamentStatus> {
    let json: Option<String> = redis::Commands::get(conn, KEY_GAMES)?;
    let json = json.ok_or_else(|| eyre!("no tournament status in Redis (key: {KEY_GAMES})"))?;
    Ok(serde_json::from_str(&json)?)
}

fn resolver_for_status<'a>(
    ctx: &'a Context,
    status: &TournamentStatus,
) -> Option<&'a dyn seismic_march_madness::LiveGameResolver> {
    let live = status
        .games
        .iter()
        .filter(|game| game.status == GameState::Live)
        .count();
    if live > 0 { Some(&ctx.resolver) } else { None }
}

fn write_team_probs(
    conn: &mut redis::Connection,
    team_names: &[String],
    results: &TeamAdvanceResults,
) -> Result<()> {
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
) -> Result<()> {
    let mut pipe = redis::pipe();
    pipe.atomic();
    pipe.del(KEY_FORECASTS);
    for (pool_idx, pool) in pools.iter().enumerate() {
        let json = serde_json::to_string(&pool_forecasts[pool_idx])?;
        pipe.hset(KEY_FORECASTS, &pool.key, &json);
    }
    let () = pipe.query(conn)?;
    info!(pools = pools.len(), "forecasts written");

    if let Some(path) = output_file {
        let mut all = BTreeMap::new();
        for (pool_idx, pool) in pools.iter().enumerate() {
            all.insert(pool.key.clone(), pool_forecasts[pool_idx].clone());
        }
        let json = serde_json::to_string_pretty(&all)?;
        std::fs::write(path, &json)?;
        info!(output = %path.display(), "forecasts written to file");
    }

    Ok(())
}
