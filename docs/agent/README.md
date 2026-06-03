# PincherOS Agent-Centric Documentation (a2a-native)

## Purpose

This directory contains **a2a-native documentation** for PincherOS. It is structured for AI agents that are tasked with understanding, integrating, or extending PincherOS within other systems. Unlike human-readable documentation, a2a-native docs are:

- **Explicit**: No metaphors, no narratives, no inference required.
- **Schema-first**: Interfaces defined with exact field names, types, and constraints.
- **State-precise**: All state machines, transitions, and guards specified exhaustively.
- **Cross-referenced**: Documents link to each other for dependency tracking.

## Document Index

| Document | File | Description |
|---|---|---|
| Integration Reference | [INTEGRATION.md](./INTEGRATION.md) | System identity, all interfaces (CLI, JSON-RPC, Python sidecar), state model, dependency graph, and integration patterns. |
| State Machine Reference | [STATE_MACHINE.md](./STATE_MACHINE.md) | Complete state machines for reflex lifecycle, session, resource, and migration. Includes transition guards, triggers, and algorithms. |
| Protocol Reference | [PROTOCOLS.md](./PROTOCOLS.md) | Wire protocols: JSON-RPC 2.0 over TCP, JSON-RPC 2.0 over UDS, .nail binary format, capability manifest schema, veto rule schema. |
| Capability Reference | [CAPABILITIES.md](./CAPABILITIES.md) | All capabilities PincherOS exposes, permission model, resource constraints, enforcement chain, and manifest declaration format. |
| Quickstart | [QUICKSTART.md](./QUICKSTART.md) | Fastest path from zero to working integration. Exact commands, exact JSON payloads, exact response formats. |

## Integration Checklist

Use this checklist to validate that your integration is complete. Each item maps to a specific document.

- [ ] **Understand system identity and interfaces** → [INTEGRATION.md § System Identity](./INTEGRATION.md#system-identity)
- [ ] **Select integration pattern** → [INTEGRATION.md § Integration Patterns](./INTEGRATION.md#integration-patterns)
- [ ] **Verify dependencies are met** → [INTEGRATION.md § Dependency Graph](./INTEGRATION.md#dependency-graph)
- [ ] **Implement CLI or JSON-RPC client** → [INTEGRATION.md § Interface Specification](./INTEGRATION.md#interface-specification)
- [ ] **Understand state transitions before mutating** → [STATE_MACHINE.md](./STATE_MACHINE.md)
- [ ] **Handle wire protocol correctly** → [PROTOCOLS.md](./PROTOCOLS.md)
- [ ] **Declare and verify capabilities** → [CAPABILITIES.md](./CAPABILITIES.md)
- [ ] **Run quickstart end-to-end** → [QUICKSTART.md](./QUICKSTART.md)
- [ ] **Handle error codes** → [PROTOCOLS.md § Error Codes](./PROTOCOLS.md#error-codes)
- [ ] **Test migration path** → [QUICKSTART.md § Step 5](./QUICKSTART.md#step-5-pack-and-migrate)

## Human-Readable Docs

For narrative context, architecture rationale, and tutorials, see:

```
pincherOS/docs/          ← human-readable documentation root
pincherOS/README.md      ← project overview and getting started
```

a2a-native docs supplement but do not replace human-readable docs. If an a2a doc contradicts a human doc, the a2a doc is authoritative for machine integration purposes.

## Conventions Used in This Folder

| Convention | Meaning |
|---|---|
| `type: string` | JSON Schema type annotation. |
| `required: true` | Field must be present and non-null. |
| `enum: [...]` | Field must be one of the listed values. |
| `→` | State transition arrow (from → to). |
| `[TRIGGER]` | Event that causes a transition. |
| `[GUARD]` | Condition that must be true for transition to fire. |
| `[ACTION]` | Side effect executed during transition. |
| `0xNN` | Hexadecimal byte value. |
| `≤`, `≥`, `<`, `>` | Numeric comparison operators. |
| Cross-references use `[Document § Section](./DOC.md#section)` format. |
