//! Historical backfill: scan all blocks for events across all contracts, write to Redis.

use crate::ParsedAddresses;
use crate::provider::{self, IndexerProvider};
use crate::redis_store;
use alloy_rpc_types_eth::Log;
use eyre::{Result, WrapErr};
use redis::aio::MultiplexedConnection;
use tracing::info;

/// Maximum block range per eth_getLogs request.
const BATCH_SIZE: u64 = 10_000;

/// Sort key for ordering events: (block_number, log_index).
fn log_sort_key(log: &Log) -> (u64, u64) {
    (log.block_number.unwrap_or(0), log.log_index.unwrap_or(0))
}

pub async fn run(
    p: &IndexerProvider,
    redis: &mut MultiplexedConnection,
    addrs: &ParsedAddresses,
    from_block: u64,
) -> Result<()> {
    let mm_addr = addrs.march_madness;
    let groups_addr = addrs.bracket_groups;
    let mirror_addr = addrs.bracket_mirror;

    let latest_block = p.block_number().await?;
    info!(from = from_block, to = latest_block, "backfilling");

    // Cache block timestamps to avoid duplicate RPC calls for events in the same block.
    let mut ts_cache: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

    // ── Pass 1: MarchMadness — BracketSubmitted ──────────────────────
    info!("scanning BracketSubmitted events");
    let mut entry_count = 0u32;
    let mut from = from_block;
    while from <= latest_block {
        let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);
        let logs = p
            .get_bracket_submitted_logs(mm_addr, from, to)
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            let address = provider::parse_bracket_submitted(log)?;
            let block_num = log
                .block_number
                .ok_or_else(|| eyre::eyre!("log missing block number"))?;
            let ts = match ts_cache.get(&block_num) {
                Some(&ts) => ts,
                None => {
                    let ts = p.get_block_timestamp(block_num).await?;
                    ts_cache.insert(block_num, ts);
                    ts
                }
            };
            let addr_str = format!("{address:#x}");
            redis_store::upsert_bracket_submitted(redis, &addr_str, block_num, ts).await?;
            entry_count += 1;
        }

        if !logs.is_empty() {
            info!(
                blocks = format!("{from}..{to}"),
                count = logs.len(),
                "BracketSubmitted"
            );
        }
        from = to + 1;
    }

    // ── Pass 2: MarchMadness — TagSet ────────────────────────────────
    info!("scanning TagSet events");
    from = from_block;
    while from <= latest_block {
        let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);
        let logs = p
            .get_tag_set_logs(mm_addr, from, to)
            .await
            .wrap_err_with(|| format!("get_logs failed for blocks {from}..{to}"))?;

        for log in &logs {
            let (address, tag) = provider::parse_tag_set(log)?;
            let addr_str = format!("{address:#x}");
            redis_store::update_tag(redis, &addr_str, &tag).await?;
        }

        if !logs.is_empty() {
            info!(
                blocks = format!("{from}..{to}"),
                count = logs.len(),
                "TagSet"
            );
        }
        from = to + 1;
    }

    // ── Pass 3: BracketGroups (sorted by block + log_index) ─────────
    {
        let groups = groups_addr;
        info!("scanning BracketGroups events");
        from = from_block;
        while from <= latest_block {
            let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);

            let created = p.get_group_created_logs(groups, from, to).await?;
            let joined = p.get_member_joined_logs(groups, from, to).await?;
            let left = p.get_member_left_logs(groups, from, to).await?;

            let mut events: Vec<((u64, u64), GroupEvent)> =
                Vec::with_capacity(created.len() + joined.len() + left.len());
            for log in &created {
                events.push((log_sort_key(log), GroupEvent::Created(log)));
            }
            for log in &joined {
                events.push((log_sort_key(log), GroupEvent::Joined(log)));
            }
            for log in &left {
                events.push((log_sort_key(log), GroupEvent::Left(log)));
            }
            events.sort_by_key(|(key, _)| *key);

            for (_, event) in &events {
                match event {
                    GroupEvent::Created(log) => {
                        let (id, slug, display_name, creator, has_password) =
                            provider::parse_group_created(log)?;
                        let creator_str = format!("{creator:#x}");
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
                        redis_store::member_joined(redis, id, &addr_str).await?;
                    }
                    GroupEvent::Left(log) => {
                        let (id, addr) = provider::parse_member_left(log)?;
                        let addr_str = format!("{addr:#x}");
                        redis_store::member_left(redis, id, &addr_str).await?;
                    }
                }
            }

            let total = created.len() + joined.len() + left.len();
            if total > 0 {
                info!(
                    blocks = format!("{from}..{to}"),
                    created = created.len(),
                    joined = joined.len(),
                    left = left.len(),
                    "BracketGroups"
                );
            }
            from = to + 1;
        }
    }

    // ── Pass 4: BracketMirror (sorted by block + log_index) ─────────
    {
        let mirror = mirror_addr;
        info!("scanning BracketMirror events");
        from = from_block;
        while from <= latest_block {
            let to = std::cmp::min(from + BATCH_SIZE - 1, latest_block);

            let created = p.get_mirror_created_logs(mirror, from, to).await?;
            let added = p.get_entry_added_logs(mirror, from, to).await?;
            let removed = p.get_entry_removed_logs(mirror, from, to).await?;

            let mut events: Vec<((u64, u64), MirrorEvent)> =
                Vec::with_capacity(created.len() + added.len() + removed.len());
            for log in &created {
                events.push((log_sort_key(log), MirrorEvent::Created(log)));
            }
            for log in &added {
                events.push((log_sort_key(log), MirrorEvent::Added(log)));
            }
            for log in &removed {
                events.push((log_sort_key(log), MirrorEvent::Removed(log)));
            }
            events.sort_by_key(|(key, _)| *key);

            for (_, event) in &events {
                match event {
                    MirrorEvent::Created(log) => {
                        let (id, slug, display_name, admin) = provider::parse_mirror_created(log)?;
                        let admin_str = format!("{admin:#x}");
                        redis_store::create_mirror(redis, id, &slug, &display_name, &admin_str)
                            .await?;
                    }
                    MirrorEvent::Added(log) => {
                        let (mirror_id, slug) = provider::parse_entry_added(log)?;
                        let u256_id = alloy_primitives::U256::from(mirror_id);
                        match p
                            .get_mirror_entry_bracket(mirror, u256_id, slug.clone())
                            .await
                        {
                            Ok(bracket) => {
                                let bracket_hex = format!("0x{}", hex::encode(bracket.as_slice()));
                                redis_store::mirror_entry_added(
                                    redis,
                                    mirror_id,
                                    &slug,
                                    &bracket_hex,
                                )
                                .await?;
                            }
                            Err(e) => {
                                info!(mirror_id, slug = %slug, error = %e, "failed to read mirror entry bracket");
                                redis_store::mirror_entry_added(redis, mirror_id, &slug, "")
                                    .await?;
                            }
                        }
                    }
                    MirrorEvent::Removed(log) => {
                        let (mirror_id, slug) = provider::parse_entry_removed(log)?;
                        redis_store::mirror_entry_removed(redis, mirror_id, &slug).await?;
                    }
                }
            }

            let total = created.len() + added.len() + removed.len();
            if total > 0 {
                info!(
                    blocks = format!("{from}..{to}"),
                    created = created.len(),
                    added = added.len(),
                    removed = removed.len(),
                    "BracketMirror"
                );
            }
            from = to + 1;
        }
    }

    // ── Update cursor ────────────────────────────────────────────────
    redis_store::set_last_block(redis, latest_block).await?;

    info!(entries = entry_count, "backfill complete");

    // Sanity check (MarchMadness only).
    info!("running sanity check");
    let on_chain_count = p.get_entry_count(mm_addr).await?;
    let redis_count = redis_store::get_entry_count(redis).await? as u32;
    if on_chain_count == redis_count {
        info!(count = redis_count, "sanity check passed");
    } else {
        info!(
            local = redis_count,
            on_chain = on_chain_count,
            "WARNING: entry count mismatch"
        );
    }

    Ok(())
}

// ── Tagged event enums for sort-then-process ────────────────────────

enum GroupEvent<'a> {
    Created(&'a Log),
    Joined(&'a Log),
    Left(&'a Log),
}

enum MirrorEvent<'a> {
    Created(&'a Log),
    Added(&'a Log),
    Removed(&'a Log),
}
