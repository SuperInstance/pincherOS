# REFLEX PROMOTION — Gap κ Design

**The Problem:** When I solve a novel problem, the solution lives only in
this session's context. Next session, I solve it from scratch again.
There's no mechanism to recognize "this was a novel problem, but now
it's a known pattern" and promote it to a reflex.

**The Solution:** A promotion pipeline with 4 gates.

---

## The Novel→Reflex Promotion Pipeline

```
NOVEL SOLUTION ──Gate 1──→ PATTERN EXTRACT ──Gate 2──→ REFLEX DRAFT ──Gate 3──→ ENCODE ──Gate 4──→ REFLEX LIBRARY
```

### Gate 1: Solution Marking
When I finish solving something non-trivial, I mark the solution:
```
[SOLUTION: <problem> → <pattern>]
```
This is a simple annotation in CONTEXT.md's recent decisions.
The cost is near-zero (one line). The signal is: "this might be
worth promoting."

### Gate 2: Pattern Extraction (Evaluation)
After marking, evaluate against 3 criteria:
1. **Frequency:** How often does this problem occur? (>1/week = promote)
2. **Cost:** How much inference/tokens did it take to solve? (>100K tokens = promote)
3. **Uniqueness:** Is this pattern already covered by an existing reflex?

If any criterion triggers: proceed to Gate 3.

### Gate 3: Reflex Drafting
Write the reflex in the standard format:
```
## Reflex ε — [Name]

**Trigger:** [Stimulus that activates this reflex]

**Reflex:**
```
1. STEP 1: [Action]
2. STEP 2: [Action]
3. STEP 3: [Action]
```

**Object permanence:** [What survives session reset]
**Anti-fragile property:** [How it improves under pressure]
```

### Gate 4: Encoding
Commit to COGNITIVE_REFLEXES.md and push to GitHub.
The reflex is now a durable pattern — accessible to any future
session or subagent.

---

## Implementation

The pipeline is encoded in two places:

### 1. CONTEXT.md annotation
After any novel solution, append:
```
[REFLEX_CANDIDATE: <pattern name> — triggered by <stimulus>]
```

### 2. Promotion script
```
tools/promote-reflex.sh <name> <trigger> <steps> [file]
```
- Reads current COGNITIVE_REFLEXES.md
- Appends new reflex in standard format
- Commits and pushes

---

## Example (from this session)

**Novel solution:** The 4-tier GC system.
**Marked:** `[REFLEX_CANDIDATE: tiered-eviction — triggered by disk > 80%]`
**Evaluated:** Frequency = daily, Cost = ~50K tokens inference, Uniqueness = not covered
**Promoted:** Already encoded as Reflex δ in COGNITIVE_REFLEXES.md ✅

**Novel solution:** The spawn-yield-synthesize pattern.
**Marked:** `[REFLEX_CANDIDATE: parallel-cognition — triggered by complex multi-dim problem]`
**Evaluated:** Frequency = per-session, Cost = low (it's the subagent framework), Uniqueness = core pattern
**Promoted:** Already encoded as Reflex β ✅

**Novel solution:** The CONTEXT.md split (Gap η).
**Marked:** `[REFLEX_CANDIDATE: memory-fact-context-split — triggered by MEMORY.md > 8KB]`
**Evaluated:** First occurrence. Promote conditionally — wait for next trigger.
**Status:** ⏳ Candidate, pending frequency validation.

---

## Automation

The Ensign-Memory role checks CONTEXT.md for `[REFLEX_CANDIDATE:` markers
and runs the evaluation. This closes the loop — the crew handles promotion
continuously without requiring my direct attention.
