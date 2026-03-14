//! Live event listener: polls for new BracketSubmitted and TagSet events.

use crate::indexer::{load_index, save_index, update_tag, upsert_bracket_submitted};
use crate::rpc::{
    RpcClient, address_from_topic, decode_abi_string, decode_hex_data, event_topic, parse_hex_u64,
};
use eyre::{Result, WrapErr};
use std::path::Path;
use tokio::signal;

/// Poll interval in seconds.
const POLL_INTERVAL_SECS: u64 = 5;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let topic_bracket = event_topic("BracketSubmitted(address)");
    let topic_tag = event_topic("TagSet(address,string)");

    // Start from the latest block
    let mut last_block = client.block_number().await?;
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
                let current_block = client.block_number().await?;
                if current_block <= last_block {
                    continue;
                }

                let from = last_block + 1;
                let to_hex = format!("0x{:x}", current_block);

                // Fetch BracketSubmitted logs
                let bracket_logs = client
                    .get_logs(contract, std::slice::from_ref(&topic_bracket), from, &to_hex)
                    .await
                    .wrap_err("failed to fetch BracketSubmitted logs")?;

                // Fetch TagSet logs
                let tag_logs = client
                    .get_logs(contract, std::slice::from_ref(&topic_tag), from, &to_hex)
                    .await
                    .wrap_err("failed to fetch TagSet logs")?;

                let mut updated = false;

                for log in &bracket_logs {
                    if log.topics.len() < 2 {
                        continue;
                    }
                    let address = address_from_topic(&log.topics[1])?;
                    let block_num = parse_hex_u64(&log.block_number)?;
                    let ts = client.get_block_timestamp(block_num).await?;
                    let addr_str = format!("0x{}", hex::encode(address.as_slice()));
                    println!(
                        "  BracketSubmitted: {} (block {})",
                        addr_str, block_num
                    );
                    upsert_bracket_submitted(&mut index, &addr_str, block_num, ts);
                    updated = true;
                }

                for log in &tag_logs {
                    if log.topics.len() < 2 {
                        continue;
                    }
                    let address = address_from_topic(&log.topics[1])?;
                    let data_bytes = decode_hex_data(&log.data)?;
                    let tag = decode_abi_string(&data_bytes)?;
                    let addr_str = format!("0x{}", hex::encode(address.as_slice()));
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
