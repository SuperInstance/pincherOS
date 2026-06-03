# PincherOS Integration Reference

## System Identity

| Field | Value |
|---|---|
| Name | PincherOS |
| Version | 0.1.0 |
| Purpose | Post-model operating system for AI agent reflexes. Embeds pattern→action mappings that execute at ~50ms with zero API cost. Supports migration via .nail files. |
| License | MIT or Apache-2.0 (dual-licensed) |
| Repository | (source root: `pincherOS/`) |
| Binary name | `pincher` |
| Crate name (library) | `pincher-core` |
| RPC port (default) | 9821 |
| UDS path (default) | `/tmp/pincher-sidecar.sock` |
| Config path (default) | `~/.config/pincher/config.toml` |
| Data path (default) | `~/.local/share/pincher/` |

## Interface Specification

### CLI Interface

All CLI commands exit with code 0 on success, non-zero on failure. Output format: JSON to stdout when `--json` flag is present; human-readable text otherwise.

| Command | Signature | Input Schema | Output Schema | Description |
|---|---|---|---|---|
| `init` | `pincher init [--db PATH]` | `{ db?: string }` | `{ status: "ok", db_path: string }` | Initialize PincherOS data store and config. |
| `status` | `pincher status [--json]` | `{}` | `{ version: string, db_path: string, reflex_count: uint32, session_state: string, resource_state: string, uptime_ms: uint64 }` | Report system status. |
| `teach` | `pincher teach --pattern TEXT --action ACTION [--confidence FLOAT] [--tags TAGS]` | `{ pattern: string, action: string, confidence?: float64, tags?: string[] }` | `{ reflex_id: string, status: "created" }` | Create a new reflex. `action` is a shell command string or JSON action object. |
| `match` | `pincher match --input TEXT [--threshold FLOAT] [--json]` | `{ input: string, threshold?: float64 }` | `{ matched: bool, reflex_id?: string, confidence?: float64, action?: string }` | Match input against known reflex patterns. |
| `do` | `pincher do --reflex-id ID [--dry-run] [--json]` | `{ reflex_id: string, dry_run?: bool }` | `{ reflex_id: string, status: "executed" | "dry_run", exit_code?: int32, stdout?: string, stderr?: string, duration_ms: uint64 }` | Execute a reflex action. |
| `confirm` | `pincher confirm --reflex-id ID [--correction ACTION]` | `{ reflex_id: string, correction?: string }` | `{ reflex_id: string, status: "confirmed", confidence: float64 }` | Confirm a reflex was correct, optionally correcting the action. |
| `pack` | `pincher pack --output PATH [--reflex-ids IDS] [--all]` | `{ output: string, reflex_ids?: string[], all?: bool }` | `{ nail_path: string, reflex_count: uint32, checksum: string, size_bytes: uint64 }` | Pack reflexes into a .nail file. See [PROTOCOLS.md § .nail File Format](./PROTOCOLS.md#nail-file-format). |
| `unpack` | `pincher unpack --input PATH [--verify-only]` | `{ input: string, verify_only?: bool }` | `{ status: "verified" | "unpacked", reflex_count: uint32, checksum_valid: bool }` | Unpack or verify a .nail file. |
| `serve` | `pincher serve [--port PORT] [--sidecar-socket PATH]` | `{ port?: uint16, sidecar_socket?: string }` | (long-running process; logs to stderr) | Start JSON-RPC server and optional Python sidecar listener. |
| `migrate` | `pincher migrate --nail PATH [--target TARGET]` | `{ nail: string, target?: string }` | `{ status: "migrating" | "completed", reflexes_imported: uint32, conflicts: uint32 }` | Full migration workflow: unpack, verify, import. |

#### CLI Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Database error |
| 4 | Reflex not found |
| 5 | Match threshold not met |
| 6 | Execution failed |
| 7 | Pack/unpack error |
| 8 | Migration error |
| 9 | Resource critical (refused) |
| 10 | Capability vetoed |

### JSON-RPC Interface

Transport: TCP on port 9821 (default). Protocol: JSON-RPC 2.0. See [PROTOCOLS.md § JSON-RPC 2.0 over TCP](./PROTOCOLS.md#json-rpc-20-over-tcp).

| Method | Request Schema | Response Schema | Description |
|---|---|---|---|
| `pincher.status` | `{}` | `{ version: string, db_path: string, reflex_count: uint32, session_state: string, resource_state: string, uptime_ms: uint64 }` | System status. |
| `pincher.teach` | `{ pattern: string, action: string, confidence?: float64, tags?: string[] }` | `{ reflex_id: string, status: "created" }` | Create a reflex. |
| `pincher.match` | `{ input: string, threshold?: float64 }` | `{ matched: bool, reflex_id?: string, confidence?: float64, action?: string }` | Match input to a reflex. |
| `pincher.execute` | `{ reflex_id: string, dry_run?: bool }` | `{ reflex_id: string, status: "executed" | "dry_run", exit_code?: int32, stdout?: string, stderr?: string, duration_ms: uint64 }` | Execute a reflex. |
| `pincher.confirm` | `{ reflex_id: string, correction?: string }` | `{ reflex_id: string, status: "confirmed", confidence: float64 }` | Confirm a reflex. |
| `pincher.list` | `{ state?: string, tag?: string, limit?: uint32 }` | `{ reflexes: [{ reflex_id: string, pattern: string, action: string, state: string, confidence: float64, tags: string[], created_at: string, updated_at: string }] }` | List reflexes with optional filters. |
| `pincher.delete` | `{ reflex_id: string }` | `{ reflex_id: string, status: "deleted" }` | Delete a reflex. |
| `pincher.pack` | `{ reflex_ids?: string[], all?: bool }` | `{ nail_path: string, reflex_count: uint32, checksum: string, size_bytes: uint64 }` | Pack reflexes. Output written to data directory. |
| `pincher.unpack` | `{ nail_path: string, verify_only?: bool }` | `{ status: "verified" | "unpacked", reflex_count: uint32, checksum_valid: bool }` | Unpack or verify a .nail file. |
| `pincher.migrate` | `{ nail_path: string, target?: string }` | `{ status: "migrating" | "completed", reflexes_imported: uint32, conflicts: uint32 }` | Full migration workflow. |
| `pincher.capability.list` | `{}` | `{ capabilities: [{ name: string, granted: bool, constrained: bool }] }` | List current capability state. |
| `pincher.capability.request` | `{ capability: string, justification?: string }` | `{ capability: string, granted: bool, reason?: string }` | Request a capability at runtime. |

### Python Sidecar Interface

Transport: JSON-RPC 2.0 over Unix Domain Socket at `/tmp/pincher-sidecar.sock` (default). See [PROTOCOLS.md § JSON-RPC 2.0 over UDS](./PROTOCOLS.md#json-rpc-20-over-uds-python-sidecar).

| Method | Request Schema | Response Schema | Description |
|---|---|---|---|
| `sidecar.embed` | `{ text: string, model?: string }` | `{ embedding: float64[], dimensions: uint32, model: string, duration_ms: uint64 }` | Generate embedding vector for text. Uses ONNX model if available, falls back to hash-based embedding. |
| `sidecar.classify` | `{ text: string, candidates: string[] }` | `{ label: string, confidence: float64, scores: { [label: string]: float64 } }` | Classify text against candidate labels. |
| `sidecar.generate` | `{ prompt: string, max_tokens?: uint32, temperature?: float64 }` | `{ text: string, tokens_generated: uint32, duration_ms: uint64 }` | Generate text via LLM. Requires `subprocess` capability if using external model. |
| `sidecar.health` | `{}` | `{ status: "ok" | "degraded" | "down", model_loaded: bool, python_version: string }` | Sidecar health check. |

#### Python Sidecar Function Signatures (for direct import)

When using `pincher_sidecar` as a Python package:

```python
def embed(text: str, model: str | None = None) -> list[float]: ...
def classify(text: str, candidates: list[str]) -> tuple[str, float, dict[str, float]]: ...
def generate(prompt: str, max_tokens: int = 256, temperature: float = 0.7) -> str: ...
def health() -> dict[str, str | bool]: ...
```

## State Model

PincherOS maintains four categories of state. Each is described in full in [STATE_MACHINE.md](./STATE_MACHINE.md).

| State Category | Storage | Persistence | Document Reference |
|---|---|---|---|
| Reflex state | SQLite (`reflexes` table) | Durable | [STATE_MACHINE.md § Reflex Lifecycle](./STATE_MACHINE.md#reflex-lifecycle) |
| Session state | In-memory + SQLite (`sessions` table) | Durable (checkpointed) | [STATE_MACHINE.md § Session States](./STATE_MACHINE.md#session-states) |
| Resource state | In-memory (sampled every 500ms) | Ephemeral | [STATE_MACHINE.md § Resource States](./STATE_MACHINE.md#resource-states) |
| Migration state | In-memory (per-migration) | Ephemeral (lost on crash = migration aborted) | [STATE_MACHINE.md § Migration States](./STATE_MACHINE.md#migration-states) |

### Key State Invariants

1. A reflex in `dormant` state cannot be executed. It must be `active` or `confirmed`.
2. Session state `quiescing` blocks new reflex executions but allows in-flight executions to complete.
3. Resource state `critical` causes all `do`/`execute` requests to return error code 9.
4. Migration state is per-file. Multiple migrations cannot run concurrently.

## Dependency Graph

### Required Dependencies

| Dependency | Minimum Version | Purpose |
|---|---|---|
| Rust toolchain | 1.85+ | Build system. Required for `pincher` binary and `pincher-core` crate. |
| SQLite | 3.39+ | Reflex and session persistence. Bundled via `rusqlite` with `bundled` feature; no system install required. |
| Tokio runtime | 1.x | Async runtime for RPC server. Compiled into binary. |

### Optional Dependencies

| Dependency | Version | Purpose | Effect if Missing |
|---|---|---|---|
| ONNX Runtime | 1.17+ | Local embedding model inference. | Falls back to hash-based embeddings (lower quality, zero latency difference). |
| ONNX model file | N/A | Embedding model weights (~120MB). | Embedding dimension = 128 (hash-based) instead of 384 (ONNX). |
| Python | 3.10+ | Sidecar process for LLM operations. | `sidecar.*` RPC methods unavailable; `sidecar.health` returns `status: "down"`. |
| `pincher_sidecar` Python package | 0.1.0 | Python sidecar implementation. | Same as above. |
| GPU (CUDA/Metal) | N/A | Accelerated ONNX inference. | ONNX runs on CPU; ~3x slower embedding generation. |

### Build Dependency Graph

```
pincher (binary)
├── pincher-core (crate)
│   ├── rusqlite (bundled SQLite)
│   ├── tokio (async runtime)
│   ├── serde + serde_json (serialization)
│   ├── ort (ONNX Runtime bindings; optional, feature = "onnx")
│   └── uuid (reflex ID generation)
├── clap (CLI parsing)
└── jsonrpc-core (RPC server)

pincher_sidecar (Python package)
├── onnxruntime (optional; for ONNX inference in Python)
└── asyncio (UDS server)
```

## Integration Patterns

### Pattern 1: CLI-Only (Simplest)

**Requirements**: `pincher` binary only. No Python, no RPC server.

**Flow**:
1. Shell out to `pincher` CLI commands.
2. Parse JSON output (use `--json` flag).
3. Handle exit codes per [CLI Exit Codes](#cli-exit-codes).

**Suitable for**: Cron jobs, shell scripts, simple automation, CI/CD pipelines.

**Limitations**:
- No programmatic embedding (uses hash-based embeddings).
- No LLM integration.
- Each command is a separate process (~10ms overhead per invocation).
- No streaming or long-running sessions.

**Example**:
```bash
pincher init
pincher teach --pattern "disk full on /var" --action "rm -rf /var/log/*.old" --json
pincher match --input "disk full on /var" --json
pincher do --reflex-id <ID> --dry-run --json
```

### Pattern 2: CLI + RPC (Programmatic Access)

**Requirements**: `pincher` binary running in `serve` mode.

**Flow**:
1. Start `pincher serve` as a daemon or background process.
2. Connect to TCP port 9821.
3. Send JSON-RPC 2.0 requests. Parse responses.
4. Use `pincher.status` for health checks.

**Suitable for**: Long-running services, web backends, any system with TCP socket access.

**Limitations**:
- No embedding generation (or hash-based only).
- No LLM text generation or classification.

**Example**:
```json
→ {"jsonrpc":"2.0","method":"pincher.teach","params":{"pattern":"disk full on /var","action":"rm -rf /var/log/*.old"},"id":1}
← {"jsonrpc":"2.0","result":{"reflex_id":"r_01HXYZ","status":"created"},"id":1}
```

### Pattern 3: CLI + RPC + Python Sidecar (Full LLM Integration)

**Requirements**: `pincher` binary + Python 3.10+ + `pincher_sidecar` package + (optionally) ONNX model.

**Flow**:
1. Start `pincher serve --sidecar-socket /tmp/pincher-sidecar.sock`.
2. PincherOS auto-starts Python sidecar process.
3. Primary RPC on TCP:9821; sidecar RPC on UDS.
4. Use `sidecar.embed` for high-quality embeddings.
5. Use `sidecar.classify` and `sidecar.generate` for LLM features.

**Suitable for**: Full-featured integrations, production deployments, systems requiring semantic matching.

**Limitations**:
- Higher memory footprint (~200MB additional for Python + ONNX model).
- Sidecar process must be monitored for crashes.

**Example**:
```json
→ {"jsonrpc":"2.0","method":"sidecar.embed","params":{"text":"disk full on /var"},"id":1}
← {"jsonrpc":"2.0","result":{"embedding":[0.023,-0.114,...],"dimensions":384,"model":"minilm-l6-v2","duration_ms":4},"id":1}
```

### Pattern 4: Embedded Library (Rust Crate)

**Requirements**: Rust 1.85+, `pincher-core` crate as dependency.

**Flow**:
1. Add `pincher-core` to `Cargo.toml`.
2. Use `pincher_core::PincherOS` struct directly.
3. Call methods without CLI or RPC overhead.

**Suitable for**: Rust applications, embedded systems, high-performance integrations.

**Rust API Surface**:
```rust
use pincher_core::PincherOS;

let p = PincherOS::init("/path/to/db")?;
let rid = p.teach("pattern text", "action text", None, None)?;
let result = p.match_input("input text", None)?;
let exec = p.execute(&rid, false)?;
p.confirm(&rid, None)?;
let nail = p.pack(&[rid.clone()])?;
p.unpack("/path/to/file.nail", false)?;
```

**Limitations**:
- Requires Rust compilation.
- Python sidecar must still be managed separately if LLM features needed.

## Configuration

Configuration file: `~/.config/pincher/config.toml` (created by `pincher init`).

```toml
[server]
port = 9821
sidecar_socket = "/tmp/pincher-sidecar.sock"

[embedding]
backend = "onnx"          # "onnx" | "hash"
model_path = ""           # path to ONNX model; empty = bundled default
dimensions = 384          # ignored if backend = "hash"

[resources]
ram_warning_mb = 512      # threshold for "light" state
ram_critical_mb = 256     # threshold for "critical" state
cpu_warning_pct = 80      # threshold for "light" state
cpu_critical_pct = 95     # threshold for "critical" state
sample_interval_ms = 500  # resource sampling interval

[migration]
fingerprint = true        # include device fingerprint in .nail
compatibility_threshold = 0.7  # minimum compatibility score to import
verify_on_unpack = true   # always verify checksum on unpack

[security]
default_capability_mode = "sandbox"  # "allow" | "sandbox" | "deny"
veto_config_path = ""     # path to veto rules TOML; empty = no veto
```
