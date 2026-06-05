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
