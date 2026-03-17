//! Redis storage backend for the indexer.
//!
//! Uses a fixed number of Redis keys (7 total) regardless of entity count.
//! Entity data is stored as JSON values inside hash fields.

use eyre::{Result, WrapErr};
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::*;

/// Connect to Redis using `REDIS_URL` env var or default.
pub async fn connect() -> Result<MultiplexedConnection> {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
    let client = redis::Client::open(url.as_str())
        .wrap_err_with(|| format!("failed to create Redis client from {url}"))?;
    let conn = client
        .get_multiplexed_async_connection()
        .await
        .wrap_err("failed to connect to Redis")?;
    Ok(conn)
}

// ── Generic read-modify-write helper ─────────────────────────────────

/// Read a JSON value from a hash field, apply a mutation, write it back.
/// If the field doesn't exist, `default` is used as the starting value.
async fn modify_hash_field<T: serde::Serialize + serde::de::DeserializeOwned>(
    conn: &mut MultiplexedConnection,
    key: &str,
    field: &str,
    default: impl FnOnce() -> T,
    mutate: impl FnOnce(&mut T),
) -> Result<()> {
    let existing: Option<String> = conn.hget(key, field).await?;
    let mut value = existing
        .as_deref()
        .and_then(|s| serde_json::from_str::<T>(s).ok())
        .unwrap_or_else(default);
    mutate(&mut value);
    let json = serde_json::to_string(&value)?;
    let () = conn.hset(key, field, &json).await?;
    Ok(())
}

// ── Entry operations ─────────────────────────────────────────────────

/// Record a BracketSubmitted event.
pub async fn upsert_bracket_submitted(
    conn: &mut MultiplexedConnection,
    address: &str,
    block: u64,
    timestamp: u64,
) -> Result<()> {
    let addr = address.to_lowercase();
    modify_hash_field(conn, KEY_ENTRIES, &addr, EntryData::default, |e| {
        e.block = block;
        e.ts = timestamp;
    })
    .await
}

/// Record a TagSet event: set the name field.
pub async fn update_tag(conn: &mut MultiplexedConnection, address: &str, tag: &str) -> Result<()> {
    let addr = address.to_lowercase();
    let tag = tag.to_string();
    modify_hash_field(conn, KEY_ENTRIES, &addr, EntryData::default, |e| {
        e.name = Some(tag);
    })
    .await
}

/// Set the bracket hex for an address (after reveal).
pub async fn set_bracket(
    conn: &mut MultiplexedConnection,
    address: &str,
    bracket_hex: &str,
) -> Result<()> {
    let addr = address.to_lowercase();
    // Only update if entry exists (HGET returns None otherwise).
    let existing: Option<String> = conn.hget(KEY_ENTRIES, &addr).await?;
    if let Some(mut entry) = existing
        .as_deref()
        .and_then(|s| serde_json::from_str::<EntryData>(s).ok())
    {
        entry.bracket = Some(bracket_hex.to_string());
        let json = serde_json::to_string(&entry)?;
        let () = conn.hset(KEY_ENTRIES, &addr, &json).await?;
    }
    Ok(())
}

// ── Group operations ─────────────────────────────────────────────────
//
// Group data is split across two Redis keys:
// - KEY_GROUPS: metadata (slug, display_name, creator, has_password, member_count)
// - KEY_GROUP_MEMBERS: member lists (JSON array of addresses)
// This lets group listings load only metadata without deserializing member arrays.

/// Record a GroupCreated event.
pub async fn create_group(
    conn: &mut MultiplexedConnection,
    group_id: u32,
    slug: &str,
    display_name: &str,
    creator: &str,
    has_password: bool,
) -> Result<()> {
    let data = GroupData {
        slug: slug.to_string(),
        display_name: display_name.to_string(),
        creator: creator.to_lowercase(),
        has_password,
        member_count: 0,
    };
    let meta_json = serde_json::to_string(&data)?;
    let members_json = serde_json::to_string(&Vec::<String>::new())?;
    let id_str = group_id.to_string();
    redis::pipe()
        .atomic()
        .hset(KEY_GROUPS, &id_str, &meta_json)
        .hset(KEY_GROUP_MEMBERS, &id_str, &members_json)
        .hset(KEY_GROUP_SLUGS, slug, &id_str)
        .exec_async(conn)
        .await
        .wrap_err("failed to create group")?;
    Ok(())
}

/// Record a MemberJoined event.
pub async fn member_joined(
    conn: &mut MultiplexedConnection,
    group_id: u32,
    address: &str,
) -> Result<()> {
    let addr = address.to_lowercase();
    let id_str = group_id.to_string();

    // Read current members list.
    let members_json: Option<String> = conn.hget(KEY_GROUP_MEMBERS, &id_str).await?;
    let mut members: Vec<String> = members_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if members.contains(&addr) {
        return Ok(());
    }
    members.push(addr);

    // Write updated members + increment metadata count atomically.
    let new_members_json = serde_json::to_string(&members)?;
    modify_hash_field(conn, KEY_GROUPS, &id_str, GroupData::default, |g| {
        g.member_count += 1;
    })
    .await?;
    let () = conn
        .hset(KEY_GROUP_MEMBERS, &id_str, &new_members_json)
        .await?;
    Ok(())
}

/// Record a MemberLeft event.
pub async fn member_left(
    conn: &mut MultiplexedConnection,
    group_id: u32,
    address: &str,
) -> Result<()> {
    let addr = address.to_lowercase();
    let id_str = group_id.to_string();

    // Read current members list.
    let members_json: Option<String> = conn.hget(KEY_GROUP_MEMBERS, &id_str).await?;
    let mut members: Vec<String> = match members_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
    {
        Some(m) => m,
        None => return Ok(()),
    };

    let before = members.len();
    members.retain(|a| a != &addr);
    if members.len() == before {
        // Member wasn't in the list — duplicate or out-of-order event. No-op.
        return Ok(());
    }

    // Write updated members + decrement metadata count.
    let new_members_json = serde_json::to_string(&members)?;
    // saturating_sub guards against member_count having drifted below
    // members.len() (e.g. from a bug or out-of-order events). In the
    // normal case member_count == before, so this is just a decrement.
    modify_hash_field(conn, KEY_GROUPS, &id_str, GroupData::default, |g| {
        g.member_count = g.member_count.saturating_sub(1);
    })
    .await?;
    let () = conn
        .hset(KEY_GROUP_MEMBERS, &id_str, &new_members_json)
        .await?;
    Ok(())
}

// ── Mirror operations ────────────────────────────────────────────────

/// Record a MirrorCreated event.
pub async fn create_mirror(
    conn: &mut MultiplexedConnection,
    mirror_id: u64,
    slug: &str,
    display_name: &str,
    admin: &str,
) -> Result<()> {
    let data = MirrorData {
        slug: slug.to_string(),
        display_name: display_name.to_string(),
        admin: admin.to_lowercase(),
    };
    let json = serde_json::to_string(&data)?;
    let id_str = mirror_id.to_string();
    redis::pipe()
        .atomic()
        .hset(KEY_MIRRORS, &id_str, &json)
        .hset(KEY_MIRROR_SLUGS, slug, &id_str)
        .exec_async(conn)
        .await
        .wrap_err("failed to create mirror")?;
    Ok(())
}

/// Record an EntryAdded event. Bracket fetched via contract read.
pub async fn mirror_entry_added(
    conn: &mut MultiplexedConnection,
    mirror_id: u64,
    slug: &str,
    bracket_hex: &str,
) -> Result<()> {
    let field = mirror_entry_field(mirror_id, slug);
    let () = conn.hset(KEY_MIRROR_ENTRIES, &field, bracket_hex).await?;
    Ok(())
}

/// Record an EntryRemoved event.
pub async fn mirror_entry_removed(
    conn: &mut MultiplexedConnection,
    mirror_id: u64,
    slug: &str,
) -> Result<()> {
    let field = mirror_entry_field(mirror_id, slug);
    let () = conn.hdel(KEY_MIRROR_ENTRIES, &field).await?;
    Ok(())
}

// ── Cursor ───────────────────────────────────────────────────────────

pub async fn get_last_block(conn: &mut MultiplexedConnection) -> Result<Option<u64>> {
    let val: Option<u64> = conn.get(KEY_LAST_BLOCK).await?;
    Ok(val)
}

pub async fn set_last_block(conn: &mut MultiplexedConnection, block: u64) -> Result<()> {
    let () = conn.set(KEY_LAST_BLOCK, block).await?;
    Ok(())
}

// ── Read helpers ─────────────────────────────────────────────────────

pub async fn get_all_entry_addresses(conn: &mut MultiplexedConnection) -> Result<Vec<String>> {
    let fields: Vec<String> = conn.hkeys(KEY_ENTRIES).await?;
    Ok(fields)
}

pub async fn get_entry_count(conn: &mut MultiplexedConnection) -> Result<usize> {
    let count: usize = conn.hlen(KEY_ENTRIES).await?;
    Ok(count)
}

pub async fn get_entry(
    conn: &mut MultiplexedConnection,
    address: &str,
) -> Result<Option<EntryData>> {
    let addr = address.to_lowercase();
    let json: Option<String> = conn.hget(KEY_ENTRIES, &addr).await?;
    Ok(json.and_then(|s| serde_json::from_str(&s).ok()))
}

/// Get all groups from Redis.
pub async fn get_all_groups(
    conn: &mut MultiplexedConnection,
) -> Result<std::collections::HashMap<String, GroupData>> {
    let all: std::collections::HashMap<String, String> = conn.hgetall(KEY_GROUPS).await?;
    let mut result = std::collections::HashMap::with_capacity(all.len());
    for (id, json) in all {
        if let Ok(data) = serde_json::from_str::<GroupData>(&json) {
            result.insert(id, data);
        }
    }
    Ok(result)
}

/// Get a single group by slug.
pub async fn get_group_by_slug(
    conn: &mut MultiplexedConnection,
    slug: &str,
) -> Result<Option<(String, GroupData)>> {
    let id: Option<String> = conn.hget(KEY_GROUP_SLUGS, slug).await?;
    let Some(id) = id else { return Ok(None) };
    let json: Option<String> = conn.hget(KEY_GROUPS, &id).await?;
    match json {
        Some(s) => Ok(Some((id, serde_json::from_str(&s)?))),
        None => Ok(None),
    }
}

/// Get the member list for a group by ID.
pub async fn get_group_members(
    conn: &mut MultiplexedConnection,
    group_id: &str,
) -> Result<Vec<String>> {
    let json: Option<String> = conn.hget(KEY_GROUP_MEMBERS, group_id).await?;
    Ok(json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default())
}
