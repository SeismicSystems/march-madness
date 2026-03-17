//! Redis key constants and stored types shared by indexer and server.
//!
//! Fixed number of keys regardless of entity count. Entity data is stored
//! as JSON values inside hash fields.
//!
//! No Redis dependency — pure constants + serde types.

use serde::{Deserialize, Serialize};

/// Default Redis URL used when `REDIS_URL` env var is not set.
pub const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379";

/// Indexer cursor: last fully processed block number (STRING).
pub const KEY_LAST_BLOCK: &str = "mm:indexer:last_block";

/// Entries: addr → JSON({ name?, bracket?, block, ts }) (HASH).
pub const KEY_ENTRIES: &str = "mm:entries";

/// Groups: groupId → JSON({ slug, display_name, creator, has_password, members }) (HASH).
pub const KEY_GROUPS: &str = "mm:groups";

/// Group slug reverse lookup: slug → groupId (HASH).
pub const KEY_GROUP_SLUGS: &str = "mm:group:slugs";

/// Mirrors: mirrorId → JSON({ slug, display_name, admin }) (HASH).
pub const KEY_MIRRORS: &str = "mm:mirrors";

/// Mirror slug reverse lookup: slug → mirrorId (HASH).
pub const KEY_MIRROR_SLUGS: &str = "mm:mirror:slugs";

/// Mirror entries: "mirrorId:entrySlug" → bracket_hex (HASH).
pub const KEY_MIRROR_ENTRIES: &str = "mm:mirror:entries";

/// Build a composite key for mirror entries: "mirrorId:entrySlug".
pub fn mirror_entry_field(mirror_id: u64, slug: &str) -> String {
    format!("{mirror_id}:{slug}")
}

// ── Stored types (JSON-serialized into hash fields) ──────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntryData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket: Option<String>,
    #[serde(default)]
    pub block: u64,
    #[serde(default)]
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupData {
    pub slug: String,
    pub display_name: String,
    pub creator: String,
    pub has_password: bool,
    pub members: Vec<String>,
    #[serde(default)]
    pub member_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorData {
    pub slug: String,
    pub display_name: String,
    pub admin: String,
}
