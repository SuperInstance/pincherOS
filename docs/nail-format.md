# .nail Format Specification

## Overview

A `.nail` file is a portable archive of an agent's complete state — its reflexes,
configuration, identity, and action history. It is the hermit crab's "rigging"
that migrates between shells.

## Binary Layout

```
┌──────────────────────────────────────────────────────────────┐
│                    NAIL ARCHIVE (tar.zst)                     │
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ manifest.json                                           │ │
│  │ {                                                       │ │
│  │   "version": "0.1.0-alpha",                             │ │
│  │   "format_version": 1,                                  │ │
│  │   "shell_fingerprint": "blake3-hex-of-source-shell",    │ │
│  │   "timestamp": "2026-06-01T12:00:00Z",                  │ │
│  │   "reflex_count": 47,                                   │ │
│  │   "db_checksum": "blake3-hex-of-reflexes.db",           │ │
│  │   "config_checksum": "blake3-hex-of-config.toml"        │ │
│  │ }                                                       │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ reflexes.db (SQLite)                                    │ │
│  │ - shells, reflexes, sessions, action_log tables         │ │
│  │ - embedding_blob stored as BLOB (384 × f32 LE)          │ │
│  │ - WAL checkpointed before packing                       │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ config.toml                                             │ │
│  │ [pincher]                                               │ │
│  │ version = "0.1.0-alpha"                                 │ │
│  │ [reflex]                                                │ │
│  │ confidence_short_circuit = 0.90                          │ │
│  │ [sandbox]                                               │ │
│  │ network = false                                          │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ identity.json                                           │ │
│  │ {                                                       │ │
│  │   "agent_id": "uuid-v4",                                │ │
│  │   "created_at": "2026-06-01T12:00:00Z"                  │ │
│  │ }                                                       │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## Compression

- Algorithm: `zstd` level 3 (default)
- Optional: Dictionary compression for reflex strings (30-50% additional savings)
- Typical size: ~5 MB for a session with 50 reflexes

## Checksums

- All checksums use `blake3` (faster than SHA-256, no length extension attacks)
- `db_checksum` and `config_checksum` are verified during unpack
- If any checksum fails, the unpack is aborted and an error is returned

## QTR (Quiesce-Transfer-Resume) Protocol

1. **Quiesce**: Flush SQLite WAL (`PRAGMA wal_checkpoint(TRUNCATE)`), end all sessions
2. **Transfer**: Stream the `.nail` file with blake3 checksums
3. **Resume**: Verify checksums, open SQLite, validate schema, re-snap hardware

## Hardware-Tagged Reflex Invalidation

On unpack, reflexes with `required_capability` that the target shell doesn't support
have their `confidence` reset to 0.0. Example: a reflex tagged `requires: cuda`
on a CPU-only Raspberry Pi will not auto-execute.

## Version Compatibility

- `format_version: 1` — current format
- Future versions may add fields; parsers must ignore unknown keys
- If `format_version` is higher than supported, unpack returns an error
