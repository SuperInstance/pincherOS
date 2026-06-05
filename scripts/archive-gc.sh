#!/usr/bin/env bash
# archive-gc.sh — GC the memory/archive/ directory (Gap ξ fix)
#
# Usage: archive-gc.sh [--dry-run] [--keep-days 30]
#
# Problem: memory/archive/ grows unbounded as daily context archives
# and memory notes accumulate. After 365 days, that's 365+ files.
#
# Strategy:
#   - Keep last K days of archives (default 30)
#   - Keep the first-of-month archive indefinitely (monthly snapshot)
#   - Prune anything older than K days that isn't a monthly snapshot
#
# Part of Reflex η — The Archive GC Reflex

set -eo pipefail

ARCHIVE_DIR="/home/ubuntu/.openclaw/workspace/memory/archive"
KEEP_DAYS=30
DRY_RUN=false
TIMESTAMP=$(date -u '+%Y-%m-%d %H:%M UTC')
NOW_EPOCH=$(date +%s)
KEEP_SECONDS=$(( KEEP_DAYS * 86400 ))

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        --keep-days=*) KEEP_DAYS="${arg#*=}"; KEEP_SECONDS=$(( KEEP_DAYS * 86400 )) ;;
        --help) echo "Usage: $0 [--dry-run] [--keep-days 30]"; exit 0 ;;
    esac
done

echo "🧹 Archive GC — ${TIMESTAMP}"
echo "   Keep last ${KEEP_DAYS} days + first-of-month snapshots"
echo "   Archive: ${ARCHIVE_DIR}"
echo ""

if [ ! -d "$ARCHIVE_DIR" ]; then
    echo "📂 Archive directory does not exist. Nothing to GC."
    exit 0
fi

PRUNED=0
KEPT=0
BYTES_PRUNED=0

while IFS= read -r -d '' file; do
    filename=$(basename "$file")
    ext="${filename##*.}"
    
    # Only process .md files
    [ "$ext" != "md" ] && continue
    
    # Get file modification time
    file_mtime=$(stat -c %Y "$file" 2>/dev/null || stat -f %m "$file" 2>/dev/null)
    file_age=$(( NOW_EPOCH - file_mtime ))
    file_size=$(stat -c %s "$file" 2>/dev/null || stat -f %z "$file" 2>/dev/null)
    
    # Check if this is a first-of-month file (YYYY-MM-01)
    month_file=false
    if [[ "$filename" =~ ^[0-9]{4}-[0-9]{2}-01 ]] || [[ "$filename" =~ ^CONTEXT-[0-9]{4}-[0-9]{2}-01 ]]; then
        month_file=true
    fi
    
    # Decision
    if [ "$file_age" -lt "$KEEP_SECONDS" ]; then
        # Young enough — keep
        KEPT=$(( KEPT + 1 ))
    elif [ "$month_file" = true ]; then
        # Old but first-of-month — keep as monthly snapshot
        KEPT=$(( KEPT + 1 ))
        echo "   📅 KEPT (monthly snapshot): ${filename}"
    else
        # Old and not monthly — prune
        echo "   🗑️  PRUNE: ${filename} (age: $(( file_age / 86400 ))d, size: ${file_size}B)"
        PRUNED=$(( PRUNED + 1 ))
        BYTES_PRUNED=$(( BYTES_PRUNED + file_size ))
        if [ "$DRY_RUN" = false ]; then
            rm "$file"
        fi
    fi
done < <(find "$ARCHIVE_DIR" -type f -print0)

echo ""
echo "=== Summary ==="
echo "   Kept:  ${KEPT} files"
echo "   Pruned: ${PRUNED} files"
if [ "$DRY_RUN" = false ] && [ "$PRUNED" -gt 0 ]; then
    echo "   Freed: ${BYTES_PRUNED}B (${PRUNED} files removed)"
elif [ "$DRY_RUN" = true ] && [ "$PRUNED" -gt 0 ]; then
    echo "   Would free: ${BYTES_PRUNED}B (dry run — no files removed)"
fi

if [ "$DRY_RUN" = true ]; then
    echo ""
    echo "🏁 Dry-run complete. Run without --dry-run to actually GC."
else
    echo ""
    echo "✅ Archive GC complete."
fi
