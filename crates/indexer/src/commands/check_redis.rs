//! Redis-internal consistency check for stored counts.
//!
//! Verifies that denormalized counters match the actual data in Redis.
//! Compares `member_count` in KEY_GROUPS (metadata) against the actual
//! member array length in KEY_GROUP_MEMBERS.
//!
//! Does NOT require RPC access — purely local checks.
//!
//! Modes:
//! - (default): total entry HLEN
//! - `--group <slug>`: specific group's member_count vs actual members
//! - `--all-groups`: all groups

use crate::redis_store;
use eyre::{Result, eyre};
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::KEY_ENTRIES;
use tracing::{error, info};

/// What to check.
pub enum CheckMode {
    /// Check total entry HLEN.
    Total,
    /// Check a specific group by slug.
    Group(String),
    /// Check all groups.
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
    let hlen: usize = redis::cmd("HLEN")
        .arg(KEY_ENTRIES)
        .query_async(redis)
        .await?;

    info!(hlen, "entry hash length");

    Ok(())
}

async fn check_group(redis: &mut MultiplexedConnection, slug: &str) -> Result<()> {
    let Some((id, data)) = redis_store::get_group_by_slug(redis, slug).await? else {
        return Err(eyre!("group not found: {slug}"));
    };

    let members = redis_store::get_group_members(redis, &id).await?;
    let actual = members.len() as u32;
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
        error!("MISMATCH: member_count in metadata != members array length");
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
        let members = redis_store::get_group_members(redis, id).await?;
        let actual = members.len() as u32;
        if data.member_count != actual {
            error!(
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
        error!(mismatches, total = groups.len(), "group check complete");
    }

    Ok(())
}
