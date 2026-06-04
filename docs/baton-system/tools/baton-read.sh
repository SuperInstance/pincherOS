#!/usr/bin/env bash
# baton-read.sh — Read and verify a baton file
# Usage: baton-read.sh <baton-file>
set -euo pipefail

FILE="${1:?Usage: baton-read.sh <baton-file>}"

if [ ! -f "${FILE}" ]; then
  echo "ERROR: File not found: ${FILE}" >&2
  exit 1
fi

echo "═══ Baton: $(basename "${FILE}") ═══"
echo ""

# Parse with python for JSON safety
python3 -c "
import json, sys

with open('${FILE}') as f:
    data = json.load(f)

typ = data.get('type', 'UNKNOWN').ljust(20)
frm = data.get('from', '?').ljust(15)
to_ = data.get('to', '?').ljust(15)
ts = data.get('timestamp', '?')[0:19]
shard = data.get('shard', {})
integrity = data.get('integrity', '')
version = data.get('version', '?')

# Verify integrity
computed = __import__('hashlib').sha256(json.dumps(shard, separators=(',', ':'), sort_keys=True).encode()).hexdigest()
valid = computed == integrity or integrity == ''

print(f'  Type:      {typ}')
print(f'  From:      {frm}')
print(f'  To:        {to_}')
print(f'  Timestamp: {ts}')
print(f'  Version:   {version}')
print(f'  Integrity: {\"✅ VALID\" if valid else \"❌ MISMATCH (computed: \" + computed + \")\"}')
print()

# Print shard contents
artifacts = shard.get('artifacts', {})
reasoning = shard.get('reasoning', [])
blockers = shard.get('blockers', [])

print('  ── Artifacts ──')
if isinstance(artifacts, dict) and artifacts:
    klen = max(len(k) for k in artifacts.keys()) if artifacts else 0
    for k, v in sorted(artifacts.items()):
        v_str = str(v)[:80]
        print(f'    {k.ljust(klen)} : {v_str}')
elif artifacts:
    print(f'    {str(artifacts)[:120]}')
else:
    print('    (empty)')

print()
print('  ── Reasoning ──')
if reasoning:
    for i, r in enumerate(reasoning):
        print(f'    {i+1}. {str(r)[:120]}')
else:
    print('    (none)')

print()
print('  ── Blockers ──')
if blockers:
    for i, b in enumerate(blockers):
        print(f'    {i+1}. {str(b)[:120]}')
else:
    print('    (none — proceed freely)')

# Print spline ref if present
spline = data.get('spline_ref')
if spline:
    print()
    print(f'  Spline: {spline}')
" 2>&1 || echo "ERROR: Failed to parse baton file" >&2
