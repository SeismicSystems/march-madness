//! Sanity check: compare Redis counts with on-chain contract state.

use crate::provider::IndexerProvider;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::{KEY_ENTRIES, KEY_ENTRY_COUNT};
use tracing::info;

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: &str,
) -> Result<()> {
    let contract_addr: Address = contract.parse().wrap_err("invalid contract address")?;

    // Read counter and HLEN atomically via pipeline.
    let (counter, hlen): (Option<u64>, u64) = redis::pipe()
        .get(KEY_ENTRY_COUNT)
        .hlen(KEY_ENTRIES)
        .query_async(redis)
        .await
        .wrap_err("failed to read Redis counts")?;

    let counter = counter.unwrap_or(0);
    let on_chain = p.get_entry_count(contract_addr).await?;

    info!(counter, hlen, on_chain, "entry counts");

    let mut ok = true;
    if counter != hlen {
        info!(counter, hlen, "MISMATCH: counter key != HLEN");
        ok = false;
    }
    if hlen != on_chain as u64 {
        info!(hlen, on_chain, "MISMATCH: HLEN != on-chain");
        ok = false;
    }
    if ok {
        info!("OK — all entry counts match");
    }

    Ok(())
}
