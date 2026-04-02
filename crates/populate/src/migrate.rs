//! Core migration logic: read V1 contract state, convert encoding, write to V2.
//!
//! Source of truth: V1 contracts (events for discovery, reads for data).
//! Progress tracking: Redis keys scoped to the migration (`mm:migrate:*`).

use std::collections::HashSet;

use alloy_network::ReceiptResponse;
use alloy_primitives::{Address, FixedBytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types_eth::Filter;
use alloy_sol_types::SolEvent;
use eyre::{Result, WrapErr, bail};
use futures::stream::{self, StreamExt};
use tracing::{info, warn};

use seismic_march_madness::migration::reverse_game_bits;

use crate::contract::{BracketGroups, BracketGroupsV2, MarchMadness, MarchMadnessV2};
use crate::provider::{ReadProvider, SignedProvider};

/// Redis keys for migration progress tracking.
const KEY_MIGRATE_ENTRIES_DONE: &str = "mm:migrate:entries_done";
const KEY_MIGRATE_TAGS_DONE: &str = "mm:migrate:tags_done";
const KEY_MIGRATE_GROUPS_DONE: &str = "mm:migrate:groups_done";

/// Maximum block range per eth_getLogs request.
const LOG_BATCH_SIZE: u64 = 10_000;

/// Concurrent RPC reads for snapshot_entries.
const SNAPSHOT_CONCURRENCY: usize = 20;

// ── Provider dispatch macros (must be before first use) ─────────────

macro_rules! dispatch_read {
    ($reader:expr, |$p:ident| $body:expr) => {
        match $reader {
            ReadProvider::Reth($p) => $body,
            ReadProvider::Foundry($p) => $body,
        }
    };
}

macro_rules! dispatch_write {
    ($writer:expr, $target:expr, $Contract:ident, |$contract:ident| $body:expr) => {
        match $writer {
            SignedProvider::Reth(p) => {
                let $contract = $Contract::new($target, p);
                $body
            }
            SignedProvider::Foundry(p) => {
                let $contract = $Contract::new($target, p);
                $body
            }
        }
    };
}

// ── Entry types ─────────────────────────────────────────────────────

struct SnapshotEntry {
    address: Address,
    /// Contract-correct bracket bytes (already reversed from legacy).
    bracket: FixedBytes<8>,
    tag: Option<String>,
}

struct SnapshotGroup {
    id: u32,
    slug: String,
    display_name: String,
    creator: Address,
    entry_fee: U256,
    members: Vec<(Address, String)>, // (address, display_name)
}

/// Common configuration for migration operations.
pub struct MigrateConfig<'a> {
    pub reader: &'a ReadProvider,
    pub writer: Option<&'a SignedProvider>,
    pub redis: redis::Connection,
    pub from_block: u64,
    pub batch_size: usize,
}

/// Canonical address string for Redis keys.
fn addr_key(addr: Address) -> String {
    format!("{:#x}", addr)
}

// ── Address derivation ──────────────────────────────────────────────

/// Read the MarchMadness address from a V1 BracketGroups contract.
pub async fn read_march_madness_addr(reader: &ReadProvider, bg: Address) -> Result<Address> {
    dispatch_read!(reader, |p| {
        let contract = BracketGroups::new(bg, p);
        let addr = contract.marchMadness().call().await?;
        Ok(addr)
    })
}

/// Read the MarchMadness address from a V2 BracketGroupsV2 contract.
pub async fn read_march_madness_addr_v2(reader: &ReadProvider, bg: Address) -> Result<Address> {
    dispatch_read!(reader, |p| {
        let contract = BracketGroupsV2::new(bg, p);
        let addr = contract.marchMadness().call().await?;
        Ok(addr)
    })
}

/// Read the entry fee from a MarchMadness(V2) contract.
async fn read_entry_fee(reader: &ReadProvider, mm: Address) -> Result<U256> {
    dispatch_read!(reader, |p| {
        let contract = MarchMadnessV2::new(mm, p);
        let fee = contract.entryFee().call().await?;
        Ok(fee)
    })
}

// ── Entry migration ─────────────────────────────────────────────────

pub async fn run_entries(
    cfg: &mut MigrateConfig<'_>,
    source: Address,
    target: Address,
) -> Result<()> {
    let addresses = discover_entries(cfg.reader, source, cfg.from_block).await?;
    info!(
        count = addresses.len(),
        "discovered entry addresses from V1 events"
    );

    if addresses.is_empty() {
        info!("no entries found in V1 contract");
        return Ok(());
    }

    let entry_fee = read_entry_fee(cfg.reader, target).await?;
    info!(entry_fee = %entry_fee, "V2 entry fee");

    let already_done = load_done_set(&mut cfg.redis, KEY_MIGRATE_ENTRIES_DONE)?;
    let already_tagged = load_done_set(&mut cfg.redis, KEY_MIGRATE_TAGS_DONE)?;

    let entries = snapshot_entries(cfg.reader, source, &addresses, &already_done).await?;
    let tags_to_import: Vec<&SnapshotEntry> = entries
        .iter()
        .filter(|e| e.tag.is_some() && !already_tagged.contains(&addr_key(e.address)))
        .collect();

    info!(
        entries = entries.len(),
        tags = tags_to_import.len(),
        skipped = addresses.len() - entries.len(),
        "snapshot complete"
    );

    if entries.is_empty() && tags_to_import.is_empty() {
        info!("all entries already migrated");
        return Ok(());
    }

    let writer = match cfg.writer {
        Some(w) => w,
        None => {
            info!(
                "dry run — would import {} entries and {} tags",
                entries.len(),
                tags_to_import.len(),
            );
            for e in &entries {
                info!(
                    address = %e.address,
                    bracket = %e.bracket,
                    tag = e.tag.as_deref().unwrap_or("<none>"),
                    "would import entry"
                );
            }
            return Ok(());
        }
    };

    if !entries.is_empty() {
        import_entries_batched(
            writer,
            target,
            &entries,
            entry_fee,
            cfg.batch_size,
            &mut cfg.redis,
        )
        .await?;
    }

    // Tags imported one at a time — V2 contract has no batch variant
    if !tags_to_import.is_empty() {
        import_tags(writer, target, &tags_to_import, &mut cfg.redis).await?;
    }

    info!("entry migration complete");
    Ok(())
}

// ── Group migration ─────────────────────────────────────────────────

pub async fn run_groups(
    cfg: &mut MigrateConfig<'_>,
    source: Address,
    target: Address,
) -> Result<()> {
    let group_ids = discover_groups(cfg.reader, source, cfg.from_block).await?;
    info!(count = group_ids.len(), "discovered groups from V1 events");

    if group_ids.is_empty() {
        info!("no groups found in V1 contract");
        return Ok(());
    }

    let already_done = load_done_set(&mut cfg.redis, KEY_MIGRATE_GROUPS_DONE)?;

    let groups = snapshot_groups(cfg.reader, source, &group_ids, &already_done).await?;
    info!(
        groups = groups.len(),
        skipped = group_ids.len() - groups.len(),
        "group snapshot complete"
    );

    if groups.is_empty() {
        info!("all groups already migrated");
        return Ok(());
    }

    let writer = match cfg.writer {
        Some(w) => w,
        None => {
            for g in &groups {
                info!(
                    id = g.id,
                    slug = g.slug,
                    members = g.members.len(),
                    "would import group"
                );
            }
            return Ok(());
        }
    };

    for group in &groups {
        info!(
            id = group.id,
            slug = group.slug,
            members = group.members.len(),
            "importing group"
        );

        match send_import_group(writer, target, group).await {
            Ok(()) => {
                info!(slug = group.slug, "group created");
            }
            Err(e) => {
                warn!(slug = group.slug, error = %e, "group creation failed, skipping");
                continue;
            }
        }

        if !group.members.is_empty() {
            import_members_batched(
                writer,
                target,
                group.id,
                &group.members,
                group.entry_fee,
                cfg.batch_size,
            )
            .await?;
        }

        mark_done(
            &mut cfg.redis,
            KEY_MIGRATE_GROUPS_DONE,
            &group.id.to_string(),
        )?;
    }

    info!("group migration complete");
    Ok(())
}

// ── Event discovery ─────────────────────────────────────────────────

async fn discover_entries(
    reader: &ReadProvider,
    source: Address,
    from_block: u64,
) -> Result<Vec<Address>> {
    let latest = get_block_number(reader).await?;
    let mut addresses = HashSet::new();
    let mut from = from_block;

    while from <= latest {
        let to = std::cmp::min(from + LOG_BATCH_SIZE - 1, latest);
        let filter = Filter::new()
            .address(source)
            .event_signature(MarchMadness::BracketSubmitted::SIGNATURE_HASH)
            .from_block(from)
            .to_block(to);

        let logs = get_logs(reader, &filter).await?;
        for log in &logs {
            let decoded = MarchMadness::BracketSubmitted::decode_log(log.inner.as_ref())
                .wrap_err("failed to decode BracketSubmitted")?;
            addresses.insert(decoded.account);
        }

        from = to + 1;
    }

    let mut sorted: Vec<Address> = addresses.into_iter().collect();
    sorted.sort();
    Ok(sorted)
}

async fn discover_groups(
    reader: &ReadProvider,
    source: Address,
    from_block: u64,
) -> Result<Vec<u32>> {
    let latest = get_block_number(reader).await?;
    let mut ids = Vec::new();
    let mut from = from_block;

    while from <= latest {
        let to = std::cmp::min(from + LOG_BATCH_SIZE - 1, latest);
        let filter = Filter::new()
            .address(source)
            .event_signature(BracketGroups::GroupCreated::SIGNATURE_HASH)
            .from_block(from)
            .to_block(to);

        let logs = get_logs(reader, &filter).await?;
        for log in &logs {
            let decoded = BracketGroups::GroupCreated::decode_log(log.inner.as_ref())
                .wrap_err("failed to decode GroupCreated")?;
            ids.push(decoded.groupId);
        }

        from = to + 1;
    }

    ids.sort();
    ids.dedup();
    Ok(ids)
}

// ── V1 contract reads ───────────────────────────────────────────────

/// Read brackets and tags from V1 concurrently, converting to contract-correct encoding.
async fn snapshot_entries(
    reader: &ReadProvider,
    source: Address,
    addresses: &[Address],
    already_done: &HashSet<String>,
) -> Result<Vec<SnapshotEntry>> {
    let to_read: Vec<Address> = addresses
        .iter()
        .copied()
        .filter(|addr| !already_done.contains(&addr_key(*addr)))
        .collect();

    let total = to_read.len();
    info!(total, "reading V1 entries");

    let results: Vec<_> = stream::iter(to_read)
        .map(|addr| async move {
            let bracket = read_bracket(reader, source, addr).await;
            let tag = read_tag(reader, source, addr).await;
            (addr, bracket, tag)
        })
        .buffer_unordered(SNAPSHOT_CONCURRENCY)
        .collect()
        .await;

    let mut entries = Vec::with_capacity(results.len());
    for (addr, bracket_result, tag_result) in results {
        let bracket_bytes = match bracket_result {
            Ok(b) => b,
            Err(e) => {
                warn!(address = %addr, error = %e, "failed to read bracket, skipping");
                continue;
            }
        };

        let legacy_bits = u64::from_be_bytes(bracket_bytes.0);
        if legacy_bits == 0 {
            warn!(address = %addr, "bracket is zero, skipping");
            continue;
        }
        let contract_bits = reverse_game_bits(legacy_bits);
        let corrected = FixedBytes::<8>::from(contract_bits.to_be_bytes());

        let tag = match tag_result {
            Ok(t) if !t.is_empty() => Some(t),
            Ok(_) => None,
            Err(e) => {
                warn!(address = %addr, error = %e, "failed to read tag, continuing without");
                None
            }
        };

        entries.push(SnapshotEntry {
            address: addr,
            bracket: corrected,
            tag,
        });
    }

    info!(
        read = entries.len(),
        skipped = total - entries.len(),
        "V1 snapshot done"
    );
    Ok(entries)
}

/// Read group metadata and members from V1.
async fn snapshot_groups(
    reader: &ReadProvider,
    source: Address,
    group_ids: &[u32],
    already_done: &HashSet<String>,
) -> Result<Vec<SnapshotGroup>> {
    let mut groups = Vec::new();

    for &id in group_ids {
        if already_done.contains(&id.to_string()) {
            continue;
        }

        let group = match read_group(reader, source, id).await {
            Ok(g) => g,
            Err(e) => {
                warn!(id, error = %e, "failed to read group, skipping");
                continue;
            }
        };

        let members = match read_members(reader, source, id).await {
            Ok(m) => m,
            Err(e) => {
                warn!(id, error = %e, "failed to read members, importing group without members");
                Vec::new()
            }
        };

        groups.push(SnapshotGroup {
            id,
            slug: group.slug.clone(),
            display_name: group.displayName.clone(),
            creator: group.creator,
            entry_fee: group.entryFee,
            members: members
                .into_iter()
                .map(|m| (m.addr, m.name.clone()))
                .collect(),
        });
    }

    Ok(groups)
}

// ── Batch import helpers ────────────────────────────────────────────

async fn import_entries_batched(
    writer: &SignedProvider,
    target: Address,
    entries: &[SnapshotEntry],
    entry_fee: U256,
    batch_size: usize,
    redis: &mut redis::Connection,
) -> Result<()> {
    let total_batches = entries.len().div_ceil(batch_size);

    for (batch_idx, chunk) in entries.chunks(batch_size).enumerate() {
        let addresses: Vec<Address> = chunk.iter().map(|e| e.address).collect();
        let brackets: Vec<FixedBytes<8>> = chunk.iter().map(|e| e.bracket).collect();
        let payment = entry_fee * U256::from(chunk.len());

        info!(
            batch = batch_idx + 1,
            total = total_batches,
            size = chunk.len(),
            payment = %payment,
            "importing entries batch"
        );

        match send_batch_import_entries(writer, target, addresses, brackets, payment).await {
            Ok(()) => {
                for e in chunk {
                    mark_done(redis, KEY_MIGRATE_ENTRIES_DONE, &addr_key(e.address))?;
                }
                info!(batch = batch_idx + 1, "batch imported successfully");
            }
            Err(e) => {
                warn!(
                    batch = batch_idx + 1,
                    error = %e,
                    "batch import failed, continuing with next batch"
                );
            }
        }
    }

    Ok(())
}

async fn import_tags(
    writer: &SignedProvider,
    target: Address,
    entries: &[&SnapshotEntry],
    redis: &mut redis::Connection,
) -> Result<()> {
    let total = entries.len();
    let mut imported = 0;
    let mut failed = 0;

    for (i, entry) in entries.iter().enumerate() {
        let tag = entry.tag.as_deref().unwrap_or_default();

        if (i + 1) % 20 == 0 || i + 1 == total {
            info!(progress = format!("{}/{}", i + 1, total), "importing tags");
        }

        match send_import_tag(writer, target, entry.address, tag.to_string()).await {
            Ok(()) => {
                mark_done(redis, KEY_MIGRATE_TAGS_DONE, &addr_key(entry.address))?;
                imported += 1;
            }
            Err(e) => {
                warn!(address = %entry.address, error = %e, "tag import failed");
                failed += 1;
            }
        }
    }

    info!(imported, failed, "tag import complete");
    Ok(())
}

async fn import_members_batched(
    writer: &SignedProvider,
    target: Address,
    group_id: u32,
    members: &[(Address, String)],
    group_entry_fee: U256,
    batch_size: usize,
) -> Result<()> {
    let total_batches = members.len().div_ceil(batch_size);

    for (batch_idx, chunk) in members.chunks(batch_size).enumerate() {
        let addrs: Vec<Address> = chunk.iter().map(|(a, _)| *a).collect();
        let names: Vec<String> = chunk.iter().map(|(_, n)| n.clone()).collect();
        let payment = group_entry_fee * U256::from(chunk.len());

        info!(
            group = group_id,
            batch = batch_idx + 1,
            total = total_batches,
            size = chunk.len(),
            payment = %payment,
            "importing members batch"
        );

        match send_batch_import_members(writer, target, group_id, addrs, names, payment).await {
            Ok(()) => info!(batch = batch_idx + 1, "member batch imported"),
            Err(e) => {
                warn!(
                    group = group_id,
                    batch = batch_idx + 1,
                    error = %e,
                    "member batch failed, continuing"
                );
            }
        }
    }

    Ok(())
}

// ── Redis progress tracking ─────────────────────────────────────────

fn load_done_set(conn: &mut redis::Connection, key: &str) -> Result<HashSet<String>> {
    let members: HashSet<String> = redis::Commands::smembers(conn, key).unwrap_or_default();
    Ok(members)
}

fn mark_done(conn: &mut redis::Connection, key: &str, value: &str) -> Result<()> {
    let _: () = redis::Commands::sadd(conn, key, value).wrap_err("failed to mark done in Redis")?;
    Ok(())
}

// ── V1 contract read helpers ────────────────────────────────────────

async fn get_block_number(reader: &ReadProvider) -> Result<u64> {
    dispatch_read!(reader, |p| Ok(p.get_block_number().await?))
}

async fn get_logs(reader: &ReadProvider, filter: &Filter) -> Result<Vec<alloy_rpc_types_eth::Log>> {
    dispatch_read!(reader, |p| Ok(p.get_logs(filter).await?))
}

async fn read_bracket(
    reader: &ReadProvider,
    source: Address,
    account: Address,
) -> Result<FixedBytes<8>> {
    dispatch_read!(reader, |p| {
        let contract = MarchMadness::new(source, p);
        let result = contract.getBracket(account).call().await?;
        Ok(result)
    })
}

async fn read_tag(reader: &ReadProvider, source: Address, account: Address) -> Result<String> {
    dispatch_read!(reader, |p| {
        let contract = MarchMadness::new(source, p);
        let result = contract.getTag(account).call().await?;
        Ok(result)
    })
}

async fn read_group(
    reader: &ReadProvider,
    source: Address,
    group_id: u32,
) -> Result<BracketGroups::Group> {
    dispatch_read!(reader, |p| {
        let contract = BracketGroups::new(source, p);
        let result = contract.getGroup(group_id).call().await?;
        Ok(result)
    })
}

async fn read_members(
    reader: &ReadProvider,
    source: Address,
    group_id: u32,
) -> Result<Vec<BracketGroups::Member>> {
    dispatch_read!(reader, |p| {
        let contract = BracketGroups::new(source, p);
        let result = contract.getMembers(group_id).call().await?;
        Ok(result)
    })
}

// ── V2 contract write helpers ───────────────────────────────────────

async fn send_batch_import_entries(
    writer: &SignedProvider,
    target: Address,
    accounts: Vec<Address>,
    brackets: Vec<FixedBytes<8>>,
    payment: U256,
) -> Result<()> {
    dispatch_write!(writer, target, MarchMadnessV2, |contract| {
        let receipt = contract
            .batchImportEntries(accounts, brackets)
            .value(payment)
            .send()
            .await
            .wrap_err("batchImportEntries send failed")?
            .get_receipt()
            .await
            .wrap_err("batchImportEntries receipt failed")?;
        if !receipt.status() {
            bail!(
                "batchImportEntries reverted: {:?}",
                receipt.transaction_hash
            );
        }
        Ok(())
    })
}

async fn send_import_tag(
    writer: &SignedProvider,
    target: Address,
    account: Address,
    tag: String,
) -> Result<()> {
    dispatch_write!(writer, target, MarchMadnessV2, |contract| {
        let receipt = contract
            .importTag(account, tag)
            .send()
            .await
            .wrap_err("importTag send failed")?
            .get_receipt()
            .await
            .wrap_err("importTag receipt failed")?;
        if !receipt.status() {
            bail!("importTag reverted: {:?}", receipt.transaction_hash);
        }
        Ok(())
    })
}

async fn send_import_group(
    writer: &SignedProvider,
    target: Address,
    group: &SnapshotGroup,
) -> Result<()> {
    dispatch_write!(writer, target, BracketGroupsV2, |contract| {
        let receipt = contract
            .importGroup(
                group.slug.clone(),
                group.display_name.clone(),
                group.entry_fee,
                group.creator,
            )
            .send()
            .await
            .wrap_err("importGroup send failed")?
            .get_receipt()
            .await
            .wrap_err("importGroup receipt failed")?;
        if !receipt.status() {
            bail!("importGroup reverted: {:?}", receipt.transaction_hash);
        }
        Ok(())
    })
}

async fn send_batch_import_members(
    writer: &SignedProvider,
    target: Address,
    group_id: u32,
    addrs: Vec<Address>,
    names: Vec<String>,
    payment: U256,
) -> Result<()> {
    dispatch_write!(writer, target, BracketGroupsV2, |contract| {
        let receipt = contract
            .batchImportMembers(group_id, addrs, names)
            .value(payment)
            .send()
            .await
            .wrap_err("batchImportMembers send failed")?
            .get_receipt()
            .await
            .wrap_err("batchImportMembers receipt failed")?;
        if !receipt.status() {
            bail!(
                "batchImportMembers reverted: {:?}",
                receipt.transaction_hash
            );
        }
        Ok(())
    })
}
