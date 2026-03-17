//! Redis-internal consistency check for stored counts.
//!
//! Verifies that denormalized counters match the actual data in Redis.
//! Does NOT require RPC access — purely local checks.
//!
//! Modes:
//! - `--total` (default): counter key vs HLEN of entries hash
//! - `--group <slug>`: specific group's member_count vs members.len()
//! - `--all-groups`: all groups

use crate::redis_store;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::{KEY_ENTRIES, KEY_ENTRY_COUNT};
use tracing::info;

/// What to check.
pub enum CheckMode {
    Total,
    Group(String),
    AllGroups,
}

pub async fn run(redis: &mut MultiplexedConnection, mode: CheckMode) -> Result<()> {
    match mode {
        CheckMode::Total => check_total(redis).await,
        CheckMode::Group(slug) => check_group(redis, &slug).await,
        CheckMode::AllGroups => check_all_groups(redis).await,
    }
}

async fn check_total(redis: &mut MultiplexedConnection) -> Result<()> {
    // Read counter and HLEN atomically via pipeline.
    let (counter, hlen): (Option<u64>, u64) = redis::pipe()
        .get(KEY_ENTRY_COUNT)
        .hlen(KEY_ENTRIES)
        .query_async(redis)
        .await
        .wrap_err("failed to read Redis counts")?;

    let counter = counter.unwrap_or(0);

    info!(counter, hlen, "entry counts");

    if counter == hlen {
        info!("OK — counter matches HLEN");
    } else {
        info!(counter, hlen, "MISMATCH: counter key != HLEN");
    }

    Ok(())
}

async fn check_group(redis: &mut MultiplexedConnection, slug: &str) -> Result<()> {
    let Some((id, data)) = redis_store::get_group_by_slug(redis, slug).await? else {
        info!(slug, "group not found");
        return Ok(());
    };

    let actual = data.members.len() as u32;
    info!(
        group_id = %id,
        slug = %data.slug,
        stored_count = data.member_count,
        actual_count = actual,
        "group member counts"
    );

    if data.member_count == actual {
        info!("OK — group member count matches");
    } else {
        info!("MISMATCH: stored member_count != members.len()");
    }

    Ok(())
}

async fn check_all_groups(redis: &mut MultiplexedConnection) -> Result<()> {
    let groups = redis_store::get_all_groups(redis).await?;

    if groups.is_empty() {
        info!("no groups found");
        return Ok(());
    }

    let mut mismatches = 0u32;
    for (id, data) in &groups {
        let actual = data.members.len() as u32;
        if data.member_count != actual {
            info!(
                group_id = %id,
                slug = %data.slug,
                stored_count = data.member_count,
                actual_count = actual,
                "MISMATCH"
            );
            mismatches += 1;
        }
    }

    if mismatches == 0 {
        info!(groups = groups.len(), "OK — all group member counts match");
    } else {
        info!(mismatches, total = groups.len(), "group check complete");
    }

    Ok(())
}
