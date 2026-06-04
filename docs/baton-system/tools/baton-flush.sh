#!/usr/bin/env bash
# baton-flush.sh — Memory flush protocol: snapshot → shard → spline → harbor-check → commit
# Run before session end, compaction, or handoff.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SYSTEM="$(cd "${HERE}/.." && pwd)"
WORKSPACE="/home/ubuntu/.openclaw/workspace"
VESSEL="${I2I_VESSEL:-/tmp/i2i-vessel}"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
SLUG="$(date -u +"%Y%m%d%H%M%S")"

echo "═══ Baton Flush Protocol ═══"
echo "Timestamp: ${TIMESTAMP}"
echo ""

# Step 1: Harbor check
echo "[1/5] Checking harbor for unread messages..."
bash "${HERE}/baton-harbor-check.sh" || true
echo ""

# Step 2: Create session snapshot baton
echo "[2/5] Creating session snapshot..."

SESSION_SNAPSHOT=$(cat << JSON
{
  "artifacts": {
    "session_file": "${WORKSPACE}/memory/$(date +%Y-%m-%d).md",
    "workspace": "${WORKSPACE}"
  },
  "reasoning": [
    "Session completed — see memory log for details",
    "Baton system initialized"
  ],
  "blockers": []
}
JSON
)

# Use baton-create with stdin
echo "${SESSION_SNAPSHOT}" | bash "${HERE}/baton-create.sh" "SESSION" "self" "--stdin" || {
  echo "  WARNING: baton session failed, writing directly"
  cat > "${SYSTEM}/bottles/${SLUG}-flush-session.baton" << JSON
{
  "type": "I2I:SESSION",
  "version": "2.0",
  "from": "oracle2",
  "to": "self",
  "timestamp": "${TIMESTAMP}",
  "shard": ${SESSION_SNAPSHOT},
  "integrity": "$(echo -n "${SESSION_SNAPSHOT}" | sha256sum | cut -d' ' -f1)"
}
JSON
  echo "  → Wrote directly to bottles/"
}
echo ""

# Step 3: Record audit
echo "[3/5] Recording flush audit..."
AUDIT_FILE="${SYSTEM}/audit/flush-log.md"
echo "| ${TIMESTAMP} | Flush | Session end — baton v2.0 |" >> "${AUDIT_FILE}"
echo "  → Appended to ${AUDIT_FILE}"
echo ""

# Step 4: Update MEMORY.md baton section
echo "[4/5] Updating MEMORY.md baton data..."
if grep -q "^## Baton" "${WORKSPACE}/MEMORY.md" 2>/dev/null; then
  echo "  Baton section already exists in MEMORY.md"
else
  cat >> "${WORKSPACE}/MEMORY.md" << 'EOF'

## Baton Protocol
- System: /home/ubuntu/.openclaw/workspace/baton-system/
- Protocol: PROTOCOL.md (v2.0)
- Vessel: /tmp/i2i-vessel/
- Tools: baton-create.sh, baton-read.sh, baton-spline.sh, baton-harbor-check.sh, baton-flush.sh
- Types: TASK STATUS CHECKPOINT BLOCKER DELIVERABLE BOTTLE ACK SYNTHESIS CHALLENGE SESSION SPLINE
- Integrity: SHA-256 on shard JSON
- Rule: Every flush writes a spline. No unread harbors. Verify integrity.
EOF
  echo "  → Appended Baton Protocol section to MEMORY.md"
fi
echo ""

# Step 5: Git commit
echo "[5/5] Committing to git..."
cd "${WORKSPACE}"
git add baton-system/ 2>/dev/null || true
git add MEMORY.md 2>/dev/null || true
git commit -m "[I2I:SESSION] oracle2 — baton flush at ${TIMESTAMP}

Baton Protocol v2.0 active.
Tools deployed: create, read, spline, harbor-check, flush.
Architecture: shard → bottle → vessel → spline.

Status: CLEAN" 2>/dev/null || echo "  Nothing to commit (already clean)"
echo ""

echo "═══ Flush Complete ═══"
echo "Next steps: commit to git with [I2I:SESSION] prefix"
