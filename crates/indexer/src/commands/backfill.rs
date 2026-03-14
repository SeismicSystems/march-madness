//! Historical backfill: scan all blocks for BracketSubmitted and TagSet events.

use crate::indexer::{load_index, save_index, update_tag, upsert_bracket_submitted};
use crate::rpc::{
    RpcClient, address_from_topic, decode_abi_string, decode_hex_data, event_topic, parse_hex_u64,
};
use eyre::{Result, WrapErr};
use std::path::Path;

/// Maximum block range per eth_getLogs request.
const BATCH_SIZE: u64 = 10_000;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path, from_block: u64) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let latest_block = client.block_number().await?;
    println!(
        "Backfilling from block {} to {} (latest)",
        from_block, latest_block
    );

    let topic_bracket = event_topic("BracketSubmitted(address)");
    let topic_tag = event_topic("TagSet(address,string)");

    // We query for both event types in a single filter using topic0 = null (any).
    // Actually, eth_getLogs supports arrays for topic0, but not all nodes do.
    // We'll do two passes: one for BracketSubmitted, one for TagSet.

    let mut index = load_index(index_path)?;
    let initial_count = index.len();

    // Pass 1: BracketSubmitted events
    println!("Scanning BracketSubmitted events...");
    let mut from = from_block;
    while from <= latest_block {
        let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);
        let logs = client
            .get_logs(
                contract,
                std::slice::from_ref(&topic_bracket),
                from,
                &format!("0x{:x}", to),
            )
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            if log.topics.len() < 2 {
                continue;
            }
            let address = address_from_topic(&log.topics[1])?;
            let block_num = parse_hex_u64(&log.block_number)?;
            let ts = client.get_block_timestamp(block_num).await?;
            upsert_bracket_submitted(
                &mut index,
                &format!("0x{}", hex::encode(address.as_slice())),
                block_num,
                ts,
            );
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
        let logs = client
            .get_logs(
                contract,
                std::slice::from_ref(&topic_tag),
                from,
                &format!("0x{:x}", to),
            )
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            if log.topics.len() < 2 {
                continue;
            }
            let address = address_from_topic(&log.topics[1])?;
            let data_bytes = decode_hex_data(&log.data)?;
            let tag = decode_abi_string(&data_bytes)?;
            update_tag(
                &mut index,
                &format!("0x{}", hex::encode(address.as_slice())),
                tag,
            );
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
    let on_chain_count = client.get_entry_count(contract).await?;
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
