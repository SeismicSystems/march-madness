use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use bracket_sim::bracket_config::BracketConfig;
use bracket_sim::team::load_teams_from_json_str;
use bracket_sim::{Team, Tournament};
use clap::Parser;
use eyre::{Result, eyre};
use rayon::prelude::*;
use serde::Serialize;
use tracing::{info, warn};

use seismic_march_madness::redis_keys::*;
use seismic_march_madness::{
    GameState, GameStatus, MultiPoolResults, Pool, ROUND_SIZES, ROUND_STARTS, TeamAdvanceResults,
    TournamentData, TournamentStatus, get_teams_in_bracket_order, kenpom_csv, parse_bracket_hex,
    reverse_game_bits, tournament_json,
};

/// Rich forecast for a single entry, serialized to Redis/JSON.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ForecastEntry {
    expected_score: f64,
    win_probability: f64,
}

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
    #[arg(long, default_value = "100000", value_parser = clap::value_parser!(u32).range(1..))]
    simulations: u32,

    /// Tournament year (for loading embedded team data).
    #[arg(long, default_value = "2026")]
    year: u16,

    /// KenPom-style Bayesian postgame metric adjustment factor.
    #[arg(short = 'u', long, default_value_t = bracket_sim::DEFAULT_KENPOM_UPDATE_FACTOR)]
    kenpom_update_factor: f64,

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
    /// Full tournament model — cloned per simulation trial.
    tournament: Tournament,
    simulations: u32,
    output_file: Option<PathBuf>,
    /// When true, ignore Redis game state and always use all-upcoming status.
    force_pre_lock: bool,
    kenpom_update_factor: f64,
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
        let status = all_upcoming_status();
        let results = run_team_advance(&ctx.tournament, &status, ctx.simulations);
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
    let tournament_data: TournamentData = serde_json::from_str(&tournament_json)?;
    let team_names = get_teams_in_bracket_order(&tournament_data);

    let kenpom = kenpom_csv(cli.year)
        .ok_or_else(|| eyre!("no embedded KenPom data for year {}", cli.year))?;
    let teams = load_teams_from_json_str(&tournament_json, kenpom)?;
    let team_map: HashMap<String, Team> = teams
        .iter()
        .cloned()
        .map(|team| (team.team.clone(), team))
        .collect();

    let bracket_config = BracketConfig::for_year(cli.year);
    let mut tournament = Tournament::new()
        .with_pace_d(bracket_sim::DEFAULT_PACE_D)
        .with_kenpom_update_factor(cli.kenpom_update_factor);
    tournament.setup_tournament(teams.to_vec(), &bracket_config);

    Ok(Context {
        team_names,
        team_map,
        tournament,
        simulations: cli.simulations,
        output_file: cli.output_file.clone(),
        force_pre_lock: cli.pre_lock,
        kenpom_update_factor: cli.kenpom_update_factor,
    })
}

fn load_tournament_json(cli: &Cli) -> Result<String> {
    match &cli.tournament_file {
        Some(path) => Ok(std::fs::read_to_string(path)?),
        None => Ok(tournament_json(cli.year)
            .ok_or_else(|| eyre!("no embedded tournament data for year {}", cli.year))?
            .to_string()),
    }
}

fn all_upcoming_status() -> TournamentStatus {
    TournamentStatus {
        games: (0..63).map(GameStatus::upcoming).collect(),
        updated_at: None,
    }
}

fn run_iteration(conn: &mut redis::Connection, ctx: &Context, iteration: u64) -> Result<()> {
    let status = if ctx.force_pre_lock {
        all_upcoming_status()
    } else {
        load_status(conn)?
    };
    let decided = status
        .games
        .iter()
        .filter(|game| game.status == GameState::Final)
        .count();
    let live = status
        .games
        .iter()
        .filter(|game| game.status == GameState::Live)
        .count();
    info!(
        iteration,
        decided,
        live,
        upcoming = 63 - decided - live,
        simulations = ctx.simulations,
        kenpom_update_factor = ctx.kenpom_update_factor,
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

    let advance_results = run_team_advance(&ctx.tournament, &status, ctx.simulations);
    write_team_probs(conn, &ctx.team_names, &advance_results)?;

    if brackets.is_empty() {
        let empty_forecasts: Vec<BTreeMap<String, ForecastEntry>> =
            pools.iter().map(|_| BTreeMap::new()).collect();
        info!("no valid brackets found, writing empty forecasts");
        write_forecasts(conn, &pools, &empty_forecasts, &ctx.output_file)?;
        return Ok(());
    }

    let multi_pool_results =
        run_multi_pool(&brackets, &pools, &ctx.tournament, &status, ctx.simulations);

    let num_sims = multi_pool_results.num_sims as f64;
    let pool_forecasts: Vec<BTreeMap<String, ForecastEntry>> = pools
        .iter()
        .enumerate()
        .map(|(pool_idx, pool)| {
            pool.members
                .iter()
                .enumerate()
                .map(|(member_idx, (member_key, _))| {
                    let wins = multi_pool_results.pool_wins[pool_idx][member_idx];
                    let score_sum = multi_pool_results.score_sums[pool_idx][member_idx];
                    (
                        member_key.clone(),
                        ForecastEntry {
                            expected_score: score_sum as f64 / num_sims,
                            win_probability: wins as f64 / num_sims,
                        },
                    )
                })
                .collect()
        })
        .collect();

    write_forecasts(conn, &pools, &pool_forecasts, &ctx.output_file)?;

    if let Some(main_pool) = pool_forecasts.first() {
        let mut sorted: Vec<(&String, &ForecastEntry)> = main_pool.iter().collect();
        sorted.sort_by(|left, right| {
            right
                .1
                .win_probability
                .partial_cmp(&left.1.win_probability)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (address, entry) in sorted.iter().take(3) {
            info!(
                addr = address.as_str(),
                pct = format!("{:.2}%", entry.win_probability * 100.0),
                expected_score = format!("{:.1}", entry.expected_score),
                "top"
            );
        }
    }

    Ok(())
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
                    entry_brackets.insert(address.clone(), reverse_game_bits(bits));
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
                .push((entry_slug.to_string(), reverse_game_bits(bits)));
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
    Pool {
        key: key.into(),
        members,
    }
}

fn load_status(conn: &mut redis::Connection) -> Result<TournamentStatus> {
    let json: Option<String> = redis::Commands::get(conn, KEY_GAMES)?;
    let json = json.ok_or_else(|| eyre!("no tournament status in Redis (key: {KEY_GAMES})"))?;
    Ok(serde_json::from_str(&json)?)
}

// ── Simulation (single codepath: full Bayesian game model) ─────────

fn accumulate_pool_results(
    pool: &Pool,
    scores: &[u32],
    pool_wins: &mut [Vec<f64>],
    score_sums: &mut [Vec<u64>],
    pool_idx: usize,
) {
    let mut best = 0u32;
    let mut count_at_max = 0usize;
    for &(_, bracket_idx) in &pool.members {
        let s = scores[bracket_idx];
        if s > best {
            best = s;
            count_at_max = 1;
        } else if s == best {
            count_at_max += 1;
        }
    }

    let win_share = 1.0 / count_at_max as f64;
    for (member_idx, &(_, bracket_idx)) in pool.members.iter().enumerate() {
        let s = scores[bracket_idx];
        score_sums[pool_idx][member_idx] += s as u64;
        if s == best {
            pool_wins[pool_idx][member_idx] += win_share;
        }
    }
}

/// Run multi-pool simulations using `simulate_tournament_bb_with_status`.
///
/// Every trial clones the tournament, runs the full NB/Poisson simulation
/// with Bayesian metric updates, and respects decided/live/upcoming game
/// states from `status`. This is the same model the oddsmaker uses.
fn run_multi_pool(
    brackets: &[u64],
    pools: &[Pool],
    tournament: &Tournament,
    status: &TournamentStatus,
    num_sims: u32,
) -> MultiPoolResults {
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = (num_sims as usize).div_ceil(num_threads);

    let chunks: Vec<u32> = (0..num_threads)
        .map(|i| {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(num_sims as usize);
            if start >= num_sims as usize {
                0
            } else {
                (end - start) as u32
            }
        })
        .filter(|&n| n > 0)
        .collect();

    #[allow(clippy::type_complexity)]
    let partial_results: Vec<(Vec<Vec<f64>>, Vec<Vec<u64>>)> = chunks
        .par_iter()
        .map(|&chunk_sims| {
            let mut rng = rand::rng();
            let mut pool_wins: Vec<Vec<f64>> = pools
                .iter()
                .map(|p| vec![0.0f64; p.members.len()])
                .collect();
            let mut score_sums: Vec<Vec<u64>> =
                pools.iter().map(|p| vec![0u64; p.members.len()]).collect();

            for _ in 0..chunk_sims {
                let mut tourn = tournament.clone();
                let results =
                    reverse_game_bits(tourn.simulate_tournament_bb_with_status(status, &mut rng));

                let mask = seismic_march_madness::get_scoring_mask(results);
                let scores: Vec<u32> = brackets
                    .iter()
                    .map(|&b| seismic_march_madness::score_bracket_with_mask(b, results, mask))
                    .collect();

                for (pool_idx, pool) in pools.iter().enumerate() {
                    accumulate_pool_results(
                        pool,
                        &scores,
                        &mut pool_wins,
                        &mut score_sums,
                        pool_idx,
                    );
                }
            }

            (pool_wins, score_sums)
        })
        .collect();

    // Merge partial results.
    let mut pool_wins: Vec<Vec<f64>> = pools
        .iter()
        .map(|p| vec![0.0f64; p.members.len()])
        .collect();
    let mut score_sums: Vec<Vec<u64>> = pools.iter().map(|p| vec![0u64; p.members.len()]).collect();
    for (pw, ss) in &partial_results {
        for (pi, pp) in pw.iter().enumerate() {
            for (mi, &c) in pp.iter().enumerate() {
                pool_wins[pi][mi] += c;
            }
        }
        for (pi, pp) in ss.iter().enumerate() {
            for (mi, &s) in pp.iter().enumerate() {
                score_sums[pi][mi] += s;
            }
        }
    }

    MultiPoolResults {
        pool_wins,
        score_sums,
        num_sims,
    }
}

/// Run team advance simulations using the full game model with status.
fn run_team_advance(
    tournament: &Tournament,
    status: &TournamentStatus,
    num_sims: u32,
) -> TeamAdvanceResults {
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = (num_sims as usize).div_ceil(num_threads);

    let chunks: Vec<u32> = (0..num_threads)
        .map(|i| {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(num_sims as usize);
            if start >= num_sims as usize {
                0
            } else {
                (end - start) as u32
            }
        })
        .filter(|&n| n > 0)
        .collect();

    let partials: Vec<Vec<[u32; 6]>> = chunks
        .par_iter()
        .map(|&chunk_sims| {
            let mut rng = rand::rng();
            let mut advance = vec![[0u32; 6]; 64];

            for _ in 0..chunk_sims {
                let mut tourn = tournament.clone();
                let results = tourn.simulate_tournament_bb_with_status(status, &mut rng);

                // Extract per-game winners from the results bits.
                let mut game_winner: [usize; 63] = [usize::MAX; 63];
                for round in 0..6 {
                    let start = ROUND_STARTS[round];
                    let count = ROUND_SIZES[round];
                    for i in 0..count {
                        let g = start + i;
                        let (t1, t2) = if round == 0 {
                            (2 * g, 2 * g + 1)
                        } else {
                            let offset = g - start;
                            let prev_start = ROUND_STARTS[round - 1];
                            (
                                game_winner[prev_start + 2 * offset],
                                game_winner[prev_start + 2 * offset + 1],
                            )
                        };
                        let bit_pos = 62 - g;
                        let team1_wins = (results >> bit_pos) & 1 == 1;
                        let winner = if team1_wins { t1 } else { t2 };
                        game_winner[g] = winner;
                        advance[winner][round] += 1;
                    }
                }
            }

            advance
        })
        .collect();

    let mut advance = vec![[0u32; 6]; 64];
    for partial in &partials {
        for (team, rounds) in partial.iter().enumerate() {
            for (r, &count) in rounds.iter().enumerate() {
                advance[team][r] += count;
            }
        }
    }

    TeamAdvanceResults { advance, num_sims }
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
    pool_forecasts: &[BTreeMap<String, ForecastEntry>],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_ties_split_fractionally() {
        let pool = Pool {
            key: "mm".to_string(),
            members: vec![
                ("a".to_string(), 0),
                ("b".to_string(), 1),
                ("c".to_string(), 2),
            ],
        };
        let scores = vec![10, 10, 5];
        let mut pool_wins = vec![vec![0.0; 3]];
        let mut score_sums = vec![vec![0u64; 3]];

        accumulate_pool_results(&pool, &scores, &mut pool_wins, &mut score_sums, 0);

        assert_eq!(pool_wins[0], vec![0.5, 0.5, 0.0]);
        assert_eq!(score_sums[0], vec![10, 10, 5]);
    }
}
