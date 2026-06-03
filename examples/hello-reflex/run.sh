#!/usr/bin/env bash
# Hello Reflex — Automated Walkthrough
# Run this script to go through the entire 5-minute tutorial automatically.
#
# Usage: ./run.sh [/path/to/pincher]
#
# If no path is given, it tries ./target/release/pincher, then `pincher` from PATH.

set -euo pipefail

# Resolve the pincher binary
if [[ -n "${1:-}" ]]; then
    PINCHER="$1"
elif [[ -x "./target/release/pincher" ]]; then
    PINCHER="./target/release/pincher"
elif command -v pincher &>/dev/null; then
    PINCHER="pincher"
else
    echo "❌ Cannot find pincher binary. Build it first with: cargo build --release"
    echo "   Or pass the path as an argument: ./run.sh /path/to/pincher"
    exit 1
fi

echo "🦀 Using pincher at: $PINCHER"
echo ""

# ── Step 1: Check your shell ─────────────────────────────────────────────
echo "━━━ Step 1: Check Your Shell ━━━"
echo "$ pincher status"
"$PINCHER" status
echo ""

# ── Step 2: Teach a reflex ──────────────────────────────────────────────
echo "━━━ Step 2: Teach a Reflex ━━━"
echo "$ pincher teach --intent \"list docker containers\" --action \"docker ps\""
"$PINCHER" teach --intent "list docker containers" --action "docker ps"
echo ""

# ── Step 3: Execute it ──────────────────────────────────────────────────
echo "━━━ Step 3: Execute the Reflex ━━━"
echo "$ pincher do \"show me my containers\""
"$PINCHER" do "show me my containers"
echo ""

echo "$ pincher do \"what's running in docker\""
"$PINCHER" do "what's running in docker"
echo ""

# ── Step 4: Match without executing ─────────────────────────────────────
echo "━━━ Step 4: Match Without Executing ━━━"
echo "$ pincher match \"are there any dockers running\""
"$PINCHER" match "are there any dockers running"
echo ""

# ── Step 5: Watch your reflex list ──────────────────────────────────────
echo "━━━ Step 5: Watch Your Reflex List ━━━"
echo "$ pincher reflexes"
"$PINCHER" reflexes
echo ""

# ── Step 6: Run benchmarks ──────────────────────────────────────────────
echo "━━━ Step 6: Run Benchmarks ━━━"
echo "$ pincher bench"
"$PINCHER" bench
echo ""

echo "🦀 Walkthrough complete! Your crab has its first reflex."
echo "   Next: try the smart-home example → ../smart-home/"
