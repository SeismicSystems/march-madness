//! Post-deadline bracket reveal: read brackets for all indexed addresses.

use crate::indexer::{load_index, save_index, set_bracket};
use crate::rpc::RpcClient;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use std::path::Path;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let mut index = load_index(index_path)?;

    if index.is_empty() {
        println!("No entries in index. Run backfill first.");
        return Ok(());
    }

    println!(
        "Revealing brackets for {} entries (post-deadline)...",
        index.len()
    );

    let addresses: Vec<String> = index.keys().cloned().collect();
    let mut revealed = 0u32;
    let mut failed = 0u32;

    for addr_str in &addresses {
        // Skip entries that already have a bracket
        if index.get(addr_str).is_some_and(|e| e.bracket.is_some()) {
            continue;
        }

        let addr_bytes = hex::decode(addr_str.strip_prefix("0x").unwrap_or(addr_str))
            .wrap_err_with(|| format!("bad address hex: {addr_str}"))?;
        if addr_bytes.len() != 20 {
            println!("  Skipping invalid address: {}", addr_str);
            continue;
        }
        let address = Address::from_slice(&addr_bytes);

        match client.get_bracket(contract, &address).await {
            Ok(bracket) => {
                let bracket_hex = format!("0x{}", hex::encode(bracket.as_slice()));
                // Skip zero brackets (should not happen, but just in case)
                if bracket_hex == "0x0000000000000000" {
                    println!("  {} — zero bracket, skipping", addr_str);
                    continue;
                }
                set_bracket(&mut index, addr_str, bracket_hex.clone());
                println!("  {} => {}", addr_str, bracket_hex);
                revealed += 1;
            }
            Err(e) => {
                println!("  {} — failed to read bracket: {}", addr_str, e);
                failed += 1;
            }
        }
    }

    save_index(index_path, &index)?;
    println!(
        "Reveal complete. {} revealed, {} failed, {} total entries",
        revealed,
        failed,
        index.len()
    );

    Ok(())
}
