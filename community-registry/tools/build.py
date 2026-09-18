#!/usr/bin/env python3
"""Build the sample Sonora community registry.

Reads package sources, writes deterministic release zips under
``packages/``, hashes them, and emits ``v1/plugins.json`` +
``v1/themes.json`` with correct SHA-256 digests. Re-running produces
byte-identical output (fixed timestamps, sorted entries).

Usage:  python3 tools/build.py   (from community-registry/)
"""
from __future__ import annotations

import hashlib
import json
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXED_DATE = (2026, 1, 1, 0, 0, 0)
BASE_DOWNLOAD_URL = "https://github.com/sonora-audio/community-registry/releases/download"

PLUGINS = [
    {
        "id": "org.sonora.lrclib",
        "name": "LRCLIB Lyrics Provider",
        "description": "Resolves plain and line-synced lyrics from the LRCLIB public API (lrclib.net) through the sandboxed host network API.",
        "author": {"name": "Sonora Contributors", "url": "https://github.com/sonora-audio/sonora"},
        "category": "lyrics",
        "homepage": "https://lrclib.net",
        "capabilities": ["lyrics:provider", "network:fetch"],
        "api_version": 1,
        "min_sonora_version": "0.1.0",
        "versions": [
            {
                "version": "1.0.0",
                "files": {
                    "manifest.json": "../plugins/lyrics-lrclib/manifest.json",
                    "plugin.wasm": "../plugins/lyrics-lrclib/plugin.wasm",
                },
                "changelog": "Initial release: LRCLIB synced/plain lyrics with rate-limit and timeout handling.",
            }
        ],
    },
    {
        "id": "org.sonora.spectrum_plus",
        "name": "Spectrum+ Visualizer",
        "description": "Advanced spectrum visualizer over the read-only audio tap: bars, wave, and mirror modes with peak normalization and release smoothing. No network, no filesystem, no output access.",
        "author": {"name": "Sonora Contributors", "url": "https://github.com/sonora-audio/sonora"},
        "category": "visualizer",
        "capabilities": ["visualizer:tap"],
        "api_version": 1,
        "min_sonora_version": "0.1.0",
        "versions": [
            {
                "version": "1.0.0",
                "files": {
                    "manifest.json": "../plugins/spectrum-plus/manifest.json",
                    "plugin.wasm": "../plugins/spectrum-plus/plugin.wasm",
                },
                "changelog": "Initial release: bars/wave/mirror modes with smoothing over the audio tap.",
            }
        ],
    },
    {
        "id": "org.sonora.lyrics_genius",
        "name": "Genius Lyrics Provider",
        "description": "Resolves lyrics from Genius public web endpoints (search API plus song pages) through the sandboxed host network API, with attribution on every answer.",
        "author": {"name": "Sonora Contributors", "url": "https://github.com/sonora-audio/sonora"},
        "category": "lyrics",
        "homepage": "https://genius.com",
        "capabilities": ["lyrics:provider", "network:fetch"],
        "api_version": 1,
        "min_sonora_version": "0.1.0",
        "versions": [
            {
                "version": "1.0.0",
                "files": {
                    "manifest.json": "../plugins/lyrics-genius/manifest.json",
                    "plugin.wasm": "../plugins/lyrics-genius/plugin.wasm",
                },
                "changelog": "Initial release: metadata-based Genius lookup with strict markup parsing and graceful fallback.",
            }
        ],
    },
]

THEMES = [
    {
        "id": "org.sonora.theme.neon_night",
        "name": "Neon Night",
        "description": "Dark theme with neon cyan/magenta accents. Styling only — no code runs.",
        "author": {"name": "Sonora Contributors"},
        "category": "dark",
        "min_sonora_version": "0.1.0",
        "versions": [
            {
                "version": "1.0.0",
                "files": {
                    "theme.json": "sources/neon-night/theme.json",
                    "theme.css": "sources/neon-night/theme.css",
                },
                "changelog": "Initial release.",
            }
        ],
    },
    {
        "id": "org.sonora.theme.retro",
        "name": "Sonora Retro",
        "description": "Amber-phosphor CRT terminal aesthetic: warm near-black surfaces, glowing amber accents, and a scanline/vignette texture. Styling only — no code runs.",
        "author": {"name": "Sonora Contributors"},
        "category": "dark",
        "min_sonora_version": "0.1.0",
        "versions": [
            {
                "version": "1.0.0",
                "files": {
                    "theme.json": "../themes/sonora-retro/theme.json",
                    "theme.css": "../themes/sonora-retro/theme.css",
                },
                "changelog": "Initial release.",
            }
        ],
    },
]


def build_zip(files: dict[str, str], out_path: Path) -> None:
    """Write a deterministic zip: sorted names, fixed mtime, 0o644 perms."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
        for arcname in sorted(files):
            src = (ROOT / files[arcname]).resolve()
            if not src.is_file():
                sys.exit(f"missing source file: {src} (run from community-registry/)")
            info = zipfile.ZipInfo(arcname, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            zf.writestr(info, src.read_bytes())


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_entries(items: list[dict], kind: str) -> list[dict]:
    entries = []
    for item in items:
        versions = []
        for v in item["versions"]:
            zip_name = f"{item['id']}-{v['version']}.zip"
            zip_path = ROOT / "packages" / zip_name
            build_zip(v["files"], zip_path)
            digest = sha256_file(zip_path)
            versions.append(
                {
                    "version": v["version"],
                    "download_url": f"{BASE_DOWNLOAD_URL}/{item['id']}-v{v['version']}/{zip_name}",
                    "sha256": digest,
                    "size_bytes": zip_path.stat().st_size,
                    "changelog": v["changelog"],
                }
            )
        entry = {k: item[k] for k in ("id", "name", "description", "author", "category")}
        if "homepage" in item:
            entry["homepage"] = item["homepage"]
        if kind == "plugin":
            entry["capabilities"] = item["capabilities"]
            entry["api_version"] = item["api_version"]
        entry["min_sonora_version"] = item["min_sonora_version"]
        entry["latest_version"] = versions[-1]["version"]
        entry["versions"] = versions
        entries.append(entry)
    return entries


def main() -> None:
    plugins = build_entries(PLUGINS, "plugin")
    themes = build_entries(THEMES, "theme")
    v1 = ROOT / "v1"
    v1.mkdir(exist_ok=True)
    (v1 / "plugins.json").write_text(
        json.dumps({"schema": 1, "plugins": plugins}, indent=2) + "\n"
    )
    (v1 / "themes.json").write_text(
        json.dumps({"schema": 1, "themes": themes}, indent=2) + "\n"
    )
    print(f"wrote {len(plugins)} plugin(s), {len(themes)} theme(s)")


if __name__ == "__main__":
    main()
