//! Host-side network transport for `sonora.net_fetch`.
//!
//! The sandbox never touches sockets: plugins call the `net_fetch` host
//! function, and the host performs the HTTPS request on their behalf — only
//! for allow-listed origins declared under `network:fetch`. Responses are
//! bounded ([`PLUGIN_NET_MAX_BYTES`]) and timed out
//! ([`PLUGIN_NET_TIMEOUT_SECS`]); redirects are never followed, so an
//! allow-listed origin cannot bounce the host to an unlisted one.
//!
//! [`FakeNetworkTransport`] provides deterministic scripted responses for
//! plugin developers and the test suite (no network required).

use std::sync::Mutex;
use std::time::Duration;

/// Timeout for a single plugin network fetch.
pub fn net_timeout() -> Duration {
    Duration::from_secs(crate::PLUGIN_NET_TIMEOUT_SECS)
}

/// Transport-level fetch outcome. HTTP error statuses are surfaced so plugins
/// can distinguish "no lyrics" (404) from "back off" (429).
#[derive(Debug, Clone)]
pub enum NetworkError {
    HttpStatus(u16),
    Timeout,
    TooLarge,
    Io(String),
}

/// Allow-list-checked HTTPS fetch performed by the host.
pub trait NetworkTransport: Send + Sync {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, NetworkError>;
}

/// Production transport: blocking HTTPS via ureq (no async runtime, so the
/// client is safe to create, use, and drop in any thread context — including
/// inside async runtimes and Tauri handlers).
pub struct RealNetworkTransport {
    agent: ureq::Agent,
    timeout: Duration,
}

impl Default for RealNetworkTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RealNetworkTransport {
    pub fn new() -> Self {
        let timeout = net_timeout();
        let agent = ureq::AgentBuilder::new()
            .timeout(timeout)
            .redirects(0)
            .user_agent("Sonora/0.1 (https://github.com/sonora-audio/sonora)")
            .build();
        Self { agent, timeout }
    }
}

impl NetworkTransport for RealNetworkTransport {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, NetworkError> {
        use std::io::Read as _;
        let start = std::time::Instant::now();
        let resp = match self.agent.get(url).call() {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, _)) => return Err(NetworkError::HttpStatus(code)),
            Err(_) => {
                // Timeout vs fast failure, without depending on error internals:
                // only a timeout can still be running after the full budget.
                return Err(if start.elapsed() >= self.timeout {
                    NetworkError::Timeout
                } else {
                    NetworkError::Io("request failed".to_string())
                });
            }
        };
        if resp.status() != 200 {
            return Err(NetworkError::HttpStatus(resp.status()));
        }
        let mut bounded = resp.into_reader().take(max_bytes as u64 + 1);
        let mut body = Vec::new();
        bounded
            .read_to_end(&mut body)
            .map_err(|e| NetworkError::Io(e.to_string()))?;
        if body.len() > max_bytes {
            return Err(NetworkError::TooLarge);
        }
        Ok(body)
    }
}

/// Scripted outcome for one fake route.
#[derive(Debug, Clone)]
pub enum FakeOutcome {
    Status(u16, Vec<u8>),
    Timeout,
    TooLarge,
    Io(String),
}

/// Deterministic transport for tests and plugin development. Routes match by
/// substring; when several match, the most recently registered wins (so a
/// test can override an earlier route). Unmatched URLs hit the fallback
/// (an I/O error by default, so tests fail loudly on unexpected egress).
pub struct FakeNetworkTransport {
    routes: Mutex<Vec<(String, FakeOutcome)>>,
    calls: Mutex<Vec<String>>,
    fallback: FakeOutcome,
}

impl FakeNetworkTransport {
    pub fn new() -> Self {
        Self {
            routes: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
            fallback: FakeOutcome::Io("no fake route for URL".to_string()),
        }
    }

    /// Register a route: on overlap the most recent registration wins.
    pub fn route(&self, url_contains: &str, outcome: FakeOutcome) {
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((url_contains.to_string(), outcome));
    }

    /// Every URL the transport was asked to fetch, in order.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Default for FakeNetworkTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkTransport for FakeNetworkTransport {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, NetworkError> {
        self.calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(url.to_string());
        let routes = self.routes.lock().unwrap_or_else(|e| e.into_inner());
        let outcome = routes
            .iter()
            .rev()
            .find(|(key, _)| url.contains(key.as_str()))
            .map(|(_, o)| o.clone())
            .unwrap_or_else(|| self.fallback.clone());
        drop(routes);
        match outcome {
            FakeOutcome::Status(200, body) => {
                if body.len() > max_bytes {
                    Err(NetworkError::TooLarge)
                } else {
                    Ok(body)
                }
            }
            FakeOutcome::Status(s, _) => Err(NetworkError::HttpStatus(s)),
            FakeOutcome::Timeout => Err(NetworkError::Timeout),
            FakeOutcome::TooLarge => Err(NetworkError::TooLarge),
            FakeOutcome::Io(e) => Err(NetworkError::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_routes_match_latest_first_and_log_calls() {
        let t = FakeNetworkTransport::new();
        t.route("lrclib.net", FakeOutcome::Status(200, b"{}".to_vec()));
        // Overlapping later route overrides the earlier one.
        t.route("lrclib.net/api", FakeOutcome::Status(404, vec![]));
        assert!(matches!(
            t.fetch("https://lrclib.net/api/get?a=1", 1024),
            Err(NetworkError::HttpStatus(404))
        ));
        assert!(t.fetch("https://lrclib.net/other", 1024).is_ok());
        assert!(matches!(
            t.fetch("https://unmatched.example/", 1024),
            Err(NetworkError::Io(_))
        ));
        assert_eq!(t.calls().len(), 3);
    }
}
