//! Core migration logic: load Redis entries/groups, diff against on-chain state,
//! batch-import into MarchMadnessV2 and BracketGroupsV2.

use std::collections::HashMap;

use alloy_network::ReceiptResponse;
use alloy_primitives::{Address, FixedBytes, U256};
use eyre::{Result, WrapErr, bail};
use tracing::{info, warn};

use seismic_march_madness::migration::reverse_game_bits;
use seismic_march_madness::redis_keys::{
    EntryData, GroupData, KEY_ENTRIES, KEY_GROUP_MEMBERS, KEY_GROUPS,
};
use seismic_march_madness::scoring::parse_bracket_hex;

use crate::contract::{BracketGroupsV2, MarchMadnessV2};
use crate::provider::SignedProvider;

/// A Redis entry ready for import: address, contract-correct bracket, optional tag.
struct ImportEntry {
    address: Address,
    bracket: FixedBytes<8>,
    tag: Option<String>,
}

/// A Redis group ready for import.
struct ImportGroup {
    id: u32,
    slug: String,
    display_name: String,
    creator: Address,
    entry_fee: U256,
    members: Vec<(Address, String)>, // (address, display_name within group)
}

/// Run the full entry migration: Redis → MarchMadnessV2.
pub async fn run_entries(
    provider: Option<&SignedProvider>,
    target: Address,
    redis_url: &str,
    batch_size: usize,
    dry_run: bool,
) -> Result<()> {
    // 1. Load entries from Redis
    let redis_entries = load_redis_entries(redis_url)?;
    info!(count = redis_entries.len(), "loaded entries from Redis");

    if redis_entries.is_empty() {
        info!("no entries in Redis, nothing to do");
        return Ok(());
    }

    // 2. Parse and convert brackets
    let import_entries = prepare_entries(&redis_entries)?;
    let total_tags = import_entries.iter().filter(|e| e.tag.is_some()).count();
    info!(
        total = import_entries.len(),
        with_tags = total_tags,
        "prepared entries for import"
    );

    // 3. Check on-chain state to find what's already imported
    let to_import = if let Some(p) = provider {
        filter_already_imported(p, target, import_entries).await?
    } else {
        info!("no provider available, skipping on-chain filtering");
        import_entries
    };

    let tags_to_import = to_import.iter().filter(|e| e.tag.is_some()).count();

    info!(
        entries = to_import.len(),
        tags = tags_to_import,
        "entries to import (after filtering already-imported)"
    );

    if to_import.is_empty() {
        info!("all entries already imported, nothing to do");
        return Ok(());
    }

    if dry_run {
        info!(
            "dry run — would import {} entries and {} tags",
            to_import.len(),
            tags_to_import
        );
        for entry in &to_import {
            info!(
                address = %entry.address,
                bracket = %entry.bracket,
                tag = entry.tag.as_deref().unwrap_or("<none>"),
                "would import"
            );
        }
        return Ok(());
    }

    let provider = provider.ok_or_else(|| eyre::eyre!("provider required for non-dry-run"))?;

    // 4. Import entries in batches (batchImportEntries is idempotent on-chain)
    import_entries_batched(provider, target, &to_import, batch_size).await?;

    // 5. Import tags one at a time (no batch function on contract)
    let with_tags: Vec<&ImportEntry> = to_import.iter().filter(|e| e.tag.is_some()).collect();
    if !with_tags.is_empty() {
        import_tags(provider, target, &with_tags).await?;
    }

    info!("entry migration complete");
    Ok(())
}

/// Run the full group migration: Redis → BracketGroupsV2.
pub async fn run_groups(
    provider: Option<&SignedProvider>,
    target: Address,
    redis_url: &str,
    batch_size: usize,
    dry_run: bool,
) -> Result<()> {
    let groups = load_redis_groups(redis_url)?;
    info!(count = groups.len(), "loaded groups from Redis");

    if groups.is_empty() {
        info!("no groups in Redis, nothing to do");
        return Ok(());
    }

    if dry_run {
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

    let provider = provider.ok_or_else(|| eyre::eyre!("provider required for non-dry-run"))?;

    for group in &groups {
        info!(
            id = group.id,
            slug = group.slug,
            members = group.members.len(),
            "importing group"
        );

        // Create the group
        match send_import_group(provider, target, group).await {
            Ok(()) => info!(slug = group.slug, "group created"),
            Err(e) => {
                warn!(slug = group.slug, error = %e, "group creation failed, skipping");
                continue;
            }
        }

        // Import members in batches
        if !group.members.is_empty() {
            import_members_batched(provider, target, group.id, &group.members, batch_size).await?;
        }
    }

    info!("group migration complete");
    Ok(())
}

// ── Redis loading ───────────────────────────────────────────────────

/// Load all entries from Redis `mm:entries` hash.
fn load_redis_entries(redis_url: &str) -> Result<HashMap<String, EntryData>> {
    let client = redis::Client::open(redis_url).wrap_err("failed to open Redis client")?;
    let mut conn = client
        .get_connection()
        .wrap_err("failed to connect to Redis")?;

    let raw: HashMap<String, String> =
        redis::Commands::hgetall(&mut conn, KEY_ENTRIES).wrap_err("failed to read mm:entries")?;

    let mut entries = HashMap::with_capacity(raw.len());
    for (address, json) in raw {
        match serde_json::from_str::<EntryData>(&json) {
            Ok(entry) => {
                entries.insert(address, entry);
            }
            Err(e) => {
                warn!(address, error = %e, "skipping corrupt entry");
            }
        }
    }
    Ok(entries)
}

/// Load all groups and their members from Redis.
fn load_redis_groups(redis_url: &str) -> Result<Vec<ImportGroup>> {
    let client = redis::Client::open(redis_url).wrap_err("failed to open Redis client")?;
    let mut conn = client
        .get_connection()
        .wrap_err("failed to connect to Redis")?;

    let raw_groups: HashMap<String, String> =
        redis::Commands::hgetall(&mut conn, KEY_GROUPS).wrap_err("failed to read mm:groups")?;

    let raw_members: HashMap<String, String> =
        redis::Commands::hgetall(&mut conn, KEY_GROUP_MEMBERS)
            .wrap_err("failed to read mm:group_members")?;

    let mut groups = Vec::with_capacity(raw_groups.len());
    for (id_str, json) in &raw_groups {
        let id: u32 = match id_str.parse() {
            Ok(id) => id,
            Err(e) => {
                warn!(id = id_str, error = %e, "skipping group with invalid ID");
                continue;
            }
        };

        let data: GroupData = match serde_json::from_str(json) {
            Ok(d) => d,
            Err(e) => {
                warn!(id = id_str, error = %e, "skipping corrupt group");
                continue;
            }
        };

        let creator: Address = match data.creator.parse() {
            Ok(a) => a,
            Err(e) => {
                warn!(id = id_str, error = %e, "skipping group with invalid creator address");
                continue;
            }
        };

        let entry_fee: U256 = match data.entry_fee.parse() {
            Ok(f) => f,
            Err(e) => {
                warn!(id = id_str, error = %e, "skipping group with invalid entry fee");
                continue;
            }
        };

        // Load members for this group
        let member_addrs: Vec<String> = match raw_members.get(id_str) {
            Some(json) => serde_json::from_str(json).unwrap_or_default(),
            None => Vec::new(),
        };

        // Members stored as addresses — display names come from the entries table.
        // The BracketGroups contract stores per-member names, but Redis only stores addresses.
        // We use the address as a placeholder name; the actual name was set at join time
        // and isn't preserved in the Redis member list.
        let members: Vec<(Address, String)> = member_addrs
            .iter()
            .filter_map(|addr_str| {
                let addr: Address = addr_str.parse().ok()?;
                // Use empty string — the contract's importMember accepts any name
                Some((addr, String::new()))
            })
            .collect();

        groups.push(ImportGroup {
            id,
            slug: data.slug,
            display_name: data.display_name,
            creator,
            entry_fee,
            members,
        });
    }

    // Sort by ID so V2 assigns IDs in the same order as V1
    groups.sort_by_key(|g| g.id);

    Ok(groups)
}

// ── Entry preparation ───────────────────────────────────────────────

/// Parse Redis entries into ImportEntry structs with contract-correct encoding.
fn prepare_entries(redis_entries: &HashMap<String, EntryData>) -> Result<Vec<ImportEntry>> {
    let mut result = Vec::with_capacity(redis_entries.len());

    for (address_str, entry) in redis_entries {
        let bracket_hex = match &entry.bracket {
            Some(hex) => hex,
            None => {
                warn!(address = address_str, "entry has no bracket, skipping");
                continue;
            }
        };

        let address: Address = address_str
            .parse()
            .wrap_err_with(|| format!("invalid address: {address_str}"))?;

        let legacy_bits = match parse_bracket_hex(bracket_hex) {
            Some(bits) => bits,
            None => {
                warn!(
                    address = address_str,
                    hex = bracket_hex,
                    "failed to parse bracket hex, skipping"
                );
                continue;
            }
        };

        // Convert legacy encoding → contract-correct encoding
        let contract_bits = reverse_game_bits(legacy_bits);
        let bytes8 = FixedBytes::<8>::from(contract_bits.to_be_bytes());

        result.push(ImportEntry {
            address,
            bracket: bytes8,
            tag: entry.name.clone(),
        });
    }

    Ok(result)
}

// ── On-chain filtering ──────────────────────────────────────────────

/// Filter out entries that are already imported on-chain.
async fn filter_already_imported(
    provider: &SignedProvider,
    target: Address,
    entries: Vec<ImportEntry>,
) -> Result<Vec<ImportEntry>> {
    let mut to_import = Vec::new();
    let total = entries.len();

    for (i, entry) in entries.into_iter().enumerate() {
        if (i + 1) % 50 == 0 || i + 1 == total {
            info!(
                progress = format!("{}/{}", i + 1, total),
                "checking on-chain state"
            );
        }

        let already_imported = has_entry(provider, target, entry.address).await?;
        if already_imported {
            continue;
        }
        to_import.push(entry);
    }

    Ok(to_import)
}

// ── Batch import helpers ────────────────────────────────────────────

/// Import entries in batches using `batchImportEntries(address[], bytes8[])`.
async fn import_entries_batched(
    provider: &SignedProvider,
    target: Address,
    entries: &[ImportEntry],
    batch_size: usize,
) -> Result<()> {
    let total_batches = entries.len().div_ceil(batch_size);

    for (batch_idx, chunk) in entries.chunks(batch_size).enumerate() {
        let addresses: Vec<Address> = chunk.iter().map(|e| e.address).collect();
        let brackets: Vec<FixedBytes<8>> = chunk.iter().map(|e| e.bracket).collect();

        info!(
            batch = batch_idx + 1,
            total = total_batches,
            size = chunk.len(),
            "importing entries batch"
        );

        match send_batch_import_entries(provider, target, addresses, brackets).await {
            Ok(()) => {
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

/// Import tags one at a time using `importTag(address, string)`.
async fn import_tags(
    provider: &SignedProvider,
    target: Address,
    entries: &[&ImportEntry],
) -> Result<()> {
    let total = entries.len();
    let mut imported = 0;
    let mut failed = 0;

    for (i, entry) in entries.iter().enumerate() {
        let tag = entry.tag.as_deref().unwrap_or_default();

        if (i + 1) % 20 == 0 || i + 1 == total {
            info!(progress = format!("{}/{}", i + 1, total), "importing tags");
        }

        match send_import_tag(provider, target, entry.address, tag.to_string()).await {
            Ok(()) => imported += 1,
            Err(e) => {
                warn!(
                    address = %entry.address,
                    error = %e,
                    "tag import failed, continuing"
                );
                failed += 1;
            }
        }
    }

    info!(imported, failed, "tag import complete");
    Ok(())
}

/// Import group members in batches using `batchImportMembers(groupId, addrs, names)`.
async fn import_members_batched(
    provider: &SignedProvider,
    target: Address,
    group_id: u32,
    members: &[(Address, String)],
    batch_size: usize,
) -> Result<()> {
    let total_batches = members.len().div_ceil(batch_size);

    for (batch_idx, chunk) in members.chunks(batch_size).enumerate() {
        let addrs: Vec<Address> = chunk.iter().map(|(a, _)| *a).collect();
        let names: Vec<String> = chunk.iter().map(|(_, n)| n.clone()).collect();

        info!(
            group = group_id,
            batch = batch_idx + 1,
            total = total_batches,
            size = chunk.len(),
            "importing members batch"
        );

        match send_batch_import_members(provider, target, group_id, addrs, names).await {
            Ok(()) => {
                info!(batch = batch_idx + 1, "member batch imported successfully");
            }
            Err(e) => {
                warn!(
                    group = group_id,
                    batch = batch_idx + 1,
                    error = %e,
                    "member batch import failed, continuing"
                );
            }
        }
    }

    Ok(())
}

// ── Contract interaction helpers ────────────────────────────────────

/// Dispatch a contract call across Reth/Foundry provider variants.
macro_rules! dispatch_mm {
    ($provider:expr, $target:expr, |$contract:ident| $body:expr) => {
        match $provider {
            SignedProvider::Reth(p) => {
                let $contract = MarchMadnessV2::new($target, p);
                $body
            }
            SignedProvider::Foundry(p) => {
                let $contract = MarchMadnessV2::new($target, p);
                $body
            }
        }
    };
}

macro_rules! dispatch_bg {
    ($provider:expr, $target:expr, |$contract:ident| $body:expr) => {
        match $provider {
            SignedProvider::Reth(p) => {
                let $contract = BracketGroupsV2::new($target, p);
                $body
            }
            SignedProvider::Foundry(p) => {
                let $contract = BracketGroupsV2::new($target, p);
                $body
            }
        }
    };
}

/// Call `hasEntry(address)` on the V2 contract.
async fn has_entry(provider: &SignedProvider, target: Address, account: Address) -> Result<bool> {
    dispatch_mm!(provider, target, |contract| {
        let result = contract.hasEntry(account).call().await?;
        Ok(result)
    })
}

/// Send `batchImportEntries(address[], bytes8[])` transaction.
async fn send_batch_import_entries(
    provider: &SignedProvider,
    target: Address,
    accounts: Vec<Address>,
    brackets: Vec<FixedBytes<8>>,
) -> Result<()> {
    dispatch_mm!(provider, target, |contract| {
        let receipt = contract
            .batchImportEntries(accounts, brackets)
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

/// Send `importTag(address, string)` transaction.
async fn send_import_tag(
    provider: &SignedProvider,
    target: Address,
    account: Address,
    tag: String,
) -> Result<()> {
    dispatch_mm!(provider, target, |contract| {
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

/// Send `importGroup(slug, displayName, entryFee, creator)` transaction.
async fn send_import_group(
    provider: &SignedProvider,
    target: Address,
    group: &ImportGroup,
) -> Result<()> {
    dispatch_bg!(provider, target, |contract| {
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

/// Send `batchImportMembers(groupId, addrs, names)` transaction.
async fn send_batch_import_members(
    provider: &SignedProvider,
    target: Address,
    group_id: u32,
    addrs: Vec<Address>,
    names: Vec<String>,
) -> Result<()> {
    dispatch_bg!(provider, target, |contract| {
        let receipt = contract
            .batchImportMembers(group_id, addrs, names)
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
