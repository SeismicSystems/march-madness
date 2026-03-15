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

impl RateLimitState {
    /// Compute how long to wait before the next request.
    fn wait_duration(&self, min_interval: Duration) -> Duration {
        if self.backoff > Duration::ZERO {
            self.backoff
        } else {
            let elapsed = self.last_request.elapsed();
            if elapsed < min_interval {
                min_interval - elapsed
            } else {
                Duration::ZERO
            }
        }
    }

    /// Record a 429 response: increment counter and double backoff.
    fn record_429(&mut self) {
        self.consecutive_429s += 1;
        self.backoff = if self.backoff == Duration::ZERO {
            Duration::from_secs(2)
        } else {
            (self.backoff * 2).min(Duration::from_secs(60))
        };
        warn!(
            "429 rate limited (consecutive: {}), backing off {}s",
            self.consecutive_429s,
            self.backoff.as_secs()
        );
    }

    /// Reset backoff state after a successful (non-429) response.
    fn record_success(&mut self) {
        if self.consecutive_429s > 0 {
            debug!(
                "rate limit cleared after {} consecutive 429s",
                self.consecutive_429s
            );
        }
        self.backoff = Duration::ZERO;
        self.consecutive_429s = 0;
    }
}

impl NcaaClient {
    /// Create a new client with the given max requests per second.
    ///
    /// Returns an error if `max_requests_per_sec` is not in (0, 5).
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
                .build()?,
            state: Arc::new(Mutex::new(RateLimitState {
                last_request: Instant::now() - min_interval,
                backoff: Duration::ZERO,
                consecutive_429s: 0,
            })),
            min_interval,
        })
    }

    /// Get the inner HTTP client (for reuse by other components, e.g. POST requests).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Make a rate-limited GET request. Handles 429 backoff automatically.
    pub async fn get(&self, url: &str) -> Result<String, NcaaApiError> {
        loop {
            self.wait_for_rate_limit().await;

            debug!("GET {url}");
            let resp = self.http.get(url).send().await?;

            let status = resp.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                self.state.lock().await.record_429();
                continue;
            }

            self.state.lock().await.record_success();

            if !status.is_success() {
                return Err(NcaaApiError::HttpStatus {
                    status,
                    url: url.to_string(),
                });
            }

            return Ok(resp.text().await?);
        }
    }

    /// Wait for rate limit, then stamp the request time.
    async fn wait_for_rate_limit(&self) {
        let wait = self.state.lock().await.wait_duration(self.min_interval);
        if wait > Duration::ZERO {
            debug!("rate limit: sleeping {}ms", wait.as_millis());
            tokio::time::sleep(wait).await;
        }
        self.state.lock().await.last_request = Instant::now();
    }
}

/// Build an NCAA GraphQL persisted-query URL.
pub(crate) fn build_gql_url(hash: &str, variables: &serde_json::Value) -> String {
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": hash
        }
    });
    format!(
        "{}?extensions={}&variables={}",
        NCAA_API_BASE,
        urlencoded(&extensions.to_string()),
        urlencoded(&variables.to_string())
    )
}

/// URL-encode a string for query parameters.
fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
