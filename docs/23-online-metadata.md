# Sonora: Online Metadata Subsystem & Provider Architecture

## 1. Problem Statement

Sonora operates under a strict **local-first principle**. Local audio file tags (ID3v2, Vorbis Comments, MP4 Atoms, FLAC metadata blocks) are often incomplete, misspelled, inconsistent across albums, or missing critical relational data such as MusicBrainz Identifiers (MBIDs), release group affiliations, original release dates, canonical genre taxonomies, and high-resolution artwork links.

Without an online metadata enrichment subsystem:
- Library views suffer from duplicated artists (e.g., "Pink Floyd" vs "Pink Floyd feat. David Gilmour").
- Multi-disc sets or special editions collapse or fragment arbitrarily.
- Search fails when users query official album names that differ slightly from raw filenames.
- Artwork and synchronized lyrics remain missing for tracks lacking embedded sidecars.

However, introducing online services risks compromising Sonora’s local-first architecture:
- Monolithic network calls can block UI rendering or audio playback loops.
- Over-reliance on online services can create cloud dependencies or break player functionality during network outages.
- Unthrottled API requests risk getting user IPs banned by community metadata endpoints (e.g., MusicBrainz).

Sonora requires an **abstracted, provider-based online metadata architecture** that enriches local library data asynchronously, caches aggressively, adheres to provider rate limits, operates without user accounts, and never interferes with the local playback path.

---

## 2. User Goals

1. **Automatic & Contextual Enrichment**: Effortlessly fetch rich metadata (canonical titles, release dates, track numbers, artist biographies, cover art, synchronized lyrics) for local files.
2. **Deterministic Control**: Inspect and preview metadata suggestions prior to committing changes to the local library database or file tags.
3. **Zero Account Friction**: Access public metadata (MusicBrainz, Cover Art Archive, LRCLIB) out of the box without registration, subscriptions, or login credentials.
4. **Resilient Offline Playback**: Experience identical local playback, library browsing, and search speed regardless of network connectivity.
5. **Privacy Assurance**: Ensure no personal library telemetry or listening habits are transmitted to external servers.

---

## 3. Product Behavior & Operational Principles

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     ONLINE METADATA OPERATIONAL MODEL                       │
│                                                                             │
│   [Local Library DB] ◄──► [Local Metadata Cache] ◄──► [Enrichment Engine]  │
│                                                            │                │
│                                                            ▼                │
│                                                 [Provider Router]           │
│                                                            │                │
│         ┌───────────────────────────┬──────────────────────┴─────┐          │
│         ▼                           ▼                            ▼          │
│   [MusicBrainz Provider]     [Cover Art Archive]           [LRCLIB]     │
│   • Entities & Search        • Canonical Release Art       • Lyrics AST │
│   • 1 req/sec Throttle       • CDN Redirect Handling       • Synced/Text│
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Non-Blocking Execution**: All network operations occur in background worker threads within `sonorad`. The audio decoding loop and frontend UI event loops remain isolated from network I/O.
2. **Metadata Provider, Not Audio Store**: MusicBrainz and associated APIs serve purely as metadata, entity relationship, artwork, and lyric resolvers. They are **never** used to query, stream, or download audio streams.
3. **Release Group vs. Release Distinction**:
   - **Release Group**: The abstract musical work (e.g., *The Dark Side of the Moon* by Pink Floyd). Used for album-level grouping, master release art, and canonical library sorting.
   - **Release**: A specific physical or digital issue/pressing (e.g., 1973 UK LP, 1984 Japanese CD, 2011 Remaster, 2023 50th Anniversary Box Set). Used for tracklist matching, exact barcode/catalog verification, and specific release cover art.
4. **Explicit User Overwrite Confirmation**: Automated background scanning can fetch and attach candidate MBIDs and metadata to SQLite cache tables, but **never overwrites existing track tags or library titles without explicit user confirmation**.

---

## 4. Unified Data Model

All online metadata responses normalize into strongly-typed Rust structures before reaching the SQLite index or frontend IPC boundary.

```rust
// Core Provider Entity Schema

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineArtist {
    pub mbid: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub artist_type: Option<ArtistType>, // Person, Group, Orchestra, Choir, Etc.
    pub country: Option<String>,
    pub disambiguation: Option<String>,
    pub biography: Option<String>,
    pub external_links: Vec<ExternalLink>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineReleaseGroup {
    pub mbid: String,
    pub title: String,
    pub primary_type: Option<String>,   // Album, Single, EP, Broadcast, Other
    pub secondary_types: Vec<String>, // Live, Soundtrack, Compilation, Remix
    pub first_release_date: Option<String>,
    pub primary_artist: OnlineArtistCredit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineRelease {
    pub mbid: String,
    pub release_group_mbid: String,
    pub title: String,
    pub status: Option<String>, // Official, Promotion, Bootleg
    pub date: Option<String>,
    pub country: Option<String>,
    pub barcode: Option<String>,
    pub media_format: Option<String>, // CD, 12" Vinyl, Digital Media
    pub track_count: u32,
    pub media: Vec<OnlineMedia>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineMedia {
    pub position: u32,
    pub format: Option<String>,
    pub track_count: u32,
    pub tracks: Vec<OnlineTrack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineTrack {
    pub recording_mbid: String,
    pub position: u32,
    pub number: String,
    pub title: String,
    pub duration_ms: Option<u64>,
    pub artist_credit: Vec<OnlineArtistCredit>,
    pub isrcs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnlineArtistCredit {
    pub artist_mbid: String,
    pub name: String,
    pub join_phrase: Option<String>, // e.g. " feat. ", " & "
}
```

---

## 5. UX Flows: Metadata Resolution & Preview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    METADATA ENRICHMENT WORKFLOW                             │
│                                                                             │
│  [User selects Track/Album] ──► Trigger "Find Metadata"                     │
│                                         │                                   │
│                                         ▼                                   │
│  [Extract Local Fingerprint] ──► (Title, Artist, Album, Duration, ISRC)     │
│                                         │                                   │
│                                         ▼                                   │
│  [Query MetadataProvider Pipeline] ──► (Cache Check -> MB Web Service)      │
│                                         │                                   │
│                                         ▼                                   │
│  [Score Candidates via Matching Engine] ──► (High / Med / Low Confidence)   │
│                                         │                                   │
│                                         ▼                                   │
│  [Display Match Inspector Modal]                                            │
│  • Side-by-Side Diff: Local Tags vs Online Candidate                        │
│  • Entity Selector: Switch Release / Pressing Edition                      │
│                                         │                                   │
│                                         ▼                                   │
│  [User Action: Apply / Save to DB / Write ID3 Tags]                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Triggering**: User right-clicks a track, album, or artist in Sonora GUI/TUI and selects **[Find Metadata]**. Alternatively, background indexing tags newly imported tracks with candidate match flags.
2. **Candidate Presentation**: The UI displays candidate matches ranked by match confidence (0.0 to 1.0).
3. **Side-by-Side Inspection**:
   - **Current Local**: Title, Artist, Album, Year, Track #, Disc #, ISRC, Cover Thumbnail.
   - **MusicBrainz Candidate**: Canonical Title, Artist Credits, Release Group, Specific Release Pressing, Label, Barcode, Cover Art Archive Preview.
4. **Selection & Application**:
   - **Apply to Database**: Updates SQLite `tracks`, `albums`, `artists` records immediately.
   - **Write File Tags** (Optional toggle): Writes updated ID3v2.4 / Vorbis Comment tags directly to the local audio file with safety backup.

---

## 6. Architecture Implications & Provider Abstraction

To ensure Sonora is not tied to a single third-party vendor, all online interactions are mediated through asynchronous provider traits:

```rust
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    
    async fn search_artists(&self, query: &str, limit: usize) -> Result<Vec<OnlineArtist>, ProviderError>;
    async fn search_release_groups(&self, query: &str, limit: usize) -> Result<Vec<OnlineReleaseGroup>, ProviderError>;
    async fn search_releases(&self, query: &str, limit: usize) -> Result<Vec<OnlineRelease>, ProviderError>;
    async fn search_recordings(&self, query: &str, limit: usize) -> Result<Vec<OnlineTrack>, ProviderError>;
    
    async fn get_artist_by_mbid(&self, mbid: &str) -> Result<OnlineArtist, ProviderError>;
    async fn get_release_group_by_mbid(&self, mbid: &str) -> Result<OnlineReleaseGroup, ProviderError>;
    async fn get_release_by_mbid(&self, mbid: &str) -> Result<OnlineRelease, ProviderError>;
    async fn get_recording_by_mbid(&self, mbid: &str) -> Result<OnlineTrack, ProviderError>;
}
```

### Rate Limiting & Queue Management
- **Token Bucket Throttler**: Every provider instance embeds a thread-safe token bucket rate limiter.
- **MusicBrainz Throttling**: Hard-capped at **1.0 request per second** (1000ms leak interval per token).
- **User-Agent Header Policy**: Every HTTP request MUST set a compliant User-Agent header:
  ```http
  User-Agent: Sonora/0.2.0 ( https://github.com/Sumeet-basfore/sonora ; contact@sonora.audio )
  ```
- Anonymous or generic User-Agent strings (e.g. `curl/7.68.0`, `reqwest/0.11`) are strictly prohibited to prevent IP ban sanctions from MusicBrainz edge servers.

### SQLite Caching Layer
All metadata API responses are cached locally in SQLite to prevent redundant network calls:
```sql
CREATE TABLE online_metadata_cache (
    provider_id TEXT NOT NULL,
    entity_type TEXT NOT NULL, -- 'artist', 'release_group', 'release', 'recording'
    entity_key TEXT NOT NULL,  -- Query string or MBID
    response_json TEXT NOT NULL,
    fetched_at INTEGER NOT NULL DEFAULT (unixepoch()),
    ttl_seconds INTEGER NOT NULL DEFAULT 604800, -- 7 days default TTL
    PRIMARY KEY (provider_id, entity_type, entity_key)
);
CREATE INDEX idx_metadata_cache_lookup ON online_metadata_cache(provider_id, entity_type, entity_key);
```

---

## 7. Provider & API Technical Evaluation

### 7.1 Primary Provider: MusicBrainz API (v2)
- **Base Endpoint**: `https://musicbrainz.org/ws/2/`
- **Format**: JSON (`fmt=json`).
- **Core Entity Endpoints**:
  - `GET /ws/2/artist?query=artist:"..."&fmt=json`
  - `GET /ws/2/release-group?query=releasegroup:"..." AND artist:"..."&fmt=json`
  - `GET /ws/2/release?query=release:"..." AND artist:"..."&fmt=json`
  - `GET /ws/2/recording?query=recording:"..." AND artist:"..."&fmt=json`
  - `GET /ws/2/recording?query=isrc:...&fmt=json`
- **Entity Lookup with Inc Parameters**:
  - `GET /ws/2/release/{mbid}?inc=artists+release-groups+recordings+media+labels&fmt=json`
- **Authentication**: None required for public read-only requests.
- **Pagination**: Parameters `limit` (max 100, default 25) and `offset`.

### 7.2 Secondary Provider: Cover Art Archive (CAA)
- **Base Endpoint**: `https://coverartarchive.org/`
- **Release Cover Endpoint**: `GET https://coverartarchive.org/release/{release_mbid}`
- **Release Group Cover Endpoint**: `GET https://coverartarchive.org/release-group/{release_group_mbid}`
- **Direct Thumbnail Shortcuts**:
  - `https://coverartarchive.org/release/{release_mbid}/front-250` (250x250 WebP/JPEG)
  - `https://coverartarchive.org/release/{release_mbid}/front-500` (500x500 WebP/JPEG)
  - `https://coverartarchive.org/release/{release_mbid}/front-1200` (1200x1200 Full resolution)
- **Redirect Handling**: Responses issue HTTP 307 redirects to Internet Archive (`archive.org`) CDN nodes. HTTP client must automatically follow up to 5 redirects.
- **Rate Limit**: No strict 1 req/sec limit, but client must respect `Retry-After` on 429/503.

### 7.3 Secondary Provider: LRCLIB (Lyrics)
- **Base Endpoint**: `https://lrclib.net/api`
- **Direct Lookup**: `GET /api/get?track_name=...&artist_name=...&album_name=...&duration=...`
- **Search Endpoint**: `GET /api/search?q=...`
- **Authentication**: None required.
- **Data Returned**: Plain text and line-synced LRC text.

### 7.4 Secondary Provider Evaluation: Fanart.tv (Artist Artwork)
- **Base Endpoint**: `http://webservice.fanart.tv/v3/`
- **Artist Artwork Endpoint**: `GET /v3/music/{artist_mbid}?api_key={API_KEY}`
- **Assets Provided**: Artist HD Backgrounds (`artistbackground`), Artist Thumbnails (`artistthumb`), HD Music Logos (`hdmusiclogo`), Banners (`musicbanner`).
- **API Key & Rate Limit Implications**:
  - Requires a **Project API Key** registered by the developer, plus an optional **Personal Client Key** provided by power users.
  - **Rate Limit**: 100,000 requests per week per project key. Exceeding this returns HTTP 429.
  - **Licensing & Attribution**: CC BY-NC 4.0. Requires user attribution in client UI if images are displayed.
  - **Decision**: Supported as an optional secondary artist artwork provider. User-configured personal API keys are supported in Settings to bypass shared project key quotas.

### 7.5 Secondary Provider Evaluation: Wikimedia Commons & Wikidata
- **SPARQL Endpoint**: `https://query.wikidata.org/sparql`
- **Mechanism**: Query Wikidata entity using MusicBrainz Artist ID (`P434`) to extract Wikidata Image property (`P18`).
- **Licensing & Cost**: 100% Free, Creative Commons / Public Domain, no API keys required. Excellent fallback for artist portraits.

---

## 8. Failure Cases & Resilience Controls

| Failure Scenario | Root Cause | System Response | User Impact |
| :--- | :--- | :--- | :--- |
| **HTTP 429 Rate Limited** | Query burst exceeded 1 req/sec | Throttler enters exponential backoff (2s, 4s, 8s). Request queued in background. | UI shows "Rate limited, retrying in background..." badge; no crash. |
| **HTTP 503 / Provider Outage** | MusicBrainz or CAA maintenance | Fallback to SQLite cache -> Local file tags -> Secondary providers (Wikidata/LRCLIB). | Online search degrades gracefully; local playback unaffected. |
| **Network Disconnected** | Offline device state | Network calls short-circuit immediately with `ProviderError::Offline`. | Player uses cached online metadata and local file tags seamlessly. |
| **No Match Found (HTTP 404)** | Obscure, unreleased, or private track | Return empty candidate set. Log search query in local mismatch registry. | User can manually search or input MBID directly in Match Inspector. |
| **Invalid / Malformed JSON** | Provider API schema change | Serde deserialization fails gracefully into `ProviderError::ParseError`. | Raw error logged; process does not panic. |

---

## 9. Security & Privacy Considerations

1. **No User Tracking or Analytics**: Sonora sends zero telemetry, user identifiers, device IDs, or listening logs to MusicBrainz, CAA, LRCLIB, or Fanart.tv.
2. **Minimal Request Payload**: Outgoing HTTP queries contain only track search strings (title, artist, album, duration) and standard User-Agent headers.
3. **Transport Layer Security**: All API endpoints enforce strict HTTPS (TLS 1.3). Plain HTTP fallback is rejected.
4. **Data Sanitization**: All incoming text metadata strings undergo UTF-8 verification, null-byte stripping, and HTML tag sanitization before insertion into SQLite or UI display.

---

## 10. Open Decisions

1. **Acoustic Fingerprinting (Chromaprint / AcoustID)**:
   - *Status*: Deferred for v0.2 initial release.
   - *Context*: AcoustID allows audio waveform fingerprinting without relying on text tag accuracy.
   - *Decision*: Documented as an optional future enhancement for Phase 2 once text-based MusicBrainz matching is stable.
2. **Local Write-Back Default**:
   - *Status*: Open for user setting default.
   - *Options*: (A) Update SQLite library database only. (B) Update SQLite database AND write ID3v2.4 / Vorbis tags back to physical audio files.
   - *Recommendation*: Default to (A) with an explicit opt-in toggle for (B) to prevent accidental modification of user audio files.

---

## 11. References

- MusicBrainz Web Service v2 Documentation: `https://musicbrainz.org/doc/MusicBrainz_API`
- MusicBrainz Search Syntax (Lucene): `https://musicbrainz.org/doc/Development/XML_Web_Service/Version_2/Search`
- Cover Art Archive API Specification: `https://coverartarchive.org/`
- LRCLIB API Documentation: `https://lrclib.net/docs`
- Fanart.tv API v3 Docs: `https://fanarttv.docs.apiary.io/`
- Wikidata SPARQL Query Service: `https://query.wikidata.org/`
