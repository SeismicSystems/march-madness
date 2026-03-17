//! Post-deadline bracket reveal: read brackets for all indexed addresses, write to Redis.

use crate::provider::IndexerProvider;
use crate::redis_store;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tracing::info;

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: &str,
) -> Result<()> {
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;

    let addresses = redis_store::get_all_entry_addresses(redis).await?;
    if addresses.is_empty() {
        info!("no entries in Redis, run backfill first");
        return Ok(());
    }

    info!(count = addresses.len(), "revealing brackets");

    let mut revealed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    for addr_str in &addresses {
        // Skip entries that already have a bracket.
        if let Some(entry) = redis_store::get_entry(redis, addr_str).await?
            && entry.bracket.is_some()
        {
            skipped += 1;
            continue;
        }

        let address: Address = addr_str
            .parse()
            .wrap_err_with(|| format!("bad address: {addr_str}"))?;

        match p.get_bracket(contract_addr, address).await {
            Ok(bracket) => {
                let bracket_hex = format!("0x{}", hex::encode(bracket.as_slice()));
                redis_store::set_bracket(redis, addr_str, &bracket_hex).await?;
                info!(addr = %addr_str, bracket = %bracket_hex, "revealed");
                revealed += 1;
            }
            Err(e) => {
                info!(addr = %addr_str, error = %e, "failed to read bracket");
                failed += 1;
            }
        }
    }

    info!(
        revealed,
        failed,
        skipped,
        total = addresses.len(),
        "reveal complete"
    );
    Ok(())
}
