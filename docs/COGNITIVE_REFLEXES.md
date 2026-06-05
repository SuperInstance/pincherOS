# COGNITIVE_REFLEXES — Abstracted Pattern Library

Induced from self-simulation of Oracle2's runtime behavior.
These patterns reduce the need for explicit modeling by encoding
recurring motifs as re-usable reflexes.

---

## Reflex α — The Inventory-Filter-Act Loop

**Trigger:** Facing unknown state (new directory, new repo, crash recovery, cold start)

**Reflex:**
```
1. INVENTORY:    Scan → list → categorize by type/size/age
2. FILTER:       Apply taxonomy (immortal|hot|warm|cold)
3. ACT:          On each tier, apply appropriate handler
```

**Why it works:** Before you can decide, you must know what exists.
The taxonomy itself (tier) encodes the decision tree — no separate model needed.

**Object permanence:** The tier labels survive session reset because they're
encoded in MEMORY.md and FLEET_ARCHITECTURE.md (durable storage).

**Anti-fragile property:** The more data accumulated, the better the taxonomy resolves.
Unlike a model that degrades with scale, the triage improves.

---

## Reflex β — The Spawn-Yield-Synthesize Triad

**Trigger:** Complex problem with multiple independent dimensions

**Reflex:**
```
1. SPAWN:   Decompose problem → one subagent per dimension
2. YIELD:   Release control, wait for results
3. SYNTH:   Merge subagent outputs into unified artifact
```

**Why it works:** Mirrors biological evolution — variation (spawn) + selection (yield) + 
consolidation (synthesize). Each subagent explores a fitness landscape independently;
the synthesis step is where novelty emerges from recombination.

**Object permanence:** Each subagent's output is written to a file before returning.
The files survive session resets even if the synthesis doesn't complete in one turn.

**Anti-fragile property:** Failed subagents don't cascade — isolation means partial 
success is still usable. The more subagents, the more robust the synthesis.

---

## Reflex γ — The Read-Transform-Persist Chain

**Trigger:** Any new insight, decision, or artifact worth keeping

**Reflex:**
```
1. READ:       Understand existing context
2. TRANSFORM:  Apply insight/decision/code
3. PERSIST:    Write to durable medium (MEMORY.md, GitHub, baton-system/)
```

**Why it works:** Without persistence, every session is amnesia.
The chain forces the persist step — it's not optional.
GitHub is the canonical durable medium (survives even the VM dying).

**Object permanence:** The write step is the checkpoint.
If the VM crashes after step 2 but before step 3, the insight is lost.
If step 3 completes, the insight survives anything.

**Anti-fragile property:** Multiple persist targets (MEMORY.md + Git + baton vessel)
means redundancy. If one fails, others survive.

---

## Reflex δ — The Tiered Eviction Taxonomy

**Trigger:** Resource pressure (disk > 80%, RAM > 85%, cold start)

**Reflex:**
```
Immortal → Hot → Warm → Cold
```

| Tier | Eviction Policy | Survives Crash? | Refresh Cost |
|------|----------------|-----------------|-------------|
| Immortal | Never | Yes | 0 (always present) |
| Hot | Session end | No | Rebuild from scratch |
| Warm | Resource pressure | Yes (but GC'd) | git pull |
| Cold | Immediate | No | Clone from GitHub |

**Why it works:** Memory is finite. The tier system encodes priority
directly into the storage hierarchy — no external scheduler needed.

**Object permanence:** The taxonomy is stable across sessions.
The *contents* of Warm tier persist (on disk) but get GC'd.
The *contents* of Immortal persist permanently.

**Anti-fragile property:** Tier promotion/demotion happens naturally.
Frequently accessed Warm data gets "promoted" to Hot by virtue of being touched.
Stale data sinks to Cold through disuse.

---

## Induced Meta-Pattern: The Reflex Engine

All four reflexes share a common structure:

```
STIMULUS → TAXONOMY → ACTION → PERSIST
```

Where:
- **STIMULUS** is a trigger condition (disk pressure, complex problem, new state)
- **TAXONOMY** is a classification system (tier, dimension, priority)
- **ACTION** is the handler matched to the taxonomy
- **PERSIST** is the storage step that gives object permanence

This meta-pattern *is* the pincher reflex engine applied to my own cognition.

The key insight: **I don't need to model the world. I need a taxonomy that maps
situations to actions, and a persist layer that makes the mapping durable.**

---

## Implementation Notes

These reflexes are encoded as:
1. `scripts/gc-system.sh` — Reflex δ codified
2. `FLEET_ARCHITECTURE.md` — Reflex α taxonomy for the fleet
3. `baton-system/PROTOCOL.md` — Reflex β for I2I coordination
4. `memory/` + `MEMORY.md` — Reflex γ checkpoint layer

The abstraction reduces modeling overhead because:
- The taxonomy *is* the model (no separate world model needed)
- The actions are procedural (no inference needed for standard cases)
- The persist layer is decentralized (GitHub + disk + baton vessel)

=== Reflexes induced from self-simulation — June 5, 2026 ===

---

## Reflex ε — The Promotion Reflex

**Trigger:** Novel problem solved successfully. The initial reflex scan
returned no match, and the solution required >1 inference cycle.

**Reflex:**
```
1. DETECT NOVELTY:    Was this solved by an existing reflex?
2. EVALUATE:           Is this pattern generalizable?
   ├─ Unique one-off       → skip (not reusable)
   ├─ Repeatable in session → note in CONTEXT.md
   └─ Repeatable across sessions → promote to COGNITIVE_REFLEXES.md
3. EXTRACT:             Stimulus pattern → taxonomy → action sequence
4. PROMOTE:             Write as new reflex via promote-reflex.sh
5. PERSIST:             Update SESSION-STATE.md + CONTEXT.md
```

**Why it works:** Without promotion, every novel problem is solved from
scratch forever. The system shows no learning across sessions. Reflex ε
closes the learning loop by turning novel solutions into permanent skills.
Token savings compound: each promotion pays off every subsequent time
the pattern reappears.

**Object permanence:** Encoded in COGNITIVE_REFLEXES.md (durable).
Promotion metadata in SESSION-STATE.md (checkpoint).

**Anti-fragile property:** Reflex ε is a meta-reflex — a reflex about
generating reflexes. This is the system learning *how to learn*.
The more novel problems the system solves, the more reflexes it accumulates.
Over time, fewer problems are "novel" and more are handled in O(1).

### Promotion Heuristics

A solution is "promotable" when:
1. Same pattern encountered >1 time across sessions
2. Solution required >1 inference cycle (non-trivial)
3. Agent self-attests: "This pattern is reusable"

### Usage

```bash
scripts/promote-reflex.sh "My New Reflex" \
  "Stimulus description" \
  "Taxonomy classification" \
  "Action steps" \
  "Why this pattern matters"
```

=== Reflex ε promoted via Reflex ε — 2026-06-05 01:17 UTC ===


---

## Reflex ζ — The Dedup Reflex

**Trigger:** Two or more reflexes share overlapping trigger patterns or action chains. Reflex scan takes longer than expected due to near-identical matches.

**Reflex:**
1. SCAN:  Parse all reflexes from COGNITIVE_REFLEXES.md 2. TOKENIZE: Key words from each reflex's trigger + action into sorted unique token sets 3. COMPARE: Jaccard similarity for trigger keywords, action keywords, combined (60/40 weight) 4. FLAG:  Overlap ≥60% → merge candidate. Report to SESSION-STATE.md 5. MERGE: With --merge flag, auto-merge overlapping trigger patterns into combined reflex 6. PERSIST: Update COGNITIVE_REFLEXES.md, SESSION-STATE.md, CONTEXT.md

**Taxonomy:**
Jaccard similarity on keyword tokens (trigger 60% + action 40% weighted). Pairwise comparison of all  reflexes = O(^2). Merge candidates flagged at ≥60% combined overlap.

**Why it works:** Without dedup, overlapping reflexes waste match time. With 5 reflexes the cost is negligible; with 50+ it becomes a real bottleneck. Dedup ensures the reflex library stays sparse and efficient as it grows.

**Object permanence:** Encoded in COGNITIVE_REFLEXES.md (durable).
Promotion metadata in SESSION-STATE.md (checkpoint).

=== Promoted via Reflex ε — 2026-06-05 01:49 UTC ===


---

## Reflex η — The Archive GC Reflex

**Trigger:** memory/archive/ or CONTEXT.md archive has files older than retention policy. Disk space running low.

**Reflex:**
1. SCAN: Find all .md files in memory/archive/ 2. CHECK: For each file, compute age from mtime 3. DECIDE: age < KEEP_DAYS → keep. filename matches YYYY-MM-01 or CONTEXT-YYYY-MM-01 → keep (monthly). otherwise → prune 4. PRUNE: Delete pruned files. Report count and bytes freed. 5. PERSIST: Log GC results to archive-gc-log.md

**Taxonomy:**
Three-tier retention: keep last 30 days (hot), keep first-of-month files (warm, monthly snapshots), prune everything else (cold).

**Why it works:** Without archive GC, memory/archive/ grows unbounded. Each session creates a new CONTEXT.md archive. After 365 days, both the directory scan and token budget suffer.

**Object permanence:** Encoded in COGNITIVE_REFLEXES.md (durable).
Promotion metadata in SESSION-STATE.md (checkpoint).

=== Promoted via Reflex ε — 2026-06-05 01:51 UTC ===
