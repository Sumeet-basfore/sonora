# Sonora: Plugin Architecture & Extensibility System

## 1. Plugin Architecture Overview & Philosophy

Sonora's extension architecture allows third-party developers to extend every major subsystem—metadata fetchers, lyrics providers, visualizer renderers, UI widgets, and DSP nodes—without endangering the host player's stability, audio playback sanctity, or user privacy.

### 1.1 Core Principles
1. **Crash-Resilient Isolation**: A crashing, hanging, or panicking plugin cannot terminate `sonorad` or crash the GUI shell.
2. **Capability-Gated Sandboxing**: Plugins cannot access arbitrary filesystem paths, execute shell commands, or open unbounded network sockets without explicit manifest declarations and user consent.
3. **Deterministic Lifecycle**: Plugins support seamless hot-reloading, deterministic initialization, clean state teardown, and live configuration updates.
4. **Ergonomic Multi-Language Authoring**: First-class support for both high-level scripting (JavaScript/TypeScript compiled via QuickJS) and high-performance compiled logic (Rust/C/AssemblyScript targeting WebAssembly).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SONORA PLUGIN RUNTIME TOPOLOGY                     │
│                                                                             │
│                        ┌────────────────────────────┐                       │
│                        │    Host Core Supervisor    │                       │
│                        │  • Lifecycle Controller    │                       │
│                        │  • Capability Security Gate│                       │
│                        │  • IPC Event Dispatcher    │                       │
│                        └─────────────┬──────────────┘                       │
│                                      │                                      │
│                ┌─────────────────────┴─────────────────────┐                │
│                ▼                                           ▼                │
│   ┌───────────────────────────┐               ┌───────────────────────────┐ │
│   │    WASM Sandbox Engine    │               │  QuickJS Isolated Context │ │
│   │     (High-Perf Logic)     │               │    (Scripting / Fetch)    │ │
│   ├───────────────────────────┤               ├───────────────────────────┤ │
│   │ • Memory Sandboxed Heap   │               │ • Isolated JS Runtime     │ │
│   │ • Strict Syscall Filtering│               │ • Scoped Fetch & DB APIs  │ │
│   │ • Audio DSP & Visualizers │               │ • Lyrics & Metadata Hooks │ │
│   └───────────────────────────┘               └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Plugin Types & Extension Points

Sonora establishes five distinct extension points with tailored host interfaces:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SONORA EXTENSION INTERFACES                        │
├─────────────────────┬───────────────────┬───────────────────────────────────┤
│ Extension Type      │ Host Boundary     │ Primary Capabilities              │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ 1. Metadata Source  │ Async Data Hook   │ Resolves artist bios, discography,│
│                     │                   │ album art, and extended tags.     │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ 2. Lyrics Provider  │ Async Query Hook  │ Fetches plain, synced LRC, and    │
│                     │                   │ syllable-level TTML lyrics.       │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ 3. Visualizer Node  │ Shared Frame Tap  │ Renders custom 2D/3D visualizers, │
│                     │                   │ GLSL/WGSL shaders, or TUI bars.   │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ 4. UI Widget/Panel  │ Declarative Slot  │ Mounts custom dockable panels,    │
│                     │                   │ status bar pills, or HUD overlays.│
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ 5. DSP Processor    │ Control Plane     │ Implements biquad filter graphs,  │
│                     │ (Constrained Host)│ crossfeed, or custom audio effects│
└─────────────────────┴───────────────────┴───────────────────────────────────┘
```

### 2.1 Metadata & Lyrics Provider Extension API
Metadata and lyrics providers operate via non-blocking asynchronous request-response pipelines.

```typescript
// Sonora Host SDK: Lyrics Provider Interface
export interface LyricsQuery {
  title: string;
  artist: string;
  album?: string;
  durationMs: number;
  fingerprint?: string;
}

export interface LyricLine {
  startTimeMs: number;
  endTimeMs?: number;
  text: string;
  syllables?: Array<{
    startTimeMs: number;
    durationMs: number;
    text: string;
  }>;
}

export interface LyricsProvider {
  id: string;
  name: string;
  fetchLyrics(query: LyricsQuery): Promise<{
    format: 'plain' | 'lrc' | 'enhanced_lrc' | 'ttml';
    lyrics: string | LyricLine[];
    attribution?: string;
  } | null>;
}
```

### 2.2 Visualizer Extension API
Visualizer extensions run client-side on the presentation layer, receiving structured audio buffer feeds from the shared memory tap:

```typescript
// Sonora Host SDK: Visualizer Plugin Interface
export interface VisualizerContext {
  canvas: HTMLCanvasElement | OffscreenCanvas;
  sampleRate: number;
  colorPalette: {
    dominant: string;
    vibrant: string;
    muted: string;
    background: string;
  };
}

export interface VisualizerPlugin {
  init(ctx: VisualizerContext): void;
  renderFrame(pcmLeft: Float32Array, pcmRight: Float32Array, fftBins: Float32Array): void;
  destroy(): void;
}
```

---

## 3. Plugin Manifest Specification (`manifest.json`)

Every Sonora extension is packaged with a strictly validated `manifest.json` describing its identity, extension points, capabilities, and required permissions.

```json
{
  "$schema": "https://sonora.audio/schemas/v1/plugin-manifest.json",
  "id": "org.sonora.provider.lrclib",
  "name": "LRCLIB Community Lyrics",
  "version": "1.2.0",
  "description": "High-accuracy line and word-level synchronized lyrics from LRCLIB.",
  "author": {
    "name": "Sonora Team",
    "url": "https://lrclib.net"
  },
  "license": "MIT",
  "minSonoraVersion": "1.0.0",
  "entrypoint": {
    "runtime": "quickjs",
    "script": "dist/index.js"
  },
  "extensionPoints": [
    "lyrics:provider"
  ],
  "permissions": [
    {
      "type": "network:fetch",
      "domains": [
        "https://lrclib.net"
      ]
    },
    {
      "type": "storage:cache",
      "maxSizeBytes": 10485760
    }
  ],
  "settingsSchema": {
    "preferSyllables": {
      "type": "boolean",
      "default": true,
      "title": "Prefer Word/Syllable Synchronized Lyrics"
    }
  }
}
```

---

## 4. Plugin Lifecycle & Crash Recovery

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PLUGIN LIFECYCLE STATE MACHINE                     │
│                                                                             │
│   [ Discovered ] ──► [ Validated ] ──► [ Permission Prompt ]                │
│                                                │                            │
│                                                ▼ (Approved)                 │
│   [ Terminated ] ◄── [ Disabled ]  ◄── [ Instantiated ]                     │
│          ▲                  ▲                  │                            │
│          │ (Error Count >3) │ (User Action)    ▼ (Init Hook)                │
│          └─────────── [ CRASHED ] ◄────── [ ACTIVE / RUNNING ]              │
│                             │                  │                            │
│                             ▼                  ▼                            │
│                      [ Auto-Restart ]     [ Hot-Reload ]                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Lifecycle States
1. **Discovery & Validation**: The plugin manager scans `$CONFIG_DIR/sonora/plugins/`, parses `manifest.json`, checks digital signatures (if marketplace-installed), and validates schema compliance.
2. **Permission Check**: Any undeclared or escalated permissions prompt the user with human-readable security warnings.
3. **Instantiation**: The host instantiates an isolated QuickJS context or WASM sandbox with scoped APIs injected.
4. **Activation**: The host invokes `onActivate()`. Event hooks are registered in the daemon event bus.
5. **Hot-Reloading**: When source files change during development or updates, the host calls `onDeactivate()`, tears down the VM context, rebuilds the sandbox, and invokes `onActivate()` with preserved state in `< 50ms`.

### 4.2 Error Boundary & Crash Recovery
- **Timeout Isolation**: Async plugin calls (e.g. `fetchLyrics`) enforce a strict execution timeout (default 5,000ms).
- **Crash Counter**: If a plugin panics or exceeds memory limits, the host catches the fault, logs the stack trace, and increments an error counter.
- **Exponential Backoff**: The host attempts automatic sandbox restart with backoff (1s -> 5s -> 15s). If 3 consecutive crashes occur within 60 seconds, the plugin is moved to `CRASHED` state and disabled, triggering a non-fatal UI notification with a one-click debug report.

---

## 5. Host API Security Boundaries

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          HOST API CAPABILITY GATEWAYS                       │
├─────────────────────┬───────────────────┬───────────────────────────────────┤
│ API Namespace       │ Sandbox Access    │ Security Enforcement Mechanism    │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ `sonora.net.fetch`  │ Whitelisted URLs  │ Domain filter checked at host;    │
│                     │                   │ blocks unlisted IP addresses.     │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ `sonora.storage`    │ Isolated KV Store │ Sandboxed SQLite sub-tree;        │
│                     │                   │ quota-enforced (10MB max).        │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ `sonora.library`    │ Read-Only Queries │ Parameterized SQL views; blocks   │
│                     │                   │ raw mutations and file paths.     │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ `sonora.audio`      │ Read-Only FFT Tap │ Zero-copy ring buffer tap;        │
│                     │                   │ cannot write to audio output.     │
├─────────────────────┼───────────────────┼───────────────────────────────────┤
│ `sonora.ui.widget`  │ Declarative VDOM  │ Sanitized virtual DOM rendering;  │
│                     │                   │ no direct host DOM injection.     │
└─────────────────────┴───────────────────┴───────────────────────────────────┘
```

---

## 6. Reversibility & Extensibility Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Choices
> - **Scripting Engine**: Using QuickJS for JavaScript plugins allows swapping to Boa (Rust JS engine) or Deno Core if future isolation demands require it.
> - **WASM Runtime**: Abstracted through a generic `PluginRuntime` trait, allowing seamless transition between Wasmtime and Wasmer.
> - **Manifest Schema Format**: Versioned (`v1`) JSON schema allows introducing new extension slots without breaking legacy plugins.
>
> ### Invariable Boundaries
> - **No Direct In-Process C/C++ Pointer Sharing**: No raw native DLL loading in the UI thread or real-time audio thread. All extensions must pass through sandbox boundaries.
