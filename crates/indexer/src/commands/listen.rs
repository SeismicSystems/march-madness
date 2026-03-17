//! Live event listener: polls for new events across all contracts, writes to Redis.

use crate::ParsedAddresses;
use crate::provider::{self, IndexerProvider};
use crate::redis_store;
use alloy_primitives::Address;
use alloy_rpc_types_eth::Log;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tokio::signal;
use tracing::info;

/// Poll interval in seconds.
const POLL_INTERVAL_SECS: u64 = 5;

/// Sort key for ordering events: (block_number, log_index).
fn log_sort_key(log: &Log) -> (u64, u64) {
    (log.block_number.unwrap_or(0), log.log_index.unwrap_or(0))
}

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    addrs: &ParsedAddresses,
) -> Result<()> {
    let mm_addr = addrs.march_madness;
    let groups_addr = addrs.bracket_groups;
    let mirror_addr = addrs.bracket_mirror;

    // Resume from last processed block or start from latest.
    let mut last_block = redis_store::get_last_block(redis)
        .await?
        .unwrap_or(p.block_number().await?);

    info!(from_block = last_block, "listening for events");

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("shutting down");
                return Ok(());
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)) => {
                let current_block = p.block_number().await?;
                if current_block <= last_block {
                    continue;
                }

                let from = last_block + 1;
                let mut event_count = 0u32;

                event_count += process_march_madness(p, redis, mm_addr, from, current_block).await?;
                event_count += process_groups(p, redis, groups_addr, from, current_block).await?;
                event_count += process_mirror(p, redis, mirror_addr, from, current_block).await?;

                // Update cursor.
                redis_store::set_last_block(redis, current_block).await?;

                if event_count > 0 {
                    info!(
                        events = event_count,
                        blocks = format!("{}..{}", from, current_block),
                        "indexed"
                    );
                }

                last_block = current_block;
            }
        }
    }
}

async fn process_march_madness(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    from: u64,
    to: u64,
) -> Result<u32> {
    let mut count = 0u32;

    let bracket_logs = p
        .get_bracket_submitted_logs(contract, from, to)
        .await
        .wrap_err("failed to fetch BracketSubmitted logs")?;

    for log in &bracket_logs {
        let address = provider::parse_bracket_submitted(log)?;
        let block_num = log
            .block_number
            .ok_or_else(|| eyre::eyre!("log missing block number"))?;
        let ts = p.get_block_timestamp(block_num).await?;
        let addr_str = format!("{address:#x}");
        info!(event = "BracketSubmitted", addr = %addr_str, block = block_num);
        redis_store::upsert_bracket_submitted(redis, &addr_str, block_num, ts).await?;
        count += 1;
    }

    let tag_logs = p
        .get_tag_set_logs(contract, from, to)
        .await
        .wrap_err("failed to fetch TagSet logs")?;

    for log in &tag_logs {
        let (address, tag) = provider::parse_tag_set(log)?;
        let addr_str = format!("{address:#x}");
        info!(event = "TagSet", addr = %addr_str, tag = %tag);
        redis_store::update_tag(redis, &addr_str, &tag).await?;
        count += 1;
    }

    Ok(count)
}

// ── Group events, sorted by (block, log_index) ─────────────────────

enum GroupEvent<'a> {
    Created(&'a Log),
    Joined(&'a Log),
    Left(&'a Log),
}

async fn process_groups(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    from: u64,
    to: u64,
) -> Result<u32> {
    let created_logs = p.get_group_created_logs(contract, from, to).await?;
    let joined_logs = p.get_member_joined_logs(contract, from, to).await?;
    let left_logs = p.get_member_left_logs(contract, from, to).await?;

    let mut events: Vec<((u64, u64), GroupEvent)> =
        Vec::with_capacity(created_logs.len() + joined_logs.len() + left_logs.len());
    for log in &created_logs {
        events.push((log_sort_key(log), GroupEvent::Created(log)));
    }
    for log in &joined_logs {
        events.push((log_sort_key(log), GroupEvent::Joined(log)));
    }
    for log in &left_logs {
        events.push((log_sort_key(log), GroupEvent::Left(log)));
    }
    events.sort_by_key(|(key, _)| *key);

    let mut count = 0u32;
    for (_, event) in &events {
        match event {
            GroupEvent::Created(log) => {
                let (id, slug, display_name, creator, has_password) =
                    provider::parse_group_created(log)?;
                let creator_str = format!("{creator:#x}");
                info!(event = "GroupCreated", id, slug = %slug);
                redis_store::create_group(
                    redis,
                    id,
                    &slug,
                    &display_name,
                    &creator_str,
                    has_password,
                )
                .await?;
            }
            GroupEvent::Joined(log) => {
                let (id, addr) = provider::parse_member_joined(log)?;
                let addr_str = format!("{addr:#x}");
                info!(event = "MemberJoined", group_id = id, addr = %addr_str);
                redis_store::member_joined(redis, id, &addr_str).await?;
            }
            GroupEvent::Left(log) => {
                let (id, addr) = provider::parse_member_left(log)?;
                let addr_str = format!("{addr:#x}");
                info!(event = "MemberLeft", group_id = id, addr = %addr_str);
                redis_store::member_left(redis, id, &addr_str).await?;
            }
        }
        count += 1;
    }

    Ok(count)
}

// ── Mirror events, sorted by (block, log_index) ────────────────────

enum MirrorEvent<'a> {
    Created(&'a Log),
    Added(&'a Log),
    Removed(&'a Log),
}

async fn process_mirror(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    contract: Address,
    from: u64,
    to: u64,
) -> Result<u32> {
    let created_logs = p.get_mirror_created_logs(contract, from, to).await?;
    let added_logs = p.get_entry_added_logs(contract, from, to).await?;
    let removed_logs = p.get_entry_removed_logs(contract, from, to).await?;

    let mut events: Vec<((u64, u64), MirrorEvent)> =
        Vec::with_capacity(created_logs.len() + added_logs.len() + removed_logs.len());
    for log in &created_logs {
        events.push((log_sort_key(log), MirrorEvent::Created(log)));
    }
    for log in &added_logs {
        events.push((log_sort_key(log), MirrorEvent::Added(log)));
    }
    for log in &removed_logs {
        events.push((log_sort_key(log), MirrorEvent::Removed(log)));
    }
    events.sort_by_key(|(key, _)| *key);

    let mut count = 0u32;
    for (_, event) in &events {
        match event {
            MirrorEvent::Created(log) => {
                let (id, slug, display_name, admin) = provider::parse_mirror_created(log)?;
                let admin_str = format!("{admin:#x}");
                info!(event = "MirrorCreated", id, slug = %slug);
                redis_store::create_mirror(redis, id, &slug, &display_name, &admin_str).await?;
            }
            MirrorEvent::Added(log) => {
                let (mirror_id, slug) = provider::parse_entry_added(log)?;
                let u256_id = alloy_primitives::U256::from(mirror_id);
                match p
                    .get_mirror_entry_bracket(contract, u256_id, slug.clone())
                    .await
                {
                    Ok(bracket) => {
                        let bracket_hex = format!("0x{}", hex::encode(bracket.as_slice()));
                        info!(event = "EntryAdded", mirror_id, slug = %slug, bracket = %bracket_hex);
                        redis_store::mirror_entry_added(redis, mirror_id, &slug, &bracket_hex)
                            .await?;
                    }
                    Err(e) => {
                        info!(event = "EntryAdded", mirror_id, slug = %slug, error = %e, "failed to read bracket, storing without it");
                        redis_store::mirror_entry_added(redis, mirror_id, &slug, "").await?;
                    }
                }
            }
            MirrorEvent::Removed(log) => {
                let (mirror_id, slug) = provider::parse_entry_removed(log)?;
                info!(event = "EntryRemoved", mirror_id, slug = %slug);
                redis_store::mirror_entry_removed(redis, mirror_id, &slug).await?;
            }
        }
        count += 1;
    }

    Ok(count)
}
