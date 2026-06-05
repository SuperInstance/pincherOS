#!/usr/bin/env bash
# checkpoint.sh — Reflex γ extension: STATE_CHANGE → CHECKPOINT → DURABLE_STORE
# Call after any major mutation to memory, vessel, or task.
#
# Usage:
#   bash scripts/checkpoint.sh "description of what changed"
set -euo pipefail

WORKSPACE="/home/ubuntu/.openclaw/workspace"
STATE_FILE="$WORKSPACE/i2i-vessel/SESSION-STATE.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

CHANGE="${1:-state update}"

# Update SESSION-STATE.md with the checkpoint
cat > "$STATE_FILE" << EOF
# SESSION STATE CHECKPOINT
# Last updated: $TIMESTAMP

checkpoint:
  timestamp: $TIMESTAMP
  change: "$CHANGE"
  
session:
  intent: "Stabilizing vessel, inducing cognitive patterns, level-based fleet architecture"

persist_targets:
  - "MEMORY.md"
  - "GitHub SuperInstance/pincher"
  - "workspace/i2i-vessel/"
EOF

echo "✅ Checkpoint: $CHANGE ($TIMESTAMP)"
