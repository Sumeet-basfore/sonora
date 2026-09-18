# Sonora v0.1.0

Local-first desktop music player: a Rust audio engine driving a Tauri desktop
GUI, a terminal UI, and a scriptable CLI. Audiophile playback, synchronized
lyrics, dynamic theming, sandboxed WASM plugins, and a Git-backed extension
marketplace — no accounts, no telemetry, no streaming lock-in.

> **Platform status (v0.1.0):** Linux is the primary, tested platform.
> macOS and Windows build from the same codebase (portable Rust + cpal +
> Tauri) and use native data directories, but receive only best-effort
> testing in this release. See [docs/11-development-roadmap.md](docs/11-development-roadmap.md).

## Quick start

Prerequisites: Rust stable toolchain, Node.js 20. On Linux you also need
system libraries for audio and the desktop shell:

```sh
sudo apt-get install -y libasound2-dev libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev libsoup-3.0-dev
```

```sh
git clone https://github.com/sonora-audio/sonora
cd sonora

# Rust workspace (engine, library, lyrics, plugins, registry, CLI, TUI)
cargo build --workspace

# Desktop frontend
npm --prefix apps/desktop install
npm --prefix apps/desktop run build

# Run it (pick one)
cargo run -p sonora-cli -- status     # scriptable CLI
cargo run -p sonora-tui               # terminal UI
cargo run -p sonora-desktop           # Tauri backend (GUI via `npm --prefix apps/desktop run dev`)
```

First run: open **Settings → Scan**, point Sonora at a music folder, and
press play. The library rescans incrementally (unchanged files are skipped by
mtime/size), and files you delete are pruned from the index on the next scan.

## Supported formats

FLAC, ALAC, WAV, AIFF, MP3, AAC/M4A, Ogg Vorbis, Opus — decoded with
Symphonia, tagged with Lofty. Corrupt or unreadable files are counted in the
scan report and skipped; they never abort a scan.

## Features

- **Playback** — gapless queue, seek, volume, ReplayGain-aware pipeline with
  10-band parametric EQ, visualizer tap (spectrum/wave/mirror).
- **Library** — SQLite + FTS5 search (fuzzy, sub-millisecond on large
  libraries), multi-folder scans, m3u playlists, play history.
- **Lyrics** — embedded tags → sidecar `.lrc` → SQLite cache → LRCLIB, plus
  sandboxed provider plugins (LRCLIB and Genius ship in `plugins/`).
- **Themes** — built-in presets plus marketplace themes (full token
  definitions + optional guarded CSS), live preview, persisted selection.
- **Plugins** — WASM-only sandbox (fuel-metered, 32 MiB memory cap,
  capability-gated host APIs: library/metadata/lyrics/visualizer/UI/cache/
  allow-listed network). A crashing plugin can never take down playback.
- **Marketplace** — Git-backed `community-registry/` (`plugins.json` /
  `themes.json`): checksum-verified atomic installs, versioned backups,
  rollback, uninstall, offline cache. No accounts, no payments, no signing
  yet (SHA-256 integrity only in v1).

## Plugin development

Each plugin is a standalone crate (not a workspace member) built to
`wasm32-unknown-unknown`:

```sh
rustup target add wasm32-unknown-unknown
cd plugins/spectrum-plus && ./build.sh   # -> plugin.wasm
cargo test                               # pure-logic unit tests (host-side)
```

Sandbox integration tests live in `crates/sonora-plugin/tests/` and run with
the workspace suite. Start from `plugins/example/` (minimal WAT module) or
copy `plugins/lyrics-lrclib/` (networked provider). Rules that matter:

- `manifest.json` (`id`, semver `version`, `api_version: 1`, declared
  capabilities, bare-`.wasm` `entry`, `allowed_domains` iff `network:fetch`).
- Import only `sonora.*` host functions your capabilities entitle you to —
  anything else (including WASI) fails the load.
- Fuel budget is shared per call: keep parsing linear; megabyte-scale HTML
  scanning is fine, infinite loops trap.
- Query `lyrics_fetch`/`metadata_fetch` return `1` + result JSON or `0` for
  miss; `visualizer_info` returns a descriptor + computed frame (rendering
  stays client-side).

Docs: [docs/05-plugin-system.md](docs/05-plugin-system.md),
[docs/09-security-and-permissions.md](docs/09-security-and-permissions.md).

## Theme development

A theme package is `theme.json` (full definition: `id`, `name`, semver
`version`, `author`, `mode`, all required design tokens) plus optional
`theme.css` for texture only. Token values must be plain colors; CSS must not
contain `url(`, `@import`, scripts, or remote fetches — both are enforced at
install and at serve time. See `themes/sonora-retro/` and
[docs/06-theming-and-customization.md](docs/06-theming-and-customization.md).
Validate with `cargo test -p sonora-registry theme`.

## Marketplace

```sh
python3 community-registry/tools/build.py   # deterministic zips + indexes
```

Submissions today are manual PRs adding sources + entries to `tools/build.py`;
the client validates schema, digests, identity, and capability ceilings, and
treats all registry metadata as untrusted. Details:
[docs/08-marketplace.md](docs/08-marketplace.md).

## Troubleshooting

| Symptom | Fix |
|---|---|
| No audio device | Sonora falls back to a virtual output and logs a warning; check PipeWire/ALSA. |
| Scan finds 0 files | Only the formats above are indexed; check the folder path and permissions. |
| Ghost entries after moving files | Rescan — missing files are pruned automatically (`pruned_missing` in the report). |
| A plugin crashes | It is isolated and disabled after 3 crashes/60 s; others keep running. Reinstall or update it from Extensions. |
| Bad update | Roll back from Extensions → Installed → Roll back (backups kept per version). |
| Offline marketplace | Last validated index is served and flagged offline in the UI. |
| Theme looks broken after uninstall | Selection resets to the built-in default automatically. |
| Slow first import | ~1 ms/file single-threaded (≈11 s per 10k tracks); rescans skip unchanged files in milliseconds. |

## Contributing

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `npm --prefix apps/desktop test` must all pass
  (enforced by CI, including deterministic registry rebuilds).
- Rust: no `unwrap`/`expect` outside tests; errors via `SonoraError`.
- Frontend: escape all untrusted strings with `escapeHtml`; keep
  `aria-label`s on controls; respect `prefers-reduced-motion`.
- Security-sensitive areas (sandbox boundary, installer, theme validation):
  fail closed, add a regression test per fix.

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE). Product direction in
[docs/01-product-vision.md](docs/01-product-vision.md).
