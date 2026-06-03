# RFC: Polyformal Synthesis as Architecture

**Status**: Abandoned
**Owner**: Research Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

The original PincherOS design used polyformal synthesis — iterative refinement
from multiple formal perspectives — as the architectural methodology. This
produced 8 perspectives, 15 analysis documents, and ~5,400 lines of Rust code
across 12 repositories.

## Why It Was Abandoned

Polyformal synthesis is an excellent **analysis** tool but a poor **architecture**
tool. It produces:
- Multiple overlapping designs that are hard to reconcile
- Intellectual vaporware (Penrose tensors, JEPA, constitutional governance)
- Analysis paralysis — 3 rounds of ideation without a `cargo build`

## What We Kept

The synthesis produced valuable insights that survived the audit:
- Hermit crab metaphor → Shell/Rigging/Claws/Exoskeleton architecture
- `.nail` migration format → the killer feature
- Reflex short-circuit → the core value proposition
- LLM as compiler, not runtime → the cost model

## What We Repurposed

Polyformal synthesis is now used as a **threat-modeling framework** — see
`docs/threats.md` — not as an architectural methodology.
