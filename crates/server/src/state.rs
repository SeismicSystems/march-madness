use eyre::{Result, WrapErr};
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use seismic_march_madness::redis_keys::*;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Shared application state with Redis connection and file-based caches for
/// tournament status and forecasts.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    redis: MultiplexedConnection,
    tournament_status_path: PathBuf,
    tournament_status_cache: RwLock<CachedJson>,
    forecasts_path: PathBuf,
    forecasts_cache: RwLock<CachedJson>,
    ttl: Duration,
}

struct CachedJson {
    data: serde_json::Value,
    fetched_at: Instant,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EntryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub creator: String,
    pub has_password: bool,
    pub member_count: usize,
}

#[derive(Debug, Serialize)]
pub struct MirrorResponse {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub admin: String,
    pub entry_count: usize,
}

#[derive(Debug, Serialize)]
pub struct MirrorEntryResponse {
    pub slug: String,
    pub bracket: String,
}

impl AppState {
    pub async fn new(
        tournament_status_path: PathBuf,
        forecasts_path: PathBuf,
        ttl: Duration,
    ) -> Result<Self> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());
        let client = redis::Client::open(url.as_str())
            .wrap_err_with(|| format!("failed to create Redis client from {url}"))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .wrap_err("failed to connect to Redis")?;

        let expired = Instant::now() - ttl - Duration::from_secs(1);
        Ok(Self {
            inner: Arc::new(Inner {
                redis: conn,
                tournament_status_path,
                tournament_status_cache: RwLock::new(CachedJson {
                    data: serde_json::Value::Null,
                    fetched_at: expired,
                }),
                forecasts_path,
                forecasts_cache: RwLock::new(CachedJson {
                    data: serde_json::Value::Null,
                    fetched_at: expired,
                }),
                ttl,
            }),
        })
    }

    fn redis(&self) -> MultiplexedConnection {
        self.inner.redis.clone()
    }

    // ── Entry queries ────────────────────────────────────────────────

    pub async fn get_entries(&self) -> Result<HashMap<String, EntryResponse>> {
        let mut conn = self.redis();
        let all: HashMap<String, String> = conn.hgetall(KEY_ENTRIES).await?;
        let mut result = HashMap::with_capacity(all.len());
        for (addr, json) in all {
            match serde_json::from_str::<EntryData>(&json) {
                Ok(data) => {
                    result.insert(addr, entry_to_response(&data));
                }
                Err(e) => tracing::warn!(addr = %addr, error = %e, "corrupt entry in Redis"),
            }
        }
        Ok(result)
    }

    pub async fn get_entry(&self, address: &str) -> Result<Option<EntryResponse>> {
        let addr = address.to_lowercase();
        let mut conn = self.redis();
        let json: Option<String> = conn.hget(KEY_ENTRIES, &addr).await?;
        match json {
            Some(s) => Ok(Some(entry_to_response(&serde_json::from_str::<EntryData>(
                &s,
            )?))),
            None => Ok(None),
        }
    }

    pub async fn get_entry_count(&self) -> Result<usize> {
        let mut conn = self.redis();
        let count: usize = conn.hlen(KEY_ENTRIES).await?;
        Ok(count)
    }

    // ── Group queries ────────────────────────────────────────────────

    pub async fn get_groups(&self) -> Result<Vec<GroupResponse>> {
        let mut conn = self.redis();
        let all: HashMap<String, String> = conn.hgetall(KEY_GROUPS).await?;
        let mut groups = Vec::with_capacity(all.len());
        for (id, json) in all {
            match serde_json::from_str::<GroupData>(&json) {
                Ok(data) => groups.push(group_to_response(&id, &data)),
                Err(e) => tracing::warn!(group_id = %id, error = %e, "corrupt group in Redis"),
            }
        }
        Ok(groups)
    }

    pub async fn get_group_by_slug(&self, slug: &str) -> Result<Option<GroupResponse>> {
        let (id, data) = match self.resolve_group(slug).await? {
            Some(v) => v,
            None => return Ok(None),
        };
        Ok(Some(group_to_response(&id, &data)))
    }

    pub async fn get_group_members(&self, slug: &str) -> Result<Option<Vec<String>>> {
        let (id, _data) = match self.resolve_group(slug).await? {
            Some(v) => v,
            None => return Ok(None),
        };
        let mut conn = self.redis();
        let json: Option<String> = conn.hget(KEY_GROUP_MEMBERS, &id).await?;
        let members: Vec<String> = json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        Ok(Some(members))
    }

    /// Slug → (id, GroupData) lookup, shared by group queries.
    async fn resolve_group(&self, slug: &str) -> Result<Option<(String, GroupData)>> {
        let mut conn = self.redis();
        let id: Option<String> = conn.hget(KEY_GROUP_SLUGS, slug).await?;
        let Some(id) = id else {
            return Ok(None);
        };
        let json: Option<String> = conn.hget(KEY_GROUPS, &id).await?;
        match json {
            Some(s) => Ok(Some((id, serde_json::from_str(&s)?))),
            None => Ok(None),
        }
    }

    // ── Mirror queries ───────────────────────────────────────────────

    pub async fn get_mirrors(&self) -> Result<Vec<MirrorResponse>> {
        let mut conn = self.redis();
        let all: HashMap<String, String> = conn.hgetall(KEY_MIRRORS).await?;

        // Count entries per mirror from the composite-key mirror:entries hash.
        let entry_keys: Vec<String> = conn.hkeys(KEY_MIRROR_ENTRIES).await?;
        let mut entry_counts: HashMap<&str, usize> = HashMap::new();
        for key in &entry_keys {
            if let Some(id) = key.split(':').next() {
                *entry_counts.entry(id).or_default() += 1;
            }
        }

        let mut mirrors = Vec::with_capacity(all.len());
        for (id, json) in &all {
            match serde_json::from_str::<MirrorData>(json) {
                Ok(data) => {
                    mirrors.push(MirrorResponse {
                        id: id.clone(),
                        slug: data.slug,
                        display_name: data.display_name,
                        admin: data.admin,
                        entry_count: entry_counts.get(id.as_str()).copied().unwrap_or(0),
                    });
                }
                Err(e) => tracing::warn!(mirror_id = %id, error = %e, "corrupt mirror in Redis"),
            }
        }
        Ok(mirrors)
    }

    pub async fn get_mirror_by_slug(&self, slug: &str) -> Result<Option<MirrorResponse>> {
        let (id, data) = match self.resolve_mirror(slug).await? {
            Some(v) => v,
            None => return Ok(None),
        };
        let entry_count = self.count_mirror_entries(&id).await?;
        Ok(Some(MirrorResponse {
            id,
            slug: data.slug,
            display_name: data.display_name,
            admin: data.admin,
            entry_count,
        }))
    }

    pub async fn get_mirror_entries(&self, slug: &str) -> Result<Option<Vec<MirrorEntryResponse>>> {
        let mut conn = self.redis();
        let id: Option<String> = conn.hget(KEY_MIRROR_SLUGS, slug).await?;
        let Some(id) = id else {
            return Ok(None);
        };
        let all_entries: HashMap<String, String> = conn.hgetall(KEY_MIRROR_ENTRIES).await?;
        let prefix = format!("{id}:");
        let result: Vec<MirrorEntryResponse> = all_entries
            .into_iter()
            .filter_map(|(key, bracket)| {
                key.strip_prefix(&prefix)
                    .map(|entry_slug| MirrorEntryResponse {
                        slug: entry_slug.to_string(),
                        bracket,
                    })
            })
            .collect();
        Ok(Some(result))
    }

    /// Slug → (id, MirrorData) lookup, shared by mirror queries.
    async fn resolve_mirror(&self, slug: &str) -> Result<Option<(String, MirrorData)>> {
        let mut conn = self.redis();
        let id: Option<String> = conn.hget(KEY_MIRROR_SLUGS, slug).await?;
        let Some(id) = id else {
            return Ok(None);
        };
        let json: Option<String> = conn.hget(KEY_MIRRORS, &id).await?;
        match json {
            Some(s) => Ok(Some((id, serde_json::from_str(&s)?))),
            None => Ok(None),
        }
    }

    /// Count entries for a mirror by scanning composite key prefixes.
    /// Uses HKEYS (keys only) to avoid loading bracket data.
    async fn count_mirror_entries(&self, mirror_id: &str) -> Result<usize> {
        let mut conn = self.redis();
        let keys: Vec<String> = conn.hkeys(KEY_MIRROR_ENTRIES).await?;
        let prefix = format!("{mirror_id}:");
        Ok(keys.iter().filter(|k| k.starts_with(&prefix)).count())
    }

    // ── File-based caches (tournament status, forecasts) ─────────────

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

    pub async fn get_forecasts(&self) -> Result<serde_json::Value> {
        {
            let cache = self.inner.forecasts_cache.read().await;
            if cache.fetched_at.elapsed() < self.inner.ttl && !cache.data.is_null() {
                return Ok(cache.data.clone());
            }
        }

        let path = self.inner.forecasts_path.clone();
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

        let mut cache = self.inner.forecasts_cache.write().await;
        cache.data = data.clone();
        cache.fetched_at = Instant::now();

        Ok(data)
    }
}

fn entry_to_response(data: &EntryData) -> EntryResponse {
    EntryResponse {
        name: data.name.clone(),
        bracket: data.bracket.clone(),
        block: Some(data.block),
        ts: Some(data.ts),
    }
}

fn group_to_response(id: &str, data: &GroupData) -> GroupResponse {
    GroupResponse {
        id: id.to_string(),
        slug: data.slug.clone(),
        display_name: data.display_name.clone(),
        creator: data.creator.clone(),
        has_password: data.has_password,
        member_count: data.member_count as usize,
    }
}
