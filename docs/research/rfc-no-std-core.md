# RFC: no_std Core

**Status**: Exploratory
**Owner**: Shell Team
**Last Updated**: 2026-06-03
**MVP Blocker**: No

## Summary

Compile `pincher-core` for `no_std` targets (embedded ARM, microcontrollers)
to enable PincherOS agents on ultra-low-power devices.

## Why It's Not on the Critical Path

- SQLite requires `std` (file I/O, threads)
- ONNX Runtime requires `std`
- Our target platforms (RPi 4, Jetson, workstations) all have full Linux
- `no_std` is a maturity move for a project at day zero of compilation

## Revisit When

- Enterprise embedded contract signed
- An embedded use case emerges from the community
- SQLite can be replaced with a `no_std` key-value store
