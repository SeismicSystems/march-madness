//! Compute tournament results bytes8 from the NCAA bracket API.
//!
//! Fetches the completed bracket from the NCAA API, determines all 63 game
//! outcomes, and encodes them into the bytes8 format used by the MarchMadness
//! contract's `submitResults(bytes8)` function.
//!
//! The output hex can be piped directly into `cast send`.

use std::collections::HashMap;

use clap::Parser;
use eyre::{Context, Result, ensure};
use ncaa_api::{BracketGame, Championship, NcaaClient};
use seismic_march_madness::scoring::{SENTINEL_BIT, score_bracket};
use tracing::{info, warn};

fn current_year() -> i32 {
    chrono::Utc::now().date_naive().year()
}

use chrono::Datelike;

#[derive(Parser)]
#[command(about = "Compute tournament results bytes8 from the NCAA bracket API")]
struct Args {
    /// Tournament year (defaults to current year).
    #[arg(long, default_value_t = current_year())]
    year: i32,

    /// Fetch the women's tournament bracket instead of men's.
    #[arg(long)]
    women: bool,

    /// NCAA division number.
    #[arg(long, default_value = "1")]
    division: u32,

    /// Max requests per second to the NCAA API.
    #[arg(long, default_value = "1.0")]
    requests_per_sec: f64,

    /// Print detailed per-game breakdown.
    #[arg(long)]
    verbose: bool,

    /// Verify by scoring against a known bracket hex (e.g. all-chalk).
    #[arg(long)]
    verify_against: Option<String>,
}

impl Args {
    fn sport_url(&self) -> &'static str {
        if self.women {
            "basketball-women"
        } else {
            "basketball-men"
        }
    }
}

/// Round name for display.
fn round_name(game_index: u8) -> &'static str {
    match game_index {
        0..=31 => "R64",
        32..=47 => "R32",
        48..=55 => "S16",
        56..=59 => "E8",
        60..=61 => "F4",
        62 => "Championship",
        _ => "???",
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .init();

    let args = Args::parse();
    let sport = args.sport_url();

    let client = NcaaClient::new(args.requests_per_sec).wrap_err("failed to create NCAA client")?;

    info!(
        "fetching bracket for {sport} d{} {}",
        args.division, args.year
    );
    let champ = ncaa_api::fetch_bracket(&client, sport, args.division, args.year)
        .await
        .wrap_err("failed to fetch bracket")?;

    let (results_hex, results_u64) = compute_results(&champ, &args)?;

    // Output the hex (primary output — can be captured by scripts).
    println!("{results_hex}");

    // Verification against a known bracket.
    if let Some(ref verify_hex) = args.verify_against {
        let bracket = seismic_march_madness::scoring::parse_bracket_hex(verify_hex)
            .ok_or_else(|| eyre::eyre!("invalid bracket hex: {verify_hex}"))?;
        let score = score_bracket(bracket, results_u64);
        eprintln!("\nVerification: score({verify_hex}, {results_hex}) = {score}/192");
    }

    Ok(())
}

/// Compute results bytes8 from a completed NCAA bracket.
///
/// Returns (hex_string, u64_bits).
fn compute_results(champ: &Championship, args: &Args) -> Result<(String, u64)> {
    // 1. Determine region order (same logic as fetch-bracket).
    let region_order = champ
        .bracket_region_order()
        .map_err(|e| eyre::eyre!("{e}"))?;

    let region_names: Vec<String> = region_order
        .iter()
        .map(|sid| {
            champ
                .region_for_section(*sid)
                .map(|r| r.name())
                .unwrap_or_else(|| format!("Section {sid}"))
        })
        .collect();

    // 2. Collect R64 bracket_position_ids in game_index order.
    let mut r64_bracket_ids: Vec<u32> = Vec::with_capacity(32);
    for section_id in &region_order {
        let r64_games = champ.r64_games(*section_id);
        ensure!(
            r64_games.len() == 8,
            "expected 8 R64 games for section {section_id}, got {}",
            r64_games.len()
        );
        for game in &r64_games {
            r64_bracket_ids.push(game.bracket_position_id);
        }
    }

    // 3. Build bracket_ids for all 63 games (same algorithm as fetch-bracket).
    let games_by_bid: HashMap<u32, &BracketGame> = champ
        .games
        .iter()
        .map(|g| (g.bracket_position_id, g))
        .collect();

    let mut bracket_ids: Vec<u32> = r64_bracket_ids.clone();
    let mut prev_round_ids = r64_bracket_ids;
    while prev_round_ids.len() > 1 {
        let next_round_ids: Vec<u32> = prev_round_ids
            .chunks(2)
            .map(|pair| {
                let bid = pair[0];
                let game = games_by_bid
                    .get(&bid)
                    .ok_or_else(|| eyre::eyre!("bracket game {bid} not found"))?;
                game.victor_bracket_position_id
                    .ok_or_else(|| eyre::eyre!("game {bid} has no victorBracketPositionId"))
            })
            .collect::<Result<_>>()?;
        bracket_ids.extend_from_slice(&next_round_ids);
        prev_round_ids = next_round_ids;
    }
    ensure!(
        bracket_ids.len() == 63,
        "expected 63 bracket IDs, got {}",
        bracket_ids.len()
    );

    // 4. For each game, determine who won (team1 = is_top team).
    let mut results: u64 = SENTINEL_BIT;
    let mut final_count = 0u8;
    let mut undecided = Vec::new();

    for (game_index, &bid) in bracket_ids.iter().enumerate() {
        let game = games_by_bid.get(&bid).ok_or_else(|| {
            eyre::eyre!("bracket game {bid} not found for game_index {game_index}")
        })?;

        // Find the winning team.
        let winners: Vec<_> = game.teams.iter().filter(|t| t.is_winner).collect();

        if winners.len() != 1 {
            if game.game_state == "FINAL" || game.game_state == "final" {
                warn!(
                    "game {game_index} (bid {bid}) is FINAL but has {} winners",
                    winners.len()
                );
            }
            undecided.push(game_index as u8);
            continue;
        }

        let winner = winners[0];
        final_count += 1;

        // team1 = is_top. If the winning team is_top, set the bit.
        let team1_won = winner.is_top;
        if team1_won {
            results |= 1u64 << game_index;
        }

        if args.verbose {
            let round = round_name(game_index as u8);
            let region_idx = match game_index {
                0..=7 => Some(0),
                8..=15 => Some(1),
                16..=23 => Some(2),
                24..=31 => Some(3),
                _ => None,
            };
            let region_label = region_idx.map(|i| region_names[i].as_str()).unwrap_or("");

            let loser: Option<&ncaa_api::BracketTeam> =
                game.teams.iter().find(|t| t.seed.is_some() && !t.is_winner);

            eprintln!(
                "  game {:>2} [{:<12}] {:<5} {:>2} {:<20} def. {:>2} {:<20}  (bit={}, team1_won={})",
                game_index,
                round,
                region_label,
                winner.seed.unwrap_or(0),
                winner.name_short,
                loser.and_then(|t| t.seed).unwrap_or(0),
                loser.map(|t| t.name_short.as_str()).unwrap_or("?"),
                if team1_won { 1 } else { 0 },
                team1_won,
            );
        }
    }

    if !undecided.is_empty() {
        eprintln!(
            "\nWARNING: {} game(s) have no winner yet: {:?}",
            undecided.len(),
            undecided
        );
        eprintln!("The tournament may not be complete. Results will be partial.");
    }

    eprintln!("\n{final_count}/63 games decided");

    let hex = format!("0x{:016x}", results);

    // Self-score sanity check.
    let self_score = score_bracket(results, results);
    eprintln!("Results: {hex}  (self-score: {self_score}/192)");

    if self_score != 192 {
        warn!("self-score is not 192 — this should never happen for valid results");
    }

    // Print Final Four / Championship summary.
    if final_count >= 60 {
        eprintln!();
        for gi in [60u8, 61, 62] {
            let bid = bracket_ids[gi as usize];
            if let Some(game) = games_by_bid.get(&bid)
                && let Some(winner) = game.teams.iter().find(|t| t.is_winner)
            {
                let round = round_name(gi);
                eprintln!(
                    "  {round}: {} ({})",
                    winner.name_short,
                    winner.seed.unwrap_or(0)
                );
            }
        }
    }

    Ok((hex, results))
}
