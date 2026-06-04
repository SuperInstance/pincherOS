#!/usr/bin/env bash
# baton-spline.sh — Distill a baton or concept into a spline
# Usage: baton-spline.sh <title> <insight> [anchor...]
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SYSTEM="$(cd "${HERE}/.." && pwd)"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
SLUG="$(date -u +"%Y%m%d%H%M%S")"

TITLE="${1:?Usage: baton-spline.sh <title> <insight> [anchor...]}"
INSIGHT="${2:?Usage: baton-spline.sh <title> <insight> [anchor...]}"
shift 2

ANCHORS=()
for a in "$@"; do
  ANCHORS+=("${a}")
done

# Sanitize title for filename
FILENAME_TITLE=$(echo "${TITLE}" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9-' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
FILENAME="${SLUG}-${FILENAME_TITLE}.spline"

SPLINE_JSON=$(cat << JSON
{
  "title": "${TITLE}",
  "insight": "${INSIGHT}",
  "anchors": [$(printf '"%s",' "${ANCHORS[@]}" | sed 's/,$//')],
  "origin": "oracle2 at ${TIMESTAMP}",
  "negative_space": ""
}
JSON
)

echo "${SPLINE_JSON}" > "${SYSTEM}/splines/${FILENAME}"
echo "[I2I:SPLINE] oracle2 — \"${TITLE}\""
echo "  File: splines/${FILENAME}"
echo "  Insight: ${INSIGHT:0:80}..."
