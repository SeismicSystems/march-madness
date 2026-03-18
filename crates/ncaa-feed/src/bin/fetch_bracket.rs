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
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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
    /// Team name. Null for First Four slots (use `firstFour.teams` or `firstFour.winner`).
    name: Option<String>,
    seed: u32,
    region: String,
    /// Short display abbreviation. Only present when `name` exceeds 9 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    abbrev: Option<String>,
    /// Present when this slot is decided by a First Four game.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_four: Option<FirstFourEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstFourEntry {
    teams: [FirstFourTeam; 2],
    /// Name of the winning team, if the First Four game has been played.
    #[serde(skip_serializing_if = "Option::is_none")]
    winner: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FirstFourTeam {
    name: String,
    abbrev: String,
}

// ── Mappings ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct MappingsConfig {
    #[serde(default)]
    abbreviations: HashMap<String, String>,
}

fn mappings_toml_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("data").join("mappings.toml"))
        .expect("could not resolve workspace root")
}

fn load_abbreviations() -> HashMap<String, String> {
    let toml_path = mappings_toml_path();
    match std::fs::read_to_string(&toml_path) {
        Ok(content) => {
            let config: MappingsConfig = toml::from_str(&content)
                .unwrap_or_else(|e| panic!("failed to parse {}: {e}", toml_path.display()));
            config.abbreviations
        }
        Err(_) => {
            warn!("data/mappings.toml not found, no abbreviations will be applied");
            HashMap::new()
        }
    }
}

/// Collect all First Four team names from the bracket that need abbreviations.
fn collect_ff_team_names(champ: &Championship) -> Vec<String> {
    let mut names = Vec::new();
    for game in champ.first_four_games() {
        for team in &game.teams {
            if team.seed.is_some() && !team.name_short.is_empty() {
                names.push(team.name_short.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Prompt the user for missing FF abbreviations, then append them to mappings.toml.
/// Names <= 5 chars are auto-abbreviated to themselves (no prompt, no write).
fn prompt_missing_ff_abbreviations(
    ff_names: &[String],
    abbreviations: &mut HashMap<String, String>,
) -> Result<()> {
    use std::io::{BufRead, Write};

    // Find names that need prompting (> 5 chars, not in abbreviations).
    let missing: Vec<&String> = ff_names
        .iter()
        .filter(|name| name.len() > 5 && !abbreviations.contains_key(name.as_str()))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut new_entries: Vec<(String, String)> = Vec::new();

    eprintln!();
    eprintln!("Missing abbreviations for First Four teams:");
    for name in &missing {
        eprint!("  Abbreviation for \"{name}\": ");
        std::io::stderr().flush()?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let abbrev = line.trim().to_string();
        if abbrev.is_empty() {
            eyre::bail!("abbreviation cannot be empty for \"{name}\"");
        }
        abbreviations.insert(name.to_string(), abbrev.clone());
        new_entries.push((name.to_string(), abbrev));
    }

    // Append new entries to mappings.toml.
    if !new_entries.is_empty() {
        let toml_path = mappings_toml_path();
        let mut content = std::fs::read_to_string(&toml_path).unwrap_or_default();
        // Ensure file ends with newline before appending.
        if !content.ends_with('\n') {
            content.push('\n');
        }
        for (name, abbrev) in &new_entries {
            content.push_str(&format!("\"{name}\" = \"{abbrev}\"\n"));
        }
        std::fs::write(&toml_path, &content)
            .wrap_err_with(|| format!("writing {}", toml_path.display()))?;
        info!(
            "added {} abbreviation(s) to {}",
            new_entries.len(),
            toml_path.display()
        );
    }

    Ok(())
}

/// Resolve abbreviation for a First Four team — always returns a value.
/// Names <= 5 chars use the name itself; longer names use mappings.toml.
fn ff_abbrev_or_name(name: &str, abbreviations: &HashMap<String, String>) -> String {
    if name.len() <= 5 {
        name.to_string()
    } else {
        abbreviations
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
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

    let mut abbreviations = load_abbreviations();
    info!(
        "loaded {} abbreviations from mappings.toml",
        abbreviations.len()
    );

    let sport = args.sport_url();
    info!(
        "fetching bracket for {sport} d{} {}",
        args.division, args.year
    );
    let champ = ncaa_api::fetch_bracket(&client, sport, args.division, args.year)
        .await
        .wrap_err("failed to fetch bracket")?;

    // Prompt for any missing First Four abbreviations before building output.
    let ff_names = collect_ff_team_names(&champ);
    prompt_missing_ff_abbreviations(&ff_names, &mut abbreviations)?;

    let tournament = build_tournament_data(&champ, &abbreviations)?;

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

fn build_tournament_data(
    champ: &Championship,
    abbreviations: &HashMap<String, String>,
) -> Result<TournamentJson> {
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
            let (top_team, bottom_team) =
                extract_game_teams(game, &first_four_by_target, abbreviations)?;

            // Top team (higher seed) goes first in bracket order.
            teams.push(TeamEntry {
                name: top_team.name,
                seed: top_team.seed,
                region: region_name.to_string(),
                abbrev: top_team.abbrev,
                first_four: top_team.first_four,
            });
            teams.push(TeamEntry {
                name: bottom_team.name,
                seed: bottom_team.seed,
                region: region_name.to_string(),
                abbrev: bottom_team.abbrev,
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
    /// Team name. None for First Four slots.
    name: Option<String>,
    seed: u32,
    /// Short display abbreviation (if name > 9 chars and a mapping exists).
    abbrev: Option<String>,
    /// If this is a First Four slot, the structured First Four data.
    first_four: Option<FirstFourEntry>,
}

/// Look up an abbreviation from mappings.toml, only if the name exceeds 9 characters.
fn abbrev_for(name: &str, abbreviations: &HashMap<String, String>) -> Option<String> {
    if name.len() > 9 {
        abbreviations.get(name).cloned()
    } else {
        None
    }
}

fn extract_game_teams(
    game: &BracketGame,
    first_four_map: &HashMap<u32, &BracketGame>,
    abbreviations: &HashMap<String, String>,
) -> Result<(ResolvedTeam, ResolvedTeam)> {
    let bid = game.bracket_position_id;

    // Standard case: game has 2 teams with seeds.
    let seeded_teams: Vec<_> = game.teams.iter().filter(|t| t.seed.is_some()).collect();

    match seeded_teams.len() {
        2 => {
            let (top, bot) = if seeded_teams[0].is_top {
                (seeded_teams[0], seeded_teams[1])
            } else {
                (seeded_teams[1], seeded_teams[0])
            };

            // Check if a First Four game feeds into this R64 slot.
            // If so, one of these two teams is the FF winner — preserve the FF context.
            if let Some(ff_game) = first_four_map.get(&bid) {
                let ff_teams: Vec<_> = ff_game.teams.iter().filter(|t| t.seed.is_some()).collect();
                ensure!(
                    ff_teams.len() == 2,
                    "First Four game {} should have 2 teams, got {}",
                    ff_game.bracket_position_id,
                    ff_teams.len()
                );

                let ff_seed = ff_teams[0].seed.unwrap();
                let winner = ff_teams
                    .iter()
                    .find(|t| t.is_winner)
                    .map(|t| t.name_short.clone());

                let ff_entry = FirstFourEntry {
                    teams: [
                        FirstFourTeam {
                            name: ff_teams[0].name_short.clone(),
                            abbrev: ff_abbrev_or_name(&ff_teams[0].name_short, abbreviations),
                        },
                        FirstFourTeam {
                            name: ff_teams[1].name_short.clone(),
                            abbrev: ff_abbrev_or_name(&ff_teams[1].name_short, abbreviations),
                        },
                    ],
                    winner,
                };

                // Figure out which of top/bot is the FF slot (same seed as FF game).
                let (ff_resolved, other_resolved) = if top.seed.unwrap() == ff_seed {
                    (
                        ResolvedTeam {
                            name: None,
                            seed: ff_seed,
                            abbrev: None,
                            first_four: Some(ff_entry),
                        },
                        ResolvedTeam {
                            name: Some(bot.name_short.clone()),
                            seed: bot.seed.unwrap(),
                            abbrev: abbrev_for(&bot.name_short, abbreviations),
                            first_four: None,
                        },
                    )
                } else {
                    (
                        ResolvedTeam {
                            name: None,
                            seed: ff_seed,
                            abbrev: None,
                            first_four: Some(ff_entry),
                        },
                        ResolvedTeam {
                            name: Some(top.name_short.clone()),
                            seed: top.seed.unwrap(),
                            abbrev: abbrev_for(&top.name_short, abbreviations),
                            first_four: None,
                        },
                    )
                };

                // Preserve bracket order: top first, bottom second.
                if top.seed.unwrap() == ff_seed {
                    Ok((ff_resolved, other_resolved))
                } else {
                    Ok((other_resolved, ff_resolved))
                }
            } else {
                // No First Four involvement — regular teams.
                Ok((
                    ResolvedTeam {
                        name: Some(top.name_short.clone()),
                        seed: top.seed.unwrap(),
                        abbrev: abbrev_for(&top.name_short, abbreviations),
                        first_four: None,
                    },
                    ResolvedTeam {
                        name: Some(bot.name_short.clone()),
                        seed: bot.seed.unwrap(),
                        abbrev: abbrev_for(&bot.name_short, abbreviations),
                        first_four: None,
                    },
                ))
            }
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

            // Detect winner from the FF game's isWinner flag.
            let winner = ff_teams
                .iter()
                .find(|t| t.is_winner)
                .map(|t| t.name_short.clone());

            let ff_entry = FirstFourEntry {
                teams: [
                    FirstFourTeam {
                        name: ff_teams[0].name_short.clone(),
                        abbrev: ff_abbrev_or_name(&ff_teams[0].name_short, abbreviations),
                    },
                    FirstFourTeam {
                        name: ff_teams[1].name_short.clone(),
                        abbrev: ff_abbrev_or_name(&ff_teams[1].name_short, abbreviations),
                    },
                ],
                winner,
            };

            let ff_resolved = ResolvedTeam {
                name: None,
                seed: ff_seed,
                abbrev: None,
                first_four: Some(ff_entry),
            };

            let known_resolved = ResolvedTeam {
                name: Some(known.name_short.clone()),
                seed: known.seed.unwrap(),
                abbrev: abbrev_for(&known.name_short, abbreviations),
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
