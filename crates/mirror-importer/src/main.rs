//! Mirror Importer — fetch brackets from external platforms and write platform.json
//! for on-chain mirroring via BracketMirror.
//!
//! Currently supports Yahoo Fantasy Sports bracket groups.

mod api;
mod cache;
mod encode;
mod names;

use clap::Parser;
use eyre::bail;
use serde::{Deserialize, Serialize};
use tracing::info;

use seismic_march_madness::TournamentData;

use api::YahooClient;
use encode::{encode_bracket, format_bracket_hex};
use names::NameResolver;

/// Import brackets from external platforms for BracketMirror.
#[derive(Parser, Debug)]
#[command(name = "mirror-importer")]
struct Args {
    /// Yahoo Fantasy group ID
    #[arg(long)]
    group_id: u32,

    /// Mirror slug (default: YAHOO-{group-id})
    #[arg(long)]
    slug: Option<String>,

    /// Tournament year
    #[arg(long, default_value = "2026")]
    year: u16,

    /// Ignore cache, re-fetch all data
    #[arg(long)]
    force_refresh: bool,
}

/// A single entry in platform.json.
#[derive(Debug, Serialize, Deserialize)]
struct PlatformEntry {
    /// Yahoo fantasy team ID
    team_id: String,
    /// Entry display name (bracket name from Yahoo)
    name: String,
    /// Yahoo user display name
    user: String,
    /// Encoded bracket as 0x-prefixed hex
    bracket: String,
    /// Predicted champion name
    champion: String,
}

/// Top-level platform.json output.
#[derive(Debug, Serialize, Deserialize)]
struct PlatformOutput {
    slug: String,
    group_id: u32,
    year: u16,
    entries: Vec<PlatformEntry>,
}

fn main() -> eyre::Result<()> {
    // Init
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mirror_importer=info".into()),
        )
        .init();

    let args = Args::parse();
    let slug = args
        .slug
        .unwrap_or_else(|| format!("YAHOO-{}", args.group_id));

    info!(
        "importing Yahoo group {} as mirror slug '{}'",
        args.group_id, slug
    );

    // Load tournament data
    let tournament = TournamentData::embedded(args.year);
    info!(
        "loaded tournament data for {}: {}",
        args.year, tournament.name
    );

    // Create Yahoo API client
    let client = YahooClient::new()?;

    // Fetch bracket structure
    let bracket_resp = client.fetch_bracket(args.force_refresh)?;
    let tournament_data = &bracket_resp.data.fantasy_game.tournament;

    // Build team key → display name mapping
    let yahoo_teams: Vec<(String, String)> = tournament_data
        .tournament_teams
        .iter()
        .map(|t| {
            (
                t.editorial_team_key.clone(),
                t.editorial_team.display_name.clone(),
            )
        })
        .collect();

    // Build name resolver
    let resolver = NameResolver::new(&tournament, &yahoo_teams)?;

    // Fetch group members
    let members = client.fetch_group_members(args.group_id, args.force_refresh)?;
    info!("found {} members in group {}", members.len(), args.group_id);

    // Fetch and encode each member's bracket
    let mut entries = Vec::new();
    for (i, member) in members.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        info!(
            "[{}/{}] fetching picks for {} (team {})",
            i + 1,
            members.len(),
            member.name,
            member.fantasy_team_key_parts.fantasy_team_id
        );

        let picks_resp =
            client.fetch_team_picks(&member.fantasy_team_key_parts.fantasy_team_id, args.group_id, args.force_refresh)?;

        let picks = &picks_resp.data.fantasy_team.bracket_picks.picks;
        if picks.len() != 63 {
            bail!(
                "team {} has {} picks (expected 63)",
                member.fantasy_team_key_parts.fantasy_team_id,
                picks.len()
            );
        }

        let (bits, champion) = encode_bracket(&tournament_data.slots, picks, &resolver)?;

        let hex = format_bracket_hex(bits);
        info!(
            "  → {} | champion: {} | bracket: {}",
            member.name, champion, hex
        );

        entries.push(PlatformEntry {
            team_id: member.fantasy_team_key_parts.fantasy_team_id.clone(),
            name: picks_resp.data.fantasy_team.name.clone(),

            user: member.user.display_name.clone(),
            bracket: hex,
            champion,
        });
    }

    // Write platform.json
    let output = PlatformOutput {
        slug: slug.clone(),
        group_id: args.group_id,
        year: args.year,
        entries,
    };

    let output_dir = cache::group_cache_dir(args.group_id);
    std::fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("platform.json");
    let json = serde_json::to_string_pretty(&output)?;
    std::fs::write(&output_path, &json)?;

    info!(
        "wrote {} entries to {}",
        output.entries.len(),
        output_path.display()
    );

    Ok(())
}
