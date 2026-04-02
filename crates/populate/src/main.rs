//! `march-madness-populate` — migrate Redis data into V2 contracts.
//!
//! Reads bracket entries, tags, and groups from Redis, converts legacy-encoded
//! brackets to contract-correct encoding via `reverse_game_bits()`, and
//! batch-imports them into MarchMadnessV2 and BracketGroupsV2.
//!
//! Idempotent: checks `hasEntry()` on-chain before importing entries, and
//! `batchImportEntries`/`batchImportMembers` skip already-present items.

mod contract;
mod migrate;
mod provider;

use alloy_primitives::Address;
use clap::Parser;
use eyre::Result;
use seismic_march_madness::redis_keys::DEFAULT_REDIS_URL;

use crate::provider::SignedProvider;

#[derive(Parser)]
#[command(name = "march-madness-populate")]
#[command(about = "Migrate Redis bracket data into MarchMadnessV2 and BracketGroupsV2 contracts")]
struct Cli {
    /// MarchMadnessV2 contract address.
    #[arg(long)]
    target: Address,

    /// BracketGroupsV2 contract address (required for group migration).
    #[arg(long)]
    groups_target: Option<Address>,

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

    let provider = if cli.dry_run {
        match std::env::var("PRIVATE_KEY") {
            Ok(pk) => Some(create_provider(&cli.network, &cli.rpc_url, &pk).await?),
            Err(_) => {
                tracing::warn!(
                    "PRIVATE_KEY not set; dry-run will show all Redis entries without on-chain filtering"
                );
                None
            }
        }
    } else {
        let pk = std::env::var("PRIVATE_KEY")
            .map_err(|_| eyre::eyre!("PRIVATE_KEY env var is required for non-dry-run mode"))?;
        Some(create_provider(&cli.network, &cli.rpc_url, &pk).await?)
    };

    if !cli.skip_entries {
        migrate::run_entries(
            provider.as_ref(),
            cli.target,
            &redis_url,
            cli.batch_size,
            cli.dry_run,
        )
        .await?;
    }

    if !cli.skip_groups {
        let groups_target = match cli.groups_target {
            Some(addr) => addr,
            None => {
                tracing::info!("no --groups-target specified, skipping group migration");
                return Ok(());
            }
        };
        migrate::run_groups(
            provider.as_ref(),
            groups_target,
            &redis_url,
            cli.batch_size,
            cli.dry_run,
        )
        .await?;
    }

    Ok(())
}

async fn create_provider(
    network: &NetworkBackend,
    rpc_url: &str,
    private_key: &str,
) -> Result<SignedProvider> {
    match network {
        NetworkBackend::Reth => SignedProvider::new_reth(rpc_url, private_key).await,
        NetworkBackend::Foundry => SignedProvider::new_foundry(rpc_url, private_key).await,
    }
}
