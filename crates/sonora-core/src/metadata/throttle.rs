//! Thread-safe asynchronous rate limiter for external providers.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Rate limiter guaranteeing a minimum delay between outbound requests.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    min_interval: Duration,
    state: Arc<Mutex<RateLimiterState>>,
}

#[derive(Debug)]
struct RateLimiterState {
    next_allowed_time: Instant,
}

impl RateLimiter {
    /// Create a new RateLimiter with the specified minimum interval between requests.
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            state: Arc::new(Mutex::new(RateLimiterState {
                next_allowed_time: Instant::now(),
            })),
        }
    }

    /// Default MusicBrainz compliant rate limiter (1 request per 1000ms).
    pub fn musicbrainz() -> Self {
        Self::new(Duration::from_millis(1000))
    }

    /// Asynchronously acquire a rate limit slot, sleeping if necessary.
    pub async fn acquire(&self) {
        let mut state = self.state.lock().await;
        let now = Instant::now();

        if state.next_allowed_time > now {
            let delay = state.next_allowed_time - now;
            state.next_allowed_time += self.min_interval;
            drop(state);
            tokio::time::sleep(delay).await;
        } else {
            state.next_allowed_time = now + self.min_interval;
        }
    }

    /// Inject an explicit backoff duration (e.g., from HTTP 429 `Retry-After`).
    pub async fn delay_until(&self, duration: Duration) {
        let mut state = self.state.lock().await;
        let target = Instant::now() + duration;
        if target > state.next_allowed_time {
            state.next_allowed_time = target;
        }
    }
}
