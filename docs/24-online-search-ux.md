# Sonora: Online Search Experience & Interaction Specification

## 1. Problem Statement

Standard music players either isolate search to local file metadata or attempt to convert global search into an online streaming store (e.g. Spotify, Apple Music).

In legacy desktop media players (foobar2000, MusicBee):
- Online search is fragmented across hidden plugin dialogs, separate tagger windows, and third-party script logs.
- Finding lyrics, cover art, or canonical release details requires navigating 3 to 4 distinct windows or modal popups.
- Users are often forced to choose between searching *only* local files or searching *only* online databases, creating visual split states.

In modern streaming platforms:
- Search prioritizes commercial cloud catalogs, rendering local files second-class or hiding local track search behind obscure settings toggles.

Sonora requires a **unified, non-fragmented search navigation model** that honors local-first library ownership while seamlessly integrating online metadata discovery, candidate previewing, lyrics matching, and artwork enrichment into a cohesive instrument UI.

---

## 2. User Goals

1. **Unified Query Surface**: Search local tracks, online metadata, lyrics, and artwork from a single, keyboard-driven global search interface (`Cmd/Ctrl + K` or `/`).
2. **Contextual Enrichment Triggering**: Initiate targeted metadata, lyric, or artwork lookups directly from any track, album, or artist context menu without navigating away from the current view.
3. **Rich Preview & Side-by-Side Verification**: Audition and inspect online search results (comparing tracklists, release dates, pressings, artwork quality, and lyric timestamps) before committing changes to the local library.
4. **Explicit Control & Transparency**: Maintain total control over library state—online search must **never** automatically rewrite local file metadata or stream third-party audio.
5. **Instant Offline Fallback**: Search local tracks instantly (< 5ms) even when offline, with clear visual badges indicating network status.

---

## 3. Product Behavior & Navigation Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    UNIFIED GLOBAL SEARCH NAVIGATION MODEL                   │
│                                                                             │
│   [ Cmd/Ctrl + K ] or Global Search Bar                                      │
│         │                                                                   │
│         ▼                                                                   │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │ Search Bar: "Pink Floyd Dark Side"                                  │   │
│   │ Mode Tabs: [ Local Library (Default) | Online Metadata | Lyrics ]   │   │
│   └──────────────────────────────────┬──────────────────────────────────┘   │
│                                      │                                       │
│                ┌─────────────────────┴─────────────────────┐                │
│                ▼                                           ▼                │
│   [Local Library Results] (Sub-5ms FTS5)      [Online Discovery Results]       │
│   • Tracks (Local Audio Files)                • MusicBrainz Recordings         │
│   • Albums (Indexed Local Albums)             • MusicBrainz Release Groups     │
│   • Artists (Local Artist Index)              • LRCLIB Synced Lyrics           │
│   • Playlists                                 • Cover Art Archive Artwork      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Local vs. Online Scope
- **Local Search (Default)**: Executes across SQLite FTS5 index instantly (< 5ms). Returns playable local audio files, local albums, local artists, and playlists.
- **Online Search Tab / Mode**: Queries external metadata providers asynchronously (MusicBrainz, Cover Art Archive, LRCLIB). Displays canonical releases, artist biographies, release groups, synced lyrics candidates, and high-resolution cover art.

### 3.2 Desired Search & Enrichment Flow
```
User Query ──► Instant Local Results (<5ms)
                   │
                   ├──► (Optional) Switch to "Online Metadata" or "Find Online"
                   │          │
                   │          ▼
                   │    Fetch Candidates (Provider Router)
                   │          │
                   │          ▼
                   │    Display Search Preview Grid
                   │          │
                   │          ▼
                   │    User Selects Candidate Match
                   │          │
                   │          ▼
                   └────► Display Side-by-Side Match Inspector Modal
                              │
                              ▼
                        [ Confirm & Enrich Local Record ]
```

### 3.3 Contextual Actions (No UI Fragmentation)
To prevent cluttering the main navigation sidebar, online discovery features are accessible as contextual tools attached directly to library items:
- **`[Find Metadata]`**: Opens Match Inspector comparing local track/album tags against MusicBrainz release candidates.
- **`[Find Lyrics]`**: Opens Dedicated Lyrics Manager to search, preview, and assign LRCLIB / local `.lrc` lyrics.
- **`[Find Artwork]`**: Opens Artwork Candidate Inspector to choose, preview, and set album or artist cover art.

---

## 4. Search Data Model & State Schema

```typescript
// Frontend Unified Search State Contract (TypeScript)

export type SearchScope = 'local' | 'online_metadata' | 'lyrics';

export interface SearchQuery {
  raw_query: string;
  scope: SearchScope;
  category?: 'tracks' | 'albums' | 'artists' | 'lyrics';
  limit: number;
  offset: number;
}

export interface UnifiedSearchResult {
  query: SearchQuery;
  local_results: {
    tracks: LocalTrackResult[];
    albums: LocalAlbumResult[];
    artists: LocalArtistResult[];
  };
  online_results?: {
    recordings: OnlineTrackCandidate[];
    release_groups: OnlineAlbumCandidate[];
    artists: OnlineArtistCandidate[];
    lyrics: LyricsCandidate[];
  };
  status: 'idle' | 'loading_local' | 'loading_online' | 'success' | 'error' | 'offline';
  error_message?: string;
  rate_limit_reset_ms?: number;
}

export interface MatchComparison {
  local_entity_id: number;
  candidate_mbid: string;
  confidence_score: number; // 0.0 to 1.0
  field_diffs: FieldDiff[];
}

export interface FieldDiff {
  field_name: string;
  local_value: string | number | null;
  online_value: string | number | null;
  status: 'exact_match' | 'fuzzy_match' | 'conflict' | 'missing_locally';
}
```

---

## 5. Detailed UX Flows & Edge Case Behavior

### 5.1 Loading & Debounce States
- **Debounce Interval**: Global search input debounces online queries by **350ms** to prevent firing excessive requests while the user is typing.
- **Loading Skeleton**: Local search results display immediately (< 5ms). The online search section renders smooth loading skeleton rows with subtle pulse animation until network responses settle.

### 5.2 No Results State
- When an online query returns 0 matches, Sonora displays a clean empty state card:
  - *"No online matches found for '{query}'."*
  - **Action**: `[ Refine Search Query ]` or `[ Enter MusicBrainz MBID Manually ]`.

### 5.3 Network Unavailable / Offline State
- If the OS network adapter is disconnected or DNS fails:
  - Online search tabs display an offline status banner: `[ Network Unavailable - Showing Local Results Only ]`.
  - Local search operates with zero delay.
  - Contextual buttons (`[Find Metadata]`, `[Find Lyrics]`) render in a disabled state with a tooltip explaining: *"Requires active internet connection"*.

### 5.4 Rate Limited (HTTP 429) Handling
- When MusicBrainz or Fanart.tv returns HTTP 429:
  - Online search UI presents a warning badge: `[ Provider Rate Limit Exceeded - Retrying in X seconds ]`.
  - A countdown timer runs based on `Retry-After` response headers. User can continue playing local music without interruption.

### 5.5 Ambiguous Artist Names Handling
- Search queries like "John Williams" yield multiple distinct MusicBrainz artist entities (e.g., Classical Film Composer vs. Australian Guitarist).
- **UI Behavior**: Online Search displays disambiguation sub-text under artist titles:
  - *John Williams (Film score composer, b. 1932, United States)*
  - *John Williams (Classical guitarist, b. 1941, Australia)*
- Selecting an artist opens their full MusicBrainz release group discography preview.

### 5.6 Different Editions of the Same Album Handling
- An album query like "Abbey Road" returns multiple pressings (1969 UK Original, 1987 CD Release, 2009 Remaster, 2019 Super Deluxe Edition).
- **UI Behavior**:
  1. The top level of search results groups pressings under the **Release Group** (*Abbey Road*).
  2. Clicking the Release Group reveals a Pressing/Release Selector dropdown showing:
     - Year, Country, Label, Format (CD/Vinyl/Digital), Track Count, Barcode.
  3. Selecting a specific pressing updates the track list preview and Cover Art Archive artwork link.

### 5.7 Explicit User Confirmation Before Overwriting
- Under no circumstances does Sonora automatically overwrite local track titles, artist names, or album years during search browsing.
- Applying online metadata requires clicking **`[ Apply Metadata to Library ]`** inside the Match Inspector modal.
- A confirmation dialog lists the exact files to be updated:
  ```
  Apply MusicBrainz metadata to 14 tracks?
  • SQLite Library Records will be updated.
  • [x] Also update physical audio file tags (.flac / .mp3)
  ```

---

## 6. Architecture & Product Constraints

1. **Local-First Sovereign Storage**: Sonora NEVER converts search into an online streaming service or cloud locker. Selecting an online search result enriches local track metadata; it does NOT stream audio from YouTube, Spotify, or Soundcloud.
2. **Zero Audio Download Capability**: Online search does not contain download buttons, torrent links, or stream rippers. It is exclusively a metadata discovery tool.
3. **UI Integration without Fragmentation**:
   - Do NOT add a bloated "Discover" top-level tab that duplicates the library.
   - Global Search (`Cmd/Ctrl + K`) serves as the universal query entry point.
   - Item-specific enrichment tools are accessible via context menus (`[Find Metadata]`, `[Find Lyrics]`, `[Find Artwork]`).

---

## 7. Provider API Technical Evaluation

### 7.1 MusicBrainz Search API
- **Endpoint**: `GET https://musicbrainz.org/ws/2/{entity}/?query={lucene_query}&fmt=json&limit=25`
- **Lucene Search Capabilities**:
  - Exact Recording Search: `recording:"Time" AND artist:"Pink Floyd"`
  - Release Group Search: `releasegroup:"The Wall" AND artist:"Pink Floyd"`
  - ISRC Direct Lookup: `isrc:USSM21100001`
- **Performance**: Average response latency 180ms – 450ms. Debouncing at 350ms ensures smooth user typing.

### 7.2 LRCLIB Search API
- **Endpoint**: `GET https://lrclib.net/api/search?q={artist_name}+{track_name}`
- **Performance**: Average response latency 100ms – 250ms. Returns synced/plain lyric availability flags.

---

## 8. Failure Cases & Degradation Matrix

| Event | Cause | UI Presentation | Recovery Strategy |
| :--- | :--- | :--- | :--- |
| **Search Timeout (> 3000ms)** | Slow network / server load | Show "Request timed out" notification badge. | Provide `[ Retry Search ]` button. |
| **Malformed Search Query** | Unescaped special characters | Sanitizer strips invalid Lucene operators. | Executes sanitized fallback query automatically. |
| **Provider HTTP 500/502** | MusicBrainz server error | Displays "Provider temporarily unavailable" alert. | Logs error; local search remains fully functional. |
| **Empty Candidate Set** | Rare/Private track | Shows "No candidates match this track" state. | Enables manual MBID input text field. |

---

## 9. Security & Privacy Considerations

1. **Zero Query Logging**: Search queries entered by the user are never transmitted to Sonora telemetry endpoints or analytics servers.
2. **Input Sanitization**: Search input strings are sanitized before being passed to HTTP query parameters to prevent injection attacks or malformed URL construction.
3. **Sandboxed Remote Asset Previews**: Image previews fetched from remote URLs (CAA, Fanart.tv) undergo pure-Rust decoding and dimension clamping before presentation in the UI.

---

## 10. Open Decisions

1. **Global Keyboard Shortcut Scope**:
   - *Status*: Open for UI preference setting.
   - *Decision*: `Cmd/Ctrl + K` triggers global search overlay in GUI; `/` triggers inline search prompt in TUI.

---

## 11. References

- MusicBrainz Lucene Query Docs: `https://musicbrainz.org/doc/Development/XML_Web_Service/Version_2/Search`
- Sonora Design System Spec: `docs/20-design-system-spec.md`
- Sonora UX Specification: `docs/15-sonora-ux-spec.md`
