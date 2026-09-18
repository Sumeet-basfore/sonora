# Sonora: Security, Permissions & Sandboxing Architecture

## 1. Threat Model & Security Philosophy

Sonora treats local user files, network privacy, and audio stability as sacred. Because Sonora supports third-party plugins, themes, and community visualizer presets, the architecture assumes an **untrusted extension model**: no third-party code is ever given raw, unmediated access to operating system APIs or memory pointers.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             SONORA THREAT MATRIX                            │
├────────────────────────────┬────────────────────────────────────────────────┤
│ Threat Vector              │ Architectural Mitigation                       │
├────────────────────────────┼────────────────────────────────────────────────┤
│ 1. Malicious Plugin Data   │ Network capability whitelisting; host-gated    │
│    Exfiltration            │ HTTP client; zero raw socket access.           │
├────────────────────────────┼────────────────────────────────────────────────┤
│ 2. Host Crashes from Bad   │ WebAssembly / QuickJS memory sandboxing;       │
│    Native Pointers         │ process isolation; timeout watchdogs.          │
├────────────────────────────┼────────────────────────────────────────────────┤
│ 3. Image Decompression     │ Memory-safe Rust decoders with strict memory   │
│    Bombs (Album Artwork)   │ allocation limits and maximum dimensions.      │
├────────────────────────────┼────────────────────────────────────────────────┤
│ 4. IPC Hijacking / Local   │ Unix Domain Socket `0600` permissions and peer │
│    Privilege Escalation    │ UID validation (`SO_PEERCRED` / Windows SID).  │
├────────────────────────────┼────────────────────────────────────────────────┤
│ 5. GPU Shader Locking /    │ Shader validation via Naga/WGPU and watchdog   │
│    Denial-of-Service       │ GPU frame timers to abort runaway loops.       │
└────────────────────────────┴────────────────────────────────────────────────┘
```

---

## 2. Capability-Based Plugin Permission Model

Plugins must declare their required capabilities inside `manifest.json`. Permissions are enforced by the host runtime at call time:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PERMISSION CAPABILITY MATRIX                        │
├────────────────────┬────────────────────┬───────────────────────────────────┤
│ Permission Scope   │ Parameters         │ Security Enforcement Action       │
├────────────────────┼────────────────────┼───────────────────────────────────┤
│ `network:fetch`    │ `domains: string[]`│ Requests to unlisted domains or   │
│                    │                    │ raw IPs are rejected at host gate.│
├────────────────────┼────────────────────┼───────────────────────────────────┤
│ `library:read`     │ None               │ Exposes sanitized read-only views;│
│                    │                    │ hides physical filesystem paths.  │
├────────────────────┼────────────────────┼───────────────────────────────────┤
│ `audio:tap`        │ `readOnly: true`   │ Grants access to visualizer buffer│
│                    │                    │ tap; zero audio injection rights. │
├────────────────────┼────────────────────┼───────────────────────────────────┤
│ `storage:cache`    │ `maxBytes: number` │ Scoped key-value store with hard  │
│                    │                    │ disk quota (default 10MB).        │
├────────────────────┼────────────────────┼───────────────────────────────────┤
│ `filesystem:export`│ None               │ Prompts native OS file save dialog│
│                    │                    │ without direct folder access.     │
└────────────────────┴────────────────────┴───────────────────────────────────┘
```

### 2.1 Permission Prompt User Experience
When installing or upgrading a plugin requiring capabilities, Sonora presents an explicit consent dialogue:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PLUGIN PERMISSION APPROVAL                         │
│                                                                             │
│  Install "Last.fm Scrobbler & Metadata Provider" (v1.4.0)?                  │
│                                                                             │
│  This plugin requests the following permissions:                            │
│  • [Network] Connect to: https://ws.audioscrobbler.com                      │
│  • [Library] Read track metadata and playback history                       │
│  • [Storage] Store up to 5 MB of cached album info                          │
│                                                                             │
│                   [ View Source Code ]   [ Deny ]   [ Approve & Install ]   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Sandbox Isolation Enforcement

### 3.1 WebAssembly Runtime Isolation (Wasmtime)
- **Linear Memory Sandboxing**: WASM code operates inside a fixed, bounded memory arena. It cannot access host memory or dereference raw operating system pointers.
- **Instruction Metering (Fuel Consumption)**: Infinite loops or compute-heavy plugins are bounded by instruction fuel limits, terminating frozen plugins before they stall CPU cores.
- **WASI Sandboxing**: Only pre-approved standard I/O handles are mapped into the WebAssembly environment.

### 3.2 QuickJS Scripting Isolation
- Executed inside a dedicated, isolated QuickJS runtime context per plugin.
- Dangerous globals (`eval()`, `Function()`, `import()`, `SharedArrayBuffer`) are completely excised from the global object.
- Garbage collection and heap allocations are capped at 32MB per plugin instance.

---

## 4. IPC Socket Security & Peer Validation

To prevent unauthorized local processes from hijacking the audio daemon:

```
POSIX Unix Domain Socket
  ├── File Permissions: Mode 0600 ($XDG_RUNTIME_DIR/sonora/sonorad.sock)
  └── Peer Credential Check:
      ├── Linux: SO_PEERCRED validates connecting PID, UID, GID == current user
      └── macOS: LOCAL_PEERCRED validates connecting EUID == current user

Windows Named Pipe
  ├── Pipe Name: \\.\pipe\sonora-ipc-%USERNAME%
  └── Security Descriptor: Restricted DACL (Discretionary Access Control List)
      allowing access ONLY to the current user's Security Identifier (SID).
```

---

## 5. Untrusted Media & Metadata Sanitization

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      MEDIA ASSET SANITIZATION PIPELINE                      │
│                                                                             │
│  Raw Cover Art File ──► [Memory Allocation Guard] (Max 64MB buffer)         │
│                               │                                             │
│                               ▼                                             │
│  [Dimension Guard] ────► Reject images > 8192 x 8192 (Zip-Bomb Defense)     │
│                               │                                             │
│                               ▼                                             │
│  [Pure-Rust Decoder] ──► Decodes via safe image-rs (PNG/JPEG/WebP)          │
│                               │                                             │
│                               ▼                                             │
│  [Downsampled Texture] ─► Composited into GPU surface                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Metadata Text Sanitization**: All embedded ID3/Vorbis strings undergo UTF-8 normalization, control character stripping (`\0`, ANSI escape codes), and HTML entity escaping before rendering in GUI/TUI viewports.
- **Database Safety**: All SQLite queries across the platform use parameterized bindings (`?1`, `:param`), completely eliminating SQL injection vectors.
- **Shader Validation**: Custom fragment shaders are pre-compiled and validated via the `naga` WGSL/GLSL shader validator to guarantee memory safety and guard against GPU driver crashes.

---

## 6. Reversibility & Security Boundaries

> [!IMPORTANT]
> ### Reversible Architecture Decisions
> - **Domain Whitelisting Granularity**: Can support wildcards (`*.last.fm`) or strict exact URLs depending on user security preference.
> - **WASM Fuel Limits**: Execution instruction thresholds are configurable per machine profile in `settings.json`.
>
> ### Invariable Boundaries
> - **Zero-Trust for Plugins**: No unsandboxed native binaries or shared libraries (`.so`, `.dll`, `.dylib`) will ever be loaded dynamically into the core daemon process.
