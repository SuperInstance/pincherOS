# RFC: Command Dynamics MLP

**Status**: Deferred
**Owner**: Rigging Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

A 500K-parameter MLP in ONNX Runtime that predicts `P(success)` and
`P(violation)` from state + command embeddings. This replaces the deterministic
veto engine with a learned model that can generalize to novel command patterns.

## Why It's Deferred

The deterministic veto engine is sufficient for MVP safety:
- Blocked pattern list catches `rm -rf /` etc. with confidence 1.0
- Capability manifest enforcement catches permission violations
- Novel commands get confidence 0.5 (require confirmation)

The MLP adds significant complexity (training data, labeling, online updates)
without proven benefit at our current scale.

## Revisit When

- 10K action logs collected
- Deterministic veto has > 5% false positive rate on real workloads
- A training pipeline can produce labeled data automatically
