//! Sonora LRCLIB lyrics provider — pure client logic (host-independent).
//!
//! This module builds LRCLIB search URLs and maps LRCLIB responses to
//! Sonora lyrics answers. It performs no I/O itself: the WASM ABI layer
//! (`abi`, wasm32 only) drives it with bytes obtained through the
//! capability-gated `sonora.net_fetch` host function. Everything here is
//! unit-testable on the host.

use serde::{Deserialize, Serialize};

/// LRCLIB "get" endpoint. The host allow-list must contain this origin.
pub const LRCLIB_GET_URL: &str = "https://lrclib.net/api/get";

/// Attribution recorded on every answer.
pub const ATTRIBUTION: &str = "LRCLIB (lrclib.net)";

/// Upper bound for a generated search URL (LRCLIB rejects absurd lengths).
pub const MAX_URL_LEN: usize = 2000;

/// Subset of the host `LyricsQueryDto` the provider needs.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LyricsQuery {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl LyricsQuery {
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("invalid lyrics query JSON: {e}"))
    }
}

/// Percent-encode a query value (RFC 3986 unreserved set).
pub fn percent_encode(raw: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => {
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 15) as usize] as char);
            }
        }
    }
    out
}

/// Build the LRCLIB search URL. Returns an error for empty titles or
/// overlong URLs; the caller treats both as a provider miss.
pub fn build_search_url(query: &LyricsQuery) -> Result<String, String> {
    let title = query.title.trim();
    if title.is_empty() {
        return Err("empty title".to_string());
    }
    let mut url = String::from(LRCLIB_GET_URL);
    url.push_str("?track_name=");
    url.push_str(&percent_encode(title));
    if let Some(artist) = query.artist.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        url.push_str("&artist_name=");
        url.push_str(&percent_encode(artist));
    }
    if let Some(album) = query.album.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        url.push_str("&album_name=");
        url.push_str(&percent_encode(album));
    }
    if let Some(ms) = query.duration_ms {
        if ms > 0 {
            url.push_str("&duration=");
            url.push_str(&(ms / 1000).to_string());
        }
    }
    if url.len() > MAX_URL_LEN {
        return Err("search URL too long".to_string());
    }
    Ok(url)
}

#[derive(Debug, Deserialize)]
struct LrclibResponse {
    #[serde(default)]
    instrumental: Option<bool>,
    #[serde(rename = "syncedLyrics", default)]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics", default)]
    plain_lyrics: Option<String>,
}

/// Resolution outcome for one LRCLIB HTTP round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveOutcome {
    /// Answer payload: (`format`, `content`) with format in
    /// `plain | lrc | enhanced_lrc | ttml`.
    Hit { format: &'static str, content: String },
    /// No usable lyrics: 404s, rate limits, timeouts, invalid bodies and
    /// instrumentals all land here. The caller logs the status class.
    Miss,
}

/// Map an HTTP status + body to an outcome. Non-200 statuses are misses by
/// construction (the host reports the code via `net_last_status` for logging).
pub fn map_response(status: i32, body: &[u8]) -> ResolveOutcome {
    if status != 200 {
        return ResolveOutcome::Miss;
    }
    let parsed: LrclibResponse = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(_) => return ResolveOutcome::Miss,
    };
    if parsed.instrumental == Some(true) {
        return ResolveOutcome::Miss;
    }
    if let Some(synced) = parsed.synced_lyrics {
        if !synced.trim().is_empty() {
            return ResolveOutcome::Hit {
                format: "lrc",
                content: synced,
            };
        }
    }
    if let Some(plain) = parsed.plain_lyrics {
        if !plain.trim().is_empty() {
            return ResolveOutcome::Hit {
                format: "plain",
                content: plain,
            };
        }
    }
    ResolveOutcome::Miss
}

/// Host `LyricsResultDto` envelope.
#[derive(Debug, Serialize)]
pub struct LyricsAnswer<'a> {
    pub format: &'a str,
    pub content: &'a str,
    pub attribution: &'a str,
}

pub fn answer_json(format: &str, content: &str) -> String {
    serde_json::to_string(&LyricsAnswer {
        format,
        content,
        attribution: ATTRIBUTION,
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_reserved_characters() {
        assert_eq!(percent_encode("AC/DC & Björk?"), "AC%2FDC%20%26%20Bj%C3%B6rk%3F");
        assert_eq!(percent_encode("abc-_.~09AZaz"), "abc-_.~09AZaz");
    }

    #[test]
    fn builds_search_url_with_optional_fields() {
        let q = LyricsQuery {
            title: "Get Lucky".to_string(),
            artist: Some("Daft Punk".to_string()),
            album: Some("Random Access Memories".to_string()),
            duration_ms: Some(369_000),
        };
        assert_eq!(
            build_search_url(&q).unwrap(),
            "https://lrclib.net/api/get?track_name=Get%20Lucky&artist_name=Daft%20Punk&album_name=Random%20Access%20Memories&duration=369"
        );
    }

    #[test]
    fn rejects_empty_title() {
        let q = LyricsQuery {
            title: "   ".to_string(),
            ..Default::default()
        };
        assert!(build_search_url(&q).is_err());
    }

    #[test]
    fn prefers_synced_over_plain() {
        let body = br#"{"syncedLyrics":"[00:01.00] Hi\n","plainLyrics":"Hi","instrumental":false}"#;
        assert_eq!(
            map_response(200, body),
            ResolveOutcome::Hit {
                format: "lrc",
                content: "[00:01.00] Hi\n".to_string()
            }
        );
    }

    #[test]
    fn falls_back_to_plain_and_rejects_instrumental() {
        let body = br#"{"syncedLyrics":null,"plainLyrics":"La la","instrumental":false}"#;
        assert!(matches!(
            map_response(200, body),
            ResolveOutcome::Hit { format: "plain", .. }
        ));
        let body = br#"{"instrumental":true,"syncedLyrics":"[00:01.00] x","plainLyrics":"x"}"#;
        assert_eq!(map_response(200, body), ResolveOutcome::Miss);
    }

    #[test]
    fn misses_on_errors_and_garbage() {
        for status in [404, 429, 500, -2, -3] {
            assert_eq!(map_response(status, b"{}"), ResolveOutcome::Miss);
        }
        assert_eq!(map_response(200, b"not json"), ResolveOutcome::Miss);
        assert_eq!(map_response(200, br#"{"syncedLyrics":"  "}"#), ResolveOutcome::Miss);
    }
}
