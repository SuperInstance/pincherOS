#!/usr/bin/env bash
# baton-harbor-check.sh — Check for incoming batons, verify, and archive
# Run this at session start and before flush.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SYSTEM="$(cd "${HERE}/.." && pwd)"
VESSEL="${I2I_VESSEL:-/tmp/i2i-vessel}"
HARBOR="${VESSEL}/harbor"
ARCHIVE="${SYSTEM}/handoffs"

mkdir -p "${HARBOR}" "${ARCHIVE}"

shopt -s nullglob
BATONS=("${HARBOR}"/*)
if [ ${#BATONS[@]} -eq 0 ]; then
  echo "🫳 Harbor empty — no incoming messages."
  exit 0
fi

echo "═══ Harbor Check — ${#BATONS[@]} incoming ═══"
echo ""

for f in "${BATONS[@]}"; do
  # Run baton-read or basic parse
  BASENAME=$(basename "${f}")
  echo "── ${BASENAME} ──"
  
  # Try to extract type/from
  python3 -c "
import json
with open('${f}') as fh:
    d = json.load(fh)
print(f'  Type: {d.get(\"type\",\"?\")}  From: {d.get(\"from\",\"?\")}  To: {d.get(\"to\",\"?\")}')

shard = d.get('shard', {})
computed = __import__('hashlib').sha256(json.dumps(shard, separators=(',', ':'), sort_keys=True).encode()).hexdigest()
expected = d.get('integrity', '')
if computed == expected:
    print(f'  Integrity: ✅')
else:
    print(f'  Integrity: ❌ (computed {computed})')

blockers = shard.get('blockers', [])
if blockers:
    print(f'  Blockers: {len(blockers)} — {blockers[0][:80]}')
else:
    print(f'  Blockers: none')
" 2>&1 || echo "  ERROR: Could not parse ${BASENAME}"

  # Archive: move to handoffs
  mv "${f}" "${ARCHIVE}/${BASENAME}.$(date +%s).received"
  echo "  → Archived to handoffs/"
  echo ""
done

echo "✅ Harbor cleared. ${#BATONS[@]} message(s) processed."
