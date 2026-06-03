#!/usr/bin/env bash
# Inspect the contents of a .nail file
set -euo pipefail

NAIL_FILE="${1:?Usage: inspect-nail.sh <path-to-nail-file>}"

if [ ! -f "$NAIL_FILE" ]; then
    echo "Error: File not found: $NAIL_FILE"
    exit 1
fi

echo "🦀 Inspecting .nail file: $NAIL_FILE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# File size
echo ""
echo "File size: $(du -h "$NAIL_FILE" | cut -f1)"

# Try to decompress and list
WORK_DIR=$(mktemp -d)
trap "rm -rf $WORK_DIR" EXIT

cp "$NAIL_FILE" "$WORK_DIR/archive.tar.zst"

if command -v zstd &>/dev/null; then
    zstd -d "$WORK_DIR/archive.tar.zst" -o "$WORK_DIR/archive.tar" 2>/dev/null
    tar -xf "$WORK_DIR/archive.tar" -C "$WORK_DIR" 2>/dev/null

    echo ""
    echo "Contents:"
    for f in "$WORK_DIR"/*; do
        fname=$(basename "$f")
        if [ "$fname" != "archive.tar.zst" ] && [ "$fname" != "archive.tar" ]; then
            fsize=$(du -h "$f" | cut -f1)
            echo "  $fname ($fsize)"
        fi
    done

    # Show manifest
    if [ -f "$WORK_DIR/manifest.json" ]; then
        echo ""
        echo "manifest.json:"
        cat "$WORK_DIR/manifest.json" | head -20
    fi

    # Show identity
    if [ -f "$WORK_DIR/identity.json" ]; then
        echo ""
        echo "identity.json:"
        cat "$WORK_DIR/identity.json"
    fi

    # Show config
    if [ -f "$WORK_DIR/config.toml" ]; then
        echo ""
        echo "config.toml:"
        cat "$WORK_DIR/config.toml"
    fi
else
    echo ""
    echo "(zstd not installed — install with: apt install zstd / brew install zstd)"
    echo "The .nail file is a tar.zst archive. You can inspect it manually:"
    echo "  cp $NAIL_FILE /tmp/inspect.tar.zst"
    echo "  zstd -d /tmp/inspect.tar.zst"
    echo "  tar -tf /tmp/inspect.tar"
fi
