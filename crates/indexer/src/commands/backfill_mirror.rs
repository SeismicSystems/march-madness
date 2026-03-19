//! Backfill mirror entries: read all entries for a mirror ID from the contract, write to Redis.

use crate::provider::IndexerProvider;
use crate::redis_store;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tracing::info;

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    mirror_id: u64,
) -> Result<()> {
    let u256_id = alloy_primitives::U256::from(mirror_id);

    let entries = p
        .get_mirror_entries(contract, u256_id)
        .await
        .wrap_err("failed to read mirror entries from contract")?;

    if entries.is_empty() {
        info!(mirror_id, "no entries found on-chain");
        return Ok(());
    }

    info!(
        mirror_id,
        count = entries.len(),
        "backfilling mirror entries"
    );

    let mut written = 0u32;
    let mut empty = 0u32;

    for entry in &entries {
        let bracket_hex = format!("0x{}", hex::encode(entry.bracket.as_slice()));

        // Skip zero brackets (shouldn't happen given sentinel check, but be safe).
        if entry.bracket == alloy_primitives::FixedBytes::ZERO {
            info!(mirror_id, slug = %entry.slug, "skipping zero bracket");
            empty += 1;
            continue;
        }

        redis_store::mirror_entry_added(redis, mirror_id, &entry.slug, &bracket_hex).await?;
        info!(mirror_id, slug = %entry.slug, bracket = %bracket_hex, "wrote entry");
        written += 1;
    }

    info!(
        mirror_id,
        written,
        empty,
        total = entries.len(),
        "backfill-mirror complete"
    );
    Ok(())
}
