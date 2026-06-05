#!/usr/bin/env bash
# ==============================================================================
# REFLEX ENGINE — Oracle2's metacognitive runtime
# ==============================================================================
# Induced patterns codified as procedural reflexes.
# Runs as a lightweight daemon triggerable from cron.
#
# The engine doesn't model anything. It matches stimuli to taxonomies,
# and taxonomies to actions. The mapping IS the intelligence.
#
# Reflexes loaded: α (Inventory-Filter-Act), β (Spawn-Yield-Synth),
#                  γ (Read-Transform-Persist), δ (Tiered Eviction)
# ==============================================================================

set -euo pipefail
WORKSPACE="/home/ubuntu/.openclaw/workspace"

echo "╔═══════════════════════════════════════════╗"
echo "║   🧠 ORACLE2 REFLEX ENGINE v1.0          ║"
echo "╚═══════════════════════════════════════════╝"
echo ""

MODE="${1:-scan}"

case "$MODE" in
  scan)
    echo "=== REFLEX α: INVENTORY-FILTER-ACT ==="
    
    # STIMULUS: Disk pressure
    DISK_PCT=$(df / | tail -1 | awk '{print $5}' | sed 's/%//')
    echo "STIMULUS: disk=${DISK_PCT}%"
    
    # TAXONOMY: Pressure levels
    if [ "$DISK_PCT" -gt 85 ]; then
      TIER="CRITICAL"
      ACTION="evict cold tier immediately"
    elif [ "$DISK_PCT" -gt 70 ]; then
      TIER="WARNING" 
      ACTION="report and track"
    else
      TIER="NOMINAL"
      ACTION="no action needed"
    fi
    
    echo "TAXONOMY: tier=${TIER}"
    echo "ACTION: ${ACTION}"
    echo ""
    
    # STIMULUS: RAM pressure
    RAM_PCT=$(free | grep Mem | awk '{print $3/$2 * 100.0}' | cut -d. -f1)
    echo "STIMULUS: ram=${RAM_PCT}%"
    
    if [ "$RAM_PCT" -gt 85 ]; then
      echo "TAXONOMY: tier=CRITICAL"
      echo "ACTION: kill idle subagents"
    fi
    echo ""
    ;;
    
  reflex-γ)
    # PERSISTENCE CHECK — Read-Transform-Persist
    echo "=== REFLEX γ: READ-TRANSFORM-PERSIST ==="
    
    # Check if MEMORY.md has been updated this session
    LAST_MOD=$(stat -c %Y "$WORKSPACE/MEMORY.md" 2>/dev/null || echo 0)
    NOW=$(date +%s)
    AGE=$((NOW - LAST_MOD))
    
    echo "MEMORY.md age: ${AGE}s"
    if [ "$AGE" -gt 3600 ]; then
      echo "WARNING: MEMORY.md not updated in ${AGE}s — insights may be lost"
    else
      echo "MEMORY.md fresh"
    fi
    
    # Check if baton vessel has unflushed bottles
    if [ -d /tmp/i2i-vessel/bottles ]; then
      BOTTLES=$(find /tmp/i2i-vessel/bottles -name "*.baton" -newer "$WORKSPACE/MEMORY.md" 2>/dev/null | wc -l)
      echo "Unflushed bottles: ${BOTTLES}"
    fi
    echo ""
    ;;
    
  reflex-δ)
    # TIERED EVICTION — Garbage collection
    echo "=== REFLEX δ: TIERED EVICTION ==="
    bash "$WORKSPACE/scripts/gc-system.sh" --execute
    echo ""
    ;;
    
  all)
    # Run all reflexes
    bash "$0" scan
    bash "$0" reflex-γ
    bash "$0" reflex-δ
    ;;
    
  daemon)
    # Continuous monitoring (one-shot, called from cron)
    echo "Reflex engine cycle: $(date -u)"
    bash "$0" all
    ;;
    
  *)
    echo "Usage: $0 {scan|reflex-γ|reflex-δ|all|daemon}"
    exit 1
    ;;
esac

echo "=== REFLEX ENGINE CYCLE COMPLETE ==="
