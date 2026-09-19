//! Metadata provider error types.

use std::time::Duration;
use thiserror::Error;

/// Error type encompassing provider communications, rate limits, parsing, and query failures.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Rate limited by provider: retry after {retry_after_secs:?} seconds")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Network connection error: {0}")]
    Network(String),

    #[error("Request timed out")]
    Timeout,

    #[error("Network unavailable / offline")]
    Offline,

    #[error("Failed to parse provider response: {0}")]
    Parse(String),

    #[error("Requested entity not found (404)")]
    NotFound,

    #[error("HTTP error from provider (status {status}): {message}")]
    Http { status: u16, message: String },

    #[error("Invalid query parameters: {0}")]
    InvalidQuery(String),

    #[error("Storage or caching error: {0}")]
    Storage(String),

    #[error("Provider configuration error: {0}")]
    Configuration(String),

    #[error("Internal provider error: {0}")]
    Other(String),
}

impl ProviderError {
    /// Helper to construct a rate limit error with optional Duration.
    pub fn rate_limited(duration: Option<Duration>) -> Self {
        Self::RateLimited {
            retry_after_secs: duration.map(|d| d.as_secs().max(1)),
        }
    }
}
