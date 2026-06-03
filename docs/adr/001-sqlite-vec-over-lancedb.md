# ADR-001: SQLite + sqlite-vec over LanceDB for MVP

## Status

Accepted

## Context

PincherOS needs a vector store for reflex embeddings and a relational store for
structured state (shells, sessions, action_log). The two options are:

1. **SQLite + sqlite-vec**: Single-file database with a vector search extension.
   WAL mode for concurrent reads. ~5 MB footprint. No Python on the hot path.
2. **LanceDB**: Purpose-built vector database with native Rust bindings. Supports
   HNSW indexing. Larger footprint. Requires separate process for persistence.

## Decision

Use **SQLite + sqlite-vec** for the MVP.

## Consequences

### Positive
- **Zero external dependencies** — SQLite is compiled into the binary via `rusqlite` with the `bundled` feature.
- **5 MB total** — fits on a Raspberry Pi 4 with 1 GB free RAM.
- **WAL mode** — concurrent reads while the agent writes action logs.
- **No Python on the hot path** — the Rust core owns all state.
- **Proven** — SQLite is the most deployed database in the world.

### Negative
- **No HNSW index** — brute-force cosine search is O(n) per query. Adequate for
  thousands of reflexes; will need HNSW or IVF for 100K+.
- **sqlite-vec is pre-1.0** — API may change. Mitigated by abstracting behind
  `ReflexEngine` in `pincher-core`.

### Migration Path
When reflex count exceeds 10,000 and match latency exceeds 200 ms on a Raspberry
Pi 4, migrate to LanceDB via `pincher-core/src/db/lancedb.rs` behind the same
`Database` trait. The `.nail` migration format abstracts the backend.
