#!/usr/bin/env bash
# baton-create.sh — Create an I2I baton message
# Usage: baton-create.sh <type> <to> [shard-json-file | --stdin]
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SYSTEM="$(cd "${HERE}/.." && pwd)"
VESSEL="${I2I_VESSEL:-/tmp/i2i-vessel}"
FROM="${I2I_FROM:-oracle2}"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
SLUG="$(date -u +"%Y%m%d%H%M%S")"

TYPE="${1:?Usage: baton-create.sh <type> <to> [shard-json-file | --stdin]}"
TO="${2:?Usage: baton-create.sh <type> <to> [shard-json-file | --stdin]}"
SHARD_INPUT="${3:-}"

# Validate type
case "${TYPE}" in
  TASK|STATUS|CHECKPOINT|BLOCKER|DELIVERABLE|BOTTLE|ACK|SYNTHESIS|CHALLENGE|SESSION|SPLINE)
    ;;
  *)
    echo "ERROR: Invalid baton type '${TYPE}'. Valid: TASK STATUS CHECKPOINT BLOCKER DELIVERABLE BOTTLE ACK SYNTHESIS CHALLENGE SESSION SPLINE" >&2
    exit 1
    ;;
esac

# Get shard JSON
if [ -n "${SHARD_INPUT}" ]; then
  if [ "${SHARD_INPUT}" = "--stdin" ]; then
    SHARD_JSON=$(cat)
  else
    SHARD_JSON=$(cat "${SHARD_INPUT}")
  fi
else
  # Empty shard
  SHARD_JSON='{"artifacts":{},"reasoning":[],"blockers":[]}'
fi

# Compute integrity hash
INTEGRITY=$(echo -n "${SHARD_JSON}" | sha256sum | cut -d' ' -f1)

# Build baton
BATON_JSON=$(cat << JSON
{
  "type": "I2I:${TYPE}",
  "version": "2.0",
  "from": "${FROM}",
  "to": "${TO}",
  "timestamp": "${TIMESTAMP}",
  "shard": ${SHARD_JSON},
  "integrity": "${INTEGRITY}"
}
JSON
)

# Write to vessel
FILENAME="${SLUG}-${FROM}-to-${TO}-${TYPE,,}.baton"
echo "${BATON_JSON}" > "${VESSEL}/bottles/${FILENAME}"
echo "${BATON_JSON}" > "${SYSTEM}/bottles/${FILENAME}"

echo "[I2I:${TYPE}] ${FROM} → ${TO} — ${FILENAME}"
echo "  Integrity: ${INTEGRITY}"
