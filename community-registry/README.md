# Sonora community registry (sample)

Git-backed extension registry foundation: static JSON indexes plus release
zips. Clients fetch `v1/plugins.json` / `v1/themes.json` over HTTPS, validate
every field as untrusted input, cache the last good copy for offline use, and
verify each package SHA-256 **before** extraction.

```
community-registry/
├── README.md            # this file
├── tools/build.py       # deterministic packaging + index generator
├── sources/neon-night/  # sample theme sources (theme.json, theme.css)
├── packages/            # release zips (built, deterministic)
│   ├── org.sonora.lrclib-1.0.0.zip
│   ├── org.sonora.lyrics_genius-1.0.0.zip
│   ├── org.sonora.spectrum_plus-1.0.0.zip
│   ├── org.sonora.theme.neon_night-1.0.0.zip
│   └── org.sonora.theme.retro-1.0.0.zip
└── v1/
    ├── plugins.json     # generated (schema, entries, versions, digests)
    └── themes.json      # generated
```

Plugin sources live outside this tree (`../plugins/lyrics-lrclib/`,
`../plugins/lyrics-genius/`, `../plugins/spectrum-plus/`); the build
script copies each `manifest.json` + `plugin.wasm` into its release zip.
First-party theme sources live in `../themes/` (e.g. `sonora-retro`).

## Rebuilding

```sh
cd community-registry
python3 tools/build.py
```

Re-running is byte-identical (fixed mtimes, sorted entries, pinned
permissions), so digests in `v1/*.json` only change when content changes.

## Index format (v1)

```json
{
  "schema": 1,
  "plugins": [{
    "id": "org.sonora.lrclib",
    "name": "LRCLIB Lyrics Provider",
    "description": "…",
    "author": {"name": "…", "url": "…"},
    "category": "lyrics",
    "homepage": "https://…",
    "capabilities": ["lyrics:provider", "network:fetch"],
    "api_version": 1,
    "min_sonora_version": "0.1.0",
    "latest_version": "1.0.0",
    "versions": [{
      "version": "1.0.0",
      "download_url": "https://…/org.sonora.lrclib-1.0.0.zip",
      "sha256": "<64 hex>",
      "size_bytes": 12345,
      "changelog": "What changed in this version."
    }]
  }]
}
```

`themes.json` mirrors this without `capabilities`/`api_version` (themes run
no code). A theme release zip contains `theme.json` — a complete theme
definition (`id`, `name`, `version`, `author`, `mode`, and all required design
tokens, matching the frontend theme schema) — plus an optional supplemental
`theme.css` (no remote fetches allowed). Client rules: `schema` must be `1`; ids follow plugin-id rules;
categories come from fixed sets; capabilities must be known; every version
needs a semver string, an `https://` URL, a 64-hex digest, and a changelog;
`latest_version` must reference a listed version.

## Submitting a package (manual process for now)

1. Add the entry metadata to `tools/build.py` and sources under `sources/`
   (or point at an external plugin directory like the LRCLIB example).
2. Run `python3 tools/build.py` and commit `packages/` + `v1/*.json`.
3. The client validates schema, digests, identity, and capability ceilings;
   no signing, accounts, payments, ratings, or social features exist yet.

## Client behavior

- Refresh fetches both documents, validates fully, then atomically replaces
  the cache. Offline falls back to the last validated cache (flagged in the UI).
- Install verifies SHA-256 before extraction, guards against path traversal,
  checks package id/version and capability ceilings, installs atomically with
  a versioned backup, and records a receipt. Updates keep backups; rollback
  restores the previous known-good version.
