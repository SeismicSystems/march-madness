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

/// Groups metadata: groupId → JSON({ slug, display_name, creator, has_password, member_count, entry_fee }) (HASH).
/// Lightweight — no member list. Queried often for group listings.
pub const KEY_GROUPS: &str = "mm:groups";

/// Group members: groupId → JSON(["0xaddr1", "0xaddr2", ...]) (HASH).
/// Separate from metadata so listing groups doesn't load all member arrays.
pub const KEY_GROUP_MEMBERS: &str = "mm:group_members";

/// Group slug reverse lookup: slug → groupId (HASH).
pub const KEY_GROUP_SLUGS: &str = "mm:group:slugs";

/// Address → groups reverse lookup: address → JSON([groupId1, groupId2, ...]) (HASH).
/// Maintained alongside KEY_GROUP_MEMBERS for fast "my groups" queries.
pub const KEY_ADDRESS_GROUPS: &str = "mm:address_groups";

/// Mirrors: mirrorId → JSON({ slug, display_name, admin }) (HASH).
pub const KEY_MIRRORS: &str = "mm:mirrors";

/// Mirror slug reverse lookup: slug → mirrorId (HASH).
pub const KEY_MIRROR_SLUGS: &str = "mm:mirror:slugs";

/// Mirror entries: "mirrorId:entrySlug" → bracket_hex (HASH).
pub const KEY_MIRROR_ENTRIES: &str = "mm:mirror:entries";

/// Tournament game status: full TournamentStatus JSON blob (STRING).
/// Written by ncaa-feed, read by server and CLI tools (forecaster, sim).
pub const KEY_GAMES: &str = "mm:games";

/// Per-pool win probability forecasts (HASH).
/// Written by forecaster, read by server.
///
/// Field keys:
/// - `"mm"` → main pool: JSON `{"0xaddr": bps, ...}` (basis points, u32)
/// - `"group:{id}"` → group pool: JSON `{"0xaddr": bps, ...}`
/// - `"mirror:{id}"` → mirror pool: JSON `{"entry-slug": bps, ...}`
///
/// Basis points: 10000 = 100% win probability.
pub const KEY_FORECASTS: &str = "mm:forecasts";

/// Per-team advance probabilities: team_name → JSON([p_r64, p_r32, p_s16, p_e8, p_f4, p_champ]) (HASH).
/// Written by forecaster, read by server. Probabilities are 0.0-1.0 floats (6 values per team).
pub const KEY_TEAM_PROBS: &str = "mm:probs";

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupData {
    pub slug: String,
    pub display_name: String,
    pub creator: String,
    pub has_password: bool,
    pub member_count: u32,
    /// Entry fee in wei as a decimal string.
    pub entry_fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorData {
    pub slug: String,
    pub display_name: String,
    pub admin: String,
}
