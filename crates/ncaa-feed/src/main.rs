//! `ncaa-feed` — polls NCAA basketball scores and writes tournament status to Redis.

mod feed;
mod mapper;
mod writer;

use clap::Parser;
use eyre::{Context, Result};
use ncaa_api::{ContestDate, NcaaClient, SportCode, fetch_schedule, fetch_scoreboard};
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::DEFAULT_REDIS_URL;
use tracing::{error, info, warn};

use crate::feed::{FeedPhase, FeedState};
use crate::mapper::GameMapper;

#[derive(Parser)]
#[command(name = "ncaa-feed", about = "NCAA live score feed → Redis (mm:games)")]
struct Cli {
    /// Path to tournament.json (team names → bracket positions derived from array index).
    /// If not specified, uses embedded 2026 tournament data.
    #[arg(long)]
    tournament_file: Option<std::path::PathBuf>,

    /// Redis URL.
    #[arg(long, env = "REDIS_URL", default_value = DEFAULT_REDIS_URL)]
    redis_url: String,

    /// Max NCAA API requests per second (must be < 5.0).
    #[arg(long, default_value = "1.0", conflicts_with = "poll_interval")]
    requests_per_sec: f64,

    /// Fixed poll interval in seconds, minimum 1 (overrides adaptive polling and requests-per-sec).
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    poll_interval: Option<u64>,

    /// Sport: mbb (men's basketball) or wbb (women's basketball).
    #[arg(long, default_value = "mbb")]
    sport: String,

    /// Contest date in YYYY/MM/DD format. Auto-detected from schedule API if omitted.
    #[arg(long)]
    date: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let sport: SportCode = cli.sport.parse().map_err(|e: String| eyre::eyre!(e))?;
    let client = NcaaClient::new(cli.requests_per_sec).map_err(|e| eyre::eyre!("{e}"))?;

    // Connect to Redis.
    let redis_client = redis::Client::open(cli.redis_url.as_str())
        .wrap_err_with(|| format!("failed to create Redis client from {}", cli.redis_url))?;
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .wrap_err("failed to connect to Redis")?;
    info!("connected to Redis");

    // Load NCAA name → bracket position mappings.
    let mut mapper = match &cli.tournament_file {
        Some(path) => {
            info!("loading name mappings from {}", path.display());
            GameMapper::load(path)?
        }
        None => {
            info!("using embedded 2026 tournament data");
            GameMapper::load_embedded(2026)
        }
    };

    // Load existing tournament status from Redis to resume from (e.g. after restart).
    let existing_status = writer::read_tournament_status(&mut conn)
        .await
        .inspect_err(|e| warn!("failed to read existing status from Redis: {e}"))
        .ok()
        .flatten();

    if existing_status.is_some() {
        info!("resuming from existing status in Redis");
    }

    // Seed mapper with existing final results.
    if let Some(ref status) = existing_status {
        for game in &status.games {
            mapper.record_winner_from_game(game);
        }
    }

    let poll_override = cli.poll_interval.map(std::time::Duration::from_secs);
    let mut state = FeedState::new(
        cli.requests_per_sec,
        poll_override,
        existing_status.as_ref(),
    );

    // Determine contest date.
    let date = if let Some(d) = &cli.date {
        ContestDate::parse(d).map_err(|e| eyre::eyre!("{e}"))?
    } else {
        detect_today(&client, sport).await?
    };

    info!("polling {sport} scoreboard for date {date}");

    // Main poll loop.
    loop {
        match fetch_scoreboard(&client, sport, &date).await {
            Ok(contests) => {
                let changes = state.update_from_contests(&contests, &mut mapper);

                if changes > 0 || state.dirty {
                    info!("{changes} game(s) changed");
                    publish_status(&state, &mut conn).await;
                    state.mark_clean();
                }
            }
            Err(e) => {
                error!("scoreboard fetch failed: {e}");
            }
        }

        let (phase, interval) = state.poll_interval();
        match phase {
            FeedPhase::Complete => {
                info!("all 63 games are final — tournament complete!");
                publish_status(&state, &mut conn).await;
                break;
            }
            _ => {
                info!("phase: {phase:?}, next poll in {}s", interval.as_secs());
                tokio::time::sleep(interval).await;
            }
        }
    }

    Ok(())
}

/// Write tournament status to Redis.
async fn publish_status(state: &FeedState, conn: &mut MultiplexedConnection) {
    let status = state.to_tournament_status();
    if let Err(e) = writer::write_tournament_status(conn, &status).await {
        error!("failed to write status: {e}");
    }
}

/// Auto-detect today's date from the NCAA schedule API.
async fn detect_today(client: &NcaaClient, sport: SportCode) -> Result<ContestDate> {
    let now = chrono::Utc::now();
    let today = ContestDate::from_naive(now.date_naive());
    let season_year = today.season_year();

    let dates = fetch_schedule(client, sport, season_year)
        .await
        .wrap_err("failed to fetch schedule")?;

    if dates.contains(&today) {
        info!("auto-detected today's date: {today}");
        return Ok(today);
    }

    for date in &dates {
        if date.date() >= today.date() {
            info!("no games today, next game date: {date}");
            return Ok(date.clone());
        }
    }

    warn!("could not find a game date in schedule, using today: {today}");
    Ok(today)
}
