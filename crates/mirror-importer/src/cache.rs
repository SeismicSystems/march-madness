//! Simple file-based cache with TTL, stored under data/cache/mirrors/yahoo/.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Bracket structure: effectively permanent (tournament structure doesn't change).
pub const TTL_BRACKET: Duration = Duration::days(60);

/// Group members: refresh daily (people can join/leave).
pub const TTL_MEMBERS: Duration = Duration::days(1);

/// Individual picks: effectively permanent (picks don't change after lock).
pub const TTL_PICKS: Duration = Duration::days(60);

#[derive(Serialize, Deserialize)]
struct CachedResponse<T> {
    fetched_at: DateTime<Utc>,
    data: T,
}

fn cache_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find workspace root")
        .join("data")
        .join("cache")
        .join("mirrors")
        .join("yahoo")
}

fn cache_path(key: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", key))
}

/// Load a cached response if it exists and hasn't expired.
pub fn load<T: DeserializeOwned>(key: &str, ttl: Duration) -> Option<T> {
    let path = cache_path(key);
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedResponse<T> = serde_json::from_str(&content).ok()?;
    let age = Utc::now() - cached.fetched_at;
    if age > ttl {
        debug!(
            "cache expired for {} (age: {}s, ttl: {}s)",
            key,
            age.num_seconds(),
            ttl.num_seconds()
        );
        return None;
    }
    debug!("using cache for {} (age: {}s)", key, age.num_seconds());
    Some(cached.data)
}

/// Save a response to the cache.
pub fn save<T: Serialize>(key: &str, data: &T) -> eyre::Result<()> {
    let path = cache_path(key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cached = CachedResponse {
        fetched_at: Utc::now(),
        data,
    };
    let json = serde_json::to_string_pretty(&cached)?;
    fs::write(&path, json)?;
    debug!("cached {}", key);
    Ok(())
}

/// Return the cache directory for a group (used by main.rs to place platform.json).
pub fn group_cache_dir(group_id: u32) -> PathBuf {
    cache_dir().join("groups").join(group_id.to_string())
}
