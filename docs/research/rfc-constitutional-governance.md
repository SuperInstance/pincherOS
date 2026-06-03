# RFC: Constitutional Governance

**Status**: Deferred
**Owner**: Research Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

A governance layer where "articles of constitution" define invariants that
no reflex may violate. Reflexes are validated against the constitution before
execution, and violations trigger quarantine.

## Why It's Deferred

Capability tokens provide the same safety guarantees for MVP:
- `CapabilityManifest` declares what a reflex may do
- `CapabilityToken` is HMAC-signed and verified in < 1 ms
- Veto engine blocks violations deterministically

Constitutional governance adds:
- A DSL for expressing invariants
- An interpreter/validator for that DSL
- A runtime enforcement layer

This is valuable for enterprise compliance but not needed at our current scale.

## Revisit When

- Regulatory audit required (SOC 2, HIPAA, etc.)
- Capability tokens proven insufficient for complex multi-step policies
- A customer demands declarative safety policies
