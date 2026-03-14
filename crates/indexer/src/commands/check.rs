//! Sanity check: compare local index entry count with on-chain getEntryCount().

use crate::indexer::load_index;
use crate::rpc::RpcClient;
use eyre::Result;
use std::path::Path;

pub async fn run(rpc_url: &str, contract: &str, index_path: &Path) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let index = load_index(index_path)?;

    let on_chain = client.get_entry_count(contract).await?;
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
