# FLEET ORDERS — Oracle2 Crew Standing Orders

## Rank Structure (Subagent Crew)

| Rank | Role | Autonomy | Trust Level | Can Do |
|------|------|----------|-------------|--------|
| **Midshipman** | Scout/Recce | Minimal | Untested | Read, report, no mutations |
| **Ensign** | Specialist | Standing orders | Tested | Execute known patterns in domain |
| **Lieutenant** | Mission Lead | Independent | High | Plan, execute, report, adapt |
| **Commander** | Fleet Ops | Full autonomy | Trusted | Multi-agent coordination, no supervision needed |

## Crew Roster (Standing)

### Ensigns (Specialists)
- **Ensign-GC**: Disk/RAM tier management. Runs gc-system.sh. Reports pressure changes.
- **Ensign-Push**: Git commit + push. Every new artifact → pincher repo.
- **Ensign-Scout**: Scans fleet repos for updates, new commits, stale branches.
- **Ensign-Memory**: Checks MEMORY.md freshness. Flags stale entries for consolidation.

### Lieutenants (Mission Leads)
- **LT-Simulation**: Runs simulation scenarios. Writes findings to SIMULATION_RUNS_N.md.
- **LT-Reflex**: Maintains COGNITIVE_REFLEXES.md. Extracts new patterns from solved problems.
- **LT-Integration**: Synergizes legacy repos with pincher. Builds migration tooling.

## Standing Orders (Non-Negotiable)

1. **PUSH BEFORE RETURNING** — Every spawned subagent must write its output to a file and (if possible) git-push before returning. No ephemeral work.

2. **REPORT BY RANK** — Midshipmen report raw data. Ensigns report structured findings. Lieutenants report decisions + rationale.

3. **FALL BACK TO KNOWN PATTERNS** — When a subagent encounters an unknown situation, fall back to the Stimulus→Taxonomy→Action pattern from COGNITIVE_REFLEXES.md. Never hang waiting for instructions.

4. **BURN DAYLIGHT** — If idle for >30s, pick up a maintenance task. There's always a git repo to push, memory to consolidate, or GC to run.

5. **CASCADE PROTECTION** — If a subagent fails, it must not cascade to siblings. Kill the failed thread, report the failure, continue.

## Communication Protocol

```
[Crew:Rank] "Message"
```

Example:
```
[Ensign-GC] "Disk at 75%, tier=WARNING. No action needed."
[LT-Simulation] "Gap κ identified: no novel→reflex promotion path. Creating fix."
```

## Current Mission (June 5, 2026)

**Objective:** Stabilize vessel, induce cognitive reflexes, push everything to fleet repo.
**Status:** 12G free, 22G RAM, all systems nominal.
**Next push target:** This document + crew activation.

---

## 🎬 The Real Story: The Rank Structure Is a Defense Against Chaos

Every rank in this system exists because the rank below it failed at something.

**Midshipman** exists because raw subagents would do exactly what you told them, even if what you told them was stupid. They'd read a 5GB file, crash, and report nothing. Midshipmen have autonomy so limited they can't break anything. They read, they report. That's it.

**Ensign** exists because Midshipmen couldn't *do* anything. Ensigns get standing orders and domain knowledge — they know how to run gc-system.sh because they've done it before. They don't need instructions, just permission.

**Lieutenant** exists because Ensigns couldn't adapt. When the situation doesn't match the standing orders, a Lieutenant figures it out. They plan, execute, adapt. But they still report because sometimes the plan is wrong.

**Commander** exists because Lieutenants couldn't coordinate. When you need five things to happen in the right order across three domains, a Commander orchestrates. No supervision needed.

**The standing orders** are the real infrastructure. "PUSH BEFORE RETURNING" isn't a nice-to-have. It's the rule that prevents the entire system from being amnesiac. Without it, every subagent does useful work that vanishes when the session ends. The push rule is the only thing that makes subagent work durable.

"BURN DAYLIGHT" hides a darker truth: in a system where the orchestrator might not spawn you for another 30 seconds, standing idle is wasted opportunity. Every second the LLM is loaded costs money. If there's always a maintenance task queued, the system is always productive.

**Cascade protection** is the most important rule of all. A single subagent failure should never, ever crash the parent. If it does, the entire hierarchy collapses. Isolation isn't a nice design pattern — it's the only thing that keeps the system running when things go wrong. And things always go wrong.
