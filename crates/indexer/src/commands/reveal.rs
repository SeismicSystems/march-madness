//! Post-deadline bracket reveal: read brackets for all indexed addresses.

use crate::indexer::{load_index, save_index, set_bracket};
use crate::provider;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use std::path::Path;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let p = provider::create_provider(rpc_url)?;
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;
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

        let address: Address = addr_str
            .parse()
            .wrap_err_with(|| format!("bad address: {addr_str}"))?;

        match provider::get_bracket(&p, contract_addr, address).await {
            Ok(bracket) => {
                let bracket_hex = format!("0x{}", hex::encode(bracket.as_slice()));
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
