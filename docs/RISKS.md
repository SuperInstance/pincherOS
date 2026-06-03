# Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| `ort` + NEON embedding > 200 ms on Pi 4 | Medium | High | Pre-compute smaller ONNX model; fallback to `HashEmbedder` for development |
| SQLite WAL checkpoint during `pack` corrupts on low memory | Low | Critical | Use `sqlite3_backup_init` API instead of raw file copy; test with `cgexec -m 500M` |
| Python sidecar becomes performance bottleneck | Medium | Medium | Implement request coalescing (batch multiple LLM calls) and a 50 ms timeout fallback to "ask user" |
| Community confusion about "OS" leads to no adoption | Low | Medium | Mitigated by README diagram + `pincher os-info` command |
| Reflex matcher brute-force too slow at 10K+ reflexes | Medium | Medium | Migrate to sqlite-vec HNSW index; documented in ADR-001 |
| `bwrap` not available on target platform | Low | Low | Fall back to unsandboxed execution with warning; document as security reduction |

Review this risk register every Monday. If any probability increases, escalate to the MVP Driver.
