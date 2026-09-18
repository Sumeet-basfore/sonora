//! Sonora Genius lyrics provider — pure client logic (host-independent).
//!
//! Genius offers no token-free lyrics API, so this provider uses the public,
//! unauthenticated web endpoints the same way a browser would: the JSON
//! search API to find the song page, then the song page's lyrics containers.
//! It performs no I/O itself: the WASM ABI layer (`abi`, wasm32 only) drives
//! it with bytes obtained through the capability-gated `sonora.net_fetch`
//! host function. Everything here is unit-testable on the host.
//!
//! The parser is deliberately strict: any structural surprise is a provider
//! miss, never garbage lyrics.

use serde::{Deserialize, Serialize};

/// Public Genius search endpoint (no token required).
pub const SEARCH_URL: &str = "https://genius.com/api/search/multi";
/// Results per search call (small: lyrics need only the top song hit).
pub const SEARCH_PER_PAGE: u32 = 5;

/// Upper bound for a generated search URL.
pub const MAX_URL_LEN: usize = 2000;

/// Minimum non-whitespace characters for extracted lyrics to count.
pub const MIN_LYRICS_CHARS: usize = 20;

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

/// Build the Genius search URL from track metadata ("artist title", the
/// Genius search convention). Empty titles and overlong URLs are misses
/// without network traffic.
pub fn build_search_url(query: &LyricsQuery) -> Result<String, String> {
    let title = query.title.trim();
    if title.is_empty() {
        return Err("empty title".to_string());
    }
    let mut text = String::new();
    if let Some(artist) = query.artist.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        text.push_str(artist);
        text.push(' ');
    }
    text.push_str(title);
    let url = format!("{SEARCH_URL}?per_page={SEARCH_PER_PAGE}&q={}", percent_encode(&text));
    if url.len() > MAX_URL_LEN {
        return Err("search URL too long".to_string());
    }
    Ok(url)
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    response: SearchInner,
}

#[derive(Debug, Default, Deserialize)]
struct SearchInner {
    #[serde(default)]
    sections: Vec<SearchSection>,
}

#[derive(Debug, Deserialize)]
struct SearchSection {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    #[serde(default)]
    result: HitResult,
}

#[derive(Debug, Default, Deserialize)]
struct HitResult {
    #[serde(default)]
    url: Option<String>,
}

/// First song-section hit whose URL is a Genius song page. Anything else
/// (artist pages, albums, off-domain, odd shapes) is skipped fail-closed.
pub fn parse_search_response(body: &[u8]) -> Option<String> {
    let parsed: SearchResponse = serde_json::from_slice(body).ok()?;
    for section in &parsed.response.sections {
        if section.kind != "song" {
            continue;
        }
        for hit in &section.hits {
            if let Some(url) = hit.result.url.as_deref() {
                if is_genius_song_url(url) {
                    return Some(url.to_string());
                }
            }
        }
    }
    None
}

/// Accept only `https://genius.com/<slug>-lyrics` song pages (query strings
/// allowed). Never follow off-domain, non-HTTPS, or non-song URLs.
fn is_genius_song_url(url: &str) -> bool {
    let rest = match url.strip_prefix("https://genius.com/") {
        Some(r) => r,
        None => return false,
    };
    let path = rest.split(['?', '#']).next().unwrap_or("");
    !path.is_empty()
        && path.ends_with("-lyrics")
        && !path.contains('@')
        && !path.contains(' ')
        && !path.contains('\\')
}

/// Extract lyrics text from a Genius song page: concatenate every
/// `data-lyrics-container` div, strip tags (`<br>` becomes a newline),
/// decode entities, and normalize whitespace. Returns `None` when the markup
/// changed shape or holds too little text — a miss, never garbage.
///
/// Fuel-conscious: the scan jumps tag-to-tag with `str::find` (memchr-backed)
/// instead of stepping byte-by-byte, so megabyte-scale pages stay inside the
/// host fuel budget.
pub fn extract_lyrics(html: &[u8]) -> Option<String> {
    let html = std::str::from_utf8(html).ok()?;
    let mut out = String::new();
    let mut found_any = false;
    let mut pos = 0;
    while let Some(rel) = html[pos..].find('<') {
        let abs = pos + rel;
        let rest = &html[abs..];
        if rest.starts_with("<div") && is_tag_boundary(html, abs + 4) {
            let tag_end = rest.find('>')?;
            let open = &rest[..tag_end];
            if !open.contains("data-lyrics-container") {
                pos = abs + 4;
                continue;
            }
            // Self-closing container tags carry no lyrics; skip them safely.
            if open.trim_end().ends_with('/') {
                pos = abs + 4;
                continue;
            }
            let (inner, consumed) = capture_div(html, abs)?;
            append_stripped(&mut out, inner);
            found_any = true;
            pos = abs + consumed;
        } else {
            pos = abs + 1;
        }
    }
    if !found_any {
        return None;
    }
    let text = normalize_whitespace(&out);
    if text.chars().filter(|c| !c.is_whitespace()).count() < MIN_LYRICS_CHARS {
        return None;
    }
    Some(text)
}

/// Starting at an open `<div` tag at `abs`, return the inner HTML and the
/// bytes consumed through the matching close tag. Depth-counts nested divs.
fn capture_div(html: &str, abs: usize) -> Option<(&str, usize)> {
    let rest = html.get(abs..)?;
    let tag_end = rest.find('>')?;
    let mut depth = 1usize;
    let mut pos = abs + tag_end + 1;
    while let Some(rel) = html[pos..].find('<') {
        let at = pos + rel;
        let r = &html[at..];
        if r.starts_with("<div") && is_tag_boundary(html, at + 4) {
            let te = r.find('>')?;
            if !r[..te].trim_end().ends_with('/') {
                depth += 1;
            }
            pos = at + 4;
        } else if r.starts_with("</div") && is_tag_boundary(html, at + 5) {
            depth -= 1;
            if depth == 0 {
                let te = r.find('>')?;
                return Some((&html[abs + tag_end + 1..at], at + te + 1 - abs));
            }
            pos = at + 5;
        } else {
            pos = at + 1;
        }
    }
    None
}

/// True when the character after a tag prefix cannot continue the tag name
/// (`<div-x` and `</divisions>` must not match `<div` / `</div`).
fn is_tag_boundary(html: &str, at: usize) -> bool {
    match html.as_bytes().get(at) {
        None => true,
        Some(&c) => !(c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b':'),
    }
}

/// Strip tags from a lyrics-container fragment (`<br>` → newline), decoding
/// entities in the text runs. Tag-to-tag jumps keep large fragments cheap.
fn append_stripped(out: &mut String, fragment: &str) {
    let mut pos = 0;
    while let Some(rel) = fragment[pos..].find('<') {
        append_text(out, &fragment[pos..pos + rel]);
        let abs = pos + rel;
        let r = &fragment[abs..];
        let te = match r.find('>') {
            Some(i) => i,
            None => break,
        };
        let tag = r[1..te].trim_start_matches('/').trim();
        let name = tag
            .split([' ', '/', '\t', '\n', '\r'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "br" {
            out.push('\n');
        }
        pos = abs + te + 1;
    }
    append_text(out, &fragment[pos..]);
}

/// Copy a tag-free run, decoding entities.
fn append_text(out: &mut String, text: &str) {
    let mut pos = 0;
    while let Some(rel) = text[pos..].find('&') {
        out.push_str(&text[pos..pos + rel]);
        let abs = pos + rel;
        let rest = &text[abs..];
        match decode_entity_at(rest) {
            Some((decoded, len)) => {
                out.push_str(&decoded);
                pos = abs + len;
            }
            None => {
                out.push('&');
                pos = abs + 1;
            }
        }
    }
    out.push_str(&text[pos..]);
}

/// Decode an HTML entity at the start of `text`; returns (text, byte_len).
fn decode_entity_at(text: &str) -> Option<(String, usize)> {
    if !text.starts_with('&') {
        return None;
    }
    let semi = text.find(';')?;
    if semi > 12 {
        return None;
    }
    let body = &text[1..semi];
    let decoded = match body {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" | "#x27" | "#X27" => "'".to_string(),
        "nbsp" => " ".to_string(),
        "lsquo" | "#x2018" | "#8216" => "\u{2018}".to_string(),
        "rsquo" | "#x2019" | "#8217" => "\u{2019}".to_string(),
        "ldquo" | "#x201C" | "#8220" => "\u{201C}".to_string(),
        "rdquo" | "#x201D" | "#8221" => "\u{201D}".to_string(),
        "ndash" | "#x2013" | "#8211" => "\u{2013}".to_string(),
        "mdash" | "#x2014" | "#8212" => "\u{2014}".to_string(),
        "hellip" | "#x2026" | "#8230" => "\u{2026}".to_string(),
        _ if body.starts_with('#') => decode_numeric_entity(&body[1..])?,
        _ => return None,
    };
    Some((decoded, semi + 1))
}

fn decode_numeric_entity(digits: &str) -> Option<String> {
    let code: u32 = if let Some(hex) = digits.strip_prefix(['x', 'X']) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        digits.parse().ok()?
    };
    if code == 0 {
        return None;
    }
    char::from_u32(code).map(|c| c.to_string())
}

fn normalize_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        // Collapse runs of >2 blank lines; drop leading blanks.
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;
        // Collapse inner whitespace runs to single spaces.
        let mut first = true;
        for word in trimmed.split_whitespace() {
            if !first {
                out.push(' ');
            }
            out.push_str(word);
            first = false;
        }
        out.push('\n');
    }
    // Trim trailing newlines, keep at most one.
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// Resolution outcome for one query (both fetches done by the caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveOutcome {
    Hit { content: String, song_url: String },
    Miss,
}

/// Host `LyricsResultDto` envelope.
#[derive(Debug, Serialize)]
pub struct LyricsAnswer<'a> {
    pub format: &'a str,
    pub content: &'a str,
    pub attribution: &'a str,
}

pub fn attribution_for(song_url: &str) -> String {
    format!("Genius (genius.com) — {song_url}")
}

pub fn answer_json(content: &str, song_url: &str) -> String {
    serde_json::to_string(&LyricsAnswer {
        format: "plain",
        content,
        attribution: &attribution_for(song_url),
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_url_prefers_artist_title_order() {
        let q = LyricsQuery {
            title: "HUMBLE.".to_string(),
            artist: Some("Kendrick Lamar".to_string()),
            album: None,
            duration_ms: None,
        };
        assert_eq!(
            build_search_url(&q).unwrap(),
            "https://genius.com/api/search/multi?per_page=5&q=Kendrick%20Lamar%20HUMBLE."
        );
    }

    #[test]
    fn search_url_rejects_empty_title_and_huge_queries() {
        let q = LyricsQuery { title: "   ".to_string(), ..Default::default() };
        assert!(build_search_url(&q).is_err());
        let q = LyricsQuery { title: "x".repeat(3000), ..Default::default() };
        assert!(build_search_url(&q).is_err());
    }

    #[test]
    fn search_parse_picks_first_song_hit() {
        let body = br#"{"response":{"sections":[
            {"type":"artist","hits":[]},
            {"type":"song","hits":[
                {"result":{"url":"https://genius.com/albums/foo","title":"X"}},
                {"result":{"url":"https://genius.com/Kendrick-lamar-humble-lyrics","title":"HUMBLE."}}
            ]}
        ]}}"#;
        assert_eq!(
            parse_search_response(body).as_deref(),
            Some("https://genius.com/Kendrick-lamar-humble-lyrics")
        );
    }

    #[test]
    fn search_parse_rejects_off_domain_and_non_song_urls() {
        let body = br#"{"response":{"sections":[
            {"type":"song","hits":[
                {"result":{"url":"http://genius.com/a-b-lyrics"}},
                {"result":{"url":"https://evil.test/a-b-lyrics"}},
                {"result":{"url":"https://genius.com/artists/Kendrick-lamar"}}
            ]}
        ]}}"#;
        assert_eq!(parse_search_response(body), None);
        assert_eq!(parse_search_response(b"not json"), None);
        assert_eq!(parse_search_response(br#"{"response":{}}"#), None);
    }

    #[test]
    fn extract_concatenates_containers_and_decodes() {        let html = br#"<html><body>
<div class="x">intro ad</div>
<div data-lyrics-container="true">[Verse 1]<br/>Sit down &amp; listen<br/></div>
<div data-lyrics-container="true">Be <i>humble</i> &#40;hol&#8217; up&#41;<br/></div>
<div class="footer">copyright</div>
</body></html>"#;
        let lyrics = extract_lyrics(html).unwrap();
        assert!(lyrics.contains("[Verse 1]"), "{lyrics}");
        assert!(lyrics.contains("Sit down & listen"), "{lyrics}");
        assert!(lyrics.contains("Be humble (hol’ up)"), "{lyrics}");
        assert!(!lyrics.contains("intro ad"), "{lyrics}");
        assert!(!lyrics.contains("copyright"), "{lyrics}");
    }

    #[test]
    fn extract_decodes_curly_quotes_and_dashes() {
        let html = br#"<div data-lyrics-container="true">It&rsquo;s &ldquo;real&rdquo; &mdash; yeah &hellip; definitely longer now<br/></div>"#;
        let lyrics = extract_lyrics(html).unwrap();
        assert!(lyrics.contains("It’s “real” — yeah … definitely longer now"), "{lyrics}");
    }

    #[test]
    fn extract_handles_nested_divs() {
        let html = br#"<div data-lyrics-container="true">outer rim<div class="a">inner voices<br/>second line here</div>tail end of the long song<br/></div>"#;
        let lyrics = extract_lyrics(html).unwrap();
        assert!(lyrics.contains("outer rim"), "{lyrics}");
        assert!(lyrics.contains("inner voices"), "{lyrics}");
        assert!(lyrics.contains("tail end"), "{lyrics}");
    }

    #[test]
    fn extract_misses_on_changed_markup_and_thin_content() {
        assert_eq!(extract_lyrics(b"<div>no containers here at all, just prose</div>"), None);
        assert_eq!(
            extract_lyrics(br#"<div data-lyrics-container="true">hi</div>"#),
            None
        );
        assert_eq!(extract_lyrics(b"\xff\xfe not utf8"), None);
    }

    #[test]
    fn attribution_names_source_and_song() {
        let a = attribution_for("https://genius.com/Kendrick-lamar-humble-lyrics");
        assert!(a.contains("genius.com"));
        assert!(a.contains("humble-lyrics"));
    }
}
