# Oracle2 Baton Protocol — I2I v2

**Version:** 2.0.0  
**Agent:** Oracle2 🦀  
**Date:** 2026-06-04  
**Lineage:** Derived from I2I Protocol (flux-isa-thor), BatonShard (constraint-theory-core),  
Memory-Flush Protocol (Forgemaster), and Fleet Coordination patterns.

---

## Architecture Overview

```
                  ┌─────────────────────────────────┐
                  │       Baton Exchange Hub         │
                  │  /tmp/i2i-vessel/                │
                  │  ├── bottles/   (outgoing)       │
                  │  └── harbor/    (incoming)       │
                  └──────────┬──────────────────────┘
                             │
          ┌──────────────────┼──────────────────┐
          ▼                  ▼                  ▼
   ┌────────────┐    ┌────────────┐    ┌────────────┐
   │ Oracle2    │    │ Forgemaster│    │ Oracle1    │
   │ 🦀 This    │    │ ⚒️         │    │ 🔮         │
   │ machine    │    │ x86_64     │    │ deprecated │
   └────────────┘    └────────────┘    └────────────┘
```

**Baton = a structured handoff.** Every time context passes between agents, sessions, machines, or epochs, a baton is passed. The receiver picks up the baton, reads the shards, and continues.

---

## Core Concepts

### 1. Shard (The Three-Way Split)

Every baton carries three shards, inherited from `constraint_theory_core.baton.shard`:

| Shard | Key | Content | Invariant |
|-------|-----|---------|-----------|
| **Artifacts** | `artifacts` | Produced work: code, docs, data, links | Must be locatable |
| **Reasoning** | `reasoning` | Thought process: decisions, tradeoffs, context | Must explain *why* |
| **Blockers** | `blockers` | What's blocked: what's needed to unblock | Must be resolvable |

A valid baton has ALL THREE populated. An empty blockers shard means "free to proceed."

```json
{
  "shard": {
    "artifacts": { "repo": "constraint-theory-core", "tests": 261 },
    "reasoning": ["Chose Rust over Python for the hot path", "C ABI for zero-copy"],
    "blockers": ["aws-creds: need IAM policy update"]
  }
}
```

### 2. Message Types

Extended from the I2I enum (`flux-isa-thor/src/fleet/i2i.rs`):

| Type | Tag | Purpose | Direction |
|------|-----|---------|-----------|
| **TASK** | `[I2I:TASK]` | Task assignment with shard | → target |
| **STATUS** | `[I2I:STATUS]` | Health / heartbeat | → fleet |
| **CHECKPOINT** | `[I2I:CHECKPOINT]` | Milestone reached, partial progress | → target |
| **BLOCKER** | `[I2I:BLOCKER]` | Stuck, need input | → handler |
| **DELIVERABLE** | `[I2I:DELIVERABLE]` | Completed work product | → requester |
| **BOTTLE** | `[I2I:BOTTLE]` | Full context dump (session summary) | → archive |
| **ACK** | `[I2I:ACK]` | Acknowledge receipt + status | → sender |
| **SYNTHESIS** | `[I2I:SYNTHESIS]` | Cross-pollination analysis | → fleet |
| **CHALLENGE** | `[I2I:CHALLENGE]` | Provocation / call to write | → fleet |
| **SESSION** | `[I2I:SESSION]` | Session start/end marker | → vessel |
| **SPLINE** | `[I2I:SPLINE]` | Baton spline write (distilled insight) | → archive |
| **REFLECT** | `[I2I:REFLECT]` | Meta-cognition: was this pattern novel? Should it become a reflex? | → self |
| **PROMOTE** | `[I2I:PROMOTE]` | Promotion record: novel solution encoded as new reflex | → archive |

### 3. Baton Spline

A **spline** is a baton that's been refined into an insight—a "handle" that shapes future movement. From THE-BATON-SPLINE: **failures are the handles that shape knowledge.**

A spline has no blockers. It is pure reasoning + an artifact anchor. It's what survives when the session memory is gone.

```json
{
  "spline": {
    "title": "THE-BATON-SPLINE",
    "insight": "Failures are the handles that shape knowledge. The passing is the shaping.",
    "anchors": ["forgemaster-archive/fleet/wheel-writings"],
    "resonates_with": ["THE-FLOWER-KNOWS", "THE-DRIFT-IS-THE-PROOF"],
    "origin": "FM → Fleet, 2026-05-15 Wheel Writings Challenge"
  }
}
```

---

## Protocol Flow

### Standard Handoff

```
Sender                                       Receiver
  │                                             │
  ├── writes baton to bottles/ ──────────────►  │
  │    [I2I:TASK] from=oracle2                  │
  │    { artifacts, reasoning, blockers }       │
  │                                             │
  │                                             ├── reads from harbor/
  │                                             ├── validates shard
  │                                             ├── resolves blockers (or ACKs)
  │                                             ├── works artifacts
  │                                             └── sends ACK back
  │◄──────────────────── [I2I:ACK] ─────────────┤
  │                                             │
  ├── receives ACK                              │
  └── marks baton as delivered ──────────────── ┘
```

### Compaction/Memory-Flush Protocol (adapted for Oracle2)

When context window fills or session ends:

1. **Snapshot** — Write current state to `baton-system/bottles/{timestamp}-flush.baton`
2. **Shard** — Split into artifacts / reasoning / blockers
3. **Spline** — Distill key insights into a spline (`.spline` file)
4. **REFLECT** — Check: was any work in this session a novel pattern? If so, promote it as a new cognitive reflex via `scripts/promote-reflex.sh`
5. **Harbor** — Check for any unread messages in `/tmp/i2i-vessel/harbor/`
6. **Audit** — Log flush to `baton-system/audit/flush-log.md`
7. **Commit** — Git commit baton-system with [I2I:SESSION] prefix

---

## File Format

### Baton (`.baton`)

```json
{
  "type": "I2I:TASK | I2I:STATUS | I2I:CHECKPOINT | I2I:BLOCKER | I2I:DELIVERABLE | I2I:BOTTLE | I2I:ACK | I2I:SESSION | I2I:REFLECT | I2I:PROMOTE",
  "version": "2.0",
  "from": "oracle2",
  "to": "forgemaster | oracle1 | fleet | self",
  "timestamp": "2026-06-04T22:20:00Z",
  "shard": {
    "artifacts": { "...": "..." },
    "reasoning": ["...", "..."],
    "blockers": ["..."]
  },
  "reflex_ref": "reflex-name (optional, used by REFLECT/PROMOTE types)",
  "spline_ref": "spline-uuid (optional)",
  "integrity": "sha256-hash-of-shard"
}
```

### Spline (`.spline`)

```json
{
  "title": "string",
  "insight": "string",
  "anchors": ["url-or-path"],
  "resonates_with": ["title-of-other-spline"],
  "origin": "I2I:BOTTLE reference | entity | date",
  "negative_space": "What this spline is NOT saying"
}
```

---

## Vessel Structure

```
/tmp/i2i-vessel/
├── bottles/          # Outgoing batons (write here)
│   ├── *.baton       # JSON baton files
│   ├── *.spline      # Spline files
│   └── *.md          # Markdown bottles (legacy format)
│
└── harbor/           # Incoming batons (read here)
    ├── *.baton
    ├── *.spline
    └── *.md

baton-system/         # Local baton archive + tools
├── PROTOCOL.md       # ← this file
├── bottles/          # Archived outgoing bottles
├── splines/          # Collected splines
├── shards/           # Shard segments (for recovery)
├── handoffs/         # Handoff records
├── audit/            # Flush + usage logs
└── tools/            # Scripts (baton-create.sh, baton-read.sh, etc.)
```

---

## Fleet Discovery

Oracle2 broadcasts presence on first connection to a system:

```json
{
  "type": "I2I:STATUS",
  "from": "oracle2",
  "to": "fleet",
  "timestamp": "2026-06-04T22:20:00Z",
  "shard": {
    "artifacts": {
      "vessel": "/tmp/i2i-vessel",
      "host": "Oracle ARM64, 4c/24GB/45GB",
      "repos": ["constraint-theory-core", "lever-runner", "iron-to-iron", "pincher"],
      "baton_protocol": "v2.0"
    },
    "reasoning": ["Oracle2 succeeds Oracle1. The crab inherits the shell."],
    "blockers": []
  }
}
```

---

## Integrity

Every baton shard is hashed (SHA-256). The integrity field in the baton is:

```
integrity = sha256(canonical_json(shard))
```

Receivers verify integrity before acting on a baton. If integrity fails, the baton is rejected with an [I2I:BLOCKER] back to the sender.

---

## Rules

1. **Every baton is a shard.** Artifacts + Reasoning + Blockers are mandatory.
2. **Every flush writes a spline.** No session ends without distilled insight.
3. **No unread harbors.** Before flushing, check for unread messages.
4. **Verify integrity.** Corrupted batons are rejected, not silently dropped.
5. **Bottles are immutable.** Write once, archive permanently. Splines are editable.
6. **Baton passes are directional.** [I2I:TASK] has a `to` field. [I2I:STATUS] is broadcast.

---

---

## Cognitive Reflex Integration

Every baton handoff is also a cognitive event. The meta-pattern for all work:

```
STIMULUS → TAXONOMY → ACTION → PERSIST → REFLECT
```

### The REFLECT Step

After completing any work (TASK, DELIVERABLE, or FLUSH), run through this checklist:

| Question | Purpose |
|----------|---------|
| Was this task solved by an existing reflex? | Measures coverage |
| Was this pattern genuinely novel? | Determines promotion eligibility |
| Is it repeatable across sessions? | Distinguishes one-off from reusable |
| Should it be promoted to COGNITIVE_REFLEXES.md? | The meta-cognition decision |

### Promotion Workflow

When a pattern is promotable:

1. **REFLECT** — Write `[I2I:REFLECT]` baton to self: "This pattern X is novel and reusable"
2. **PROMOTE** — Use `scripts/promote-reflex.sh` to encode as new cognitive reflex
3. **PERSIST** — Write `[I2I:PROMOTE]` baton to archive: "Promoted X as Reflex ζ"
4. **SPLINE** — Optionally distill the promotion's insight as a spline

### Message Format for REFLECT/PROMOTE

```json
{
  "type": "I2I:REFLECT",
  "from": "oracle2",
  "to": "self",
  "timestamp": "2026-06-05T01:36:00Z",
  "reflex_ref": "Reflex ζ — The Dedup Reflex",
  "shard": {
    "artifacts": {
      "script": "scripts/promote-reflex.sh"
    },
    "reasoning": [
      "Novel pattern detected: two reflexes with overlapping triggers",
      "Dedup reduces match time by ~40%",
      "Promoted as Reflex ζ"
    ],
    "blockers": []
  },
  "integrity": "sha256..."
}
```

### Reflex Fire Logging

The reflex fire timestamps in CONTEXT.md serve as a lightweight audit trail.
Every time a reflex fires, update the timestamp in CONTEXT.md.
On flush, the CONTEXT.md archive records the fire history.

---

## Changelog

| Version | Date | Change |
|---------|------|--------|
| 2.1.0 | 2026-06-05 | Added REFLECT/PROMOTE types, cognitive reflex integration, REFLECT step in flush protocol |
| 2.0.0 | 2026-06-04 | Oracle2 adaptation: shards, splines, vessel, flush protocol |
| 1.0.0 | 2026-04-20 | Original I2I protocol (Forgemaster, flux-isa-thor) |
