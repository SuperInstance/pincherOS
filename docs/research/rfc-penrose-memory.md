# RFC: Penrose Tensor Memory

**Status**: Exploratory
**Owner**: Research Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

Penrose tiling-based vector indices use aperiodic local structure to create
efficient approximate nearest-neighbor search. In theory, a Penrose-based index
could outperform HNSW for high-dimensional embeddings by creating natural
cluster boundaries.

## Why It's Not on the Critical Path

- No classical implementation exists for vector databases
- The existing quantum-mechanics papers do not apply to classical vector DBs
- HNSW with golden-ratio pruning is a buildable compromise (see ADR-001)
- This is a research project within a research project

## Revisit When

- Research funding secured
- `penrose-memory` crate produces benchmark results on standard ANN datasets
- HNSW becomes a proven bottleneck at scale
