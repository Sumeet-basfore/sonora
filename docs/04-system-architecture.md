# Sonora: System Architecture & Technical Specification

## 1. High-Level Architecture Topology

Sonora is engineered as a decoupled, multi-process client-daemon system. The platform separates audio decoding, DSP, and library database management from frontend graphical rendering and terminal interaction.

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                   SONORA SYSTEM TOPOLOGY                                │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                               FRONTEND PRESENTATION LAYER                               │
│                                                                                         │
│   ┌───────────────────────────────┐                  ┌──────────────────────────────┐   │
│   │     Sonora Desktop (GUI)      │                  │      Sonora TUI Client       │   │
│   │  • GPU Canvas (WGPU / Shaders)│                  │  • Ratatui Terminal Shell    │   │
│   │  • Kinetic Lyrics Typography  │                  │  • Vim Key Engine            │   │
│   │  • Modular Docking Workspaces │                  │  • ASCII/Braille Spectrum    │   │
│   │  • Dynamic Palette Manager    │                  │  • Kitty/Sixel Art Renderer  │   │
│   └──────────────┬────────────────┘                  └──────────────┬───────────────┘   │
│                  │                                                  │                   │
│                  │           ┌──────────────────────────────┐       │                   │
│                  │           │      Sonora CLI Utility      │       │                   │
│                  │           │  • Scriptable Shell Commands │       │                   │
│                  │           └──────────────┬───────────────┘       │                   │
│                  │                          │                       │                   │
├──────────────────┼──────────────────────────┼───────────────────────┼───────────────────┤
│                  │ IPC / RPC                │ IPC / RPC             │ IPC / RPC         │
│                  ▼                          ▼                       ▼                   │
│         Unix Domain Socket (`/run/user/$UID/sonorad.sock`) / Windows Named Pipe         │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                  HEADLESS DAEMON (`sonorad`)                            │
│                                                                                         │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐  │
│  │ IPC Server & Event Bus (Protobuf / JSON-RPC + Shared Memory Visualizer Ring)    │  │
│  └──────────────────────────────────┬───────────────────────────────────────────────┘  │
│                                     │                                                   │
│       ┌─────────────────────────────┼──────────────────────────────┐                    │
│       ▼                             ▼                              ▼                    │
│  ┌─────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────────┐  │
│  │   Playback & Queue      │  │  Library & Search Core  │  │   Plugin Host Sandbox   │  │
│  │ • State Machine         │  │ • SQLite WAL + FTS5     │  │ • WASM / QuickJS VM     │  │
│  │ • Gapless Pipeline      │  │ • Tag Extraction Engine │  │ • Permission Gatekeeper │  │
│  │ • Crossfade Controller  │  │ • File System Watcher   │  │ • Metadata/Lyrics Hook  │  │
│  └───────────┬─────────────┘  └─────────────────────────┘  └─────────────────────────┘  │
│              │ (Lockless Audio Ring)                                                    │
│              ▼                                                                          │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐  │
│  │ Audio Engine (Real-Time Thread: SCHED_FIFO / MMCSS)                              │  │
│  │                                                                                  │  │
│  │ ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────────┐   │  │
│  │ │ File Decoder │──►│ Preamp & R128│──►│ 10-Band PEQ  │──►│  Master Limiter  │   │  │
│  │ └──────────────┘   └──────────────┘   └──────────────┘   └─────────┬────────┘   │  │
│  │                                                                    │            │  │
│  │                                                                    ▼            │  │
│  │                                   Lockless Audio Tap ──► Output Driver (WASAPI/ │  │
│  │                                  (Shared Memory Ring)   PipeWire/CoreAudio)      │  │
│  └──────────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Application Layer Breakdown

### 2.1 Layer 1: Presentation & Frontends
- **Desktop GUI (`sonora`)**: Native cross-platform application utilizing hardware-accelerated 2D/3D graphics (WGPU / Metal / Vulkan / DirectX). Renders the layout grid, dynamic theme canvas, syllable-accurate lyrics, projectM shaders, and interactive equalizers.
- **Terminal Client (`sonora-tui`)**: Terminal interface built on raw terminal primitives (`ratatui` / `crossterm`). Provides keyboard-driven navigation, Unicode/Braille spectrum meters, and inline terminal graphics.
- **Command-Line Interface (`sonora-cli`)**: Lightweight binary that sends instantaneous single-shot commands (e.g. `sonora-cli volume +5`) and exits immediately.

### 2.2 Layer 2: Inter-Process Communication (IPC) Gateway
- Manages client authentication, state synchronization, RPC command execution, and real-time event broadcasting over local Unix Domain Sockets or Windows Named Pipes.
- Houses a zero-copy shared memory circular buffer (`SonoraSharedRing`) for broadcasting raw PCM audio samples and FFT frequency bins to visualizers at 60/120 FPS.

### 2.3 Layer 3: Headless Core Daemon (`sonorad`)
- **Playback Controller**: Orchestrates playback queues, track transitions, repeat/shuffle algorithms, and gapless buffer preloading.
- **Library Engine**: Manages SQLite storage in WAL mode, coordinates non-blocking background folder watching, and handles embedded tag parsing across multiple audio formats.
- **Lyrics Resolver**: Executes cascading resolution pipelines (embedded tags -> local sidecars -> remote APIs -> plugin providers).
- **Plugin Sandbox**: Instantiates and supervises isolated WASM / QuickJS runtimes for third-party extensions under strict capability security constraints.

### 2.4 Layer 4: Real-Time Audio Engine
- Dedicated, isolated high-priority thread running deterministic sample-by-sample audio processing.
- Zero memory allocation, zero locks, and zero disk/network I/O inside the active audio render loop.

---

## 3. Audio Engine Boundary & Real-Time Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           REAL-TIME AUDIO PIPELINE                          │
│                                                                             │
│  Track N File ──► [Decoder Ring Buffer]                                     │
│                          │ (f32 Interleaved PCM)                            │
│                          ▼                                                  │
│  Track N+1 ─────► [Pre-Decode Buffer] (Gapless Transition Stage)            │
│                          │                                                  │
│                          ▼                                                  │
│                  [Resampler Node] (44.1/48/96/192 kHz match output)         │
│                          │                                                  │
│                          ▼                                                  │
│                  [ReplayGain / EBU R128 Pre-Amp Node]                       │
│                          │                                                  │
│                          ▼                                                  │
│                  [10-Band Parametric Equalizer Cascade]                     │
│                  (Direct Form II Transposed Biquad Filters)                 │
│                          │                                                  │
│                          ▼                                                  │
│                  [Optional DSP Node / Convolution IR]                       │
│                          │                                                  │
│                          ▼                                                  │
│                  [True-Peak Master Limiter]                                 │
│                          │                                                  │
│            ┌─────────────┴────────────────────────┐                         │
│            ▼                                      ▼                         │
│  [Hardware Output Ring]               [Lockless Visual Tap Ring]            │
│  (WASAPI / PipeWire / CoreAudio)      (Downsampled 2048-Sample Window)      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 Real-Time Thread Constraints
To guarantee zero-glitch playback, the audio render thread strictly complies with the standard rules of real-time audio programming:
1. **NO Memory Allocations/Deallocations**: No calls to `malloc`, `free`, or dynamic heap reallocations. All audio frame buffers are pre-allocated during pipeline initialization.
2. **NO Blocking Locks / Mutexes**: Thread communication between the daemon controller and the audio loop uses Single-Producer Single-Consumer (SPSC) lock-free ring buffers (`rtrb` / atomic read-write pointers).
3. **NO File System or Network I/O**: Decoding runs on a separate worker thread that fills the lockless decoder ring buffer ahead of time.
4. **Thread Priority**: Elevated to real-time priority at launch (`SCHED_FIFO` with priority 85+ on Linux; `THREAD_PRIORITY_TIME_CRITICAL` / MMCSS `Pro Audio` on Windows; `THREAD_TIME_CONSTRAINT_POLICY` on macOS).

### 3.2 Biquad Filter Mathematics & Equalizer Cascade
The 10-band parametric equalizer is implemented as a series cascade of Direct Form II Transposed biquad filter sections. For each band $k$:

$$y[n] = b_0 x[n] + b_1 x[n-1] + b_2 x[n-2] - a_1 y[n-1] - a_2 y[n-2]$$

Filter coefficients ($b_0, b_1, b_2, a_1, a_2$) are recalculated on the control thread using Robert Bristow-Johnson's Audio EQ Cookbook formulas with smooth parameter interpolation to prevent zipper noise during user interaction.

### 3.3 Audio Output Backends
- **Linux**: Primary backend is native **PipeWire** (via `libpipewire-0.3`) for low-latency routing, with fallback to **ALSA** (Direct Hardware/Dmix) and **PulseAudio**.
- **macOS**: Native **CoreAudio** (`AudioUnit` / `AudioQueue`) with bit-perfect integer stream support.
- **Windows**: Native **WASAPI** supporting both Exclusive mode (bit-perfect, bypasses Windows audio mixer) and Shared mode.

---

## 4. Inter-Process Communication (IPC) Protocol

### 4.1 Transport Layer
- **POSIX Systems (Linux, macOS)**: Unix Domain Socket located at `$XDG_RUNTIME_DIR/sonora/sonorad.sock` (mode `0600`).
- **Windows**: Named Pipe located at `\\.\pipe\sonora-ipc-$USERNAME`.

### 4.2 Protocol Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          IPC PROTOCOL MESSAGE FORMAT                        │
├──────────────┬──────────────┬──────────────┬────────────────────────────────┤
│ Field        │ Size         │ Type         │ Description                    │
├──────────────┼──────────────┼──────────────┼────────────────────────────────┤
│ Magic Header │ 4 Bytes      │ `u32` (BE)   │ `0x534F4E4F` ("SONO")          │
│ Message Type │ 2 Bytes      │ `u16` (BE)   │ 1=RPC Req, 2=RPC Resp, 3=Event │
│ Sequence ID  │ 4 Bytes      │ `u32` (BE)   │ Request-Response matching ID   │
│ Payload Len  │ 4 Bytes      │ `u32` (BE)   │ Length of body payload in bytes│
│ Payload Body │ Variable     │ Protobuf/JSON│ Serialized RPC / Event payload │
└──────────────┴──────────────┴──────────────┴────────────────────────────────┘
```

#### Core IPC RPC Methods
- `Playback.Play()`, `Playback.Pause()`, `Playback.Seek(position_ms)`
- `Playback.SetVolume(volume_f32)`, `Playback.SetEQBand(index, gain_db, freq, q)`
- `Queue.Get()`, `Queue.Insert(index, track_ids[])`, `Queue.Remove(index)`
- `Library.Search(query, limit, offset) -> TrackResult[]`
- `Lyrics.Resolve(track_id) -> LyricPayload`

#### Core Broadcast Events
- `Event.TrackChanged(track_id, metadata, duration_ms)`
- `Event.PlaybackState(state: Playing|Paused|Stopped, position_ms, clock_epoch)`
- `Event.VolumeChanged(volume_f32, is_muted)`
- `Event.LibraryRescanProgress(indexed_count, total_count)`

### 4.3 Visualizer Shared Memory Tap (`SonoraSharedRing`)
To eliminate IPC serialization overhead for 60/120 FPS visualizers, `sonorad` provisions a shared memory region (`shm_open` on POSIX / `CreateFileMapping` on Windows):
```c
struct SonoraVisualizerSharedBuffer {
    uint32_t sample_rate;       // e.g. 48000
    uint32_t channels;          // 2
    uint64_t frame_index;       // Monotonically increasing atomic sequence
    float pcm_stereo_l[2048];   // Most recent 2048 samples (Left channel)
    float pcm_stereo_r[2048];   // Most recent 2048 samples (Right channel)
    float fft_bins[256];        // Pre-computed logarithmic magnitude bands
};
```
Frontend visualizers attach read-only to this shared buffer and poll at native display refresh rates without generating IPC round-trips.

---

## 5. Extensibility & Sandboxing Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           PLUGIN EXECUTION BOUNDARY                         │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                         Headless Core Host                          │   │
│   │                                                                     │   │
│   │   ┌───────────────────────┐            ┌────────────────────────┐   │   │
│   │   │ Capability Controller │            │   Plugin Manager       │   │   │
│   │   │ (Security & ACLs)     │            │   (Lifecycle & Config) │   │   │
│   │   └───────────┬───────────┘            └───────────┬────────────┘   │   │
│   │               │                                    │                │   │
│   │               ▼                                    ▼                │   │
│   │   ┌─────────────────────────────────────────────────────────────┐   │   │
│   │   │                 Sandboxed Isolation Runtime                 │   │   │
│   │   │  (Wasmtime WebAssembly Engine / QuickJS Isolated Context)    │   │   │
│   │   │                                                             │   │   │
│   │   │   ┌───────────────────────┐     ┌───────────────────────┐   │   │   │
│   │   │   │ Lyrics Fetcher Plugin │     │ Metadata Source Plugin│   │   │   │
│   │   │   │ (Restricted Fetch API)│     │ (Read-Only DB Hook)   │   │   │   │
│   │   │   └───────────────────────┘     └───────────────────────┘   │   │   │
│   │   └─────────────────────────────────────────────────────────────┘   │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **WASM / QuickJS Isolation**: Plugins execute in isolated memory heaps. A memory fault or uncaught exception inside a plugin terminates only that plugin's instance, triggering host-level crash recovery without affecting audio or UI.
2. **Capability-Based Host APIs**: Plugins receive strictly scoped capabilities (e.g. `sonora:fetch` allows HTTP requests only to explicit whitelist domains declared in `manifest.json`).

---

## 6. Major Technical Risks & Mitigation Strategies

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SYSTEM TECHNICAL RISK MATRIX                       │
├────────────────────────────┬──────────┬─────────┬───────────────────────────┤
│ Risk Description           │ Severity │ Chance  │ Architectural Mitigation  │
├────────────────────────────┼──────────┼─────────┼───────────────────────────┤
│ 1. Real-Time Audio Dropouts│ CRITICAL │ Medium  │ Complete lockless thread  │
│    from Memory Allocation  │          │         │ isolation; pre-allocated  │
│    or System Contention    │          │         │ audio frame ring buffers. │
├────────────────────────────┼──────────┼─────────┼───────────────────────────┤
│ 2. IPC Bus Saturation from │ HIGH     │ Medium  │ Shared-memory lockless    │
│    High-Rate Visualizer Tap│          │         │ buffer for FFT/PCM audio; │
│                            │          │         │ event rate-limiting.      │
├────────────────────────────┼──────────┼─────────┼───────────────────────────┤
│ 3. Plugin Malice / Memory  │ HIGH     │ High    │ Strict WASM sandboxing +  │
│    Leak Exhaustion         │          │         │ declarative capability-   │
│                            │          │         │ based security model.     │
├────────────────────────────┼──────────┼─────────┼───────────────────────────┤
│ 4. Large-Library Search    │ MEDIUM   │ Low     │ SQLite WAL + FTS5 indices │
│    Degradation (100k+ file)│          │         │ executed strictly on async│
│                            │          │         │ background worker pools.  │
├────────────────────────────┼──────────┼─────────┼───────────────────────────┤
│ 5. Cross-Platform Audio    │ HIGH     │ Medium  │ Clean abstraction layer   │
│    Device Inconsistencies  │          │         │ isolating WASAPI, PipeWire│
│                            │          │         │ CoreAudio drivers.        │
└────────────────────────────┴──────────┴─────────┴───────────────────────────┘
```

---

## 7. Explicit Reversible Architectural Decisions

To prevent premature lock-in while maintaining velocity, the following architectural choices are explicitly encapsulated behind abstract interfaces and remain reversible:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        REVERSIBLE ARCHITECTURAL DECISIONS                   │
├────────────────────────────┬───────────────────────┬────────────────────────┤
│ Decision Area              │ Current Baseline      │ Reversal / Shift Path  │
├────────────────────────────┼───────────────────────┼────────────────────────┤
│ IPC Message Serialization  │ Binary Protobuf over  │ JSON-RPC 2.0 or Cap'n  │
│                            │ Framed Stream         │ Proto via codec swap   │
├────────────────────────────┼───────────────────────┼────────────────────────┤
│ Plugin Sandbox Engine      │ Dual WASM (Wasmtime)  │ Pure WASM or isolated  │
│                            │ + QuickJS runtime     │ V8/Deno sub-processes  │
├────────────────────────────┼───────────────────────┼────────────────────────┤
│ Library Search Indexer     │ SQLite FTS5 extension │ Tantivy (Rust Lucene)  │
│                            │                       │ behind SearchProvider  │
├────────────────────────────┼───────────────────────┼────────────────────────┤
│ Audio Frame Precision      │ 32-bit floating point │ 64-bit float internal  │
│                            │ (`f32`) pipeline      │ via type alias wrapper │
├────────────────────────────┼───────────────────────┼────────────────────────┤
│ GUI Rendering Backend      │ WGPU Hardware Canvas  │ Skia / Slint backend   │
│                            │ with custom shaders   │ behind RenderSurface   │
└────────────────────────────┴───────────────────────┴────────────────────────┤
│ IRREVERSIBLE CORE COMMITMENTS (Non-Negotiable)                              │
│ • Client-Daemon Multi-Process Architecture (Headless daemon + IPC)          │
│ • Dedicated Lockless Real-Time Audio Thread (No allocation in audio loop)   │
│ • Capability-Based Plugin Security (Zero unsandboxed native pointers)       │
└─────────────────────────────────────────────────────────────────────────────┘
```
