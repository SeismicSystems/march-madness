//! Sanity check: compare Redis entry count with on-chain getEntryCount().

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

    let on_chain = p.get_entry_count(contract_addr).await?;
    let local = redis_store::get_entry_count(redis).await? as u32;

    info!(local, on_chain, "entry counts");

    if local == on_chain {
        info!("OK — counts match");
    } else {
        info!(local, on_chain, "MISMATCH — consider running backfill");
    }

    Ok(())
}
