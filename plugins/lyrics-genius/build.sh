#!/usr/bin/env bash
# Build plugins/lyrics-genius/plugin.wasm from Rust sources.
# Usage: ./build.sh   (run from this directory)
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/sonora_plugin_lyrics_genius.wasm plugin.wasm
ls -la plugin.wasm
