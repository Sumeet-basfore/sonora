# Sonora: Architecture Decision Log & Technical Records (ADR)

## 1. Document Overview & ADR Index

This document establishes the official **Architecture Decision Records (ADR)** for Sonora. It records the architectural context, evaluated alternatives, decision rationale, consequences, and reversibility boundaries for all foundational choices made across the codebase and documentation.

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│                                   SONORA DECISION INDEX (ADRs)                                   │
├─────────┬────────────────────────────────────────────────────────┬─────────────┬─────────────────┤
│ ADR ID  │ Title & Decision Domain                                │ Status      │ Reversibility   │
├─────────┼────────────────────────────────────────────────────────┼─────────────┼─────────────────┤
│ ADR-001 │ Decoupled Multi-Process Client-Daemon Architecture     │ ACCEPTED    │ Irreversible    │
│ ADR-002 │ Dedicated Lockless Real-Time Audio Engine Loop         │ ACCEPTED    │ Irreversible    │
│ ADR-003 │ SQLite WAL Mode with FTS5 for Metadata Indexing & Query│ ACCEPTED    │ Reversible      │
│ ADR-004 │ Dual Presentation Layer: GPU Desktop GUI & Terminal TUI│ ACCEPTED    │ Irreversible    │
│ ADR-005 │ Capability-Based Sandboxed Plugin Host (WASM + QuickJS)│ ACCEPTED    │ Reversible Eng  │
│ ADR-006 │ Decentralized Git-Backed Community Marketplace Model   │ ACCEPTED    │ Reversible Host │
│ ADR-007 │ Semantic Token Hierarchy & Mathematical WCAG Contrast  │ ACCEPTED    │ Reversible Clr  │
│ ADR-008 │ Multi-Tier Cascading Lyrics Resolver & Universal AST   │ ACCEPTED    │ Reversible Prov │
│ ADR-009 │ Lockless Shared-Memory Circular Buffer for Visualizers │ ACCEPTED    │ Reversible Tap  │
│ ADR-010 │ Zero-Trust Media Asset Sanitization & Dimension Guards │ ACCEPTED    │ Irreversible    │
│ ADR-011 │ Anti-Bloat Boundaries: No DRM, Social Feeds, Cloud Lock│ ACCEPTED    │ Irreversible    │
│ ADR-012 │ Production Audio Engine & DSP Signal Pipeline          │ ACCEPTED    │ Irreversible    │
│ ADR-013 │ Online Metadata & Abstracted Provider Engine (MB/CAA)  │ ACCEPTED    │ Reversible Prov │
│ ADR-014 │ Unified Multi-Source Artwork Pipeline & Cache Hierarchy│ ACCEPTED    │ Reversible Cache│
│ ADR-015 │ Dedicated Lyrics Manager & Multi-Candidate Override UX │ ACCEPTED    │ Reversible Workspace
│ ADR-016 │ Deterministic Explainable Metadata Matching & Scoring  │ ACCEPTED    │ Reversible Score│
│ ADR-017 │ Integrated Contextual Discovery & Universal Search Model│ ACCEPTED    │ Reversible UI   │
└─────────┴────────────────────────────────────────────────────────┴─────────────┴─────────────────┘
```

---

## 2. Architecture Decision Records

### ADR-001: Decoupled Multi-Process Client-Daemon Architecture

- **Status**: `ACCEPTED` (Irreversible Foundation)
- **Context**: Legacy music players (foobar2000, Winamp) run as monolithic single-process applications where heavy UI repaints, skin glitches, or plugin crashes directly terminate or stutter the audio stream. Conversely, monolithic Electron clients consume excessive resources and cannot support lightweight terminal usage.
- **Decision**: Sonora is architected as a decoupled, multi-process client-daemon system. The headless background daemon (`sonorad`) owns audio decoding, DSP chains, library indexing, and IPC transport. Frontends (`sonora` GUI, `sonora-tui`, `sonora-cli`) act as stateless or cache-backed clients communicating via local Unix Domain Sockets or Windows Named Pipes.
- **Alternatives Considered**:
  - *Monolithic Embedded GUI/Audio Binary*: Simpler initial build, but a UI thread block or shader crash causes audible playback dropouts, and headless/terminal operation becomes impossible without building two separate cores.
  - *MPD Protocol Re-implementation*: Leverages existing MPD clients, but standard MPD protocol lacks rich support for complex biquad EQ curves, real-time shared memory FFT taps, kinetic syllable typography, and capability-based plugin hooks.
- **Consequences**:
  - *Positive*: Unbreakable playback stability; independent client lifecycles; native support for simultaneous GUI and TUI interfaces; scriptable CLI control.
  - *Trade-off*: Requires maintaining IPC message framing, serialization contracts, and state synchronization across process boundaries.

---

### ADR-002: Dedicated Lockless Real-Time Audio Engine Loop

- **Status**: `ACCEPTED` (Irreversible Foundation)
- **Context**: Audiophile-grade playback demands sample-accurate gapless playback and zero buffer underruns. Standard thread models that perform dynamic memory allocations, file I/O, or mutex locking on the audio callback suffer unavoidable dropouts under system contention.
- **Decision**: The real-time audio thread executes with elevated priority (`SCHED_FIFO` / MMCSS `Pro Audio`) under strict real-time audio programming constraints: **zero dynamic memory allocations (`malloc`/`free`)**, **zero mutex locks**, and **zero disk/network I/O**. Communication with daemon control threads uses lock-free Single-Producer Single-Consumer (SPSC) ring buffers (`rtrb`).
- **Alternatives Considered**:
  - *Mutex-Protected Audio Buffers*: Standard mutexes lead to priority inversion and audio buffer starvation when background threads contend for locks.
  - *High-Level Audio Frameworks (e.g., SDL2_mixer, PortAudio)*: Lack fine-grained control over sample-by-sample biquad cascade filters, Direct Form II transposition, lockless visualizer taps, and bit-perfect exclusive output backends.
- **Consequences**:
  - *Positive*: Zero audio pops, clicks, or dropouts under 100% CPU/disk load; bit-perfect bitstream passthrough; sample-accurate gapless transitions.
  - *Trade-off*: Requires pre-allocating all audio ring buffers and implementing biquad DSP coefficient updates via atomic pointer swaps.

---

### ADR-003: SQLite WAL Mode with FTS5 for Metadata Indexing & Query

- **Status**: `ACCEPTED` (Reversible Search Engine Layer)
- **Context**: Music collectors often possess libraries exceeding 100,000 tracks. The indexing engine must handle rapid background tag parsing while delivering instantaneous (< 5ms) fuzzy search responses without locking the database.
- **Decision**: Use embedded SQLite configured in Write-Ahead Logging (`PRAGMA journal_mode = WAL`) and memory temp storage (`PRAGMA temp_store = MEMORY`), paired with SQLite's native `FTS5` full-text search module using trigram tokenization.
- **Alternatives Considered**:
  - *In-Memory Hashmaps / Vector Indices*: Fast for small libraries, but incurs high RAM overhead (200MB+ for 100k tracks) and requires slow re-parsing on cold startup.
  - *Tantivy (Lucene in Rust)*: Exceptional full-text search, but introduces separate database/index files and synchronization complexity compared to an integrated SQLite relational schema.
- **Consequences**:
  - *Positive*: Single-file database portability; ACID transactions; sub-5ms fuzzy queries across 100k tracks; zero lock contention between background file scanners and frontend queries.
  - *Reversibility*: Abstracted behind a `SearchIndexProvider` interface; Tantivy can be introduced as an alternative backend if phonetic search requirements grow.

---

### ADR-004: Dual Presentation Layer: GPU-Accelerated Desktop GUI & Terminal TUI

- **Status**: `ACCEPTED` (Irreversible Architectural Commitment)
- **Context**: Desktop music listeners are fragmented between users desiring rich visual ambiance (dynamic shaders, cover art palettes, animated lyrics) and developer/power users working in terminal multiplexers (`tmux`) who demand minimal memory footprints and keyboard navigation.
- **Decision**: Sonora provides two first-class official frontends backed by the exact same daemon and IPC interface:
  1. **Desktop GUI (`sonora`)**: Built on hardware-accelerated graphics (WGPU / shaders) featuring kinetic typography, dynamic ambient backdrops, and interactive EQ curves.
  2. **Terminal TUI (`sonora-tui`)**: Built on `ratatui`/`crossterm` featuring complete Vim navigation (`h/j/k/l`, `/`), ASCII/Braille CAVA spectrum visualizers, and ANSI color theme adaptation consuming < 15MB RAM.
- **Alternatives Considered**:
  - *Web/Electron Wrapper*: Fast initial UI assembly, but unacceptable memory footprint (500MB–1.5GB) and inability to provide a native terminal client.
  - *Single TUI with Graphics Sixel Hacks*: Leaves graphical desktop users without smooth 120 FPS GPU shaders, fluid gesture scrolling, or rich typographic rendering.
- **Consequences**:
  - *Positive*: Complete market coverage across audiophiles, aesthetic customizers, and terminal minimalists with shared configuration.
  - *Trade-off*: Dual UI codebases requiring feature-parity alignment across releases.

---

### ADR-005: Capability-Based Sandboxed Plugin Host (WASM + QuickJS)

- **Status**: `ACCEPTED` (Reversible Engine Implementation)
- **Context**: Third-party extensions (lyrics providers, metadata fetchers, visualizers, DSP nodes) enrich the player, but legacy native plugins (e.g. C++ DLLs in foobar2000/Winamp) crash the entire player when bugs occur and present severe security risks.
- **Decision**: Enforce a capability-based untrusted plugin model. Extensions execute in isolated sandboxes: **Wasmtime** for high-performance DSP/visualizers and **QuickJS** for lightweight JavaScript scripting. Plugins must declare capabilities in `manifest.json` (`network:fetch`, `library:read`, `audio:tap`, `storage:cache`), and permissions are explicitly reviewed and gated at the host API level.
- **Alternatives Considered**:
  - *Unsandboxed Native Shared Libraries (`.so`/`.dll`)*: Maximum performance, but fatal crashes and arbitrary code execution vulnerabilities directly threaten player stability and user privacy.
  - *Full Node.js / Deno Subprocesses*: Heavy resource consumption (50MB+ per plugin instance) that violates Sonora's lean performance budget.
- **Consequences**:
  - *Positive*: Complete crash resilience (a panicked plugin cannot terminate `sonorad` or the GUI); zero data exfiltration to unauthorized network endpoints; live hot-reloading.
  - *Reversibility*: Sandboxing is encapsulated behind the `PluginRuntime` trait; runtime engines (Wasmtime, QuickJS) can be swapped or upgraded without altering manifest schema contracts.

---

### ADR-006: Decentralized Git-Backed Community Marketplace Model

- **Status**: `ACCEPTED` (Reversible Host & Mirror Architecture)
- **Context**: Centralized proprietary app stores introduce vendor lock-in, recurring infrastructure costs, and single points of failure. Conversely, completely unindexed plugin distribution makes discovery and updates painful for non-technical users.
- **Decision**: Sonora adopts a decentralized, Git-backed community registry (inspired by Obsidian and Homebrew Cask). Manifest indices (`plugins.json`, `themes.json`) are maintained in an open-source GitHub repository, synced to global CDN edges, and verified cryptographically (SHA-256 asset checksums and Ed25519 author signatures) prior to client installation.
- **Alternatives Considered**:
  - *Proprietary Centralized Backend / Database*: High maintenance cost, subscription pressure, and vulnerability to server downtime.
  - *Manual Zip File Downloads Only*: Safe from centralization, but lacks discoverability, automated updates, and version compatibility tracking.
- **Consequences**:
  - *Positive*: Zero hosting fees; complete community transparency; automated PR validation; support for private/enterprise registries and air-gapped offline `.sonora-plugin` installation.
  - *Trade-off*: Registry updates depend on Git commit sync cycles and CDN cache invalidation.

---

### ADR-007: Semantic Design Token Hierarchy & Mathematical WCAG Contrast

- **Status**: `ACCEPTED` (Reversible Color Algorithm Layer)
- **Context**: Dynamic cover-art color theming frequently results in unreadable interfaces when album artwork contains low-contrast, pure black, or over-saturated white palettes.
- **Decision**: Theming is governed by a semantic design token hierarchy (`--bg-canvas`, `--accent-primary`, `--text-primary`). When extracting palettes from active artwork, colors are transformed into perceptual Oklab color space and run through an automated WCAG 2.1 AA mathematical clamping algorithm to strictly guarantee a **$\ge$ 4.5:1 contrast ratio** for all text and interactive icons.
- **Alternatives Considered**:
  - *Static Themes Only*: Completely legible, but forfeits the immersive visual atmosphere that differentiates Sonora.
  - *Unconstrained Dynamic Colors*: Visually striking on select albums, but completely unreadable on high-contrast or monochrome album covers.
- **Consequences**:
  - *Positive*: Guaranteed readability across all possible album artwork; seamless theme authoring via CSS token overrides.
  - *Reversibility*: Color extraction algorithms (ColorThief, Material You HCT, Oklab) can be iterated independently of the semantic token schema.

---

### ADR-008: Multi-Tier Cascading Lyrics Resolver & Universal AST

- **Status**: `ACCEPTED` (Reversible Provider Hierarchy)
- **Context**: Lyrics formats and sources are fragmented (embedded ID3 `SYLT`/`USLT`, local `.lrc` and `.ttml` sidecars, online community APIs). Rendering engines require a standardized structure to support line scrolling and word-level karaoke fill.
- **Decision**: Implement a cascading fallback resolver pipeline (Embedded Tags $\to$ Local Sidecars $\to$ Local SQLite Cache $\to$ LRCLIB Public API $\to$ Plugin Providers $\to$ Plain Text Fallback). All raw lyric payloads are normalized by a universal streaming parser into a unified in-memory `LyricsDocument` AST.
- **Alternatives Considered**:
  - *Single Format Hardcoding (LRC only)*: Ignores emerging word/syllable-level TTML and Enhanced LRC standards, preventing Apple-grade karaoke typography.
  - *Direct UI Parsing*: Parsing formats directly inside UI components leads to duplicated parsing logic, rendering lag, and difficulty supporting dual GUI/TUI rendering.
- **Consequences**:
  - *Positive*: High lyrics hit rate (> 95%); seamless support for both line-level and syllable-level lyrics; instantaneous cache reads (< 2ms).
  - *Reversibility*: Additional online providers or parsing formats can be added to the resolver pipeline without altering downstream presentation renderers.

---

### ADR-009: Lockless Shared-Memory Circular Buffer for Visualizers

- **Status**: `ACCEPTED` (Reversible Tap Buffer Model)
- **Context**: High-refresh visualizers (projectM Milkdrop, CAVA FFT spectrum) require raw PCM samples and FFT frequency bins at 60 to 144 FPS. Serializing this high-rate audio stream over IPC domain sockets saturates the IPC gateway and burns CPU cycles.
- **Decision**: Provision a dedicated zero-copy shared memory circular buffer (`SonoraSharedRing` via `shm_open` on POSIX and `CreateFileMapping` on Windows). The real-time audio thread writes downsampled 2048-sample windows and pre-computed 256-band FFT magnitudes into this ring buffer using atomic frame sequence indices. Frontend visualizers attach read-only without generating socket round-trips.
- **Alternatives Considered**:
  - *Protobuf/JSON IPC Audio Streaming*: Results in excessive serialization CPU overhead and IPC socket buffer overflow at 120 FPS.
  - *Direct Audio Loopback Capture in Clients (Pulse/WASAPI Loopback)*: Introduces OS latency, device-matching complexities, and fails on headless or virtual audio setups.
- **Consequences**:
  - *Positive*: Silky 120+ FPS visualizer performance with < 2% CPU overhead; zero IPC bus congestion.
  - *Reversibility*: The shared memory layout is versioned with header magic bytes (`0x534F4E4F`) to allow future layout expansions without breaking client compatibility.

---

### ADR-010: Zero-Trust Media Asset Sanitization & Dimension Guards

- **Status**: `ACCEPTED` (Irreversible Security Guardrail)
- **Context**: Embedded album artwork and metadata tags extracted from untrusted audio files downloaded from the internet can exploit image parser vulnerabilities (decompression bombs, memory corruption in C decoders).
- **Decision**: All image extraction pipelines enforce strict memory allocation limits (max 64MB buffer), dimension clamps (rejecting images $> 8192 \times 8192$ pixels), and utilize memory-safe pure-Rust decoders (`image-rs`). Text metadata undergoes UTF-8 sanitization and ANSI control character stripping before presentation in GUI or TUI.
- **Alternatives Considered**:
  - *Native OS Image Decoders (GDI+, ImageIO)*: Vulnerable to platform-specific CVEs and unpredictable cross-platform rendering artifacts.
  - *Unbounded Image Loading*: Exposes memory exhaustion vulnerabilities from malicious high-resolution files.
- **Consequences**:
  - *Positive*: Immune to image-based denial-of-service and arbitrary memory corruption; robust cross-platform texture compositing.
  - *Trade-off*: Ultra-high-resolution album art scans (> 8K) are downsampled to optimal display textures.

---

### ADR-011: Strict Anti-Bloat Scope Commitments

- **Status**: `ACCEPTED` (Irreversible Product Commitment)
- **Context**: Modern desktop audio applications degrade over time as product teams introduce algorithmic social feeds, advertising monetization, cloud storage upsells, and DRM streaming hacks.
- **Decision**: Sonora explicitly codifies non-goals into the architectural foundation:
  1. **NO Social Feeds or Algorithmic Discovery Radios**: No friend-activity monitors or proprietary machine learning recommendation feeds.
  2. **NO DRM Decryption / Stream Ripping**: No bypasses of Widevine or proprietary streaming DRM (Spotify, Apple Music, Tidal).
  3. **NO Digital Audio Workstation (DAW) Capabilities**: No multi-track audio recording, editing, or MIDI sequencing.
  4. **NO Cloud Storage Lock-in**: No proprietary subscription cloud storage lockers; users own their local storage and personal Subsonic/Navidrome servers.
  5. **NO Telemetry or Surveillance Monetization**: No surveillance analytics, tracking beacons, or ad banners.
- **Alternatives Considered**:
  - *Adding Streaming DRM Wrappers*: High legal risk, constant breakage, and architectural compromise.
  - *Cloud Subscription Monetization*: Conflicts with Sonora's sovereign, local-first product thesis.
- **Consequences**:
  - *Positive*: Preserves laser focus on audio fidelity, aesthetic craft, and ruthless performance; establishes high user trust.
  - *Trade-off*: Sonora does not attempt to replace official streaming apps for users seeking algorithmic radio or social commenting.

---

### ADR-012: Production Audio Engine & DSP Signal Pipeline

- **Status**: `ACCEPTED` (Irreversible Foundation)
- **Context**: The prototype playback pipeline utilized linear sample interpolation, lacked true-peak suppression, failed to handle inter-track transitions gaplessly, and caused severe audio distortion and ALSA underruns due to buffer misalignment and naive resampling.
- **Decision**: Replace the prototype pipeline with the research-backed, audiophile-grade Sonora DSP Signal Chain and Engine:
  1. **Strict 8-Stage Signal Chain**: `Source -> Symphonia/Lofty Decode -> Channel Normalization (Mono/Stereo) -> Rubato Polyphase Sinc Resampler -> ReplayGain 2.0 / EBU R128 + Dynamic Headroom Pre-cut -> Direct Form II Transposed Parametric EQ -> True-Peak Lookahead Limiter (ITU-R BS.1770-4 / AES17) -> TPDF Dither -> Lock-Free CPAL Audio Output`.
  2. **Polyphase Sinc Resampling**: Replaced `LinearResampler` with `rubato::Async<f32>` (Kaiser/Blackman-Harris filterbanks, >140 dB stopband rejection, linear phase response, and arbitrary-block streaming FIFOs). Exact passthrough is enforced whenever rates match.
  3. **Four Explicit Playback Modes**:
     - `DefaultShared`: OS shared audio engine with high-quality sinc resampling and limiter protection.
     - `HighQuality`: Maximum fidelity sinc resampling (256-tap Blackman-Harris), ReplayGain 2.0, and True-Peak limiting at -0.3 dBTP.
     - `BitPerfect`: Exact digital bitstream passthrough (100% mathematical identity, bypassing all EQ/volume/limiting/dither; error $\Delta = 0.0, -\infty\text{ dBFS}$).
     - `ExclusiveDsp`: Hardware output isolation with full parametric EQ and DSP processing.
  4. **Dual-Decoder Gapless Audio Handover**: Implemented asynchronous pre-opening and priming of subsequent audio streams in `AudioPlayer` (`sonora-decoder-worker`). Transitions occur sample-accurately without flushing ring buffers or restarting the CPAL stream.
  5. **True-Peak Lookahead Limiter**: 4.0ms circular lookahead delay line paired with 4x polyphase FIR true-peak estimation and instantaneous lookahead attack to prevent inter-sample clipping on hot masters.
  6. **ReplayGain 2.0 / EBU R128**: Dynamic loudness normalization supporting Track and Album modes, preamps, fallback gains, and peak-limiting clipping prevention (`effective_gain = min(target, 1.0/peak)`).
- **Alternatives Considered**:
  - *Linear/Cubic Resampling*: Fast, but suffers severe high-frequency attenuation and aliasing distortion above 10 kHz.
  - *Single Decoder with Stop-Start*: Introduces audible pops, gap dropouts, and ALSA stream teardowns between tracks.
  - *Hard Sample Clamping*: Clips inter-sample peaks during digital-to-analog conversion on external DACs.
- **Consequences**:
  - *Positive*: Crystal-clear audiophile sound quality; zero distortion; bit-perfect verification passed; zero-discontinuity gapless playback; full ALSA/WASAPI/CoreAudio device discovery.
  - *Trade-off*: Sinc resampling incurs higher CPU usage than linear interpolation, but stays well under <1.5% CPU on modern cores due to SIMD vectorization in `rubato`.

---

### ADR-013: Online Metadata Subsystem & Abstracted Provider Architecture

- **Status**: `ACCEPTED` (Reversible Provider Layer)
- **Context**: Local audio file tags are frequently incomplete or inconsistent. Connecting to online metadata services (MusicBrainz, Cover Art Archive, LRCLIB) expands library richness, but unthrottled or tightly coupled API calls threaten player stability, offline functionality, and user privacy.
- **Decision**: Architect an asynchronous, provider-based metadata enrichment layer in `sonorad`. MusicBrainz is the canonical metadata authority and Cover Art Archive is the primary release art source. All HTTP requests are rate-limited via a thread-safe Token Bucket (1.0 req/sec for MusicBrainz), supply compliant User-Agent headers, enforce HTTPS, require no user accounts, and operate strictly outside the audio path.
- **Alternatives Considered**:
  - *Direct Hardcoded HTTP Fetching in UI*: Couples UI components to third-party REST schemas, risks IP bans due to unthrottled requests, and breaks when offline.
  - *Proprietary Cloud Metadata Proxy*: High operational overhead, vendor lock-in, and privacy compromise.
- **Consequences**:
  - *Positive*: Rich entity metadata (Release Groups vs Pressing Releases); 100% offline playback resilience; zero user account friction; zero telemetry.
  - *Reversibility*: Provider implementations adhere to Rust `async_trait` interfaces (`MetadataProvider`, `ArtworkProvider`, `LyricsProvider`) and can be added or upgraded without modifying UI logic.

---

### ADR-014: Unified Multi-Source Artwork Pipeline & Cache Hierarchy

- **Status**: `ACCEPTED` (Reversible Caching & Storage Layer)
- **Context**: Managing artwork from multiple sources (embedded ID3 APIC, local folder `cover.jpg`, Cover Art Archive, Fanart.tv, Wikidata) often leads to silent file overwrites, low-resolution upscaling artifacts, and security vulnerabilities from untrusted remote image headers.
- **Decision**: Establish a strict multi-tiered artwork resolution hierarchy. Distinguish between Embedded Artwork, Local Folder Artwork, Cached Downloaded Artwork, Release Art (specific pressing), Release-Group Art (master album), and Artist Visuals. All downloaded assets are validated through pure-Rust zero-trust dimension guards (max 8192x8192) and stored as WebP textures (`thumbnails/` 300x300 and `full/` 1200x1200) in `$XDG_CACHE_HOME/sonora/covers/`. Applying online artwork **never** silently mutates physical audio tags or local folder images without explicit user opt-in.
- **Alternatives Considered**:
  - *Direct File Overwrite*: Silently replacing `cover.jpg` or rewriting ID3 APIC tags corrupts user files without recovery paths.
  - *On-the-Fly Dynamic Image Fetching without Local Caching*: Causes network latency, stuttering UI grid scrolls, and broken offline artwork display.
- **Consequences**:
  - *Positive*: High-DPI visual legibility; protection against C-based image exploit CVEs; complete user control over file mutation.
  - *Reversibility*: Image cache format (WebP) and storage directory structure can be migrated independently of SQLite library schema.

---

### ADR-015: Dedicated Lyrics Manager & Multi-Candidate Manual Override UX

- **Status**: `ACCEPTED` (Reversible UI/Workspace Domain)
- **Context**: Automatic lyric resolvers occasionally select incorrect pressings, plain text instead of synced LRC, or undesirable language translations. Burying lyric settings inside application preferences makes discovering, switching, and fine-tuning lyrics tedious.
- **Decision**: Elevate lyrics into a dedicated, first-class **Lyrics Manager Workspace** (`Cmd/Ctrl + L` or context menu). The workspace displays ranked candidate options (showing duration delta $\Delta t$, sync type, provider name, and match score), allows real-time live candidate auditioning against playing audio, provides an interactive timing offset bar ($\pm 100\text{ ms}$), supports local `.lrc` sidecar exporting, and enables manual text/timestamp editing.
- **Alternatives Considered**:
  - *Automated Single-Candidate Selection Only*: Leaves users unable to correct mismatched lyrics for obscure or live recordings.
  - *Settings Sub-Menu Configuration*: Fragments the lyric synchronization workflow and forces users away from active playback UI.
- **Consequences**:
  - *Positive*: Instant lyric correction; precision timing alignment; open export to standard `.lrc` sidecars.
  - *Reversibility*: The Lyrics Manager interacts with `sonorad` via normalized IPC messages and the universal `LyricsDocument` AST.

---

### ADR-016: Deterministic Explainable Metadata Matching & Scoring Model

- **Status**: `ACCEPTED` (Reversible Scoring Metric Layer)
- **Context**: Automated metadata matchers using fuzzy algorithms often fail on minor title variations (e.g. `"Title (Remastered)"` vs `"Title"`) or silently mis-tag user tracks using low-confidence probabilistic guesses.
- **Decision**: Implement a **100% deterministic, explainable matching engine** in `sonorad`. Normalize text (diacritics removal, Unicode NFKD, string cleanup, feature artist extraction, version noise removal) and calculate composite weighted confidence scores $S \in [0.0, 1.0]$ based on Jaro-Winkler title similarity, token-set artist similarity, album similarity, track position, and Gaussian duration penalty decay. Enforce strict confidence action tiers:
  - *High Confidence ($S \ge 0.90$)*: Safe suggestion / optional 1-click batch match.
  - *Medium Confidence ($0.65 \le S < 0.89$)*: Requires user confirmation via side-by-side diff inspector.
  - *Low Confidence ($S < 0.65$)*: Displays alternatives; requires manual selection.
  - *Exact MBID / ISRC Shortcuts*: Instantly score $1.00$.
- **Alternatives Considered**:
  - *Black-Box AI / LLM Matching*: Nondeterministic, expensive, requires external cloud APIs, and offers zero mathematical score transparency.
  - *Binary Exact-String Matching*: Rejects valid matches due to minor formatting discrepancies.
- **Consequences**:
  - *Positive*: High match accuracy; zero silent tag corruption; 100% mathematical score breakdown transparency in UI.
  - *Reversibility*: Metric weights ($w_i$) and string distance algorithms (Jaro-Winkler, Levenshtein) can be tuned independently of IPC contracts.

---

### ADR-017: Integrated Contextual Discovery & Universal Search Model

- **Status**: `ACCEPTED` (Reversible Presentation & Navigation Model)
- **Context**: Adding online search and enrichment to desktop media players frequently leads to UI fragmentation—either creating an unnecessary "Discover" sidebar section that duplicates local library views or forcing users into separate plugin dialogs.
- **Decision**: Integrate online discovery directly into Sonora's existing universal search overlay (`Cmd/Ctrl + K` or `/`) and item context menus (`[Find Metadata]`, `[Find Artwork]`, `[Find Lyrics]`). Global Search presents tabbed scopes (*Local Library*, *Online Metadata*, *Lyrics*), returning sub-5ms local FTS5 results immediately while streaming debounced online candidates.
- **Alternatives Considered**:
  - *Adding a Top-Level "Discover" Sidebar Section*: Creates visual clutter and fragments local vs online browsing modes.
  - *Modal Dialog per Feature*: Isolates metadata, lyrics, and artwork into disparate, non-standardized popups.
- **Consequences**:
  - *Positive*: Streamlined, low-friction navigation; zero sidebar bloat; cohesive contextual actions across all views.
  - *Reversibility*: Contextual actions communicate via standard frontend state stores and IPC event handlers.
