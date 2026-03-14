//! Sanity check: compare local index entry count with on-chain getEntryCount().

use crate::indexer::load_index;
use crate::provider;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use std::path::Path;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let p = provider::create_provider(rpc_url)?;
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;
    let index = load_index(index_path)?;

    let on_chain = provider::get_entry_count(&p, contract_addr).await?;
    let local = index.len() as u32;

    println!("Local index entries:   {}", local);
    println!("On-chain entry count:  {}", on_chain);

    if local == on_chain {
        println!("OK — counts match.");
    } else {
        println!(
            "MISMATCH — local has {} entries, on-chain has {}. Consider running backfill.",
            local, on_chain
        );
    }

    Ok(())
}
