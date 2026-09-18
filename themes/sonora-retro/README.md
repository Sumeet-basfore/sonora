# Sonora Retro (`org.sonora.theme.retro`)

A first-party theme pack: an amber-phosphor CRT terminal aesthetic for
Sonora. Warm near-black surfaces, glowing amber accents, paper-white text,
plus a faint scanline/vignette texture. Styling only — **no code runs**,
ever.

```
themes/sonora-retro/
├── theme.json   # complete definition: id, mode, all required tokens
├── theme.css    # supplemental CRT texture only (scanlines + vignette)
└── README.md    # this file
```

## Palette

| Role | Token | Value |
|---|---|---|
| Base canvas | `--bg-base` | `#14100a` |
| Surface / cards | `--bg-surface`, `--bg-card` | `#1e170c` |
| Primary accent | `--accent-primary` | `#ffb000` (amber) |
| Secondary accent | `--accent-secondary` | `#ff6b35` (burnt orange) |
| Primary text | `--text-primary` | `#ffe8c2` (warm paper) |
| Muted text | `--text-muted` | `#8a6f45` |
| Focus / active border | `--border-focus` | `#ffb000` |
| Mode | `mode` | `dark` |

Example chrome (textual mock — tokens, not pixels):

```
+--------------------------------------------------+
| SONORA RETRO              [Albums][Artists][Tracks|
+--------------------------------------------------+
| ♪ Kind of Blue                                   |
|   Miles Davis — 1959 · FLAC 24/96                |
|                                                  |
| ████████████████░░░░░░░░░░░░░░  24:12 / 45:44     |
| [⏮][⏯][⏭]  Vol ████████░░                        |
+--------------------------------------------------+
```

## What theme.css is for (and only that)

`theme.json` tokens carry the entire palette. `theme.css` adds just the two
textures tokens cannot express: 3 px scanlines (`body::after`) and a soft
edge vignette (`body::before`). Both layers are `pointer-events: none`, UTF-8,
under the size cap, and contain no `url(`, `@import`, or script vectors —
the registry installer and the serve path both reject such content.

## Validation & packaging

There is nothing to compile. Validate and package with the workspace tools:

```sh
# Schema + security validation (required tokens, mode, value guards, CSS guard)
cargo test -p sonora-registry theme
# Deterministic registry packaging (byte-identical rebuilds)
python3 community-registry/tools/build.py
```

The registry entry lives in `community-registry/tools/build.py` (`THEMES`);
installing through the marketplace verifies SHA-256 before extraction,
validates `theme.json` again on disk, and keeps versioned backups with
rollback. Malformed themes are rejected gracefully — never partially applied.
