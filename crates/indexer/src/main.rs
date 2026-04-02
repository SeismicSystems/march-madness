mod commands;
mod contract;
#[allow(dead_code)]
mod indexer;
mod provider;
mod redis_store;

use clap::{Parser, Subcommand, ValueEnum};
use eyre::{Result, WrapErr};
use provider::IndexerProvider;
use tracing::info;

const DEFAULT_DEPLOYMENTS_PATH: &str = "data/deployments.json";
const DEFAULT_YEAR: &str = "2026";
const DEFAULT_CHAIN_ID: &str = "5124";

#[derive(Parser)]
#[command(name = "march-madness-indexer")]
#[command(about = "Index March Madness bracket events from the Seismic network")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Seismic network backend to connect to
    #[arg(long, global = true, default_value = "reth")]
    network: NetworkBackend,

    /// MarchMadness contract address (defaults to data/deployments.json)
    #[arg(long, global = true)]
    contract: Option<String>,

    /// BracketGroups contract address (defaults to data/deployments.json)
    #[arg(long, global = true)]
    groups_contract: Option<String>,

    /// BracketMirror contract address (defaults to data/deployments.json)
    #[arg(long, global = true)]
    mirror_contract: Option<String>,
}

/// Which Seismic network implementation to use for RPC calls.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum NetworkBackend {
    /// seismic-reth (production / testnet)
    Reth,
    /// seismic-foundry / sanvil (local development)
    Foundry,
}

/// Parsed contract addresses ready for use.
pub struct ParsedAddresses {
    pub march_madness: alloy_primitives::Address,
    pub bracket_groups: alloy_primitives::Address,
    pub bracket_mirror: alloy_primitives::Address,
}

/// Raw contract address strings from CLI / deployments.json.
#[derive(Clone, Debug)]
pub struct ContractAddresses {
    pub march_madness: String,
    pub bracket_groups: String,
    pub bracket_mirror: String,
}

impl ContractAddresses {
    /// Parse string addresses into alloy Address types.
    pub fn parse(&self) -> Result<ParsedAddresses> {
        Ok(ParsedAddresses {
            march_madness: self
                .march_madness
                .parse()
                .wrap_err("invalid MarchMadness address")?,
            bracket_groups: self
                .bracket_groups
                .parse()
                .wrap_err("invalid BracketGroups address")?,
            bracket_mirror: self
                .bracket_mirror
                .parse()
                .wrap_err("invalid BracketMirror address")?,
        })
    }
}

#[derive(Subcommand)]
enum Command {
    /// Listen for live events and update Redis in real time
    Listen {
        /// JSON-RPC endpoint URL (falls back to VITE_RPC_URL env var)
        #[arg(long, env = "VITE_RPC_URL")]
        rpc_url: String,
    },

    /// Backfill historical events and rebuild the index in Redis
    Backfill {
        /// JSON-RPC endpoint URL (falls back to VITE_RPC_URL env var)
        #[arg(long, env = "VITE_RPC_URL")]
        rpc_url: String,

        /// Block number to start scanning from
        #[arg(long, default_value = "0")]
        from_block: u64,
    },

    /// Reveal brackets for all indexed addresses (post-deadline only)
    Reveal {
        /// JSON-RPC endpoint URL (falls back to VITE_RPC_URL env var)
        #[arg(long, env = "VITE_RPC_URL")]
        rpc_url: String,
    },

    /// Sanity check: compare Redis entry count with on-chain getEntryCount()
    #[command(name = "check")]
    SanityCheck {
        /// JSON-RPC endpoint URL (falls back to VITE_RPC_URL env var)
        #[arg(long, env = "VITE_RPC_URL")]
        rpc_url: String,
    },

    /// Redis-internal consistency check for stored counts (no RPC needed)
    #[command(name = "check-redis")]
    CheckRedis {
        /// Check a specific group by slug
        #[arg(long)]
        group: Option<String>,

        /// Check all groups
        #[arg(long)]
        all_groups: bool,
    },

    /// Backfill mirror metadata + entries from the contract into Redis.
    /// If --mirror-id is given, backfills that mirror only; otherwise backfills all mirrors.
    BackfillMirror {
        /// JSON-RPC endpoint URL (falls back to VITE_RPC_URL env var)
        #[arg(long, env = "VITE_RPC_URL")]
        rpc_url: String,

        /// Mirror ID to backfill (omit to backfill all mirrors)
        #[arg(long)]
        mirror_id: Option<u64>,
    },

    /// Seed Redis with fake bracket data for local development
    Seed {
        /// Number of fake entries to generate
        #[arg(long, default_value = "50")]
        entries: usize,

        /// Clear existing data before seeding
        #[arg(long)]
        clean: bool,
    },
}

fn rpc_url(command: &Command) -> Option<&str> {
    match command {
        Command::Listen { rpc_url }
        | Command::Backfill { rpc_url, .. }
        | Command::Reveal { rpc_url }
        | Command::SanityCheck { rpc_url }
        | Command::BackfillMirror { rpc_url, .. } => Some(rpc_url),
        Command::CheckRedis { .. } | Command::Seed { .. } => None,
    }
}

/// Load contract addresses from CLI overrides or data/deployments.json.
fn resolve_addresses(cli: &Cli) -> Result<ContractAddresses> {
    // Load base addresses from deployments.json.
    let path = std::path::Path::new(DEFAULT_DEPLOYMENTS_PATH);
    let chain = if path.exists() {
        let content = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read {DEFAULT_DEPLOYMENTS_PATH}"))?;
        let deployments: serde_json::Value =
            serde_json::from_str(&content).wrap_err("failed to parse deployments.json")?;
        deployments[DEFAULT_YEAR][DEFAULT_CHAIN_ID].clone()
    } else {
        serde_json::Value::Null
    };

    // Prefer v2 addresses when present (2026 migration).
    let v2 = &chain["v2"];
    let mm = cli
        .contract
        .clone()
        .or_else(|| v2["marchMadness"].as_str().map(String::from))
        .or_else(|| chain["marchMadness"].as_str().map(String::from))
        .ok_or_else(|| eyre::eyre!("marchMadness address not found (CLI or deployments.json)"))?;
    let groups = cli
        .groups_contract
        .clone()
        .or_else(|| v2["bracketGroups"].as_str().map(String::from))
        .or_else(|| chain["bracketGroups"].as_str().map(String::from))
        .ok_or_else(|| eyre::eyre!("bracketGroups address not found (CLI or deployments.json)"))?;
    let mirror = cli
        .mirror_contract
        .clone()
        .or_else(|| chain["bracketMirror"].as_str().map(String::from))
        .ok_or_else(|| eyre::eyre!("bracketMirror address not found (CLI or deployments.json)"))?;

    info!(
        march_madness = %mm,
        groups = %groups,
        mirror = %mirror,
        "resolved contract addresses"
    );

    Ok(ContractAddresses {
        march_madness: mm,
        bracket_groups: groups,
        bracket_mirror: mirror,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let mut redis_conn = redis_store::connect().await?;

    // Redis-only commands — no provider or contract addresses needed.
    match &cli.command {
        Command::CheckRedis {
            group, all_groups, ..
        } => {
            let mode = if let Some(slug) = group {
                commands::check_redis::CheckMode::Group(slug.clone())
            } else if *all_groups {
                commands::check_redis::CheckMode::AllGroups
            } else {
                commands::check_redis::CheckMode::Total
            };
            return commands::check_redis::run(&mut redis_conn, mode).await;
        }
        Command::Seed { entries, clean } => {
            return commands::seed::run(&mut redis_conn, *entries, *clean).await;
        }
        _ => {}
    }

    let rpc = rpc_url(&cli.command).expect("rpc_url required for this command");
    let provider = match cli.network {
        NetworkBackend::Reth => IndexerProvider::new_reth(rpc)?,
        NetworkBackend::Foundry => IndexerProvider::new_foundry(rpc)?,
    };
    let addrs = resolve_addresses(&cli)?.parse()?;

    match cli.command {
        Command::Listen { .. } => {
            commands::listen::run(&provider, &mut redis_conn, &addrs).await?;
        }
        Command::Backfill { from_block, .. } => {
            commands::backfill::run(&provider, &mut redis_conn, &addrs, from_block).await?;
        }
        Command::Reveal { .. } => {
            let mm = format!("{:#x}", addrs.march_madness);
            commands::reveal::run(&provider, &mut redis_conn, &mm).await?;
        }
        Command::BackfillMirror { mirror_id, .. } => {
            commands::backfill_mirror::run(
                &provider,
                &mut redis_conn,
                addrs.bracket_mirror,
                mirror_id,
            )
            .await?;
        }
        Command::SanityCheck { .. } => {
            let mm = format!("{:#x}", addrs.march_madness);
            commands::check::run(&provider, &mut redis_conn, &mm).await?;
        }
        Command::CheckRedis { .. } | Command::Seed { .. } => unreachable!(),
    }

    Ok(())
}
