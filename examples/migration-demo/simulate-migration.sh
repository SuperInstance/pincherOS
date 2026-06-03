#!/usr/bin/env bash
# PincherOS Migration Demo — simulate two shells on one machine
set -euo pipefail

echo "🦀 PincherOS Migration Demo"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━"

PINCHER="${PINCHER:-./target/release/pincher}"

if [ ! -f "$PINCHER" ]; then
    echo "Error: pincher binary not found at $PINCHER"
    echo "Build first: cargo build --release"
    exit 1
fi

# Clean up any previous demo
rm -rf /tmp/pincher-shell-a /tmp/pincher-shell-b /tmp/agent-migration.nail

# ── Shell A: Train ──────────────────────────────────────────────────────
echo ""
echo "━━ Step 1: Training on Shell A ━━"
echo ""

export PINCHER_DB="/tmp/pincher-shell-a/reflexes.db"
mkdir -p /tmp/pincher-shell-a

$PINCHER teach --intent "list running processes" --action "ps aux" 2>/dev/null
$PINCHER teach --intent "check memory usage" --action "free -h" 2>/dev/null
$PINCHER teach --intent "show disk space" --action "df -h" 2>/dev/null
$PINCHER teach --intent "check network connections" --action "ss -tlnp" 2>/dev/null
$PINCHER teach --intent "view recent logs" --action "journalctl --since '10 min ago' --no-pager" 2>/dev/null
$PINCHER teach --intent "find large files" --action "du -sh /* | sort -rh | head -10" 2>/dev/null
$PINCHER teach --intent "check docker status" --action "docker ps -a" 2>/dev/null
$PINCHER teach --intent "git status" --action "git status --short" 2>/dev/null

echo "Shell A status:"
$PINCHER status

# ── Pack ────────────────────────────────────────────────────────────────
echo ""
echo "━━ Step 2: Packing into .nail file ━━"
echo ""

$PINCHER pack --output /tmp/agent-migration.nail

echo ""
echo "File size: $(du -h /tmp/agent-migration.nail | cut -f1)"

# ── Inspect ─────────────────────────────────────────────────────────────
echo ""
echo "━━ Step 3: Inspecting .nail contents ━━"
echo ""

cp /tmp/agent-migration.nail /tmp/inspect.nail.tar.zst 2>/dev/null
if command -v zstd &>/dev/null; then
    zstd -d /tmp/inspect.nail.tar.zst -o /tmp/inspect.nail.tar 2>/dev/null
    echo "Archive contents:"
    tar -tf /tmp/inspect.nail.tar 2>/dev/null || echo "(could not inspect tar)"
    rm -f /tmp/inspect.nail.tar /tmp/inspect.nail.tar.zst
else
    echo "(zstd not installed, skipping archive inspection)"
fi

# ── Shell B: Unpack ─────────────────────────────────────────────────────
echo ""
echo "━━ Step 4: Unpacking on Shell B ━━"
echo ""

export PINCHER_DB="/tmp/pincher-shell-b/reflexes.db"
mkdir -p /tmp/pincher-shell-b

$PINCHER unpack /tmp/agent-migration.nail

echo ""
echo "Shell B status:"
$PINCHER status

# ── Verify ──────────────────────────────────────────────────────────────
echo ""
echo "━━ Step 5: Verifying reflexes on Shell B ━━"
echo ""

$PINCHER reflexes

echo ""
echo "━━ Step 6: Testing a match ━━"
echo ""

$PINCHER match "how much RAM is free"

# ── Cleanup ─────────────────────────────────────────────────────────────
echo ""
echo "━━ Cleanup ━━"
echo "To clean up demo files:"
echo "  rm -rf /tmp/pincher-shell-a /tmp/pincher-shell-b /tmp/agent-migration.nail"
echo ""
echo "✓ Migration demo complete!"
