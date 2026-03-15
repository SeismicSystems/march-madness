use eyre::Result;
use march_madness_common::EntryIndex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Shared application state holding the index file path and a TTL cache.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    index_path: PathBuf,
    cache: RwLock<CachedIndex>,
    ttl: Duration,
    tournament_status_path: PathBuf,
    tournament_status_cache: RwLock<CachedJson>,
    /// API key for POST /api/tournament-status.
    api_key: Option<String>,
}

struct CachedIndex {
    data: EntryIndex,
    fetched_at: Instant,
}

struct CachedJson {
    data: serde_json::Value,
    fetched_at: Instant,
}

impl AppState {
    pub fn new(
        index_path: PathBuf,
        ttl: Duration,
        tournament_status_path: PathBuf,
        api_key: Option<String>,
    ) -> Self {
        let expired = Instant::now() - ttl - Duration::from_secs(1);
        Self {
            inner: Arc::new(Inner {
                index_path,
                cache: RwLock::new(CachedIndex {
                    data: EntryIndex::new(),
                    fetched_at: expired,
                }),
                ttl,
                tournament_status_path,
                tournament_status_cache: RwLock::new(CachedJson {
                    data: serde_json::Value::Null,
                    fetched_at: expired,
                }),
                api_key,
            }),
        }
    }

    /// Get the current entry index, using a TTL cache to avoid reading the
    /// file on every request. Acquires a shared/read file lock via fs2 so we
    /// can coexist with the indexer's write locks.
    pub async fn get_index(&self) -> Result<EntryIndex> {
        // Fast path: cache is still valid.
        {
            let cache = self.inner.cache.read().await;
            if cache.fetched_at.elapsed() < self.inner.ttl {
                return Ok(cache.data.clone());
            }
        }

        // Slow path: reload from disk.
        let path = self.inner.index_path.clone();
        let data = tokio::task::spawn_blocking(move || -> Result<EntryIndex> {
            if !path.exists() {
                return Ok(EntryIndex::new());
            }
            let file = std::fs::File::open(&path)?;
            file.lock_shared()?;
            let reader = std::io::BufReader::new(&file);
            let index: EntryIndex = serde_json::from_reader(reader)?;
            file.unlock()?;
            Ok(index)
        })
        .await??;

        let mut cache = self.inner.cache.write().await;
        cache.data = data.clone();
        cache.fetched_at = Instant::now();

        Ok(data)
    }

    /// Get the tournament status JSON, with TTL cache.
    pub async fn get_tournament_status(&self) -> Result<serde_json::Value> {
        {
            let cache = self.inner.tournament_status_cache.read().await;
            if cache.fetched_at.elapsed() < self.inner.ttl && !cache.data.is_null() {
                return Ok(cache.data.clone());
            }
        }

        let path = self.inner.tournament_status_path.clone();
        let data = tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
            if !path.exists() {
                return Ok(serde_json::Value::Null);
            }
            let file = std::fs::File::open(&path)?;
            let reader = std::io::BufReader::new(&file);
            let value: serde_json::Value = serde_json::from_reader(reader)?;
            Ok(value)
        })
        .await??;

        let mut cache = self.inner.tournament_status_cache.write().await;
        cache.data = data.clone();
        cache.fetched_at = Instant::now();

        Ok(data)
    }

    /// Write tournament status JSON to disk and invalidate cache.
    pub async fn set_tournament_status(&self, value: serde_json::Value) -> Result<()> {
        let path = self.inner.tournament_status_path.clone();
        let data = value.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let contents = serde_json::to_string_pretty(&data)?;
            std::fs::write(&path, contents)?;
            Ok(())
        })
        .await??;

        let mut cache = self.inner.tournament_status_cache.write().await;
        cache.data = value;
        cache.fetched_at = Instant::now();

        Ok(())
    }

    /// Check if the provided API key is valid.
    pub fn check_api_key(&self, key: &str) -> bool {
        match &self.inner.api_key {
            Some(expected) => key == expected,
            None => false,
        }
    }
}
