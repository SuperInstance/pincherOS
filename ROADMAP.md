# PincherOS Roadmap

## 12-Week MVP Sprint

| Week | Phase | Deliverable | Exit Criteria |
|------|-------|-------------|---------------|
| 1 | Shell | Cargo workspace + SQLite schema + hardware probe + PID loop + `pincher status` | `cargo build --release && ./target/release/pincher --version` prints `0.1.0-alpha` |
| 2 | Rigging | LanceDB/sqlite-vec + MiniLM embeddings + reflex schema + 10 built-in reflexes | `pincher teach "make a folder" "mkdir -p {path}"` stores a reflex |
| 3 | Short-Circuit | Cosine search + threshold logic + execute + log | `pincher do "make a folder called test"` executes `mkdir test` |
| 4 | Compiler | Python sidecar + `/teach` flow + validation + store reflex | Novel inputs route to LLM and produce working commands |
| 5 | Claws/Security | Landlock+seccomp + capability manifest + bwrap sandbox | `pincher do "rm -rf /"` returns `EPERM: Blocked by policy` |
| 6 | Dynamics Model | Train MLP on action_log (deferred to post-MVP) + deterministic veto | Veto engine blocks dangerous commands with confidence 1.0 |
| 7 | Migration | `.nail` format + pack/unpack + hardware-tagged invalidation | Agent migrates between two machines retaining all reflexes |
| 8 | Demo | Record magic moment + README rewrite + launch | 500+ GitHub stars, first external issue |

## 6-Month Ecosystem Horizon

| Month | Track | Deliverable |
|-------|-------|-------------|
| 4 | Plugin Registry | `pincher install github.com/user/skillpack.nail` |
| 4 | A2UI TUI | Terminal UI for `pincher teach` |
| 5 | JEPA Research | `docs/research/rfc-jepa-integration.md` — train on 10K action logs |
| 5 | Penrose Research | `docs/research/rfc-penrose-memory.md` — POC on subset of LanceDB |
| 6 | Enterprise | Signed audit logs, remote gRPC API, `pincherd` daemon mode |
| 6 | Cross-compile | `aarch64-unknown-linux-gnu` CI target for Raspberry Pi |

## MVP Exit Criteria

See [MVP_CHECKLIST.md](./MVP_CHECKLIST.md) for the definitive checklist.
