# RFC: JEPA Integration

**Status**: Exploratory
**Owner**: Research Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

Joint-Embedding Predictive Architecture (JEPA) could replace the Command
Dynamics Model for predicting command outcomes. Instead of a 500K-param MLP,
JEPA would learn a latent representation of command trajectories and predict
`P(success)` and `P(violation)` in that latent space.

## Why It's Not on the Critical Path

- JEPA is a research architecture for video/image prediction, not bash commands
- No proven implementation for the command prediction domain exists
- The deterministic veto engine is sufficient for MVP safety
- Training JEPA requires 10K+ action logs — we have zero today

## Revisit When

- 10K action logs collected
- MLP proven insufficient (false positive rate > 5%)
- Research funding secured for a dedicated ML engineer
