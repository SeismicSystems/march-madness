//! `march-madness-populate` — migrate V1 contract data into V2 contracts.
//!
//! Reads entries, tags, groups, and members directly from V1 contracts
//! (MarchMadness + BracketGroups), converts legacy-encoded brackets to
//! contract-correct encoding, and batch-imports into V2 contracts.
//!
//! Uses Redis only for tracking migration progress (new keys scoped to
//! the migration). The V1 contract is the source of truth.

mod contract;
mod migrate;
mod provider;

use alloy_primitives::Address;
use clap::Parser;
use eyre::{Result, WrapErr};
use seismic_march_madness::redis_keys::DEFAULT_REDIS_URL;

use crate::provider::{ReadProvider, SignedProvider};

#[derive(Parser)]
#[command(name = "march-madness-populate")]
#[command(about = "Migrate V1 contract data into V2 contracts")]
struct Cli {
    /// V1 MarchMadness contract address (source).
    #[arg(long)]
    source: Address,

    /// V2 MarchMadnessV2 contract address (target).
    #[arg(long)]
    target: Address,

    /// V1 BracketGroups contract address (source, required for group migration).
    #[arg(long)]
    groups_source: Option<Address>,

    /// V2 BracketGroupsV2 contract address (target, required for group migration).
    #[arg(long)]
    groups_target: Option<Address>,

    /// Block number at which the V1 contract was deployed (for event scanning).
    #[arg(long, default_value = "0")]
    from_block: u64,

    /// RPC URL for the Seismic node.
    #[arg(long, env = "RPC_URL", default_value = "http://localhost:8545")]
    rpc_url: String,

    /// Network backend: "reth" for production/testnet, "foundry" for sanvil.
    #[arg(long, default_value = "reth")]
    network: NetworkBackend,

    /// Number of entries per batch transaction.
    #[arg(long, default_value = "50")]
    batch_size: usize,

    /// Print what would be imported without sending transactions.
    #[arg(long)]
    dry_run: bool,

    /// Skip entry migration (only migrate groups).
    #[arg(long)]
    skip_entries: bool,

    /// Skip group migration (only migrate entries).
    #[arg(long)]
    skip_groups: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum NetworkBackend {
    Reth,
    Foundry,
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

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let redis_client = redis::Client::open(redis_url.as_str()).wrap_err("invalid REDIS_URL")?;
    let redis_conn = redis_client
        .get_connection()
        .wrap_err("failed to connect to Redis")?;

    let reader = create_reader(&cli.network, &cli.rpc_url)?;

    // dry_run ↔ writer=None: no PRIVATE_KEY needed, no transactions sent
    let writer = if cli.dry_run {
        None
    } else {
        let pk = std::env::var("PRIVATE_KEY")
            .map_err(|_| eyre::eyre!("PRIVATE_KEY env var is required for non-dry-run mode"))?;
        Some(create_writer(&cli.network, &cli.rpc_url, &pk).await?)
    };

    let mut cfg = migrate::MigrateConfig {
        reader: &reader,
        writer: writer.as_ref(),
        redis: redis_conn,
        from_block: cli.from_block,
        batch_size: cli.batch_size,
    };

    if !cli.skip_entries {
        migrate::run_entries(&mut cfg, cli.source, cli.target).await?;
    }

    if !cli.skip_groups {
        let groups_source = match cli.groups_source {
            Some(addr) => addr,
            None => {
                tracing::info!("no --groups-source specified, skipping group migration");
                return Ok(());
            }
        };
        let groups_target = match cli.groups_target {
            Some(addr) => addr,
            None => {
                tracing::info!("no --groups-target specified, skipping group migration");
                return Ok(());
            }
        };
        migrate::run_groups(&mut cfg, groups_source, groups_target).await?;
    }

    Ok(())
}

fn create_reader(network: &NetworkBackend, rpc_url: &str) -> Result<ReadProvider> {
    match network {
        NetworkBackend::Reth => ReadProvider::new_reth(rpc_url),
        NetworkBackend::Foundry => ReadProvider::new_foundry(rpc_url),
    }
}

async fn create_writer(
    network: &NetworkBackend,
    rpc_url: &str,
    private_key: &str,
) -> Result<SignedProvider> {
    match network {
        NetworkBackend::Reth => SignedProvider::new_reth(rpc_url, private_key).await,
        NetworkBackend::Foundry => SignedProvider::new_foundry(rpc_url, private_key).await,
    }
}
