# Sonora: Unified Artwork System & Media Asset Pipeline

## 1. Problem Statement

Album and artist visual assets in audio applications are often managed haphazardly.

Common issues in media players include:
- **Source Confusion**: Players blur the line between audio-file embedded artwork (ID3 APIC / FLAC PICTURE blocks), local folder images (`cover.jpg`), and remote cached images.
- **Silent Overwrites**: Updating an artwork file in one view silently replaces embedded ID3 image tags or overwrites local folder images without user knowledge.
- **Resolution & Aspect Ratio Distortion**: Low-resolution thumbnails (100x100) are upscaled onto 4K display stages, creating blurred canvas backdrops.
- **Missing Artist Visuals**: While album cover art is frequently embedded in tags, artist portraits, banners, and logos are rarely stored locally, leaving artist views visually empty.
- **Security Vulnerabilities**: Naive image decoders can crash the player or suffer buffer overflow attacks when encountering malformed image headers or decompression bombs.

Sonora requires a **unified, multi-tiered artwork model** that cleanly decouples artwork types, enforces strict resolution and safety boundaries, uses Cover Art Archive as the primary release-art authority, supports candidate selection UX, and never silently replaces local media files.

---

## 2. User Goals

1. **Multi-Source Clarity**: Clearly identify where any displayed artwork originated (Embedded Tag, Local Folder, Cover Art Archive, Fanart.tv, Wikidata).
2. **High-Fidelity Visuals**: Enjoy crisp album cover textures (up to 1200x1200px) and legibly themed ambient backdrops optimized for Retina/4K displays.
3. **Interactive Artwork Finder**: Browse candidate images from online providers with clear resolution, aspect ratio, format, and match confidence metadata before applying.
4. **Artist Visual Representation**: View high-quality artist portraits, fanart backgrounds, and logos alongside album discographies.
5. **Absolute Asset Integrity**: Guaranteed protection against accidental tag overwrites or file deletions.

---

## 3. Unified Artwork Hierarchy & Operational Rules

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       SONORA ARTWORK SOURCE RESOLUTION                      │
│                                                                             │
│  [Requested Entity: Album / Release / Artist]                              │
│                         │                                                   │
│        ┌────────────────┼────────────────┬────────────────┐                 │
│        ▼ Tier 1         ▼ Tier 2         ▼ Tier 3         ▼ Tier 4          │
│   [User Assigned]  [Local Sidecar]  [Embedded Tags]  [Online Provider]      │
│   (Custom SQLite)  (cover.jpg/png)  (ID3 APIC/FLAC)  (CAA / Wikidata /      │
│                         │                │            Fanart.tv)            │
│                         └────────────────┴────────────────┘                 │
│                                          │                                  │
│                                          ▼                                  │
│                       [Zero-Trust Image Sanitizer & Cache]                  │
│                                          │                                  │
│                                          ▼                                  │
│                       [WebP Thumbnail & Texture Pipeline]                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Artwork Asset Types & Classification

| Artwork Type | Scope | Target Entity | Canonical Primary Source |
| :--- | :--- | :--- | :--- |
| **Embedded Artwork** | Per-Track File | Audio File (.flac/.mp3) | ID3 APIC frame / FLAC METADATA_BLOCK_PICTURE |
| **Local Folder Artwork** | Per-Folder | Directory | `cover.jpg`, `folder.png`, `front.jpg` in album folder |
| **Downloaded / Cached Art** | Local Cache | SQLite Cache Table | Cover Art Archive / Fanart.tv / Wikidata |
| **Release Artwork** | Specific Pressing | MusicBrainz Release | Cover Art Archive (`/release/{mbid}`) |
| **Release-Group Artwork** | Master Album | MusicBrainz Release Group | Cover Art Archive (`/release-group/{mbid}`) |
| **Artist Portrait / Banner** | Artist Entity | MusicBrainz Artist | Wikidata (P18) / Fanart.tv (`/v3/music/{mbid}`) |

### 3.2 Canonical Rules
1. **Cover Art Archive as Primary Album Authority**: For online album cover retrieval, Cover Art Archive (CAA) is the canonical source.
2. **Never Silently Replace Artwork**: Online searches produce candidate image previews. Applying an online image writes to Sonora's local cache directory and SQLite index. It **never** mutates physical audio tags or deletes local folder images unless the user explicitly checks `[x] Also write cover art into audio file tags`.
3. **Multi-Resolution Texture Caching**: Images are stored in two canonical cache dimensions:
   - `thumbnails/` (300x300 WebP): Used for dense grid views, tracklists, and TUI ANSI previews.
   - `full/` (1200x1200 WebP): Used for high-DPI Now Playing stage, Atmospheric Theater, and Oklab palette extraction.

---

## 4. Unified Data Model & Candidates Schema

```rust
// Core Artwork Data Structures in Sonora Core

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtworkSourceType {
    EmbeddedTag { file_path: String },
    LocalSidecar { file_path: String },
    UserCustom { file_path: String },
    CoverArtArchive { release_mbid: Option<String>, release_group_mbid: Option<String> },
    FanartTv { artist_mbid: String },
    Wikidata { image_url: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtworkKind {
    FrontCover,
    BackCover,
    Booklet,
    Medium,       // Vinyl disc, CD label scan
    ArtistPortrait,
    ArtistBackground,
    ArtistBanner,
    ArtistLogo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtworkCandidate {
    pub id: String,
    pub provider_name: String,
    pub source_type: ArtworkSourceType,
    pub kind: ArtworkKind,
    pub original_url: String,
    pub preview_thumbnail_url: String,
    pub width: u32,
    pub height: u32,
    pub format: String, // "JPEG", "PNG", "WebP"
    pub size_bytes: Option<u64>,
    pub match_confidence: f32, // 0.0 to 1.0
    pub is_canonical: bool,
}
```

---

## 5. UX Flow: Interactive Artwork Candidate Inspector

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ARTWORK CANDIDATE SELECTION FLOW                         │
│                                                                             │
│  [User selects Album/Artist] ──► Trigger [Find Artwork]                     │
│                                        │                                    │
│                                        ▼                                    │
│  [Fetch Candidates from Providers] ──► (CAA + Wikidata + Fanart.tv + Local) │
│                                        │                                    │
│                                        ▼                                    │
│  [Display Candidate Gallery Modal]                                          │
│  • Visual Grid: Resolution Badges (1200x1200, 500x500, etc.)                │
│  • Provider Tag: [Cover Art Archive] / [Fanart.tv] / [Embedded Tag]         │
│  • Type Tag: [Front Cover] / [Back Cover] / [Artist Fanart]                 │
│                                        │                                    │
│                                        ▼                                    │
│  [User Clicks Candidate Preview] ──► (Full-size Zoom & Color Palette Check) │
│                                        │                                    │
│                                        ▼                                    │
│  [User Clicks "Set as Album Artwork"] ──► Cache to WebP & Update SQLite DB  │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Triggering**: User clicks **`[Find Artwork]`** on any album card or artist detail page.
2. **Candidate Aggregation**:
   - Local: Extracts embedded audio tags and scans folder for `cover.jpg`/`folder.png`.
   - Cover Art Archive: Queries `/release/{mbid}` and `/release-group/{mbid}`.
   - Fanart.tv / Wikidata: Queries artist portrait and background banners.
3. **Grid Inspector Presentation**:
   - Displays candidate cards with metadata badges: Provider Name, Dimensions (e.g. `1200 x 1200`), Aspect Ratio (1:1), File Format (`WebP`/`JPEG`), and Match Score.
4. **Preview & Selection**:
   - Clicking a card previews the image rendered in full-screen resolution alongside Oklab color palette extractions.
   - User clicks **`[ Set as Active Artwork ]`**. Sonora downloads, sanitizes, converts to WebP, writes to `$XDG_CACHE_HOME/sonora/covers/`, and updates SQLite `cover_art_uri`.

---

## 6. Architecture & Security Implications

### 6.1 Zero-Trust Media Asset Sanitization (ADR-010 Integration)
All incoming remote or local image files pass through pure-Rust image decoding validation prior to caching or presentation:
- **Maximum File Allocation Guard**: Rejects image payloads exceeding **64 MB**.
- **Dimension Clamp**: Rejects images exceeding **8192 x 8192** pixels to prevent decompression bombs.
- **Memory-Safe Decoder**: Uses `image-rs` in pure Rust without linking untrusted C libraries (e.g. vulnerable libpng/libjpeg C decoders).
- **Format Normalization**: All external JPEGs, PNGs, and GIFs are re-encoded into optimized WebP assets upon local caching.

### 6.2 Cache Directory Structure
```
$XDG_CACHE_HOME/sonora/covers/
├── full/
│   ├── release_group_3f25b1a0.webp  (1200x1200 Full resolution)
│   └── artist_a74b12c8.webp         (1920x1080 Fanart background)
└── thumbnails/
    ├── release_group_3f25b1a0.webp  (300x300 Thumbnail)
    └── artist_a74b12c8.webp         (300x300 Thumbnail)
```

---

## 7. Provider & API Technical Evaluation for Artist Artwork

### 7.1 Cover Art Archive (Canonical Album Art)
- **APIs**: `GET https://coverartarchive.org/release/{mbid}` and `GET https://coverartarchive.org/release-group/{mbid}`
- **Evaluation**: Outstanding reliability, open community access, no API keys, hosted by Internet Archive.

### 7.2 Fanart.tv (Artist Portraits, Logos, & Backgrounds)
- **API**: `GET http://webservice.fanart.tv/v3/music/{artist_mbid}?api_key={API_KEY}`
- **Returned Assets**:
  - `artistbackground`: High-definition fanart backgrounds (1920x1080).
  - `artistthumb`: Square artist portraits (1000x1000).
  - `hdmusiclogo`: Transparent PNG artist logos (800x310).
  - `musicbanner`: Horizontal artist banners (1000x185).
- **Licensing & Terms**: CC BY-NC 4.0. Requires user attribution if displayed.
- **API Key Policy**: Project API Key required. Users can optionally supply a personal API Key in Sonora Settings.

### 7.3 Wikimedia Commons / Wikidata (Free Artist Portraits)
- **SPARQL API**: `https://query.wikidata.org/sparql`
- **Query Mechanism**: Match MusicBrainz Artist ID (`P434`) to extract Wikidata Image (`P18`).
- **Licensing & Terms**: Public Domain / Creative Commons (CC-BY / CC-BY-SA). 100% free, no API keys, unlimited requests.

### 7.4 TheAudioDB (Alternative Provider Evaluation)
- **API**: `https://www.theaudiodb.com/api/v1/json/{API_KEY}/artist.php?i={musicbrainz_artist_id}`
- **Evaluation**: Free API key `2` is strictly for testing (low-resolution 250px images, rate limited). Production high-res access requires a $5/month Patreon subscription API key.
- **Decision**: Deferred as non-default due to commercial API key paywall.

---

## 8. Failure Cases & Resilience Controls

| Scenario | Cause | System Response | User Impact |
| :--- | :--- | :--- | :--- |
| **No Online Art Found (404)** | Obscure release | Fallback: Local folder `cover.jpg` -> Embedded ID3 tag -> Generative CSS Gradient. | Player renders crisp fallback canvas with album initials. |
| **Decompression Bomb Attack** | Malicious 100,000x100,000 image | Dimension guard rejects image before buffer allocation. | Warning logged; fallback artwork displayed safely. |
| **Corrupted WebP Cache** | Disk write failure | Cache reader detects corrupt magic bytes; re-fetches or re-extracts artwork. | Player auto-recovers thumbnail without crashing. |
| **Image CDN Redirect 307** | Cover Art Archive -> Internet Archive | HTTP client follows redirect automatically up to 5 hops. | Seamless artwork loading. |

---

## 9. Security & Privacy Considerations

1. **Pure Rust Decoding**: Zero risk of C buffer overflows (e.g. CVE-2023-4863 libwebp vulnerabilities) by isolating decoding inside `image-rs`.
2. **Referrer & Telemetry Stripping**: Image requests to external CDNs strip `Referer` headers to prevent leaking local IP activity to image hosts.

---

## 10. Open Decisions

1. **Default Local Sidecar File Writing**:
   - *Status*: Open setting.
   - *Options*: When user chooses a new cover art, should Sonora save a `cover.jpg` file into the local album folder?
   - *Recommendation*: Keep as an explicit toggle inside Artwork Inspector (`[ ] Save copy as cover.jpg in album folder`) to avoid modifying user directories unexpectedly.

---

## 11. References

- Cover Art Archive Specification: `https://coverartarchive.org/`
- Fanart.tv API Documentation: `https://fanarttv.docs.apiary.io/`
- Wikidata SPARQL Query Service: `https://query.wikidata.org/`
- Zero-Trust Media Asset Sanitization: `docs/12-decision-log.md#adr-010-zero-trust-media-asset-sanitization--dimension-guards`
