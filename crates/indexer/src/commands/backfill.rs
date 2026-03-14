//! Historical backfill: scan all blocks for BracketSubmitted and TagSet events.

use crate::indexer::{load_index, save_index, update_tag, upsert_bracket_submitted};
use crate::provider::{self, IndexerProvider};
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use std::path::Path;

/// Maximum block range per eth_getLogs request.
const BATCH_SIZE: u64 = 10_000;

pub async fn run(
    p: &IndexerProvider,
    contract: &str,
    index_path: &Path,
    from_block: u64,
) -> Result<()> {
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;

    let latest_block = p.block_number().await?;
    println!(
        "Backfilling from block {} to {} (latest)",
        from_block, latest_block
    );

    let mut index = load_index(index_path)?;
    let initial_count = index.len();

    // Pass 1: BracketSubmitted events
    println!("Scanning BracketSubmitted events...");
    let mut from = from_block;
    while from <= latest_block {
        let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);
        let logs = p
            .get_bracket_submitted_logs(contract_addr, from, to)
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            let address = provider::parse_bracket_submitted(log)?;
            let block_num = log
                .block_number
                .ok_or_else(|| eyre::eyre!("log missing block number"))?;
            let ts = p.get_block_timestamp(block_num).await?;
            let addr_str = format!("{address:#x}");
            upsert_bracket_submitted(&mut index, &addr_str, block_num, ts);
        }

        if !logs.is_empty() {
            println!(
                "  blocks {from}..{to}: {} BracketSubmitted logs",
                logs.len()
            );
        }
        from = to + 1;
    }

    // Pass 2: TagSet events
    println!("Scanning TagSet events...");
    from = from_block;
    while from <= latest_block {
        let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);
        let logs = p
            .get_tag_set_logs(contract_addr, from, to)
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            let (address, tag) = provider::parse_tag_set(log)?;
            let addr_str = format!("{address:#x}");
            update_tag(&mut index, &addr_str, tag);
        }

        if !logs.is_empty() {
            println!("  blocks {from}..{to}: {} TagSet logs", logs.len());
        }
        from = to + 1;
    }

    save_index(index_path, &index)?;
    println!(
        "Backfill complete. {} entries ({} new)",
        index.len(),
        index.len() - initial_count
    );

    // Sanity check
    println!("Running sanity check...");
    let on_chain_count = p.get_entry_count(contract_addr).await?;
    let local_count = index.len() as u32;
    if on_chain_count == local_count {
        println!(
            "Sanity check passed: {} entries match on-chain",
            local_count
        );
    } else {
        println!(
            "WARNING: entry count mismatch — local: {}, on-chain: {}",
            local_count, on_chain_count
        );
    }

    Ok(())
}
