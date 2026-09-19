# Sonora: Lyrics Experience & Dedicated Manager Specification

## 1. Problem Statement

Sonora's v0.1 lyrics subsystem provides sub-millisecond line and syllable synchronization, spring-physics scrolling, and LRCLIB integration. However, the user experience surrounding **lyrics discovery, selection, verification, and timing adjustment** remains weak and buried:
- **No Dedicated Manager**: Finding or switching lyric candidates requires navigating sub-menus or settings popups.
- **Single-Match Lock-in**: If the automatic lyric fetcher matches an incorrect version (e.g., live version instead of studio album, or a language translation the user did not want), switching to an alternative candidate is cumbersome.
- **Opaque Candidate Selection**: Users cannot see candidate metadata (duration difference, provider source, line vs syllable sync, confidence score) before applying lyrics.
- **Manual Timing Isolation**: Adjusting timing offsets ($\pm 100\text{ ms}$) occurs via keyboard shortcuts without visual feedback comparing lyric timestamps against audio waveforms.

Sonora requires a **dedicated, first-class Lyrics Manager workspace** that elevates lyric discovery, multi-candidate selection, side-by-side verification, timing alignment, and local `.lrc` sidecar exporting into a central product feature.

---

## 2. User Goals

1. **Dedicated Workspace**: Access a dedicated Lyrics Manager overlay or view (`Cmd/Ctrl + L` or context menu) to inspect, manage, and synchronize track lyrics.
2. **Multi-Candidate Discovery**: Search online repositories (LRCLIB, local sidecars, embedded tags) and view a ranked list of candidate lyrics with confidence and duration metrics.
3. **Instant Candidate Preview**: Audition candidate lyrics in real time against active audio playback before assigning them to the track.
4. **Precision Timing Alignment**: Visually fine-tune global or line-by-line timing offsets with instant waveform/clock alignment and per-track SQLite persistence.
5. **Local Sidecar Export**: Export verified synchronized lyrics to local `.lrc` sidecars in the track directory for universal interoperability.

---

## 3. Dedicated Lyrics Manager Architecture & Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DEDICATED LYRICS MANAGER WORKSPACE                       │
│                                                                             │
│  [Now Playing Stage / Track Context] ──► Open Lyrics Manager (Cmd/Ctrl+L)   │
│                                              │                              │
│                                              ▼                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Active Lyrics Display & Timing Control                                │  │
│  │ • Current Source: [ LRCLIB (Synced) v ]  Offset: [ +150 ms ]         │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │                                      │
│                ┌─────────────────────┴─────────────────────┐                │
│                ▼                                           ▼                │
│   [ Candidate Discovery Drawer ]              [ Interactive Timing Editor ] │
│   • Ranked Candidate List                     • Real-Time Audio Scrubbing   │
│   • Source, Sync Type, Duration Delta         • Tap-to-Sync Timestamping    │
│   • Side-by-Side Lyrics Preview               • Save to SQLite / Export .lrc│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Core Operations
1. **Source Switching**: Seamlessly toggle between Embedded Tags (ID3 SYLT/USLT), Local `.lrc` Sidecars, LRCLIB Online Candidates, 3rd-Party Plugin Providers, or Custom User Edits.
2. **Multi-Candidate Search & Filtering**:
   - Query online providers with custom search terms (Title, Artist, Duration).
   - Display a list of candidates ranked by duration match ($\Delta t$), title similarity, and sync type (Word-Synced > Line-Synced > Plain Text).
3. **Side-by-Side Preview**: Audition candidate lyrics alongside live audio playback before saving.
4. **Dedicated Timing Offset Controls**: Interactive offset slider and precision buttons (`-100ms`, `-10ms`, `+10ms`, `+100ms`) with immediate audio-visual re-alignment.

---

## 4. Unified Lyrics Candidate Data Model

```rust
// Core Lyrics Candidate & Manager Types in Sonora Core

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LyricsSyncType {
    SyllableSynced, // TTML / Enhanced LRC (Word level)
    LineSynced,     // Standard LRC ([mm:ss.xx] Line level)
    PlainText,      // Unsynchronized text block
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LyricsSourceKind {
    EmbeddedTag,
    LocalSidecar { path: String },
    SQLiteCache,
    LrclibPublicApi { id: u64 },
    PluginProvider { plugin_id: String },
    UserManualEdit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricsCandidate {
    pub candidate_id: String,
    pub source_kind: LyricsSourceKind,
    pub provider_name: String,
    pub sync_type: LyricsSyncType,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub duration_seconds: f64,
    pub duration_delta_seconds: f64, // |track_duration - candidate_duration|
    pub match_confidence: f32,       // 0.0 to 1.0
    pub language_code: Option<String>, // e.g. "en", "ja", "es"
    pub is_instrumental: bool,
    pub raw_content: String,
}
```

---

## 5. UX Flows & Interactions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       LYRICS ENRICHMENT WORKFLOW                            │
│                                                                             │
│  [User clicks "Find Lyrics" / Cmd+L] ──► Opens Lyrics Manager Overlay       │
│                                                │                            │
│                                                ▼                            │
│  [Fetch Candidates] ──► Query (LRCLIB + Local Sidecars + Embedded Tags)     │
│                                                │                            │
│                                                ▼                            │
│  [Display Candidate List]                                                   │
│  • Candidate 1: LRCLIB (Synced, Δt = +0.2s, Score: 0.98) [Active]          │
│  • Candidate 2: Local Sidecar (Synced, Δt = 0.0s, Score: 0.95)             │
│  • Candidate 3: LRCLIB (Plain Text, Score: 0.70)                            │
│                                                │                            │
│                                                ▼                            │
│  [User Selects Candidate 2] ──► Live Preview Lyrics on Screen               │
│                                                │                            │
│                                                ▼                            │
│  [User Adjusts Offset (+100ms)] ──► Real-Time AST Offset Update             │
│                                                │                            │
│                                                ▼                            │
│  [User Clicks "Save Lyrics"] ──► Updates SQLite Cache & (Opt) Exports .lrc  │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **Triggering**: User presses `Cmd/Ctrl + L` or right-clicks a track and selects **`[Find Lyrics]`**.
2. **Candidate Search & Display**:
   - The Lyrics Manager drawer slides in, showing active lyrics on the main stage and candidate options in a side panel.
   - Candidates show badges: `[Synced]`, `[Plain]`, Provider Source (`LRCLIB`), Duration Delta (`+0.2s`), and Language (`EN`).
3. **Live Audition**:
   - Clicking any candidate immediately parses its raw payload into the Universal `LyricsDocument` AST and syncs rendering to current playback.
4. **Timing Fine-Tuning**:
   - If lyrics lag or lead audio vocals, the user adjusts offset using the timing bar or keyboard shortcuts (`[`: -100ms, `]`: +100ms).
   - Offset updates `LyricsDocument.offset_ms` in real time without restarting audio.
5. **Persisting Options**:
   - **`[ Save to Library Cache ]`**: Stores normalized AST in SQLite `lyrics_cache`.
   - **`[ Export .lrc Sidecar ]`**: Writes `.lrc` file to local track directory (e.g. `Song.lrc`) for external player compatibility.

---

## 6. Architecture Implications & Subsystem Boundaries

### 6.1 Multi-Tier Resolver Hierarchy Integration (ADR-008 Refinement)
The Lyrics Manager acts as the control surface over the Multi-Tier Resolver Pipeline:

```
Tier 1: User Manual Edits & Saved Overrides (SQLite)  [Priority 1]
Tier 2: Embedded Audio Tags (ID3 SYLT/USLT)          [Priority 2]
Tier 3: Local File Sidecars (.lrc / .ttml)           [Priority 3]
Tier 4: LRCLIB Community API                         [Priority 4]
Tier 5: 3rd-Party Plugin Providers                   [Priority 5]
Tier 6: Plain Text Fallback                          [Priority 6]
```

### 6.2 Universal AST Parsing Guard
Raw lyric strings (LRC, Enhanced LRC, TTML) from any candidate MUST be parsed through the memory-safe pure-Rust `UniversalLyricsParser` into a `LyricsDocument` AST before reaching the GUI or TUI renderer.

---

## 7. Provider & API Technical Evaluation: LRCLIB

- **Base Endpoint**: `https://lrclib.net/api`
- **Lookup Endpoint**: `GET /api/get?track_name={title}&artist_name={artist}&album_name={album}&duration={secs}`
- **Search Endpoint**: `GET /api/search?track_name={title}&artist_name={artist}`
- **Community Submission API**: `POST /api/publish`
  - Allows Sonora users to submit verified synced lyrics back to the open-source LRCLIB repository.
- **Rate Limits & Terms**: Public, free, open-source. Requires User-Agent header. No API keys required.

---

## 8. Failure Cases & Degradation Matrix

| Scenario | Cause | System Response | User Impact |
| :--- | :--- | :--- | :--- |
| **No Lyrics Found (404)** | Obscure track | Display "No lyrics found" empty state card. | User can manually paste lyrics into Timing Editor. |
| **Malformed LRC Syntax** | Invalid timestamp tags `[99:99:99]` | Parser discards bad timestamps; fallback to line text. | Lyrics display cleanly without crashing. |
| **Network Offline** | No internet connection | Disables online LRCLIB candidate search tab. | Local sidecars, embedded tags, and SQLite cache work 100%. |
| **Timing Desync (> 2.0s)** | Candidate matches wrong pressing | Duration delta warning highlighted in red badge. | User can select alternative candidate or adjust offset. |

---

## 9. Security & Privacy Considerations

1. **No User Listening Logs**: LRCLIB requests include only track name, artist, album, and duration. No user accounts or tokens are required for reading lyrics.
2. **Input Sanitization**: Unsanitized raw text from external lyric providers undergoes HTML/script tag stripping before rendering in GUI/TUI webviews.

---

## 10. Open Decisions

1. **LRCLIB Submission Integration**:
   - *Status*: Open feature design for v0.2.
   - *Question*: Should Sonora allow 1-click publishing of user-created synced lyrics back to LRCLIB?
   - *Recommendation*: Include a `[ Publish to LRCLIB ]` button in the Lyrics Manager with an explicit confirmation step.

---

## 11. References

- LRCLIB Official Documentation: `https://lrclib.net/docs`
- Multi-Tier Cascading Lyrics Resolver Specification: `docs/07-lyrics-system.md`
- Sonora ADR-008 Decision Record: `docs/12-decision-log.md#adr-008-multi-tier-cascading-lyrics-resolver--universal-ast`
