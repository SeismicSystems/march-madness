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
}

struct CachedIndex {
    data: EntryIndex,
    fetched_at: Instant,
}

impl AppState {
    pub fn new(index_path: PathBuf, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Inner {
                index_path,
                cache: RwLock::new(CachedIndex {
                    data: EntryIndex::new(),
                    fetched_at: Instant::now() - ttl - Duration::from_secs(1), // force first load
                }),
                ttl,
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
}
