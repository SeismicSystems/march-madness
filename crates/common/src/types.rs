use serde::{Deserialize, Serialize};

/// An indexed bracket entry, keyed by address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryRecord {
    /// Optional display name (from setTag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// When this entry was last updated on-chain.
    pub updated: UpdateInfo,

    /// Hex-encoded bracket bytes (after reveal / post-deadline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub block: u64,
    pub ts: u64,
}

/// The full index file written by the indexer and served by the server.
pub type EntryIndex = std::collections::BTreeMap<String, EntryRecord>;
