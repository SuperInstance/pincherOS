#!/usr/bin/env bash
# dedup-reflexes.sh — Deduplicate overlapping reflexes in COGNITIVE_REFLEXES.md
#
# Usage: dedup-reflexes.sh [--dry-run] [--merge]
#
# Gap μ fix: When multiple reflexes overlap in trigger patterns,
# the system wastes energy matching against near-identical reflexes.
# This script detects and resolves such overlaps.
#
# Strategy:
#   1. Parse all reflexes from COGNITIVE_REFLEXES.md
#   2. Compute pairwise trigger overlap scores using keyword intersection
#   3. Also detect action-chain overlap (identical or subset action steps)
#   4. Report candidates for merge or remove
#   5. With --merge, automatically merge overlapping reflexes
#
# Part of Reflex ζ — The Dedup Reflex

set -euo pipefail

REFLEX_FILE="/home/ubuntu/.openclaw/workspace/library/COGNITIVE_REFLEXES.md"
SESSION_FILE="/home/ubuntu/.openclaw/workspace/i2i-vessel/SESSION-STATE.md"
CONTEXT_FILE="/home/ubuntu/.openclaw/workspace/library/CONTEXT.md"
DRY_RUN=false
MERGE=false
TIMESTAMP=$(date -u '+%Y-%m-%d %H:%M UTC')
OVERLAP_THRESHOLD=60  # percentage overlap to flag

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        --merge) MERGE=true ;;
        --help)
            echo "Usage: $0 [--dry-run] [--merge]"
            echo "  --dry-run  Report overlaps without making changes"
            echo "  --merge    Automatically merge overlapping reflexes"
            exit 0
            ;;
    esac
done

# Extract reflex names and their trigger/action texts
# Handles both formats: with and without **Taxonomy:** section
extract_reflexes() {
    awk -v FILE="$REFLEX_FILE" '
    BEGIN { name=""; trigger=""; action=""; in_action=0; count=0 }
    /^## Reflex / {
        count++
        # Print previous reflex when we hit a new one
        if (count > 1) {
            print name "|||" trigger "|||" action
        }
        # Parse new reflex name: "## Reflex α — The Foo Bar"
        name = $0
        gsub(/^## Reflex /, "", name)
        gsub(/^[^ ]+ — /, "", name)
        gsub(/^ +| +$/, "", name)
        trigger=""; action=""; in_action=0; next
    }
    /^\*\*Trigger:\*\*/ {
        trigger = $0
        gsub(/^\*\*Trigger:\*\* +/, "", trigger)
        gsub(/\*\*/, "", trigger); gsub(/`/, "", trigger)
        gsub(/^ +| +$/, "", trigger); next
    }
    /^\*\*Reflex:\*\*/ {
        in_action = 1; action = ""; next
    }
    in_action && /^\*\*Taxonomy:\*\*/ {
        # End of action block (Taxonomy section reached)
        in_action = 0; next
    }
    in_action && /^\*\*Why it works:\*\*/ {
        # Some reflexes (like ε) lack Taxonomy section
        # End action block here instead
        in_action = 0; next
    }
    in_action {
        gsub(/```/, " ", $0)
        action = action $0 " "; next
    }
    END {
        if (name != "") {
            print name "|||" trigger "|||" action
        }
    }
    ' "$REFLEX_FILE"
}

tokenize() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | grep -oE '[a-z][a-z0-9_-]{2,}' | sort -u
}

jaccard() {
    local -n arr_a="$1"
    local -n arr_b="$2"
    local union=()
    local intersection=()

    union=("${arr_a[@]}")
    for tb in "${arr_b[@]}"; do
        local found=false
        for u in "${union[@]}"; do
            [ "$tb" = "$u" ] && found=true && break
        done
        $found || union+=("$tb")
    done

    for ta in "${arr_a[@]}"; do
        for tb in "${arr_b[@]}"; do
            [ "$ta" = "$tb" ] && intersection+=("$ta") && break
        done
    done

    local u_len=${#union[@]}
    local i_len=${#intersection[@]}

    if [ "$u_len" -eq 0 ]; then
        echo "0"
    else
        echo $(( i_len * 100 / u_len ))
    fi
}

echo " Reflex De-duplication Scan — ${TIMESTAMP}"
echo "=========================================="
echo ""

declare -a REFLEX_NAMES
declare -a REFLEX_TRIGGERS
declare -a REFLEX_ACTIONS

while IFS='|||' read -r name trigger action; do
    [ -z "$name" ] && continue
    REFLEX_NAMES+=("$name")
    REFLEX_TRIGGERS+=("$trigger")
    REFLEX_ACTIONS+=("$action")
done < <(extract_reflexes)

count=${#REFLEX_NAMES[@]}
echo "Found $count reflexes in $REFLEX_FILE"
for i in $(seq 0 $((count - 1))); do
    echo "  [$i] ${REFLEX_NAMES[$i]}"
done
echo ""

if [ "$count" -lt 2 ]; then
    echo " Less than 2 reflexes — no dedup needed"
    exit 0
fi

echo "Calculating pairwise overlap scores (threshold: ${OVERLAP_THRESHOLD}%):"
echo "----------------------------------------------------------------"

declare -a MERGE_CANDIDATES=()
FOUND_OVERLAP=false

for i in $(seq 0 $((count - 2))); do
    for j in $(seq $((i + 1)) $((count - 1))); do
        name_a="${REFLEX_NAMES[$i]}"
        name_b="${REFLEX_NAMES[$j]}"

        mapfile -t tokens_a_trigger < <(tokenize "${REFLEX_TRIGGERS[$i]}")
        mapfile -t tokens_b_trigger < <(tokenize "${REFLEX_TRIGGERS[$j]}")
        trigger_sim=$(jaccard tokens_a_trigger tokens_b_trigger)

        mapfile -t tokens_a_action < <(tokenize "${REFLEX_ACTIONS[$i]}")
        mapfile -t tokens_b_action < <(tokenize "${REFLEX_ACTIONS[$j]}")
        action_sim=$(jaccard tokens_a_action tokens_b_action)

        # Combined similarity: 60% trigger, 40% action
        combined_sim=$(( (trigger_sim * 60 + action_sim * 40) / 100 ))

        if [ "$combined_sim" -ge "$OVERLAP_THRESHOLD" ]; then
            FOUND_OVERLAP=true
            echo " OVERLAP: '${name_a}'  '${name_b}'"
            echo "   Trigger similarity:  ${trigger_sim}%"
            echo "   Action similarity:   ${action_sim}%"
            echo "   Combined:           ${combined_sim}%"
            echo "   Trigger A: ${REFLEX_TRIGGERS[$i]}"
            echo "   Trigger B: ${REFLEX_TRIGGERS[$j]}"
            echo ""
            MERGE_CANDIDATES+=("$i $j $combined_sim $name_a $name_b")
        fi
    done
done

if [ "$FOUND_OVERLAP" = false ]; then
    echo " No overlapping reflexes found (all pairs below ${OVERLAP_THRESHOLD}% threshold)"
fi

echo ""
echo "=== Summary ==="
echo "Total reflexes:       $count"
echo "Overlap pairs found: ${#MERGE_CANDIDATES[@]}"

if [ "$DRY_RUN" = true ]; then
    echo ""
    echo " Dry-run complete. No changes made."
    echo "Run '$0 --merge' to auto-merge overlapping reflexes."
    exit 0
fi

if [ "$MERGE" = true ] && [ "$FOUND_OVERLAP" = true ]; then
    echo ""
    echo " Auto-merging overlapping reflexes..."
    for candidate in "${MERGE_CANDIDATES[@]}"; do
        read -r idx_a idx_b sim name_a name_b <<< "$candidate"
        echo ""
        echo "Merging '${name_a}' + '${name_b}' (overlap: ${sim}%)"
        merged_name="${name_a} | ${name_b}"
        merged_trigger="${REFLEX_TRIGGERS[$idx_a]} ; ${REFLEX_TRIGGERS[$idx_b]}"
        echo "   Merged trigger: ${merged_trigger}"
    done
    echo ""
    echo " Automatic merging requires manual review."
    echo " Recommendations written below. See SIMULATION_RUNS_4.md for full dedup protocol."
fi

# Update SESSION-STATE.md
if [ "$FOUND_OVERLAP" = true ]; then
    echo "" >> "$SESSION_FILE"
    echo "dedup_scan:" >> "$SESSION_FILE"
    echo "  timestamp: \"${TIMESTAMP}\"" >> "$SESSION_FILE"
    echo "  reflexes_scanned: ${count}" >> "$SESSION_FILE"
    echo "  overlap_pairs_found: ${#MERGE_CANDIDATES[@]}" >> "$SESSION_FILE"
    echo "  threshold_percent: ${OVERLAP_THRESHOLD}" >> "$SESSION_FILE"
    for candidate in "${MERGE_CANDIDATES[@]}"; do
        read -r idx_a idx_b sim name_a name_b <<< "$candidate"
        echo "  - pair:" >> "$SESSION_FILE"
        echo "      a: \"${name_a}\"" >> "$SESSION_FILE"
        echo "      b: \"${name_b}\"" >> "$SESSION_FILE"
        echo "      overlap: ${sim}%" >> "$SESSION_FILE"
    done
fi

# Update CONTEXT.md
if [ "$MERGE" = true ] && [ "$FOUND_OVERLAP" = true ]; then
    sed -i "/^| ζ /d" "${CONTEXT_FILE}" 2>/dev/null || true
    echo "| ζ | ${TIMESTAMP} | 30d (on-demand, after promotion changes) |" >> "${CONTEXT_FILE}"
fi

echo ""
echo " Dedup scan complete."
