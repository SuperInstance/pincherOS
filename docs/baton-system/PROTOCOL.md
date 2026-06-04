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
4. **Harbor** — Check for any unread messages in `/tmp/i2i-vessel/harbor/`
5. **Audit** — Log flush to `baton-system/audit/flush-log.md`
6. **Commit** — Git commit baton-system with [I2I:SESSION] prefix

---

## File Format

### Baton (`.baton`)

```json
{
  "type": "I2I:TASK | I2I:STATUS | I2I:CHECKPOINT | I2I:BLOCKER | I2I:DELIVERABLE | I2I:BOTTLE | I2I:ACK | I2I:SESSION",
  "version": "2.0",
  "from": "oracle2",
  "to": "forgemaster | oracle1 | fleet | self",
  "timestamp": "2026-06-04T22:20:00Z",
  "shard": {
    "artifacts": { "...": "..." },
    "reasoning": ["...", "..."],
    "blockers": ["..."]
  },
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

## Changelog

| Version | Date | Change |
|---------|------|--------|
| 2.0.0 | 2026-06-04 | Oracle2 adaptation: shards, splines, vessel, flush protocol |
| 1.0.0 | 2026-04-20 | Original I2I protocol (Forgemaster, flux-isa-thor) |
