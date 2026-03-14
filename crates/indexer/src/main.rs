mod commands;
mod contract;
mod indexer;
mod provider;

use clap::{Parser, Subcommand, ValueEnum};
use eyre::Result;
use provider::IndexerProvider;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "march-madness-indexer")]
#[command(about = "Index March Madness bracket events from the Seismic network")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Seismic network backend to connect to
    #[arg(long, global = true, default_value = "reth")]
    network: NetworkBackend,
}

/// Which Seismic network implementation to use for RPC calls.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum NetworkBackend {
    /// seismic-reth (production / testnet)
    Reth,
    /// seismic-foundry / sanvil (local development)
    Foundry,
}

#[derive(Subcommand)]
enum Command {
    /// Listen for live events and update the index in real time
    Listen {
        /// JSON-RPC endpoint URL
        #[arg(long)]
        rpc_url: String,

        /// MarchMadness contract address (0x-prefixed)
        #[arg(long)]
        contract: String,

        /// Path to the index JSON file
        #[arg(long, default_value = "data/entries.json")]
        index_file: PathBuf,
    },

    /// Backfill historical events and rebuild the index
    Backfill {
        /// JSON-RPC endpoint URL
        #[arg(long)]
        rpc_url: String,

        /// MarchMadness contract address (0x-prefixed)
        #[arg(long)]
        contract: String,

        /// Path to the index JSON file
        #[arg(long, default_value = "data/entries.json")]
        index_file: PathBuf,

        /// Block number to start scanning from
        #[arg(long, default_value = "0")]
        from_block: u64,
    },

    /// Reveal brackets for all indexed addresses (post-deadline only)
    Reveal {
        /// JSON-RPC endpoint URL
        #[arg(long)]
        rpc_url: String,

        /// MarchMadness contract address (0x-prefixed)
        #[arg(long)]
        contract: String,

        /// Path to the index JSON file
        #[arg(long, default_value = "data/entries.json")]
        index_file: PathBuf,
    },

    /// Sanity check: compare local entry count with on-chain getEntryCount()
    #[command(name = "check")]
    SanityCheck {
        /// JSON-RPC endpoint URL
        #[arg(long)]
        rpc_url: String,

        /// MarchMadness contract address (0x-prefixed)
        #[arg(long)]
        contract: String,

        /// Path to the index JSON file
        #[arg(long, default_value = "data/entries.json")]
        index_file: PathBuf,
    },
}

/// Extract the RPC URL from any command variant.
fn rpc_url(command: &Command) -> &str {
    match command {
        Command::Listen { rpc_url, .. }
        | Command::Backfill { rpc_url, .. }
        | Command::Reveal { rpc_url, .. }
        | Command::SanityCheck { rpc_url, .. } => rpc_url,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let provider = match cli.network {
        NetworkBackend::Reth => IndexerProvider::new_reth(rpc_url(&cli.command))?,
        NetworkBackend::Foundry => IndexerProvider::new_foundry(rpc_url(&cli.command))?,
    };

    match cli.command {
        Command::Listen {
            contract,
            index_file,
            ..
        } => {
            commands::listen::run(&provider, &contract, &index_file).await?;
        }
        Command::Backfill {
            contract,
            index_file,
            from_block,
            ..
        } => {
            commands::backfill::run(&provider, &contract, &index_file, from_block).await?;
        }
        Command::Reveal {
            contract,
            index_file,
            ..
        } => {
            commands::reveal::run(&provider, &contract, &index_file).await?;
        }
        Command::SanityCheck {
            contract,
            index_file,
            ..
        } => {
            commands::check::run(&provider, &contract, &index_file).await?;
        }
    }

    Ok(())
}
