use serde::{Deserialize, Serialize};

pub const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com";
pub const KALSHI_WS_URL: &str = "wss://api.elections.kalshi.com/trade-api/ws/v2";
pub const KALSHI_WS_PATH: &str = "/trade-api/ws/v2";
pub const YEAR: u16 = 2026;

/// One market/round we care about.
pub struct MarketDef {
    pub event_ticker: &'static str,
    pub round: usize,
    pub label: &'static str,
    pub expected_sum: f64,
    /// Floor probability for backfilled (no-bid) teams.
    pub floor_prob: f64,
}

pub const MARKETS: &[MarketDef] = &[
    MarketDef {
        event_ticker: "KXMARMADROUND-26RO32",
        round: 1,
        label: "R32",
        expected_sum: 32.0,
        floor_prob: 1.0 / 128.0,
    },
    MarketDef {
        event_ticker: "KXMARMADROUND-26S16",
        round: 2,
        label: "S16",
        expected_sum: 16.0,
        floor_prob: 1.0 / 256.0,
    },
    MarketDef {
        event_ticker: "KXMARMADROUND-26E8",
        round: 3,
        label: "E8",
        expected_sum: 8.0,
        floor_prob: 1.0 / 512.0,
    },
    MarketDef {
        event_ticker: "KXMARMADROUND-26F4",
        round: 4,
        label: "F4",
        expected_sum: 4.0,
        floor_prob: 1.0 / 1024.0,
    },
    MarketDef {
        event_ticker: "KXMARMADROUND-26T2",
        round: 5,
        label: "ChampGame",
        expected_sum: 2.0,
        floor_prob: 1.0 / 2048.0,
    },
    MarketDef {
        event_ticker: "KXMARMAD-26",
        round: 6,
        label: "Champion",
        expected_sum: 1.0,
        floor_prob: 1.0 / 4096.0,
    },
];

// ---------------------------------------------------------------------------
// Kalshi API types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MarketsResponse {
    pub cursor: Option<String>,
    pub markets: Vec<Market>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Market {
    pub ticker: String,
    pub title: String,
    pub yes_sub_title: Option<String>,
    pub yes_bid_dollars: Option<String>,
    pub yes_ask_dollars: Option<String>,
    pub last_price_dollars: Option<String>,
    #[serde(default)]
    pub yes_bid_size_fp: Option<String>,
    #[serde(default)]
    pub yes_ask_size_fp: Option<String>,
    #[serde(default)]
    pub volume_fp: Option<String>,
}

// ---------------------------------------------------------------------------
// Cache types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedRound {
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub event_ticker: String,
    pub round: usize,
    pub markets: Vec<Market>,
}

// ---------------------------------------------------------------------------
// Orderbook types
// ---------------------------------------------------------------------------

/// A single price level in an orderbook (price in cents, quantity in contracts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookLevel {
    pub price: u32,
    pub quantity: u32,
}

/// Parsed orderbook for a single market: YES-side bids and asks, sorted by price.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orderbook {
    pub ticker: String,
    /// YES bids sorted descending by price (best bid first).
    pub yes_bids: Vec<OrderbookLevel>,
    /// YES asks sorted ascending by price (best ask first).
    pub yes_asks: Vec<OrderbookLevel>,
}

/// Raw API response: `{"orderbook": {"yes": [[p,q],...], "no": [[p,q],...]}}`.
#[derive(Debug, Deserialize)]
pub struct OrderbookResponse {
    pub orderbook: OrderbookResponseInner,
}

#[derive(Debug, Deserialize)]
pub struct OrderbookResponseInner {
    #[serde(default)]
    pub yes: Vec<[u32; 2]>,
    #[serde(default)]
    pub no: Vec<[u32; 2]>,
}

/// Orderbook data for a specific team, round, and ticker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamOrderbook {
    pub team: String,
    pub round: usize,
    pub ticker: String,
    pub orderbook: Orderbook,
}

/// Cached orderbooks for a round (mirrors CachedRound pattern).
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedOrderbooks {
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub event_ticker: String,
    pub round: usize,
    pub orderbooks: Vec<Orderbook>,
}

// ---------------------------------------------------------------------------
// WebSocket message types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct WsTickerMsg {
    pub market_ticker: String,
    pub yes_bid_dollars: Option<String>,
    pub yes_ask_dollars: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WsEnvelope {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub sid: Option<u64>,
    pub seq: Option<u64>,
    pub msg: Option<serde_json::Value>,
}
