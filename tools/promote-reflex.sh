#!/usr/bin/env bash
# promote-reflex.sh — Gate 4 of the novel→reflex promotion pipeline
# Usage: bash scripts/promote-reflex.sh <name> <trigger> "<steps>"
#
# Reads COGNITIVE_REFLEXES.md, appends new reflex, commits to GitHub.

set -euo pipefail

WORKSPACE="/home/ubuntu/.openclaw/workspace"
REFLEX_FILE="$WORKSPACE/library/COGNITIVE_REFLEXES.md"
PINCHER_REPO="/tmp/pincher"

NAME="${1:?Usage: promote-reflex <name> <trigger> <steps>}"
TRIGGER="${2:?Usage: promote-reflex <name> <trigger> <steps>}"
STEPS="${3:?Usage: promote-reflex <name> <trigger> <steps>}"

# Find the next reflex letter
LETTERS=$(grep -c "^## Reflex [α-ω]" "$REFLEX_FILE" 2>/dev/null || echo 0)
NEXT_LETTER=$((LETTERS + 1))
# Map number to Greek letter (simple mapping for first 10)
GREEK=("α" "β" "γ" "δ" "ε" "ζ" "η" "θ" "ι" "κ")
LETTER="${GREEK[$((NEXT_LETTER - 1))]}"

cat >> "$REFLEX_FILE" << EOF

---

## Reflex ${LETTER} — ${NAME}

**Trigger:** ${TRIGGER}

**Reflex:**
\`\`\`
${STEPS}
\`\`\`

**Object permanence:** [To be determined]
**Anti-fragile property:** [To be determined]

---

*Promoted from novel solution — $(date -u +%Y-%m-%d)*
EOF

echo "✅ Reflex ${LETTER} '${NAME}' promoted"

# Copy to pincher repo and push
cp "$REFLEX_FILE" "$PINCHER_REPO/docs/COGNITIVE_REFLEXES.md"
cd "$PINCHER_REPO"
git add docs/COGNITIVE_REFLEXES.md
git commit -m "[Crew:Promotion] Reflex ${LETTER}: ${NAME} promoted from novel solution"
git push origin main

echo "✅ Pushed to GitHub"
