# Sonora: Lyrics Subsystem & Kinetic Typography Architecture

## 1. Lyrics Architecture & Pipeline Overview

Sonora's lyrics subsystem is designed to deliver sub-millisecond synchronized, aesthetically stunning, and multi-lingual lyrics. It spans a multi-tier fallback resolver, a streaming parser, a physics-based kinetic renderer, and a local cache engine.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LYRICS SUBSYSTEM PIPELINE                         │
│                                                                             │
│  Track Change ──► [Resolver Coordinator]                                    │
│                          │                                                  │
│         ┌────────────────┼────────────────┬────────────────┐                │
│         ▼ (Tier 1)       ▼ (Tier 2)       ▼ (Tier 3)       ▼ (Tier 4)       │
│    [Embedded Tags]  [Local Sidecars]  [LRCLIB Public API] [Plugin Providers]│
│    (SYLT / USLT)    (.lrc / .ttml)    (Synced / Syllable) (Musix/NetEase)   │
│         │                │                │                │                │
│         └────────────────┼────────────────┴────────────────┘                │
│                          │ (Raw Payload)                                    │
│                          ▼                                                  │
│                 [Universal Parser] ──► [Format AST: Syllable / Line / Plain]│
│                          │                                                  │
│                          ▼                                                  │
│                 [Lyrics Cache (SQLite)]                                     │
│                          │                                                  │
│         ┌────────────────┴────────────────┐                                 │
│         ▼                                 ▼                                 │
│  [Desktop GUI Renderer]            [Terminal TUI Renderer]                  │
│  • Syllable Kinetic Typography     • Line-Level Auto-Scroll                 │
│  • Spring-Physics Smooth Scroll    • ANSI Highlight Active Line             │
│  • CJK Furigana / Romaji           • Plain Text Fallback                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Multi-Tier Fallback Resolution Pipeline

When a track begins playing, the resolver coordinator queries sources sequentially with a configurable per-tier timeout:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       RESOLVER PRIORITY ORDER & TIMEOUTS                    │
├──────┬──────────────────────┬─────────────┬─────────────────────────────────┤
│ Tier │ Provider Source      │ Timeout     │ Description                     │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 1    │ Embedded Audio Tags  │ < 1 ms      │ ID3 `SYLT`/`USLT`, Vorbis, MP4  │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 2    │ Local File Sidecar   │ < 5 ms      │ `.lrc` / `.ttml` in same folder │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 3    │ SQLite Local Cache   │ < 2 ms      │ Previously resolved & stored    │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 4    │ LRCLIB Community API │ 1500 ms     │ Free, open-source synced API    │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 5    │ 3rd-Party Plugins    │ 2000 ms     │ User-installed plugin providers │
├──────┼──────────────────────┼─────────────┼─────────────────────────────────┤
│ 6    │ Plain Text Fallback  │ N/A         │ Unsynced static text view       │
└──────┴──────────────────────┴─────────────┴─────────────────────────────────┘
```

---

## 3. Data Structures & Universal Lyrics AST

All lyric formats are normalized into a unified, high-performance in-memory Abstract Syntax Tree (AST):

```rust
// Core Rust Data Structures for Sonora Lyrics AST
pub struct LyricsDocument {
    pub track_id: u64,
    pub format: LyricFormat,
    pub offset_ms: i32,
    pub lines: Vec<LyricLine>,
    pub metadata: LyricMetadata,
}

pub enum LyricFormat {
    PlainText,
    LineSyncedLrc,
    WordSyncedEnhancedLrc,
    SyllableSyncedTtml,
}

pub struct LyricLine {
    pub id: u32,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub text: String,
    pub translation: Option<String>,
    pub transliteration: Option<String>, // e.g. Romaji / Pinyin
    pub syllables: Vec<LyricSyllable>,
}

pub struct LyricSyllable {
    pub start_time_ms: u64,
    pub duration_ms: u32,
    pub text: String,
}

pub struct LyricMetadata {
    pub source: String,
    pub author: Option<String>,
    pub is_instrumental: bool,
}
```

---

## 4. Typography & Kinetic Physics Engine

### 4.1 Spring-Physics Auto-Scrolling
Rather than jumping abruptly between lines, the active lyrics container uses a critically damped harmonic oscillator to animate scroll offset $y(t)$:

$$F_{spring} = -k (y - y_{target}) - c \frac{dy}{dt}$$

- **Stiffness ($k$)**: $180.0$
- **Damping Ratio ($c$)**: $24.0$ (Critically damped to eliminate oscillations while ensuring snappy centering)
- **Viewport Alignment**: The active line is centered at exactly 40% of the lyrics canvas height, providing optimal visual anticipation for upcoming lines.

### 4.2 Syllable Progress Interpolation & Shaders
For syllable-level formats (Enhanced LRC / TTML), each word's fill color is rendered via a smooth horizontal progress gradient evaluated on each frame:

$$\text{Progress}(t) = \text{clamp}\left(\frac{t_{audio} - t_{start}}{t_{duration}}, 0.0, 1.0\right)$$

- High-precision audio clock interpolation accounts for DAC hardware latency, ensuring the visual fill perfectly matches vocal phonemes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       SYLLABLE-FILL PROGRESSION TIMELINE                    │
│                                                                             │
│   Time: 00:14.200 ──► "Nev-" [██████████] (100% Filled)                     │
│   Time: 00:14.500 ──► "-er " [██████████] (100% Filled)                     │
│   Time: 00:14.800 ──► "gon-" [██████░░░░] (60% Interpolating...)           │
│   Time: 00:15.100 ──► "-na " [░░░░░░░░░░] (0% Pending)                     │
│   Time: 00:15.300 ──► "give " [░░░░░░░░░░] (0% Pending)                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Multi-Lingual Transliteration Engine

Sonora incorporates a non-blocking transliteration pipeline for East Asian typography:
1. **Japanese Furigana**: Kanji words parsed with embedded `MeCab` / `Kuroshiro` dictionary tokens generate standard `<ruby>` / Furigana annotations above characters.
2. **Romaji & Pinyin**: Phonetic transcriptions can be rendered as an optional secondary subtitle line directly below original script lines.
3. **Dual-Line Translation**: When bilingual metadata is available, translated lyrics are rendered in `--text-secondary` font below each original line.

---

## 6. Interactive Timing Editor & Synchronizer

Users can visually create, edit, and adjust lyric timing in real time:
- **Interactive Scrubbing**: Click any line or syllable in the editor to immediately seek the playback engine to that timestamp.
- **Global / Per-Track Offset**: Fine-tune synchronization using keyboard shortcuts (`[`: -100ms, `]`: +100ms) with per-track persistence in SQLite.
- **Tap-to-Sync Recording Mode**: Play the track in the editor and press `Space` to timestamp lines and words in real time.
- **Export & Submission**: 1-click export to local `.lrc` / `.ttml` file or direct community submission to LRCLIB via authenticated API key.

---

## 7. Reversibility & Lyrics Subsystem Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Decisions
> - **Primary Public API**: LRCLIB is the default public provider, but can be swapped or combined with other open lyric repositories via provider configuration.
> - **Transliteration Dictionary**: CJK dictionaries are bundled as modular asset packs and can be updated independently of the core binary.
>
> ### Invariable Boundaries
> - **Universal AST Standard**: All parsers (LRC, TTML, SYLT) must normalize into the unified `LyricsDocument` AST before reaching presentation layers.
