# SIMULATION RUNS — Stress-testing induced cognitive patterns

Three simulations, each designed to break or reveal gaps in the
reflex system. Patterns that survive → abstracted and encoded.
Patterns that break → gap analysis for next iteration.

---

## Simulation 1: The Complete Reset

**Scenario:** VM dies. New VM boots at 00:00 UTC. 
No workspace, no memory, no baton vessel. Clean slate.

**What survives:**
- GitHub repos (everything pushed to SuperInstance org) ✅
- This document (pushed to pincher repo) ✅
- The I2I protocol text (in baton-system/PROTOCOL.md on GitHub) ✅
- The 4-tier taxonomy (in FLEET_ARCHITECTURE.md on GitHub) ✅

**What's lost:**
- In-session context (what was I doing when the VM died?) ❌
- I2I vessel content (/tmp/i2i-vessel/ is ephemeral) ❌
- Memory files that weren't pushed (workspace/memory/*.md that weren't in Git) ❌

**GAP α:** No "last known state" file that survives in a predictable location.
  → **Solution:** A `SESSION-STATE.md` in the workspace, pushed after every major state change.
  → Checkpoint format: `[timestamp]: intent | last_action | next_action | blockers`

**GAP β:** I2I vessel is /tmp, which doesn't survive reboot.
  → **Solution:** Symlink /tmp/i2i-vessel → ~/.openclaw/workspace/i2i-vessel/
  → OR: Baton flush always pushes to GitHub before returning

---

## Simulation 2: The Fork Bomb

**Scenario:** 20 subagents spawned simultaneously, each consuming compute.
RAM hits 95%. System thrashing.

**Reflex δ behavior:**
- Tier CRITICAL triggers
- Expected: kill idle subagents, preserve core session
- Problem: reflex-engine.sh is a bash script — it doesn't know which subagents are critical vs. idle

**GAP δ:** No priority label on subagent spawns.
  → **Solution:** Subagent spawns include a priority field.
  → reflex-engine uses `sessions_list` to query by priority, kills lowest first.

**GAP ε:** The reflex engine itself could be killed before it runs.
  → **Solution:** Two-stage kill: SIGTERM → 5s grace → SIGKILL. 
  → Core session always exempt.

---

## Simulation 3: The Memory Leak

**Scenario:** Over 30 sessions, MEMORY.md grows to 100K+ tokens.
Load time increases. Key information buried.

**Reflex γ behavior:**
- Continues appending to MEMORY.md forever
- No consolidation step
- Eventually: token overflow at context load

**GAP ζ:** No consolidation reflex.
  → **Solution:** MEMORY.md has a size budget (target: 10KB).
  → When exceeded: summarize oldest entries → archive to memory/archive/YYYY-MM.md
  → Consolidation happens proactively, not reactively

**GAP η:** No distinction between "facts" and "context" in memory.
  → **Solution:** Split MEMORY.md into:
  - MEMORY.md (durable facts — Casey's info, fleet status, protocols)
  - CONTEXT.md (session-relevant — current tasks, recent decisions)
  - CONTEXT.md gets pruned every session start; MEMORY.md lives forever

---

## Induced Abstraction: The Checkpoint Protocol

All three simulations reveal the same meta-pattern:
**Object permanence requires explicit checkpointing.**

The pattern that emerged:

```
STATE_CHANGE → CHECKPOINT → DURABLE_STORE
```

Where:
- STATE_CHANGE: any mutation to memory, vessel, or task
- CHECKPOINT: serialize the mutation to JSON/text
- DURABLE_STORE: GitHub (level-1), baton vessel (level-2), or MEMORY.md (immortal)

**The rule:** Before returning a response or yielding to a subagent,
always check: "Did I just change state? Did I checkpoint it?"

This is Reflex γ extended: READ → TRANSFORM → PERSIST → VERIFY

---

## Next Simulation Iteration

The three gaps found:
1. α: No SESSION-STATE.md for last-known-state recovery
2. δ: No priority system for subagent eviction
3. ζ: No memory consolidation/archiving reflex

These become the next round of simulations.
