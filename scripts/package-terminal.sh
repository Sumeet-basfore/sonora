#!/usr/bin/env bash
set -euo pipefail

# Sonora Terminal Package Generator
# Usage: ./scripts/package-terminal.sh <os> <arch> [target_dir] [out_dir] [version]
# Examples:
#   ./scripts/package-terminal.sh linux x86_64 target/release dist-release 0.1.0
#   ./scripts/package-terminal.sh macos aarch64 target/aarch64-apple-darwin/release dist-release 0.1.0
#   ./scripts/package-terminal.sh windows x86_64 target/release dist-release 0.1.0

OS="${1:-linux}"
ARCH="${2:-x86_64}"
TARGET_DIR="${3:-target/release}"
OUT_DIR="${4:-dist-release}"
VERSION="${5:-}"

if [[ -z "$VERSION" ]]; then
  VERSION=$(grep '^version = ' Cargo.toml | head -1 | tr -d ' "' | cut -d= -f2 || echo "0.1.0")
fi

mkdir -p "$OUT_DIR"

STAGE_DIR=$(mktemp -d)
CLEANUP() {
  rm -rf "$STAGE_DIR"
}
trap CLEANUP EXIT

BUNDLE_DIR="$STAGE_DIR/sonora-terminal"
mkdir -p "$BUNDLE_DIR"

EXE_EXT=""
if [[ "$OS" == "windows" ]]; then
  EXE_EXT=".exe"
fi

CLI_BIN="$TARGET_DIR/sonora$EXE_EXT"
TUI_BIN="$TARGET_DIR/sonora-tui$EXE_EXT"

# Fallback: if 'sonora' wasn't built directly, try 'sonora-cli'
if [[ ! -f "$CLI_BIN" && -f "$TARGET_DIR/sonora-cli$EXE_EXT" ]]; then
  CLI_BIN="$TARGET_DIR/sonora-cli$EXE_EXT"
fi

if [[ ! -f "$CLI_BIN" ]]; then
  echo "Error: CLI binary not found at $TARGET_DIR/sonora$EXE_EXT or $TARGET_DIR/sonora-cli$EXE_EXT" >&2
  exit 1
fi

if [[ ! -f "$TUI_BIN" ]]; then
  echo "Error: TUI binary not found at $TUI_BIN" >&2
  exit 1
fi

cp "$CLI_BIN" "$BUNDLE_DIR/sonora$EXE_EXT"
cp "$TUI_BIN" "$BUNDLE_DIR/sonora-tui$EXE_EXT"

if [[ "$OS" != "windows" ]]; then
  chmod +x "$BUNDLE_DIR/sonora" "$BUNDLE_DIR/sonora-tui"
fi

cat > "$BUNDLE_DIR/README.txt" << EODOC
================================================================================
Sonora Terminal Audio Player (v${VERSION})
================================================================================

Sonora is a local-first audio player built with a Rust audio engine,
SQLite + FTS5 metadata indexing, and lock-free audio playback.

This archive contains the standalone terminal tools for Sonora:
  - sonora${EXE_EXT}      : Scriptable Command-Line Interface (CLI)
  - sonora-tui${EXE_EXT}  : Full-screen interactive Terminal User Interface (TUI)

--------------------------------------------------------------------------------
1. QUICK START
--------------------------------------------------------------------------------

EODOC

if [[ "$OS" == "windows" ]]; then
  cat >> "$BUNDLE_DIR/README.txt" << EODOC
Open Command Prompt (cmd) or PowerShell in this directory:

Run the Interactive Terminal Player:
    .\\sonora-tui.exe

Run CLI Commands:
    .\\sonora.exe --help
    .\\sonora.exe status
    .\\sonora.exe scan "C:\\Users\\YourName\\Music"

--------------------------------------------------------------------------------
2. ADDING TO YOUR PATH (OPTIONAL)
--------------------------------------------------------------------------------

To run 'sonora' and 'sonora-tui' from any terminal:
  1. Copy 'sonora.exe' and 'sonora-tui.exe' to a directory (e.g. C:\\Tools\\Sonora)
  2. Add that directory to your User or System PATH in Windows Environment Variables.
EODOC
else
  cat >> "$BUNDLE_DIR/README.txt" << EODOC
Make sure the binaries are executable:
    chmod +x sonora sonora-tui

Run the Interactive Terminal Player:
    ./sonora-tui

Run CLI Commands:
    ./sonora --help
    ./sonora status
    ./sonora scan /path/to/your/music

--------------------------------------------------------------------------------
2. ADDING TO YOUR PATH (OPTIONAL)
--------------------------------------------------------------------------------

To run 'sonora' and 'sonora-tui' from any terminal directory:

Option A — System-wide (requires sudo):
    sudo cp sonora sonora-tui /usr/local/bin/

Option B — User-only (no sudo required):
    mkdir -p ~/.local/bin
    cp sonora sonora-tui ~/.local/bin/
    # Ensure ~/.local/bin is in your PATH in ~/.bashrc or ~/.zshrc:
    export PATH="\$HOME/.local/bin:\$PATH"
EODOC
fi

cat >> "$BUNDLE_DIR/README.txt" << EODOC

--------------------------------------------------------------------------------
3. SYSTEM REQUIREMENTS
--------------------------------------------------------------------------------

Linux:
    ALSA audio libraries (standard on Linux distributions, e.g. libasound2)

Windows:
    Windows 10 or later (WASAPI audio output)

macOS:
    macOS 10.15 Catalina or later (CoreAudio output)

--------------------------------------------------------------------------------
4. VERIFYING CHECKSUMS
--------------------------------------------------------------------------------

Compare archive integrity against the official 'checksums.txt' on:
https://github.com/Sumeet-basfore/sonora/releases/latest

Linux / macOS:
    sha256sum Sonora-v${VERSION}-${OS}-${ARCH}-terminal.tar.gz

Windows (PowerShell):
    Get-FileHash Sonora-v${VERSION}-${OS}-${ARCH}-terminal.zip -Algorithm SHA256

--------------------------------------------------------------------------------
5. DOCUMENTATION & LINKS
--------------------------------------------------------------------------------

GitHub Repository: https://github.com/Sumeet-basfore/sonora
Releases:          https://github.com/Sumeet-basfore/sonora/releases
Documentation:     https://github.com/Sumeet-basfore/sonora/tree/main/docs
License:           MIT OR Apache-2.0
================================================================================
EODOC

ARCHIVE_NAME="Sonora-v${VERSION}-${OS}-${ARCH}-terminal"

if [[ "$OS" == "windows" ]]; then
  OUT_FILE="$OUT_DIR/${ARCHIVE_NAME}.zip"
  if command -v zip >/dev/null 2>&1; then
    (cd "$STAGE_DIR" && zip -r -q "$OLDPWD/$OUT_FILE" sonora-terminal)
  elif command -v 7z >/dev/null 2>&1; then
    (cd "$STAGE_DIR" && 7z a -tzip "$OLDPWD/$OUT_FILE" sonora-terminal >/dev/null)
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c "import shutil; shutil.make_archive('$OUT_DIR/${ARCHIVE_NAME}', 'zip', '$STAGE_DIR', 'sonora-terminal')"
  else
    echo "Error: No zip tool found (zip, 7z, or python3)" >&2
    exit 1
  fi
else
  OUT_FILE="$OUT_DIR/${ARCHIVE_NAME}.tar.gz"
  tar -czf "$OUT_FILE" -C "$STAGE_DIR" sonora-terminal
fi

echo "Created terminal package: $OUT_FILE"
