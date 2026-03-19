//! Backfill mirrors: read metadata + entries from the contract, write to Redis.

use crate::provider::IndexerProvider;
use crate::redis_store;
use alloy_primitives::Address;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tracing::info;

/// Backfill a single mirror (metadata + entries) from chain into Redis.
async fn backfill_one(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    mirror_id: u64,
) -> Result<()> {
    let u256_id = alloy_primitives::U256::from(mirror_id);

    // Fetch and write mirror metadata
    let mirror = p
        .get_mirror(contract, u256_id)
        .await
        .wrap_err("failed to read mirror metadata from contract")?;

    let admin = format!("{:#x}", mirror.admin);
    redis_store::create_mirror(redis, mirror_id, &mirror.slug, &mirror.displayName, &admin)
        .await
        .wrap_err("failed to write mirror metadata to Redis")?;
    info!(
        mirror_id,
        slug = %mirror.slug,
        display_name = %mirror.displayName,
        admin = %admin,
        "wrote mirror metadata"
    );

    // Fetch and write entries
    let entries = p
        .get_mirror_entries(contract, u256_id)
        .await
        .wrap_err("failed to read mirror entries from contract")?;

    if entries.is_empty() {
        info!(mirror_id, "no entries found on-chain");
        return Ok(());
    }

    let mut written = 0u32;
    let mut empty = 0u32;

    for entry in &entries {
        let bracket_hex = format!("0x{}", hex::encode(entry.bracket.as_slice()));

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
        "backfill complete for mirror"
    );
    Ok(())
}

/// Backfill a specific mirror or all mirrors from chain into Redis.
pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    mirror_id: Option<u64>,
) -> Result<()> {
    match mirror_id {
        Some(id) => backfill_one(p, redis, contract, id).await,
        None => {
            let next_id = p
                .get_next_mirror_id(contract)
                .await
                .wrap_err("failed to read nextMirrorId from contract")?;

            if next_id <= 1 {
                info!("no mirrors found on-chain");
                return Ok(());
            }

            info!(count = next_id - 1, "backfilling all mirrors");

            for id in 1..next_id {
                backfill_one(p, redis, contract, id).await?;
            }

            info!(count = next_id - 1, "backfill-mirror complete");
            Ok(())
        }
    }
}
