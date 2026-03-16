//! Fetch the NCAA tournament bracket and write `tournament.json`.
//!
//! Queries the NCAA bracket API for the official tournament bracket, then writes:
//! - `data/{year}/men/tournament.json` (or `women/`) — team list with seeds and regions
//!
//! Teams are ordered in ByteBracket bracket order: 4 regions (F4 pairings determine
//! order), 16 teams per region in seed order [1,16,8,9,5,12,4,13,6,11,3,14,7,10,2,15].
//! The array index IS the bracket position (0-63).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::Parser;
use eyre::{Context, Result, bail, ensure};
use ncaa_api::{BracketGame, Championship, NcaaClient};
use serde::Serialize;
use tracing::info;

/// Current year from system time.
fn current_year() -> i32 {
    chrono::Utc::now().date_naive().year()
}

use chrono::Datelike;

#[derive(Parser)]
#[command(about = "Fetch NCAA bracket and write tournament.json")]
struct Args {
    /// Tournament year (defaults to current year).
    #[arg(long, default_value_t = current_year())]
    year: i32,

    /// Output directory (default: data/{year}/men/ or data/{year}/women/).
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Fetch the women's tournament bracket instead of men's.
    #[arg(long)]
    women: bool,

    /// NCAA division number.
    #[arg(long, default_value = "1")]
    division: u32,

    /// Max requests per second to the NCAA API.
    #[arg(long, default_value = "1.0")]
    requests_per_sec: f64,

    /// Print the bracket to stdout without writing files.
    #[arg(long)]
    dry_run: bool,
}

impl Args {
    fn sport_url(&self) -> &'static str {
        if self.women {
            "basketball-women"
        } else {
            "basketball-men"
        }
    }

    fn default_output_dir(&self) -> PathBuf {
        if self.women {
            PathBuf::from(format!("data/{}/women", self.year))
        } else {
            PathBuf::from(format!("data/{}/men", self.year))
        }
    }
}

// ── Output types ───────────────────────────────────────────────────

#[derive(Serialize)]
struct TournamentJson {
    name: String,
    regions: Vec<String>,
    teams: Vec<TeamEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamEntry {
    name: String,
    seed: u32,
    region: String,
    /// Present when this slot is decided by a First Four game.
    /// Contains the two team names competing for the slot.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_four: Option<[String; 2]>,
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    let client = NcaaClient::new(args.requests_per_sec).wrap_err("failed to create NCAA client")?;

    let sport = args.sport_url();
    info!(
        "fetching bracket for {sport} d{} {}",
        args.division, args.year
    );
    let champ = ncaa_api::fetch_bracket(&client, sport, args.division, args.year)
        .await
        .wrap_err("failed to fetch bracket")?;

    let tournament = build_tournament_data(&champ)?;

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&tournament)?);
        return Ok(());
    }

    let base_dir = match args.output_dir {
        Some(ref dir) => dir.clone(),
        None => args.default_output_dir(),
    };

    std::fs::create_dir_all(&base_dir)?;
    write_tournament_json(&base_dir, &tournament)?;

    Ok(())
}

// ── Core logic ─────────────────────────────────────────────────────

fn build_tournament_data(champ: &Championship) -> Result<TournamentJson> {
    // 1. Determine region order from F4 pairings.
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

    info!(
        "region order: {:?} (indices 0,1 play F4; indices 2,3 play F4)",
        region_names
    );

    // 2. Build First Four lookup: R64 bracketPositionId → First Four game.
    let first_four_by_target = build_first_four_map(champ);

    // 3. Build teams list in bracket order.
    let mut teams: Vec<TeamEntry> = Vec::with_capacity(64);

    for section_id in &region_order {
        let region_name = region_names
            .iter()
            .zip(region_order.iter())
            .find(|(_, sid)| *sid == section_id)
            .map(|(name, _)| name.as_str())
            .unwrap_or("Unknown");

        let r64_games = champ.r64_games(*section_id);

        ensure!(
            r64_games.len() == 8,
            "expected 8 R64 games for region {region_name}, got {}",
            r64_games.len()
        );

        for game in &r64_games {
            let (top_team, bottom_team) = extract_game_teams(game, &first_four_by_target)?;

            // Top team (higher seed) goes first in bracket order.
            teams.push(TeamEntry {
                name: top_team.name,
                seed: top_team.seed,
                region: region_name.to_string(),
                first_four: top_team.first_four,
            });
            teams.push(TeamEntry {
                name: bottom_team.name,
                seed: bottom_team.seed,
                region: region_name.to_string(),
                first_four: bottom_team.first_four,
            });
        }
    }

    Ok(TournamentJson {
        name: champ.title.clone(),
        regions: region_names,
        teams,
    })
}

/// A resolved team for a bracket slot — either a single team or a First Four pair.
struct ResolvedTeam {
    /// Display name (single team name, or "A/B" for First Four).
    name: String,
    seed: u32,
    /// If this is a First Four slot, the two team names.
    first_four: Option<[String; 2]>,
}

fn extract_game_teams(
    game: &BracketGame,
    first_four_map: &HashMap<u32, &BracketGame>,
) -> Result<(ResolvedTeam, ResolvedTeam)> {
    let bid = game.bracket_position_id;

    // Standard case: game has 2 teams with seeds.
    let seeded_teams: Vec<_> = game.teams.iter().filter(|t| t.seed.is_some()).collect();

    match seeded_teams.len() {
        2 => {
            // Both teams present — no First Four involvement.
            let (top, bot) = if seeded_teams[0].is_top {
                (seeded_teams[0], seeded_teams[1])
            } else {
                (seeded_teams[1], seeded_teams[0])
            };
            Ok((
                ResolvedTeam {
                    name: top.name_short.clone(),
                    seed: top.seed.unwrap(),
                    first_four: None,
                },
                ResolvedTeam {
                    name: bot.name_short.clone(),
                    seed: bot.seed.unwrap(),
                    first_four: None,
                },
            ))
        }
        1 => {
            // One team present — the other comes from a First Four game.
            let known = seeded_teams[0];
            let ff_game = first_four_map
                .get(&bid)
                .ok_or_else(|| eyre::eyre!("R64 game {bid} has 1 team but no First Four feeder"))?;

            let ff_teams: Vec<_> = ff_game.teams.iter().filter(|t| t.seed.is_some()).collect();
            ensure!(
                ff_teams.len() == 2,
                "First Four game {} should have 2 teams, got {}",
                ff_game.bracket_position_id,
                ff_teams.len()
            );

            let ff_seed = ff_teams[0].seed.unwrap();
            let ff_name = format!("{}/{}", ff_teams[0].name_short, ff_teams[1].name_short);

            let ff_resolved = ResolvedTeam {
                name: ff_name,
                seed: ff_seed,
                first_four: Some([
                    ff_teams[0].name_short.clone(),
                    ff_teams[1].name_short.clone(),
                ]),
            };

            let known_resolved = ResolvedTeam {
                name: known.name_short.clone(),
                seed: known.seed.unwrap(),
                first_four: None,
            };

            // Determine which is top (higher seed) and which is bottom.
            if known.is_top {
                Ok((known_resolved, ff_resolved))
            } else {
                Ok((ff_resolved, known_resolved))
            }
        }
        0 => {
            // Both teams come from First Four (unlikely but handle it).
            bail!("R64 game {bid} has 0 seeded teams — not supported");
        }
        n => bail!("R64 game {bid} has {n} seeded teams, expected 1 or 2"),
    }
}

/// Build a map from R64 bracketPositionId → the First Four game that feeds into it.
fn build_first_four_map(champ: &Championship) -> HashMap<u32, &BracketGame> {
    let mut map = HashMap::new();
    for game in champ.first_four_games() {
        if let Some(target_bid) = game.victor_bracket_position_id {
            map.insert(target_bid, game);
        }
    }
    map
}

// ── File I/O ───────────────────────────────────────────────────────

fn write_tournament_json(base_dir: &Path, data: &TournamentJson) -> Result<()> {
    let path = base_dir.join("tournament.json");
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(&path, json + "\n").wrap_err_with(|| format!("writing {}", path.display()))?;
    info!("wrote {}", path.display());
    Ok(())
}
