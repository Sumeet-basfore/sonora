#!/usr/bin/env bash
set -euo pipefail

# Sonora Release Validation Utility
# Usage: ./scripts/validate-release.sh [dist_dir] [expected_version]

DIST_DIR="${1:-dist-release}"
VERSION="${2:-}"

if [[ -z "$VERSION" ]]; then
  VERSION=$(grep '^version = ' Cargo.toml | head -1 | tr -d ' "' | cut -d= -f2 || echo "0.1.0")
fi

echo "================================================================================"
echo "Validating Sonora v${VERSION} Release Artifacts in '$DIST_DIR'"
echo "================================================================================"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "❌ Error: Directory '$DIST_DIR' not found." >&2
  exit 1
fi

CHECKSUM_FILE="$DIST_DIR/checksums.txt"
if [[ ! -f "$CHECKSUM_FILE" ]]; then
  echo "❌ Error: '$CHECKSUM_FILE' missing. Run generate-checksums.sh first." >&2
  exit 1
fi

echo "1. Checking SHA-256 integrity against checksums.txt..."
(cd "$DIST_DIR" && sha256sum -c checksums.txt)
echo "✅ All checksums verified."

echo ""
echo "2. Validating artifact sizes and non-emptiness..."
for f in "$DIST_DIR"/*; do
  if [[ -f "$f" ]]; then
    size=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null || wc -c < "$f")
    fname=$(basename "$f")
    if [[ "$size" -le 0 ]]; then
      echo "❌ Error: '$fname' is 0 bytes!" >&2
      exit 1
    fi
    echo "  - $fname: $(numfmt --to=iec-i --suffix=B "$size" 2>/dev/null || echo "$size bytes") (OK)"
  fi
done
echo "✅ All files are non-empty."

echo ""
echo "3. Validating terminal archives..."
for archive in "$DIST_DIR"/Sonora-*-terminal.tar.gz; do
  if [[ -f "$archive" ]]; then
    echo "  Checking $(basename "$archive")..."
    tar_contents=$(tar -ztvf "$archive")
    if ! echo "$tar_contents" | grep -q "sonora-terminal/sonora"; then
      echo "❌ Error: '$archive' missing 'sonora' binary!" >&2
      exit 1
    fi
    if ! echo "$tar_contents" | grep -q "sonora-terminal/sonora-tui"; then
      echo "❌ Error: '$archive' missing 'sonora-tui' binary!" >&2
      exit 1
    fi
    if ! echo "$tar_contents" | grep -q "sonora-terminal/README.txt"; then
      echo "❌ Error: '$archive' missing 'README.txt'!" >&2
      exit 1
    fi
    echo "  ✅ Archive structure valid."
  fi
done

for zip_archive in "$DIST_DIR"/Sonora-*-terminal.zip; do
  if [[ -f "$zip_archive" ]]; then
    echo "  Checking $(basename "$zip_archive")..."
    if command -v unzip >/dev/null 2>&1; then
      zip_contents=$(unzip -l "$zip_archive")
      if ! echo "$zip_contents" | grep -q "sonora.exe"; then
        echo "❌ Error: '$zip_archive' missing 'sonora.exe'!" >&2
        exit 1
      fi
      if ! echo "$zip_contents" | grep -q "sonora-tui.exe"; then
        echo "❌ Error: '$zip_archive' missing 'sonora-tui.exe'!" >&2
        exit 1
      fi
      if ! echo "$zip_contents" | grep -q "README.txt"; then
        echo "❌ Error: '$zip_archive' missing 'README.txt'!" >&2
        exit 1
      fi
      echo "  ✅ Windows zip structure valid."
    fi
  fi
done

echo ""
echo "================================================================================"
echo "🎉 Release validation passed for v${VERSION}!"
echo "================================================================================"
