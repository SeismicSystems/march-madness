use chrono::{Duration, Utc};
use reqwest::blocking::Client;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::auth::{KalshiAuth, workspace_root};
use crate::types::{CachedRound, KALSHI_API_BASE, Market, MarketDef, MarketsResponse};

pub struct KalshiRestClient {
    client: Client,
    auth: KalshiAuth,
}

impl KalshiRestClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let auth = KalshiAuth::from_env()?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { client, auth })
    }

    pub fn get_all_markets(
        &self,
        event_ticker: &str,
        sleep_ms: u64,
    ) -> Result<Vec<Market>, Box<dyn std::error::Error>> {
        let path = "/trade-api/v2/markets";
        let mut all_markets = Vec::new();
        let mut cursor: Option<String> = None;

        let mut page = 0usize;
        loop {
            page += 1;
            let mut url = format!(
                "{}{}?event_ticker={}&limit=200",
                KALSHI_API_BASE, path, event_ticker
            );
            if let Some(ref c) = cursor {
                url.push_str(&format!("&cursor={}", c));
            }
            debug!("page {} for {}", page, event_ticker);

            let headers = self.auth.auth_headers("GET", path)?;
            let mut req = self.client.get(&url);
            for (k, v) in &headers {
                req = req.header(k.as_str(), v.as_str());
            }
            let resp = req.send()?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().unwrap_or_default();
                return Err(format!("Kalshi API error ({}): {}", status, body).into());
            }

            let data: MarketsResponse = resp.json()?;
            let count = data.markets.len();
            all_markets.extend(data.markets);
            cursor = data.cursor.filter(|c| !c.is_empty());
            debug!(
                "page {} for {}: got {} markets (total: {}, more: {})",
                page,
                event_ticker,
                count,
                all_markets.len(),
                cursor.is_some()
            );
            if cursor.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        }

        Ok(all_markets)
    }
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

fn cache_dir() -> PathBuf {
    workspace_root().join("data").join("cache")
}

fn cache_path(market: &MarketDef) -> PathBuf {
    cache_dir().join(format!("round{}_{}.json", market.round, market.label))
}

pub fn load_cache(market: &MarketDef, ttl: Duration) -> Option<CachedRound> {
    let path = cache_path(market);
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedRound = serde_json::from_str(&content).ok()?;
    let age = Utc::now() - cached.fetched_at;
    if age > ttl {
        debug!(
            "cache expired for {} (age: {}s, ttl: {}s)",
            market.label,
            age.num_seconds(),
            ttl.num_seconds()
        );
        return None;
    }
    debug!(
        "using cached {} (age: {}s)",
        market.label,
        age.num_seconds()
    );
    Some(cached)
}

pub fn save_cache(
    market: &MarketDef,
    markets: &[Market],
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedRound {
        fetched_at: Utc::now(),
        event_ticker: market.event_ticker.to_string(),
        round: market.round,
        markets: markets.to_vec(),
    };
    let json = serde_json::to_string_pretty(&cached)?;
    let path = cache_path(market);
    fs::write(&path, json)?;
    Ok(())
}

/// Load team names from first column of a CSV file with headers.
pub fn load_known_teams(path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)?;
    let mut names = Vec::new();
    for result in reader.records() {
        let record = result?;
        if let Some(name) = record.get(0) {
            names.push(name.to_string());
        }
    }
    Ok(names)
}
