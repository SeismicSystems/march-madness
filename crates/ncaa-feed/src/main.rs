//! `ncaa-feed` — polls NCAA basketball scores and writes tournament-status.json.

mod feed;
mod mapper;
mod writer;

use std::path::{Path, PathBuf};

use chrono::Datelike;
use clap::Parser;
use eyre::{Context, Result};
use ncaa_api::{NcaaClient, SportCode, fetch_schedule, fetch_scoreboard};
use tracing::{error, info, warn};

use crate::feed::{FeedPhase, FeedState};
use crate::mapper::GameMapper;

#[derive(Parser)]
#[command(
    name = "ncaa-feed",
    about = "NCAA live score feed → tournament-status.json"
)]
struct Cli {
    /// Path to tournament.json (team data).
    #[arg(long, default_value = "data/2026/tournament.json")]
    tournament_file: PathBuf,

    /// Path to write tournament-status.json.
    #[arg(long, default_value = "data/tournament-status.json")]
    output_file: PathBuf,

    /// Max NCAA API requests per second (must be < 5.0).
    #[arg(long, default_value = "1.0")]
    requests_per_sec: f64,

    /// Sport: mbb (men's basketball) or wbb (women's basketball).
    #[arg(long, default_value = "mbb")]
    sport: String,

    /// Contest date in YYYY/MM/DD format. Auto-detected from schedule API if omitted.
    #[arg(long)]
    date: Option<String>,

    /// Optional: POST results to this server URL (e.g. http://localhost:3000/api/tournament-status).
    #[arg(long)]
    api_url: Option<String>,

    /// API key for server POST. Also reads TOURNAMENT_API_KEY env var.
    #[arg(long, env = "TOURNAMENT_API_KEY")]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let sport: SportCode = cli.sport.parse().map_err(|e: String| eyre::eyre!(e))?;

    let client = NcaaClient::new(cli.requests_per_sec).map_err(|e| eyre::eyre!("{e}"))?;

    // Load tournament data.
    let tournament_json = std::fs::read_to_string(&cli.tournament_file)
        .wrap_err_with(|| format!("failed to read {}", cli.tournament_file.display()))?;
    let tournament: seismic_march_madness::tournament::TournamentData =
        serde_json::from_str(&tournament_json).wrap_err("failed to parse tournament.json")?;

    info!(
        "loaded tournament: {} ({} teams)",
        tournament.name,
        tournament.teams.len()
    );

    let mut mapper = GameMapper::new(&tournament);

    // Load existing tournament status if present.
    let existing_status = if cli.output_file.exists() {
        match std::fs::read_to_string(&cli.output_file) {
            Ok(json) => {
                match serde_json::from_str::<seismic_march_madness::TournamentStatus>(&json) {
                    Ok(status) => {
                        info!(
                            "loaded existing tournament status from {}",
                            cli.output_file.display()
                        );
                        Some(status)
                    }
                    Err(e) => {
                        warn!("failed to parse existing status: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                warn!("failed to read existing status: {e}");
                None
            }
        }
    } else {
        None
    };

    // Seed mapper with existing final results.
    if let Some(ref status) = existing_status {
        for game in &status.games {
            mapper.record_winner_from_game(game);
        }
    }

    let mut state = FeedState::new(cli.requests_per_sec, existing_status.as_ref());

    // Determine contest date.
    let date = if let Some(d) = &cli.date {
        d.clone()
    } else {
        detect_today(&client, sport).await?
    };

    info!("polling {sport} scoreboard for date {date}");

    // Main poll loop.
    loop {
        match fetch_scoreboard(&client, sport, &date).await {
            Ok(contests) => {
                let tournament_games: Vec<_> = contests
                    .iter()
                    .filter(|c| c.teams.iter().any(|t| !t.seed.is_empty()))
                    .cloned()
                    .collect();

                let changes = state.update_from_contests(&tournament_games, &mut mapper);

                if changes > 0 || state.dirty {
                    info!("{changes} game(s) changed");
                    publish_status(
                        &state,
                        &cli.output_file,
                        cli.api_url.as_deref(),
                        cli.api_key.as_deref(),
                        client.http(),
                    )
                    .await;
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
                publish_status(
                    &state,
                    &cli.output_file,
                    cli.api_url.as_deref(),
                    cli.api_key.as_deref(),
                    client.http(),
                )
                .await;
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

/// Write tournament status to file and optionally POST to server.
async fn publish_status(
    state: &FeedState,
    output_file: &Path,
    api_url: Option<&str>,
    api_key: Option<&str>,
    http: &reqwest::Client,
) {
    let status = state.to_tournament_status();

    if let Err(e) = writer::write_tournament_status(output_file, &status) {
        error!("failed to write status: {e}");
    }

    if let (Some(url), Some(key)) = (api_url, api_key)
        && let Err(e) = writer::post_tournament_status(http, url, key, &status).await
    {
        error!("failed to POST status: {e}");
    }
}

/// Auto-detect today's date from the NCAA schedule API.
async fn detect_today(client: &NcaaClient, sport: SportCode) -> Result<String> {
    let now = chrono::Utc::now();
    let year = now.year();
    let month = now.month();
    let season_year = ncaa_api::scoreboard::season_year(year, month);

    let dates = fetch_schedule(client, sport, season_year)
        .await
        .wrap_err("failed to fetch schedule")?;

    let today = now.format("%Y/%m/%d").to_string();

    // Find today's date in the schedule.
    if dates.contains(&today) {
        info!("auto-detected today's date: {today}");
        return Ok(today);
    }

    // Find the next upcoming date (lexicographic compare works for YYYY/MM/DD format).
    for date in &dates {
        if date.as_str() >= today.as_str() {
            info!("no games today, next game date: {date}");
            return Ok(date.clone());
        }
    }

    // Fall back to today.
    warn!("could not find a game date in schedule, using today: {today}");
    Ok(today)
}
