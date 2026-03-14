mod commands;
mod contract;
mod indexer;
mod provider;

use clap::{Parser, Subcommand};
use eyre::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "march-madness-indexer")]
#[command(about = "Index March Madness bracket events from the Seismic network")]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
    Check {
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Listen {
            rpc_url,
            contract,
            index_file,
        } => {
            commands::listen::run(&rpc_url, &contract, &index_file).await?;
        }
        Command::Backfill {
            rpc_url,
            contract,
            index_file,
            from_block,
        } => {
            commands::backfill::run(&rpc_url, &contract, &index_file, from_block).await?;
        }
        Command::Reveal {
            rpc_url,
            contract,
            index_file,
        } => {
            commands::reveal::run(&rpc_url, &contract, &index_file).await?;
        }
        Command::Check {
            rpc_url,
            contract,
            index_file,
        } => {
            commands::check::run(&rpc_url, &contract, &index_file).await?;
        }
    }

    Ok(())
}
