use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::Request;
use tracing::{debug, info, warn};

use crate::auth::KalshiAuth;
use crate::types::{KALSHI_WS_PATH, KALSHI_WS_URL, WsEnvelope, WsTickerMsg};

fn parse_f64(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse().ok()).unwrap_or(0.0)
}

/// Max market tickers per subscription message.
const SUBSCRIBE_BATCH_SIZE: usize = 100;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub struct KalshiWs {
    ws: WsStream,
    next_id: u64,
}

impl KalshiWs {
    pub async fn connect(auth: &KalshiAuth) -> Result<Self, Box<dyn std::error::Error>> {
        let headers = auth.auth_headers("GET", KALSHI_WS_PATH)?;

        let mut req = Request::builder().uri(KALSHI_WS_URL);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let req = req.body(())?;

        info!("connecting to Kalshi WebSocket");
        let (ws, _resp) = tokio_tungstenite::connect_async(req).await?;
        info!("WebSocket connected");

        Ok(Self { ws, next_id: 1 })
    }

    /// Subscribe to the `ticker` channel for the given market tickers.
    /// Batches into chunks of 100 to stay within limits.
    pub async fn subscribe_ticker(
        &mut self,
        tickers: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for chunk in tickers.chunks(SUBSCRIBE_BATCH_SIZE) {
            let id = self.next_id;
            self.next_id += 1;
            let msg = serde_json::json!({
                "id": id,
                "cmd": "subscribe",
                "params": {
                    "channels": ["ticker"],
                    "market_tickers": chunk,
                }
            });
            debug!(id, n = chunk.len(), "subscribing to ticker batch");
            self.ws.send(Message::Text(msg.to_string())).await?;
        }
        Ok(())
    }

    /// Read the next ticker update. Returns (market_ticker, yes_bid, yes_ask).
    /// Bid/ask are 0.0 when absent. Skips non-ticker messages.
    /// Returns None on stream close.
    pub async fn next_ticker(
        &mut self,
    ) -> Option<Result<(String, f64, f64), Box<dyn std::error::Error>>> {
        loop {
            let msg = self.ws.next().await?;
            let msg = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Ping(data)) => {
                    if let Err(e) = self.ws.send(Message::Pong(data)).await {
                        return Some(Err(e.into()));
                    }
                    continue;
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => continue,
                Err(e) => return Some(Err(e.into())),
            };

            let envelope: WsEnvelope = match serde_json::from_str(&msg) {
                Ok(e) => e,
                Err(e) => {
                    debug!(error = %e, raw = %msg, "failed to parse WS message");
                    continue;
                }
            };

            match envelope.msg_type.as_str() {
                "ticker" => {
                    if let Some(raw_msg) = envelope.msg {
                        match serde_json::from_value::<WsTickerMsg>(raw_msg) {
                            Ok(t) => {
                                let bid = parse_f64(t.yes_bid_dollars.as_deref());
                                let ask = parse_f64(t.yes_ask_dollars.as_deref());
                                return Some(Ok((t.market_ticker, bid, ask)));
                            }
                            Err(e) => {
                                debug!(error = %e, "failed to parse ticker msg");
                                continue;
                            }
                        }
                    }
                }
                "subscribed" => {
                    debug!(sid = envelope.sid, "subscription confirmed");
                }
                "error" => {
                    let body = envelope.msg.map(|v| v.to_string()).unwrap_or_default();
                    warn!(body = %body, "WS error message");
                }
                other => {
                    debug!(msg_type = other, "ignoring WS message type");
                }
            }
        }
    }
}
