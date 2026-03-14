//! Live event listener: polls for new BracketSubmitted and TagSet events.

use crate::indexer::{load_index, save_index, update_tag, upsert_bracket_submitted};
use crate::provider;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use std::path::Path;
use tokio::signal;

/// Poll interval in seconds.
const POLL_INTERVAL_SECS: u64 = 5;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let p = provider::create_provider(rpc_url)?;
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;

    // Start from the latest block
    let mut last_block = provider::block_number(&p).await?;
    println!("Listening for events from block {}...", last_block);
    println!("Press Ctrl+C to stop.");

    let mut index = load_index(index_path)?;
    println!("Loaded index with {} entries", index.len());

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\nShutting down gracefully...");
                save_index(index_path, &index)?;
                println!("Index saved with {} entries", index.len());
                return Ok(());
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)) => {
                let current_block = provider::block_number(&p).await?;
                if current_block <= last_block {
                    continue;
                }

                let from = last_block + 1;

                // Fetch BracketSubmitted logs
                let bracket_logs = provider::get_bracket_submitted_logs(
                    &p, contract_addr, from, current_block,
                )
                .await
                .wrap_err("failed to fetch BracketSubmitted logs")?;

                // Fetch TagSet logs
                let tag_logs = provider::get_tag_set_logs(
                    &p, contract_addr, from, current_block,
                )
                .await
                .wrap_err("failed to fetch TagSet logs")?;

                let mut updated = false;

                for log in &bracket_logs {
                    let address = provider::parse_bracket_submitted(log)?;
                    let block_num = log
                        .block_number
                        .ok_or_else(|| eyre::eyre!("log missing block number"))?;
                    let ts = provider::get_block_timestamp(&p, block_num).await?;
                    let addr_str = format!("{address:#x}");
                    println!(
                        "  BracketSubmitted: {} (block {})",
                        addr_str, block_num
                    );
                    upsert_bracket_submitted(&mut index, &addr_str, block_num, ts);
                    updated = true;
                }

                for log in &tag_logs {
                    let (address, tag) = provider::parse_tag_set(log)?;
                    let addr_str = format!("{address:#x}");
                    println!("  TagSet: {} => \"{}\"", addr_str, tag);
                    update_tag(&mut index, &addr_str, tag);
                    updated = true;
                }

                if updated {
                    save_index(index_path, &index)?;
                    println!(
                        "Index updated: {} entries (blocks {}..{})",
                        index.len(),
                        from,
                        current_block
                    );
                }

                last_block = current_block;
            }
        }
    }
}
