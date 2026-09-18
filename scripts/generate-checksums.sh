#!/usr/bin/env bash
set -euo pipefail

# Sonora Checksums Generator & Validator
# Usage: ./scripts/generate-checksums.sh [dist_dir]

DIST_DIR="${1:-dist-release}"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "Error: Directory '$DIST_DIR' does not exist." >&2
  exit 1
fi

cd "$DIST_DIR"

# Remove any old checksums file before computing
rm -f checksums.txt SHA256SUMS.txt

FILES=$(find . -maxdepth 1 -type f -not -name "checksums.txt" -not -name "SHA256SUMS.txt" -printf "%P\n" | sort)

if [[ -z "$FILES" ]]; then
  echo "Error: No release files found in '$DIST_DIR'." >&2
  exit 1
fi

echo "Calculating SHA-256 for release artifacts in $DIST_DIR:"
for f in $FILES; do
  if [[ ! -s "$f" ]]; then
    echo "Error: File '$f' is empty (0 bytes)!" >&2
    exit 1
  fi
  sha256sum "$f"
done > checksums.txt

cat checksums.txt
echo "Checksum generation complete: $(wc -l < checksums.txt) artifacts recorded in checksums.txt."
