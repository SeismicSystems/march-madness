//! Rate-limited NCAA API HTTP client with 429 backoff.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, warn};

use crate::NcaaApiError;

/// NCAA API base URL.
pub const NCAA_API_BASE: &str = "https://sdataprod.ncaa.com/";

/// Rate-limited HTTP client for the NCAA API.
#[derive(Clone)]
pub struct NcaaClient {
    http: reqwest::Client,
    state: Arc<Mutex<RateLimitState>>,
    min_interval: Duration,
}

struct RateLimitState {
    last_request: Instant,
    backoff: Duration,
    consecutive_429s: u32,
}

impl NcaaClient {
    /// Create a new client with the given max requests per second.
    ///
    /// # Panics
    /// Panics if `max_requests_per_sec` >= 5.0 or <= 0.0.
    pub fn new(max_requests_per_sec: f64) -> Result<Self, NcaaApiError> {
        if max_requests_per_sec <= 0.0 || max_requests_per_sec >= 5.0 {
            return Err(NcaaApiError::Config(
                "max_requests_per_sec must be > 0 and < 5".into(),
            ));
        }

        let min_interval = Duration::from_secs_f64(1.0 / max_requests_per_sec);

        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| NcaaApiError::Http(e.to_string()))?,
            state: Arc::new(Mutex::new(RateLimitState {
                last_request: Instant::now() - min_interval,
                backoff: Duration::ZERO,
                consecutive_429s: 0,
            })),
            min_interval,
        })
    }

    /// Make a rate-limited GET request. Handles 429 backoff automatically.
    pub async fn get(&self, url: &str) -> Result<String, NcaaApiError> {
        loop {
            // Wait for rate limit
            {
                let state = self.state.lock().await;
                let elapsed = state.last_request.elapsed();

                // Apply backoff if we've been getting 429s
                let wait = if state.backoff > Duration::ZERO {
                    state.backoff
                } else if elapsed < self.min_interval {
                    self.min_interval - elapsed
                } else {
                    Duration::ZERO
                };

                if wait > Duration::ZERO {
                    drop(state);
                    debug!("rate limit: sleeping {}ms", wait.as_millis());
                    tokio::time::sleep(wait).await;
                }
            }

            // Make the request
            {
                let mut state = self.state.lock().await;
                state.last_request = Instant::now();
            }

            debug!("GET {url}");
            let resp = self
                .http
                .get(url)
                .send()
                .await
                .map_err(|e| NcaaApiError::Http(e.to_string()))?;

            let status = resp.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let mut state = self.state.lock().await;
                state.consecutive_429s += 1;
                let new_backoff = if state.backoff == Duration::ZERO {
                    Duration::from_secs(2)
                } else {
                    (state.backoff * 2).min(Duration::from_secs(60))
                };
                state.backoff = new_backoff;
                warn!(
                    "429 rate limited (consecutive: {}), backing off {}s",
                    state.consecutive_429s,
                    new_backoff.as_secs()
                );
                continue;
            }

            // Reset backoff on success
            {
                let mut state = self.state.lock().await;
                if state.consecutive_429s > 0 {
                    debug!(
                        "rate limit cleared after {} consecutive 429s",
                        state.consecutive_429s
                    );
                }
                state.backoff = Duration::ZERO;
                state.consecutive_429s = 0;
            }

            if !status.is_success() {
                return Err(NcaaApiError::Http(format!("HTTP {status} from {url}")));
            }

            let body = resp
                .text()
                .await
                .map_err(|e| NcaaApiError::Http(e.to_string()))?;

            return Ok(body);
        }
    }
}
