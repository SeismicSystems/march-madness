use chrono::{Duration, Utc};
use reqwest::blocking::Client;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::auth::{KalshiAuth, workspace_root};
use crate::types::{
    CachedOrderbooks, CachedRound, KALSHI_API_BASE, Market, MarketDef, MarketsResponse, Orderbook,
    OrderbookLevel, OrderbookResponse,
};

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

    /// Fetch the orderbook for a single market ticker.
    ///
    /// `GET /trade-api/v2/markets/{ticker}/orderbook?depth={depth}`
    ///
    /// NO bids at price X are converted to YES asks at (100 - X) cents.
    pub fn get_orderbook(
        &self,
        ticker: &str,
        depth: usize,
    ) -> Result<Orderbook, Box<dyn std::error::Error>> {
        let path = format!("/trade-api/v2/markets/{}/orderbook", ticker);
        let url = format!("{}{}?depth={}", KALSHI_API_BASE, path, depth);

        let headers = self.auth.auth_headers("GET", &path)?;
        let mut req = self.client.get(&url);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send()?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(format!("Kalshi orderbook API error ({}): {}", status, body).into());
        }

        let body = resp.text()?;
        let data: OrderbookResponse = serde_json::from_str(&body).map_err(|e| {
            format!(
                "failed to parse orderbook for {}: {} — body: {}",
                ticker,
                e,
                &body[..body.len().min(500)]
            )
        })?;

        // Parse levels from whichever format the API returned
        let (raw_yes, raw_no): (Vec<[u32; 2]>, Vec<[u32; 2]>) = match data {
            OrderbookResponse::Legacy { orderbook } => (orderbook.yes, orderbook.no),
            OrderbookResponse::Fp { orderbook_fp } => {
                let parse = |entries: &[[String; 2]]| -> Result<Vec<[u32; 2]>, Box<dyn std::error::Error>> {
                    entries
                        .iter()
                        .map(|[price_str, qty_str]| {
                            let price_dollars: f64 = price_str.parse()?;
                            let qty_dollars: f64 = qty_str.parse()?;
                            Ok([
                                (price_dollars * 100.0).round() as u32,
                                qty_dollars.round() as u32,
                            ])
                        })
                        .collect()
                };
                (parse(&orderbook_fp.yes_dollars)?, parse(&orderbook_fp.no_dollars)?)
            }
        };

        // YES bids come directly from the YES side, sorted descending by price
        let mut yes_bids: Vec<OrderbookLevel> = raw_yes
            .iter()
            .map(|[p, q]| OrderbookLevel {
                price: *p,
                quantity: *q,
            })
            .collect();
        yes_bids.sort_by(|a, b| b.price.cmp(&a.price));

        // NO bids at price X → YES asks at (100 - X) cents, sorted ascending by price
        let mut yes_asks: Vec<OrderbookLevel> = raw_no
            .iter()
            .map(|[p, q]| OrderbookLevel {
                price: 100 - p,
                quantity: *q,
            })
            .collect();
        yes_asks.sort_by(|a, b| a.price.cmp(&b.price));

        debug!(
            ticker,
            yes_bids = yes_bids.len(),
            yes_asks = yes_asks.len(),
            "fetched orderbook"
        );

        Ok(Orderbook {
            ticker: ticker.to_string(),
            yes_bids,
            yes_asks,
        })
    }

    /// Fetch orderbooks for all tickers in a round.
    pub fn get_round_orderbooks(
        &self,
        markets: &[Market],
        depth: usize,
        sleep_ms: u64,
    ) -> Result<Vec<Orderbook>, Box<dyn std::error::Error>> {
        let mut orderbooks = Vec::new();
        for (i, market) in markets.iter().enumerate() {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
            }
            let ob = self.get_orderbook(&market.ticker, depth)?;
            orderbooks.push(ob);
        }
        Ok(orderbooks)
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

// ---------------------------------------------------------------------------
// Orderbook cache
// ---------------------------------------------------------------------------

fn orderbook_cache_path(market: &MarketDef) -> PathBuf {
    cache_dir().join(format!(
        "orderbook_round{}_{}.json",
        market.round, market.label
    ))
}

pub fn load_orderbook_cache(market: &MarketDef, ttl: Duration) -> Option<CachedOrderbooks> {
    let path = orderbook_cache_path(market);
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedOrderbooks = serde_json::from_str(&content).ok()?;
    let age = Utc::now() - cached.fetched_at;
    if age > ttl {
        debug!(
            "orderbook cache expired for {} (age: {}s, ttl: {}s)",
            market.label,
            age.num_seconds(),
            ttl.num_seconds()
        );
        return None;
    }
    debug!(
        "using cached orderbook {} (age: {}s)",
        market.label,
        age.num_seconds()
    );
    Some(cached)
}

pub fn save_orderbook_cache(
    market: &MarketDef,
    orderbooks: &[Orderbook],
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedOrderbooks {
        fetched_at: Utc::now(),
        event_ticker: market.event_ticker.to_string(),
        round: market.round,
        orderbooks: orderbooks.to_vec(),
    };
    let json = serde_json::to_string_pretty(&cached)?;
    let path = orderbook_cache_path(market);
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
