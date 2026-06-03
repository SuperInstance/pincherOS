# PincherOS MVP Architecture — Engineering Specification

> **Version**: 0.1.0-mvp  
> **Target Hardware**: Raspberry Pi 4 (4GB RAM, ARM Cortex-A72 quad-core @ 1.5GHz) / Jetson Nano (4GB LPDDR4, Cortex-A57 + 128 CUDA cores)  
> **Paradigm**: Apps are assembled from learned reflexes, not written as code. An AI rigging lives inside a hardware shell and can migrate between shells.  
> **Constraint**: Every decision must survive on 4GB RAM with ~1.5GB available for the ML stack after OS overhead.

---

## Table of Contents

1. [MVP Core Loop](#1-mvp-core-loop)
2. [MVP Data Model](#2-mvp-data-model)
3. [MVP Tech Stack](#3-mvp-tech-stack)
4. [Minimum Shell](#4-minimum-shell)
5. [Migration Story](#5-migration-story)
6. [Expansion Hooks](#6-expansion-hooks)
7. [One-Line Install](#7-one-line-install)
8. [MVP vs Full Vision](#8-mvp-vs-full-vision)
9. [File System Layout](#9-file-system-layout)
10. [API Protocols](#10-api-protocols)
11. [Resource Budget](#11-resource-budget)

---

## 1. MVP Core Loop

### 1.1 Absolute Minimum Components

The core loop has exactly **7 components**. No more. Each maps to a single Rust module or Python service.

| # | Component | Responsibility | Runs On |
|---|-----------|---------------|---------|
| 1 | **Input Normalizer** | Accept text/voice/signal → canonical `InputEvent` | Device (Rust) |
| 2 | **Reflex Matcher** | Embed input → cosine search in LanceDB → top-K reflexes | Device (Python sidecar) |
| 3 | **Context Assembler** | Merge: input + matched reflexes + recent memory + shell state → prompt | Device (Rust) |
| 4 | **LLM Runtime** | Generate action plan or conversational response | Device (Python sidecar) |
| 5 | **Action Parser** | Extract structured `Action[]` from LLM output → validate against whitelist | Device (Rust) |
| 6 | **Sandbox Executor** | Run actions in isolated namespace → capture stdout/stderr/exit | Device (Rust + bwrap) |
| 7 | **Memory Writer** | Store input→action→outcome as new reflex or update existing | Device (Python sidecar) |

### 1.2 Data Flow Diagram

```
                     ┌─────────────────────────────────────────────────────┐
                     │                  USER INPUT                         │
                     │  "organize my downloads by file type"              │
                     └──────────────────────┬──────────────────────────────┘
                                            │
                                            ▼
                     ┌──────────────────────────────────────────────────────┐
                     │  1. INPUT NORMALIZER (Rust)                          │
                     │  Raw text → InputEvent { text, timestamp, source }  │
                     └──────────────────────┬──────────────────────────────┘
                                            │
                    ┌───────────────────────┼───────────────────────────┐
                    │                       ▼                           │
                    │  ┌────────────────────────────────────────┐      │
                    │  │  2. REFLEX MATCHER (Python sidecar)     │      │
                    │  │  embed(input) → search LanceDB          │      │
                    │  │  → top-5 reflexes by cosine sim         │      │
                    │  └────────────────────┬───────────────────┘      │
                    │                       │                           │
                    │         ┌─────────────▼──────────────┐          │
                    │         │  Confidence > 0.90?         │          │
                    │         │  (reflex short-circuit)     │          │
                    │         └──────┬──────────┬──────────┘          │
                    │           YES  │          │  NO                  │
                    │                ▼          ▼                      │
                    │   ┌──────────────┐  ┌──────────────────────┐   │
                    │   │  SKIP LLM    │  │ 3. CONTEXT ASSEMBLER  │   │
                    │   │  (muscle     │  │ (Rust)                │   │
                    │   │   memory)    │  │ Build prompt from:    │   │
                    │   └──────┬───────┘  │  input + reflexes +   │   │
                    │          │          │  memory + shell_state  │   │
                    │          │          └──────────┬────────────┘   │
                    │          │                     ▼                 │
                    │          │          ┌──────────────────────┐   │
                    │          │          │ 4. LLM RUNTIME       │   │
                    │          │          │ (Python sidecar)      │   │
                    │          │          │ llama-cpp-python      │   │
                    │          │          │ → ActionPlan JSON     │   │
                    │          │          └──────────┬───────────┘   │
                    │          │                     │                 │
                    │          │                     ▼                 │
                    │          │          ┌──────────────────────┐   │
                    │          ├──────────▶5. ACTION PARSER      │   │
                    │          │          │ (Rust)                │   │
                    │          │          │ Validate + whitelist  │   │
                    │          │          └──────────┬───────────┘   │
                    │          │                     │                 │
                    │          │                     ▼                 │
                    │          │          ┌──────────────────────┐   │
                    │          │          │ 6. SANDBOX EXECUTOR   │   │
                    │          │          │ (Rust + bwrap)        │   │
                    │          │          │ Execute actions        │   │
                    │          │          │ Capture result         │   │
                    │          │          └──────────┬───────────┘   │
                    │          │                     │                 │
                    │          │                     ▼                 │
                    │          │          ┌──────────────────────┐   │
                    │          │          │ 7. MEMORY WRITER      │   │
                    │          │          │ (Python sidecar)      │   │
                    │          │          │ Store/update reflex   │   │
                    │          │          │ Update confidence     │   │
                    │          │          └──────────┬───────────┘   │
                    │          │                     │                 │
                    └──────────┴─────────────────────┘
                                            │
                                            ▼
                     ┌──────────────────────────────────────────────────────┐
                     │                  OUTPUT TO USER                       │
                     │  Result + reflex confidence indicator                │
                     └──────────────────────────────────────────────────────┘
```

### 1.3 The Reflex Short-Circuit (Critical Design Decision)

The most important architectural feature is the **reflex short-circuit**:

- **First time**: Full path — Input → Embed → Search → Assemble → LLM → Parse → Execute → Store
- **Second time (high confidence)**: Short path — Input → Embed → Search → **Match!** → Execute → Update

This means:
1. The system **gets faster as it learns** — repeated tasks skip LLM inference entirely
2. **Resource usage decreases over time** — LLM is only consulted for novel situations  
3. This is fundamentally different from a chatbot — it's a **learning operating system**

Confidence thresholds:

| Confidence | Path | Latency (Pi 4) | RAM Used |
|-----------|------|-----------------|----------|
| > 0.90 | Reflex direct | ~50ms | ~20MB (embed only) |
| 0.70–0.90 | Reflex + LLM confirmation | ~3s | ~1.2GB |
| < 0.70 | Full LLM reasoning | ~5s | ~1.2GB |

### 1.4 On-Device vs Optional

| Component | On-Device (MVP) | Optional (Expansion) |
|-----------|-----------------|---------------------|
| Input Normalizer | ✅ Always | Voice via Whisper (expansion) |
| Reflex Matcher | ✅ Always | JEPA world model pre-filter (expansion) |
| Context Assembler | ✅ Always | Penrose tensor composition (expansion) |
| LLM Runtime | ✅ TinyLlama 1.1B | Cloud LLM fallback; larger local models with CUDA |
| Action Parser | ✅ Always | Formal verification layer (expansion) |
| Sandbox Executor | ✅ bwrap | Docker/Podman (expansion) |
| Memory Writer | ✅ Always | JEPA predictive memory (expansion) |
| Embedding Model | ✅ MiniLM-L6 | Larger embed model with CUDA (expansion) |
| UI | CLI only | A2UI dynamic interface (expansion) |
| GPU Compute | ❌ Not required | cudaclaw for GPU-accelerated inference (expansion) |

---

## 2. MVP Data Model

### 2.1 Storage Strategy: SQLite + LanceDB Hybrid

**Why both, not just LanceDB?**

- LanceDB is excellent for vector similarity search but poor at relational queries
- SQLite is the world's most deployed database, proven on ARM, ~600KB footprint
- The hybrid gives us: LanceDB for "find reflexes like X" → SQLite for "show me reflex metadata for these IDs"

**The split:**

| Store | Holds | Why |
|-------|-------|-----|
| **SQLite** | Session metadata, shell profiles, plugin registry, action audit log, reflex IDs + metadata | Relational queries, transactions, exact lookups |
| **LanceDB** | Reflex embeddings (384-dim), memory embeddings, raw text chunks | Vector similarity search, columnar compression |

**Communication pattern:**
```
Query: "Find reflexes similar to X"
  1. Python sidecar: embed(X) → vector
  2. Python sidecar: LanceDB search(vector, top_k=5) → [reflex_ids]
  3. Rust core: SQLite SELECT * FROM reflexes WHERE id IN (...) → full metadata
```

### 2.2 SQLite Schema

```sql
-- /opt/pincheros/data/pincher.db

CREATE TABLE shells (
    id              TEXT PRIMARY KEY,          -- UUID v7
    device_type     TEXT NOT NULL,             -- "rpi4", "jetson_nano", "custom"
    fingerprint     TEXT NOT NULL UNIQUE,      -- hardware hash
    capabilities    TEXT NOT NULL,             -- JSON: {"ram_mb":4096, "cpu_cores":4, "gpu":"none"|"cuda_128"}
    limits          TEXT NOT NULL,             -- JSON: {"max_model_mb":1400, "max_concurrent":2}
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE reflexes (
    id              TEXT PRIMARY KEY,          -- UUID v7
    trigger_pattern TEXT NOT NULL,             -- Natural language trigger description
    action_template TEXT NOT NULL,             -- JSON: {"type":"shell"|"llm"|"composite", "template":"..."}
    confidence      REAL NOT NULL DEFAULT 0.5, -- 0.0–1.0, incremented on success
    usage_count     INTEGER NOT NULL DEFAULT 0,
    success_count   INTEGER NOT NULL DEFAULT 0,
    source          TEXT NOT NULL DEFAULT 'learned', -- "learned"|"imported"|"distilled"
    shell_id        TEXT REFERENCES shells(id), -- Shell where reflex was created
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used       TEXT,
    expires_at      TEXT                       -- Optional TTL for imported reflexes
);

CREATE INDEX idx_reflexes_confidence ON reflexes(confidence DESC);
CREATE INDEX idx_reflexes_source ON reflexes(source);
CREATE INDEX idx_reflexes_last_used ON reflexes(last_used);

CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,          -- UUID v7
    shell_id        TEXT REFERENCES shells(id),
    started_at      TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at        TEXT,
    reflexes_used   TEXT NOT NULL DEFAULT '[]' -- JSON array of reflex IDs
);

CREATE TABLE action_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT REFERENCES sessions(id),
    reflex_id       TEXT REFERENCES reflexes(id),
    action_type     TEXT NOT NULL,             -- "shell_command"|"llm_response"|"system"
    action_detail   TEXT NOT NULL,             -- The actual command or response
    exit_code       INTEGER,                   -- 0 = success
    stdout_snippet  TEXT,                      -- First 1KB of stdout
    stderr_snippet  TEXT,                      -- First 1KB of stderr
    duration_ms     INTEGER,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE plugin_registry (
    id              TEXT PRIMARY KEY,          -- Plugin name
    version         TEXT NOT NULL,
    hooks           TEXT NOT NULL,             -- JSON array of HookPoint names
    enabled         INTEGER NOT NULL DEFAULT 1,
    config          TEXT NOT NULL DEFAULT '{}' -- JSON plugin config
);

-- The rigging identity — migrates between shells
CREATE TABLE rigging (
    id              TEXT PRIMARY KEY,          -- Single row, UUID v7
    name            TEXT NOT NULL DEFAULT 'default',
    personality     TEXT NOT NULL DEFAULT '',  -- Accumulated personality/mode
    total_sessions  INTEGER NOT NULL DEFAULT 0,
    total_reflexes  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 2.3 LanceDB Schema

```python
# Reflex table — stored in /opt/pincheros/data/reflexes/
import lancedb
import pyarrow as pa

reflex_schema = pa.schema([
    pa.field("id", pa.string()),                    # UUID v7, foreign key to SQLite
    pa.field("embedding", pa.list_(pa.float32(), 384)),  # MiniLM-L6 embedding
    pa.field("trigger_text", pa.string()),           # Raw trigger text for re-embedding
    pa.field("context_tags", pa.list_(pa.string())),  # ["file_management", "organize"]
    pa.field("shell_fingerprint", pa.string()),       # Which shell created this
])

# Memory table — stored in /opt/pincheros/data/memories/
memory_schema = pa.schema([
    pa.field("id", pa.string()),                    # UUID v7
    pa.field("embedding", pa.list_(pa.float32(), 384)),
    pa.field("content", pa.string()),                # Raw memory text
    pa.field("memory_type", pa.string()),            # "interaction"|"observation"|"distillation"
    pa.field("session_id", pa.string()),             # Reference to session
    pa.field("timestamp", pa.string()),
])
```

### 2.4 Storage Budget on 4GB Device

| Item | Size | Notes |
|------|------|-------|
| SQLite DB | ~5–20MB | Grows slowly, vacuum monthly |
| LanceDB reflexes (50K reflexes) | ~75MB | 384 floats × 4 bytes × 50K + metadata |
| LanceDB memories (100K entries) | ~150MB | More text-heavy |
| **Total data** | **~250MB max** | Leaves ~1.2GB for LLM + OS |

**Compaction strategy**: When LanceDB exceeds 500MB, run compaction:
- Merge reflexes with >0.95 similarity into one
- Archive memories older than 90 days with confidence < 0.3
- Vacuum SQLite

---

## 3. MVP Tech Stack

### 3.1 Language: Rust Core + Python Inference Sidecar

```
┌─────────────────────────────────────────────────────────┐
│                   pincher-core (Rust)                    │
│  The "body" — system-level, fast, memory-safe            │
│                                                          │
│  Cargo.toml dependencies:                                │
│    rusqlite      0.31   — SQLite bindings                │
│    tokio         1.x    — Async runtime                  │
│    serde         1.x    — Serialization                  │
│    serde_json    1.x    — JSON handling                  │
│    uuid          1.x    — UUID v7 generation             │
│    sysinfo       0.30   — Hardware probing               │
│    axum          0.7    — Local HTTP + UDS server        │
│    tracing       0.1    — Structured logging             │
│    clap          4.x    — CLI argument parsing           │
│    anyhow        1.x    — Error handling                 │
│    rust-bert     0.22   — Optional: future ONNX in Rust  │
│                                                          │
│  Communicates with Python sidecar via:                    │
│    Unix Domain Socket (/opt/pincheros/run/infer.sock)    │
│    Protocol: JSON-RPC 2.0 over length-prefixed frames    │
└──────────────────────────┬──────────────────────────────┘
                           │ UDS (JSON-RPC 2.0)
┌──────────────────────────▼──────────────────────────────┐
│               pincher-infer (Python 3.11)                │
│  The "brain" — ML inference, vector operations           │
│                                                          │
│  requirements.txt:                                       │
│    lancedb              0.6.x   — Embedded vector DB     │
│    llama-cpp-python     0.2.x   — LLM inference         │
│    onnxruntime          1.17.x  — Embedding inference    │
│    sentence-transformers 2.x    — Embedding pipeline     │
│    numpy                1.26.x  — Numerical ops          │
│    pydantic             2.x     — Schema validation      │
│    fastapi              0.109.x — Local API server       │
│    uvicorn              0.27.x  — ASGI server            │
└─────────────────────────────────────────────────────────┘
```

**Why this split?**

| Concern | Rust (core) | Python (sidecar) |
|---------|-------------|-------------------|
| Start time | ~20ms | ~3s (lazy-load models) |
| Memory safety | ✅ Critical for OS layer | Less critical for inference |
| ML ecosystem | Poor (no llama.cpp bindings) | Rich (everything works) |
| Concurrency | tokio async, zero-cost | GIL-limited, but OK for inference |
| ARM support | Excellent | Good (manylinux wheels) |
| Sandboxing | Direct namespace calls | N/A — Rust handles it |

### 3.2 Vector DB: LanceDB

**Choice**: LanceDB v0.6+

**Why LanceDB over alternatives:**

| Option | ARM64 | Embedded | Rust API | Disk-based | Footprint |
|--------|-------|----------|----------|------------|-----------|
| **LanceDB** | ✅ | ✅ | ✅ (via FFI) | ✅ | ~10MB |
| ChromaDB | ✅ | ✅ | ❌ | Partial | ~50MB+ |
| Qdrant | ✅ | Client only | ❌ | ❌ (server) | ~100MB |
| Milvus Lite | ❌ | ✅ | ❌ | ✅ | ~200MB |
| SQLite-vec | ✅ | ✅ | ✅ | ✅ | ~1MB but immature |

**Key advantage**: LanceDB uses the Lance columnar format, which gives:
- O(1) random access to any vector without loading all into RAM
- Built-in compaction and versioning
- Incremental writes (no full reindex)
- Metadata filtering alongside vector search

### 3.3 Embedding Model: all-MiniLM-L6-v2 via ONNX Runtime

**Choice**: `sentence-transformers/all-MiniLM-L6-v2` exported to ONNX

| Spec | Value |
|------|-------|
| Model size | 22MB |
| Embedding dimensions | 384 |
| Inference time (Pi 4, single sentence) | ~30–50ms |
| Inference time (Jetson Nano, CPU) | ~25–40ms |
| Inference time (Jetson Nano, CUDA) | ~5–10ms |
| Max sequence length | 256 tokens |
| ONNX Runtime ARM64 | ✅ Supported |

**Why this model:**
- 384 dimensions keeps LanceDB storage small (384 × 4 bytes = 1.5KB per vector)
- 22MB fits comfortably in RAM alongside the LLM
- Quality is sufficient for reflex matching (cosine similarity for "did you mean this?" not for "understand this deeply")
- ONNX Runtime has first-class ARM64 + CUDA support

**Export pipeline (run once at build time):**
```bash
python -c "
from sentence_transformers import SentenceTransformer
model = SentenceTransformer('all-MiniLM-L6-v2')
model.save('/opt/pincheros/models/all-MiniLM-L6-v2')
"
# ONNX export via optimum:
python -c "
from optimum.onnxruntime import ORTModelForFeatureExtraction
model = ORTModelForFeatureExtraction.from_pretrained('all-MiniLM-L6-v2', export=True)
model.save_pretrained('/opt/pincheros/models/all-MiniLM-L6-v2-onnx')
"
```

### 3.4 LLM Runtime: llama.cpp via llama-cpp-python

**Choice**: `llama-cpp-python` with `llama.cpp` backend

**Default model**: `TinyLlama-1.1B-Chat-v1.0-GGUF` (Q4_K_M quantization)

| Spec | Value |
|------|-------|
| Model file size | ~700MB |
| RAM at inference | ~1.1GB |
| Tokens/sec (Pi 4, 4 threads) | ~5–8 t/s |
| Tokens/sec (Jetson Nano, CUDA) | ~15–25 t/s |
| Context window | 2048 tokens |
| Quantization | Q4_K_M (4-bit with importance matrix) |

**Upgrade path for Jetson Nano:**
- `Phi-2-2.7B-Q4_K_M.gguf` (~1.6GB) — enables CUDA via Jetson's 128 cores
- Configured in `/opt/pincheros/config/pincher.toml` → `[model]` section
- Snap algorithm auto-selects based on available RAM + GPU

**Why llama.cpp over alternatives:**

| Option | ARM64 | CUDA | GGUF | Python API | Footprint |
|--------|-------|------|------|------------|-----------|
| **llama.cpp** | ✅ | ✅ | ✅ | ✅ (python) | ~5MB binary |
| ONNX Runtime (direct) | ✅ | ✅ | ❌ | ✅ | ~50MB |
| candle (Rust) | ✅ | ✅ | ✅ | ❌ | ~10MB |
| ollama | ✅ | ✅ | ✅ | ✅ (HTTP) | ~200MB+ |

llama.cpp wins because: GGUF quantization is the most space-efficient, ARM NEON is heavily optimized, and CUDA support is mature for Jetson upgrade.

**Model loading strategy (critical for 4GB RAM):**

```python
# pincher-infer loads model LAZILY and can UNLOAD it
class InferenceServer:
    def __init__(self):
        self._llm = None  # Not loaded until first request
        self._embed_model = None
        self._last_inference = 0
        self._unload_after_s = 300  # Unload after 5 min idle
    
    @property
    def llm(self):
        if self._llm is None:
            self._llm = Llama(
                model_path=MODEL_PATH,
                n_ctx=2048,
                n_threads=4,
                n_gpu_layers=0,  # 0 for CPU; Snap sets this for CUDA
                use_mmap=True,
                use_mlock=False,  # Don't lock RAM on 4GB device
                verbose=False,
            )
        self._last_inference = time.time()
        return self._llm
    
    async def _unload_check(self):
        """Background task: unload model if idle > 5 min"""
        if self._llm and time.time() - self._last_inference > self._unload_after_s:
            del self._llm
            self._llm = None
            gc.collect()
```

### 3.5 Sandbox: bubblewrap (bwrap)

**Choice**: `bubblewrap` (bwrap)

**Why bwrap over alternatives:**

| Option | Footprint | Linux Namespaces | ARM64 | Complexity |
|--------|-----------|------------------|-------|------------|
| **bwrap** | ~200KB | ✅ | ✅ | Low |
| Docker | ~300MB | ✅ | ✅ | Very high |
| Podman | ~150MB | ✅ | ✅ | High |
| Firejail | ~1MB | ✅ | ✅ | Medium |
| nsjail | ~500KB | ✅ | ✅ | Medium |
| Pure subprocess | 0 | ❌ | ✅ | None |

bwrap wins because: it's what Flatpak uses, it's the lightest proper namespace sandbox, it's maintained by the GNOME project, and it works on stock Linux kernels (no patches needed).

**MVP sandbox profile:**

```bash
# /opt/pincheros/config/sandbox.profile
# Executed by Rust core via: bwrap --args FD < sandbox.profile

# Read-only system
--ro-bind /usr /usr
--ro-bind /lib /lib
--ro-bind /bin /bin
--ro-bind /sbin /sbin
--ro-bind /etc /etc

# Writable user home (scoped)
--bind ~/Downloads ~/Downloads        # Allowed directories (configured per reflex)
--bind /tmp/pincheros-sandbox /tmp    # Sandbox tmp

# Proc and dev
--proc /proc
--dev /dev
--ro-bind /sys /sys

# Network: DISABLED by default
--unshare-net

# Resource limits
--die-with-parent
--new-session
```

**Per-reflex filesystem access**: Each reflex specifies what directories it needs. The Rust core builds the bwrap command dynamically:

```rust
// Rust pseudo-code for sandbox execution
fn execute_in_sandbox(action: &ShellAction, shell: &ShellProfile) -> Result<ExecutionResult> {
    let mut cmd = Command::new("bwrap");
    
    cmd.arg("--ro-bind").arg("/usr").arg("/usr")
       .arg("--ro-bind").arg("/lib").arg("/lib")
       .arg("--proc").arg("/proc")
       .arg("--dev").arg("/dev")
       .arg("--unshare-net")          // No network by default
       .arg("--die-with-parent")
       .arg("--new-session");
    
    // Add writable directories from reflex's allowed_paths
    for path in &action.allowed_paths {
        cmd.arg("--bind").arg(path).arg(path);
    }
    
    // Add the command
    cmd.arg("--").arg("/bin/sh").arg("-c").arg(&action.command);
    
    // Set resource limits via prlimit
    // RLIMIT_AS: max 512MB virtual memory
    // RLIMIT_CPU: max 60 seconds CPU time
    // RLIMIT_FSIZE: max 100MB file creation
    
    let output = cmd.output()?;
    // ...
}
```

---

## 4. Minimum Shell

### 4.1 Snap Algorithm (MVP)

The Snap algorithm adapts the rigging to its current hardware shell. It runs:
1. **At boot** (every startup)
2. **On detect** (when hardware changes are sensed — USB device, new GPU)
3. **On migrate** (when a rigging is unpacked onto a new device)

```rust
// src/shell/snap.rs

use sysinfo::{System, SystemExt, CpuExt};

pub struct ShellProfile {
    pub device_type: DeviceType,
    pub fingerprint: String,
    pub capabilities: Capabilities,
    pub limits: Limits,
}

pub struct Capabilities {
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub cpu_cores: usize,
    pub cpu_freq_mhz: u64,
    pub gpu: GpuType,
    pub disk_free_mb: u64,
}

pub enum GpuType {
    None,
    Cuda { cores: usize, vram_mb: u64 },  // Jetson Nano: 128 cores, shared RAM
}

pub struct Limits {
    pub max_model_mb: u64,        // Max LLM model size to load
    pub max_concurrent: usize,    // Max concurrent reflex executions
    pub inference_threads: usize, // Threads for llama.cpp
    pub gpu_layers: u32,          // Layers to offload to GPU (0 if no GPU)
    pub sandbox_mem_mb: u64,      // Max memory per sandboxed action
    pub sandbox_cpu_secs: u64,    // Max CPU seconds per action
}

pub fn snap() -> ShellProfile {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let ram_total_mb = sys.total_memory() / 1024;
    let ram_available_mb = sys.available_memory() / 1024;
    let cpu_cores = sys.cpus().len();
    let cpu_freq_mhz = sys.cpus().first().map(|c| c.frequency()).unwrap_or(1000);
    
    // Detect GPU
    let gpu = detect_gpu();  // Check /sys/class/devfreq or nvidia-smi
    
    // Generate hardware fingerprint (stable across reboots, changes on hardware swap)
    let fingerprint = generate_fingerprint(&sys);
    
    // Calculate limits based on capabilities
    let limits = calculate_limits(ram_total_mb, ram_available_mb, cpu_cores, &gpu);
    
    // Classify device type
    let device_type = classify_device(ram_total_mb, &gpu);
    
    ShellProfile {
        device_type,
        fingerprint,
        capabilities: Capabilities {
            ram_total_mb,
            ram_available_mb,
            cpu_cores,
            cpu_freq_mhz,
            gpu,
            disk_free_mb: disk_free_space_mb(),
        },
        limits,
    }
}

fn calculate_limits(ram_total: u64, ram_available: u64, cores: usize, gpu: &GpuType) -> Limits {
    // Reserve 30% for OS + system overhead
    let usable_ram = (ram_available as f64 * 0.70) as u64;
    
    let (max_model_mb, gpu_layers) = match gpu {
        GpuType::Cuda { cores, vram_mb } => {
            // With CUDA: can use larger model, offload layers
            let model_budget = usable_ram * 55 / 100;  // 55% of usable for model
            let layers = std::cmp::min(32, *cores / 4) as u32; // Approx layer offload
            (model_budget, layers)
        }
        GpuType::None => {
            // CPU only: smaller model budget
            let model_budget = usable_ram * 45 / 100;  // 45% of usable for model
            (model_budget, 0)
        }
    };
    
    Limits {
        max_model_mb,
        max_concurrent: std::cmp::max(1, cores / 2),
        inference_threads: std::cmp::min(cores, 4),
        gpu_layers,
        sandbox_mem_mb: usable_ram * 20 / 100,   // 20% for sandbox
        sandbox_cpu_secs: 60,
    }
}

fn generate_fingerprint(sys: &System) -> String {
    // Hash: CPU model + total RAM + disk serial + MAC address
    // Stable across reboots, changes on hardware swap
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(sys.cpus().first().map(|c| c.brand()).unwrap_or("unknown"));
    hasher.update(sys.total_memory().to_le_bytes());
    // + disk serial, MAC address, etc.
    format!("{:x}", hasher.finalize())
}
```

**Snap results for target hardware:**

| Parameter | Raspberry Pi 4 | Jetson Nano |
|-----------|----------------|-------------|
| `max_model_mb` | ~960MB | ~1200MB |
| `max_concurrent` | 2 | 2 |
| `inference_threads` | 4 | 4 |
| `gpu_layers` | 0 | 16 |
| Default model | TinyLlama 1.1B Q4_K_M | TinyLlama 1.1B Q4_K_M (GPU offload) |
| Upgrade model | (none — too large) | Phi-2 2.7B Q4_K_M (full GPU) |

### 4.2 Self-Knowledge of Limits

The system continuously monitors its own resource usage:

```rust
// src/shell/monitor.rs

pub struct ResourceMonitor {
    last_check: Instant,
    check_interval: Duration,  // 5 seconds
}

impl ResourceMonitor {
    pub fn check(&mut self) -> ResourceStatus {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu();
        
        ResourceStatus {
            ram_used_pct: (sys.used_memory() * 100 / sys.total_memory()) as u8,
            cpu_used_pct: sys.global_cpu_info().cpu_usage() as u8,
            disk_used_pct: disk_usage_pct(),
            model_loaded: is_model_loaded(),
        }
    }
    
    pub fn should_degrade(&self, status: &ResourceStatus) -> Option<DegradationLevel> {
        if status.ram_used_pct > 90 {
            return Some(DegradationLevel::Critical);
        }
        if status.ram_used_pct > 80 {
            return Some(DegradationLevel::Moderate);
        }
        if status.ram_used_pct > 70 {
            return Some(DegradationLevel::Light);
        }
        None
    }
}
```

### 4.3 Graceful Degradation

```rust
// src/shell/degradation.rs

pub enum DegradationLevel {
    Light,     // 70-80% RAM
    Moderate,  // 80-90% RAM  
    Critical,  // >90% RAM
}

pub fn apply_degradation(level: DegradationLevel, infer: &InferenceBridge) {
    match level {
        DegradationLevel::Light => {
            // Reduce context window from 2048 to 1024
            infer.send_command(InferCommand::SetContextSize(1024));
        }
        DegradationLevel::Moderate => {
            // Unload LLM, keep only embedding model
            // All queries go to reflex short-circuit only
            infer.send_command(InferCommand::UnloadLLM);
            // New situations get a "I need to think about this" response
            // and queue for processing when resources free up
        }
        DegradationLevel::Critical => {
            // Unload everything except core reflex matcher
            infer.send_command(InferCommand::UnloadAll);
            // Only reflex short-circuit path works (no LLM at all)
            // System responds: "Running in low-power mode. Learned reflexes only."
        }
    }
}
```

**Degradation is reversible**: When resources free up, the system automatically promotes back up:

```
Critical → Moderate: RAM drops below 85% → reload embedding model
Moderate → Light: RAM drops below 75% → reload LLM with reduced context
Light → Normal: RAM drops below 65% → full LLM with 2048 context
```

---

## 5. Migration Story

### 5.1 Distill a CLI Tool into a PincherOS Reflex

**Command:**
```bash
pincher distill --observe "ffmpeg -i input.mp4 -vf scale=1280:720 output.mp4"
```

**What happens:**

```
1. USER RUNS: pincher distill --observe "ffmpeg -i input.mp4 -vf scale=1280:720 output.mp4"

2. RUST CORE:
   - Executes the command in sandbox
   - Captures: stdin, stdout, stderr, exit code, file changes
   - Records: command template, arguments, filesystem paths touched

3. PYTHON SIDECAR:
   - LLM analyzes the observation:
     "This command uses ffmpeg to resize a video to 1280x720.
      The pattern is: ffmpeg -i {input_file} -vf scale={width}:{height} {output_file}"
   
   - Generates a ReflexTemplate:
     {
       "trigger_pattern": "resize video | scale video | change video resolution",
       "action_template": {
         "type": "shell",
         "template": "ffmpeg -i {input_file} -vf scale={width}:{height} {output_file}",
         "params": [
           {"name": "input_file", "type": "path", "required": true},
           {"name": "width", "type": "integer", "default": 1280},
           {"name": "height", "type": "integer", "default": 720},
           {"name": "output_file", "type": "path", "required": true}
         ],
         "allowed_paths": ["~/Videos", "~/Downloads"]
       },
       "source": "distilled"
     }

4. STORAGE:
   - Embed trigger_pattern → store in LanceDB
   - Store metadata in SQLite
   - Reflex starts at confidence 0.5

5. RESULT:
   → Distilled reflex: resize_video (confidence: 0.50)
   → Trigger: "resize video", "scale video", "change video resolution"
   → Try it: pincher "resize my video to 1920x1080"
```

**Alternative: distill from man page**

```bash
pincher distill --from-man tar
# Reads tar(1), generates reflexes for common tar operations
# Creates reflexes: tar_extract, tar_create, tar_list, tar_gz_extract, etc.
```

### 5.2 Import a Skillpack

A **skillpack** is a portable `.nail` file (named after a fingernail — the shell's "nail").

**Command:**
```bash
pincher import ./video-tools.nail
```

**`.nail` file format** (it's a tar.gz):

```
video-tools.nail/
├── manifest.json            # Metadata
├── reflexes/
│   ├── data/                # LanceDB export (parquet files)
│   │   ├── part-0.parquet
│   │   └── ...
│   └── metadata.json        # Reflex metadata export
├── templates/
│   └── actions.json         # Action templates
├── scripts/
│   ├── validate.sh          # Optional: validate prerequisites
│   └── setup.sh             # Optional: install dependencies
└── metadata/
    ├── description.md       # Human-readable description
    └── changelog.md         # Version history
```

**manifest.json:**
```json
{
  "name": "video-tools",
  "version": "1.0.0",
  "pincher_version_min": "0.1.0",
  "author": "pincher-community",
  "description": "Video manipulation reflexes using ffmpeg",
  "reflex_count": 15,
  "dependencies": {
    "commands": ["ffmpeg", "ffprobe"],
    "disk_mb": 10,
    "ram_mb": 0
  },
  "checksum": "sha256:abc123..."
}
```

**Import flow:**

```
1. Validate: checksum, pincher_version_min, dependencies
2. Run: validate.sh (check ffmpeg is installed)
3. Run: setup.sh (install ffmpeg if missing, with user consent)
4. Import: LanceDB parquet → merge into reflexes table
5. Import: metadata.json → insert into SQLite reflexes table
6. Re-embed: trigger_patterns through local embedding model
   (Why? Because the embedding model must be consistent —
    the exporter's model might differ from ours)
7. Set: source="imported", confidence=0.5, expires_at=now+90days
8. Report: "Imported 15 reflexes from video-tools.nail"
```

**Auto-expiry**: Imported reflexes start at confidence 0.5 and expire in 90 days. If they're used successfully, confidence increases and expiry is removed. This prevents skillpack bloat — unused imported reflexes clean themselves up.

### 5.3 Migrate from One Shell to Another

**Export (pack):**
```bash
# On the OLD shell:
pincher pack --output ~/my-rigging.nail
```

This creates a `.nail` file containing:
- The rigging identity (personality, name, session count)
- ALL reflexes (both LanceDB vectors and SQLite metadata)
- Recent memories (last 30 days)
- Shell profile from the current device (for Snap comparison)

**Import (unpack):**
```bash
# On the NEW shell:
pincher unpack ~/my-rigging.nail
```

**Migration flow:**

```
1. UNPACK my-rigging.nail

2. SNAP: Run snap() on new hardware → new ShellProfile

3. COMPARE: New vs old shell profile
   - If new shell has MORE capability → promote reflexes, enable larger model
   - If new shell has LESS capability → compact reflexes, use smaller model

4. ADAPT:
   - Re-embed all reflexes (embedding model is consistent, but re-index in LanceDB)
   - Update sandbox profiles for new filesystem layout
   - Run Snap to set new Limits
   - Auto-select model based on new Limits.max_model_mb

5. VERIFY:
   - Test top-5 most-used reflexes on new hardware
   - If any fail, reduce confidence and flag for re-learning

6. READY:
   → "Rigging migrated! 142 reflexes loaded, 3 need re-learning."
   → "Model: TinyLlama 1.1B (auto-selected for 4GB shell)"
```

**Identity persistence**: The rigging keeps its UUID, name, and accumulated personality across migrations. It's the same "mind" in a different "body."

---

## 6. Expansion Hooks

### 6.1 Plugin Architecture

```rust
// src/plugin/api.rs

/// Every PincherOS plugin implements this trait
#[async_trait]
pub trait PincherPlugin: Send + Sync {
    /// Unique plugin identifier (e.g., "cudaclaw", "jepa-world-model")
    fn name(&self) -> &str;
    
    /// Semantic version
    fn version(&self) -> &str;
    
    /// Which hook points this plugin subscribes to
    fn hooks(&self) -> Vec<HookPoint>;
    
    /// Initialize plugin with access to PincherOS context
    async fn init(&mut self, ctx: &PincherContext) -> Result<(), PluginError>;
    
    /// Shutdown gracefully
    async fn shutdown(&mut self) -> Result<(), PluginError>;
}

/// Hook points in the core loop where plugins can intervene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookPoint {
    /// Before context assembly — can augment or pre-filter reflex candidates
    PreContext,
    
    /// After context assembly, before LLM — can modify the prompt
    PostContext,
    
    /// After LLM inference, before action parsing — can validate/modify output
    PostInference,
    
    /// After action execution — can observe outcomes, trigger side effects
    PostAction,
    
    /// Before writing to memory — can filter, compress, or enrich memories
    MemoryWrite,
    
    /// During model loading — can swap models, configure GPU layers, etc.
    ModelLoad,
    
    /// During hardware adaptation (Snap) — can override limit calculations
    ShellSnap,
    
    /// For UI plugins — can render dynamic interfaces
    UI,
    
    /// Custom hook point for plugin-specific communication
    Custom(String),
}

/// Context object passed to plugins at init time
pub struct PincherContext {
    /// Read/write access to SQLite (via message passing to core)
    pub db: DbHandle,
    
    /// Read/write access to LanceDB (via Python sidecar bridge)
    pub vectors: VectorHandle,
    
    /// Current shell profile and limits
    pub shell: ShellProfile,
    
    /// Configuration for this plugin (from pincher.toml)
    pub config: toml::Value,
    
    /// Channel for sending commands back to the core loop
    pub core_channel: mpsc::Sender<CoreCommand>,
}

/// Plugin execution context — passed during hook invocations
pub struct HookContext {
    /// The current input event
    pub input: Option<InputEvent>,
    
    /// Matched reflexes (for PreContext/PostContext hooks)
    pub reflexes: Option<Vec<ReflexMatch>>,
    
    /// Assembled context/prompt (for PostContext hooks)
    pub context: Option<String>,
    
    /// LLM output (for PostInference hooks)
    pub inference_output: Option<String>,
    
    /// Execution result (for PostAction hooks)
    pub execution_result: Option<ExecutionResult>,
    
    /// Memory being written (for MemoryWrite hooks)
    pub memory: Option<MemoryEntry>,
}
```

**Plugin loading:**

```rust
// src/plugin/loader.rs

pub struct PluginManager {
    plugins: Vec<Box<dyn PincherPlugin>>,
    hook_subscribers: HashMap<HookPoint, Vec<usize>>,  // hook → plugin indices
}

impl PluginManager {
    /// Load plugins from /opt/pincheros/plugins/
    /// Each plugin is a directory with:
    ///   - plugin.toml (metadata)
    ///   - libplugin.so (Rust dynamic library) OR
    ///   - main.py (Python plugin, runs as subprocess)
    pub async fn load_all(&mut self, ctx: &PincherContext) -> Result<()> {
        let plugin_dir = Path::new("/opt/pincheros/plugins");
        for entry in fs::read_dir(plugin_dir)? {
            let path = entry?.path();
            if path.join("plugin.toml").exists() {
                self.load_plugin(&path, ctx).await?;
            }
        }
        Ok(())
    }
    
    /// Invoke all plugins subscribed to a hook point
    pub async fn invoke(&self, hook: HookPoint, ctx: &mut HookContext) -> Result<()> {
        if let Some(indices) = self.hook_subscribers.get(&hook) {
            for &idx in indices {
                self.plugins[idx].on_hook(&hook, ctx).await?;
            }
        }
        Ok(())
    }
}
```

### 6.2 Where Each Expansion Plugs In

#### JEPA (Joint-Embedding Predictive Architecture)

**Hook points**: `PreContext`, `PostAction`, `MemoryWrite`

```
How it works:
1. PreContext: JEPA predicts what context SHOULD be relevant based on
   the input. This pre-filters reflex candidates before embedding search,
   making retrieval faster and more accurate.

2. PostAction: JEPA validates whether the action outcome matches its
   world-model prediction. If outcome diverges from prediction,
   flag for learning (the model's world model was wrong).

3. MemoryWrite: JEPA compresses memories by storing only the
   prediction error (delta from what the world model expected),
   not the full experience. This dramatically reduces storage.

MVP status: NOT included. When added, JEPA is a Rust plugin
that loads an ONNX model and implements the PincherPlugin trait.
It sits in /opt/pincheros/plugins/jepa-world-model/

Config in pincher.toml:
  [plugins.jepa-world-model]
  enabled = false
  model_path = "models/jepa-v1.onnx"
  prediction_threshold = 0.8
```

#### cudaclaw (CUDA Acceleration Layer)

**Hook points**: `ModelLoad`, `ShellSnap`, `Custom("inference")`

```
How it works:
1. ModelLoad: cudaclaw overrides the default model loading to:
   - Detect available CUDA devices
   - Set n_gpu_layers appropriately
   - Select larger models when CUDA VRAM allows
   - Manage GPU memory allocation

2. ShellSnap: cudaclaw extends the Snap algorithm to account for
   GPU capabilities, adjusting limits.max_model_mb upward when
   GPU offloading is available.

3. Custom("inference"): cudaclaw intercepts inference calls to
   route them through CUDA-accelerated paths:
   - Tensor operations via cuBLAS
   - Token generation with KV-cache optimization
   - Batched inference for concurrent requests

MVP status: NOT included. On Jetson Nano, basic CUDA support
comes from llama.cpp's built-in CUDA backend (n_gpu_layers > 0).
cudaclaw extends this with GPU memory management, multi-GPU support,
and custom CUDA kernels for future tensor operations.

Config in pincher.toml:
  [plugins.cudaclaw]
  enabled = false
  auto_detect = true
  vram_reserve_mb = 256
  custom_kernels = false
```

#### A2UI (Adaptive Application UI)

**Hook points**: `UI`, `PostAction`

```
How it works:
1. UI: When the rigging detects a display is available, A2UI
   generates a dynamic interface based on:
   - Available reflexes (each reflex can declare a UI widget)
   - Current context (what the user is likely to want)
   - Shell capabilities (touch? keyboard? voice?)
   - User preference history

2. PostAction: A2UI updates the interface to show action results,
   confirmations, and next-step suggestions.

MVP status: NOT included. MVP uses CLI only.
When added, A2UI is a Python plugin that generates HTML/CSS/JS
and renders via a lightweight webview (webkitgtk on Pi) or
a local web server accessible from any browser.

Config in pincher.toml:
  [plugins.a2ui]
  enabled = false
  render_mode = "webview"  # "webview" | "webserver" | "tty"
  port = 8080
  theme = "auto"
```

#### Penrose Tensors

**Hook points**: `PreContext`, `Custom("reasoning")`

```
How it works:
1. PreContext: Penrose tensors provide structured reasoning over
   the relationships between reflexes. Instead of flat cosine
   similarity, they compute higher-order relationships:
   - Reflex A is a SPECIALIZATION of Reflex B
   - Reflex C CONFLICTS with Reflex D
   - Reflex E REQUIRES Reflex F to have run first

2. Custom("reasoning"): When the LLM encounters a complex query,
   Penrose tensors compose multiple reflexes into a plan,
   considering dependencies and conflicts. This enables
   multi-step autonomous behavior.

MVP status: NOT included. The MVP uses flat cosine similarity
for reflex matching. Penrose tensors require a graph reasoning
engine and structured reflex metadata that goes beyond the MVP schema.

When added: Penrose tensors live in /opt/pincheros/plugins/penrose/
and extend the reflex schema with a `relations` field:
  ALTER TABLE reflexes ADD COLUMN relations TEXT DEFAULT '{}';
  -- JSON: {"specializes": ["reflex_id_1"], "conflicts": ["reflex_id_2"], ...}

Config in pincher.toml:
  [plugins.penrose]
  enabled = false
  reasoning_depth = 3
  max_composition_steps = 5
```

### 6.3 Plugin Config in pincher.toml

```toml
# /opt/pincheros/config/pincher.toml

[core]
log_level = "info"                # trace | debug | info | warn | error
data_dir = "/opt/pincheros/data"
run_dir = "/opt/pincheros/run"
plugin_dir = "/opt/pincheros/plugins"

[shell]
auto_snap = true                  # Run Snap at startup
degradation = true                # Enable graceful degradation
idle_unload_seconds = 300         # Unload LLM after 5 min idle

[model]
# "auto" = Snap selects based on hardware
model_path = "auto"               # Or explicit: "models/phi-2-q4_k_m.gguf"
context_size = 2048               # Tokens
inference_threads = 4             # 0 = auto (uses CPU count)
gpu_layers = 0                    # 0 = CPU only; set by Snap or cudaclaw

[embedding]
model_path = "models/all-MiniLM-L6-v2-onnx"
dimensions = 384
max_seq_length = 256

[reflex]
match_threshold = 0.70            # Below this = full LLM path
short_circuit_threshold = 0.90    # Above this = reflex direct
max_results = 5                   # Top-K reflexes to retrieve
confidence_increment = 0.05       # Boost per successful use
confidence_decay_days = 90        # Decay period for unused reflexes

[sandbox]
backend = "bwrap"                 # "bwrap" | "none" (for development only)
default_timeout_secs = 60
max_memory_mb = 512
network = false                   # Allow network in sandbox?

[migration]
auto_expire_days = 90             # Imported reflexes expire after 90 days
relearn_threshold = 0.30          # Below this confidence = needs re-learning

# --- EXPANSION PLUGINS (all disabled by default) ---

[plugins.jepa-world-model]
enabled = false
# model_path = "models/jepa-v1.onnx"
# prediction_threshold = 0.8

[plugins.cudaclaw]
enabled = false
# auto_detect = true
# vram_reserve_mb = 256

[plugins.a2ui]
enabled = false
# render_mode = "webview"
# port = 8080

[plugins.penrose]
enabled = false
# reasoning_depth = 3
```

---

## 7. One-Line Install

### 7.1 The Install Command

```bash
curl -fsSL https://pincher.dev/install | bash
```

**What this does:**

```bash
#!/bin/bash
# install.sh — PincherOS MVP installer

set -euo pipefail

ARCH=$(uname -m)  # aarch64 on Pi 4 / Jetson Nano
INSTALL_DIR="/opt/pincheros"
DATA_DIR="${INSTALL_DIR}/data"
CONFIG_DIR="${INSTALL_DIR}/config"
PLUGIN_DIR="${INSTALL_DIR}/plugins"

echo "🦀 PincherOS MVP Installer"
echo "   Architecture: ${ARCH}"
echo "   Target: ${INSTALL_DIR}"

# 1. Check prerequisites
command -v python3 >/dev/null || { echo "ERROR: python3 required"; exit 1; }
python3 -c "import sys; assert sys.version_info >= (3, 11)" || { echo "ERROR: Python 3.11+ required"; exit 1; }

# 2. Create directories
mkdir -p "${INSTALL_DIR}"/{bin,lib,pincher_py,models,data,config,plugins,run}

# 3. Download and install pincher-core (Rust binary)
curl -fsSL "https://github.com/pincheros/pincher-core/releases/latest/download/pincher-core-${ARCH}.tar.gz" \
  | tar xz -C "${INSTALL_DIR}/bin/"

# 4. Set up Python virtual environment
python3 -m venv "${INSTALL_DIR}/venv"
source "${INSTALL_DIR}/venv/bin/activate"

# 5. Install Python dependencies
pip install --quiet \
    lancedb==0.6.* \
    llama-cpp-python==0.2.* \
    onnxruntime==1.17.* \
    sentence-transformers==2.* \
    numpy==1.26.* \
    pydantic==2.* \
    fastapi==0.109.* \
    uvicorn==0.27.*

# 6. Download default model (TinyLlama 1.1B Q4_K_M)
echo "📦 Downloading TinyLlama 1.1B (Q4_K_M)..."
curl -fsSL "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf" \
  -o "${INSTALL_DIR}/models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf"

# 7. Download embedding model
echo "📦 Downloading embedding model..."
python3 -c "
from sentence_transformers import SentenceTransformer
model = SentenceTransformer('all-MiniLM-L6-v2')
model.save('${INSTALL_DIR}/models/all-MiniLM-L6-v2')
"

# 8. Initialize SQLite database
"${INSTALL_DIR}/bin/pincher-core" init-db

# 9. Install bubblewrap (sandbox)
if ! command -v bwrap >/dev/null; then
    apt-get install -y bubblewrap 2>/dev/null || echo "WARNING: bwrap not installed. Sandbox disabled."
fi

# 10. Install systemd service
cat > /etc/systemd/system/pincheros.service << 'EOF'
[Unit]
Description=PincherOS Core Daemon
After=network.target

[Service]
Type=simple
ExecStart=/opt/pincheros/bin/pincher-core daemon
Restart=on-failure
RestartSec=5
Environment=PINCHER_HOME=/opt/pincheros

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable pincheros

# 11. Add shell alias
echo 'alias pincher="/opt/pincheros/bin/pincher"' >> ~/.bashrc

# 12. Start the daemon
systemctl start pincheros

echo ""
echo "✅ PincherOS installed!"
echo ""
echo "   First run:  pincher \"hello\""
echo "   Configure:  nano ${CONFIG_DIR}/pincher.toml"
echo "   Status:     pincher status"
```

### 7.2 What Gets Installed (Disk Budget)

| Item | Size | Path |
|------|------|------|
| Rust binary (pincher-core) | ~15MB | `/opt/pincheros/bin/` |
| Python venv + packages | ~250MB | `/opt/pincheros/venv/` |
| TinyLlama 1.1B Q4_K_M | ~700MB | `/opt/pincheros/models/` |
| Embedding model (MiniLM-L6) | ~90MB | `/opt/pincheros/models/` |
| SQLite DB (empty) | ~1MB | `/opt/pincheros/data/` |
| Config files | ~5KB | `/opt/pincheros/config/` |
| **Total** | **~1.06GB** | |

### 7.3 The First Thing the User Does

```bash
pincher "hello"
```

This triggers:
1. Rust core receives the input
2. Snap runs (first time) → profiles the hardware
3. Python sidecar starts (lazy, first time = ~5s cold start)
4. Embedding model loads (~2s)
5. LLM loads (~3s)
6. "hello" embeds → search reflexes → none found (first time!) → full LLM path
7. LLM generates greeting → output to user

**Subsequent starts**: Python sidecar stays warm. Response in ~500ms.

### 7.4 The "Magic Moment" Demo

The user should experience the learning loop within 60 seconds of first use:

```bash
$ pincher "create a folder called projects in my home directory"
→ 🧠 Learning new reflex... (LLM reasoning)
→ 📂 mkdir -p ~/projects
→ ✅ Done. Reflex learned: create_folder (confidence: 0.50)

$ pincher "create a folder called documents in my home directory"
→ ⚡ Reflex matched: create_folder (confidence: 0.72) → confirming...
→ 📂 mkdir -p ~/documents
→ ✅ Done. Reflex reinforced: create_folder (confidence: 0.75)

$ pincher "make a folder called photos in my home directory"
→ ⚡ Reflex matched: create_folder (confidence: 0.90) → executing directly!
→ 📂 mkdir -p ~/photos
→ ✅ Done. Reflex reinforced: create_folder (confidence: 0.95)

$ pincher "make a folder called music in home"
→ ⚡ Reflex matched: create_folder (confidence: 0.97) → MUSCLE MEMORY
→ 📂 mkdir -p ~/music
→ ✅ Done.
```

**What the user sees**: 
1. First request: slow, thoughtful (LLM is reasoning)
2. Second request: faster, with confirmation (LLM validates)
3. Third request: fast, direct execution (reflex fires)
4. Fourth request: instant (muscle memory, no LLM needed)

This is the core aha moment — **the system learns from its own actions and gets faster over time**.

---

## 8. MVP vs Full Vision

| Dimension | MVP (0.1.0) | Full Vision (1.0.0+) |
|-----------|-------------|----------------------|
| **Core Loop** | 7 components: Input → Reflex Match → Context → LLM → Parse → Execute → Memory | + JEPA pre-filter, Penrose reasoning, A2UI rendering, cudaclaw acceleration |
| **Reflex Matching** | Flat cosine similarity (MiniLM-L6, 384-dim) | Penrose tensor composition with dependency graphs and conflict resolution |
| **LLM** | TinyLlama 1.1B Q4_K_M (~700MB, CPU only) | Dynamic model selection: tiny → small → medium → large based on shell capability |
| **Embedding** | MiniLM-L6 (384-dim, 22MB, CPU) | Task-specific embedders; JEPA joint embeddings; multi-modal embeddings |
| **World Model** | None (LLM generates from context only) | JEPA world model that predicts outcomes before execution |
| **Reasoning** | Single-shot LLM inference | Multi-step Penrose tensor composition with backtracking |
| **Memory** | LanceDB + SQLite, append-only with confidence decay | JEPA predictive memory: store only prediction errors, reconstruct from world model |
| **UI** | CLI only (`pincher "..."`) | A2UI: dynamic interfaces generated from available reflexes, shell capabilities, and user context |
| **GPU** | CPU only (ARM NEON optimized) | cudaclaw: CUDA acceleration, GPU memory management, custom kernels |
| **Sandbox** | bwrap (Linux namespaces) | Multi-tier: bwrap → Docker → hardware isolation for critical ops |
| **Shell Adaptation** | Snap: probe hardware → set limits | Snap + continuous adaptation, thermal throttling, battery awareness |
| **Degradation** | 3 levels (Light/Moderate/Critical) → unload model | Smooth degradation with predictive resource allocation, proactive model swapping |
| **Migration** | Pack/unpack .nail files, manual | Automatic migration via network, incremental sync, hot-swapping |
| **Distillation** | Observe CLI command → extract template | Observe any app (GUI, web, API), record user behavior, generalize patterns |
| **Skillpacks** | Manual import of .nail files | Skillpack marketplace, auto-discovery, dependency resolution |
| **Plugins** | Trait-based Rust plugins, loaded at startup | Hot-loadable plugins, WASM sandboxing, plugin marketplace |
| **Reflex Types** | `shell` (command) + `llm` (conversational) | + `composite` (multi-step), `conditional` (branching), `reactive` (event-driven), `sensor` (hardware) |
| **Concurrency** | Single interaction at a time | Multiple concurrent sessions, background reflexes, proactive actions |
| **Networking** | None (fully offline) | Optional cloud LLM fallback, P2P rigging sync, remote shell access |
| **Security** | bwrap sandbox, command whitelist | Formal verification of action plans, capability-based security, attestation |
| **Target Hardware** | Pi 4 / Jetson Nano (4GB) | Any Linux device from 2GB (tiny mode) to 128GB+ (full mode) |
| **Install Size** | ~1.1GB | ~2GB base + optional model downloads |
| **Cold Start** | ~10s (first use) | ~3s (warm) + progressive loading |
| **Reflex Short-Circuit** | ✅ Yes — core feature | ✅ Yes — enhanced with JEPA prediction validation |
| **Offline** | ✅ Fully offline | ✅ Fully offline + optional online features |

---

## 9. File System Layout

```
/opt/pincheros/
├── bin/
│   ├── pincher                    # Main CLI (Rust binary)
│   ├── pincher-core               # Core daemon (Rust binary)
│   └── pincher-infer              # Inference server entry point (Python)
│
├── venv/                          # Python 3.11 virtual environment
│   ├── bin/
│   │   ├── python3
│   │   ├── uvicorn
│   │   └── ...
│   └── lib/
│       └── python3.11/
│           └── site-packages/     # lancedb, llama-cpp-python, onnxruntime, etc.
│
├── models/
│   ├── tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf    # Default LLM (~700MB)
│   ├── all-MiniLM-L6-v2/                        # Sentence transformer model
│   │   ├── config.json
│   │   ├── tokenizer.json
│   │   └── model.onnx
│   └── README.md                                 # Model catalog
│
├── data/
│   ├── pincher.db                 # SQLite: sessions, metadata, audit log
│   ├── reflexes/                  # LanceDB: reflex embeddings
│   │   ├── data/
│   │   │   └── *.lance
│   │   └── versions/
│   └── memories/                  # LanceDB: memory embeddings
│       ├── data/
│       │   └── *.lance
│       └── versions/
│
├── config/
│   ├── pincher.toml               # Main configuration
│   ├── sandbox.profile            # bwrap sandbox defaults
│   └── reflex-whitelist.toml      # Allowed commands for reflex execution
│
├── plugins/                       # Expansion plugins
│   └── .gitkeep
│
├── run/                           # Runtime files (pid, socket)
│   ├── pincher-core.pid
│   └── infer.sock                 # Unix domain socket (Rust ↔ Python)
│
└── pincher_py/                    # Python inference package
    ├── __init__.py
    ├── server.py                  # FastAPI + UDS server
    ├── inference.py               # LLM inference wrapper
    ├── embed.py                   # Embedding generation
    ├── memory.py                  # LanceDB read/write
    ├── distill.py                 # Reflex distillation logic
    └── schema.py                  # Pydantic models for UDS protocol

~/.pincher/                        # User-specific data
├── rigging/                       # Rigging identity
│   ├── identity.toml              # Name, UUID, personality
│   └── preferences.toml           # User preferences
└── history/                       # Shell history (for distillation)
    └── bash_history.json          # Imported shell history
```

### 9.1 Reflex Whitelist

```toml
# /opt/pincheros/config/reflex-whitelist.toml

# Commands allowed in shell-type reflexes
# Format: command = [allowed_flags]
# Wildcard * means any flags allowed

[allowed_commands]
mkdir = ["-p"]
cp = ["-r", "-v"]
mv = []
rm = ["-r", "-f"]       # ⚠️ Requires confirmation
cat = []
grep = []
find = []
ffmpeg = ["*"]
ffprobe = ["*"]
python3 = ["*"]
pip = ["install", "show"]
git = ["*"]
curl = ["*"]
wget = []
tar = ["*"]
ls = []
echo = []
chmod = []
chown = []

[blocked_patterns]
# These patterns are NEVER allowed in any reflex
deny = [
    "rm -rf /",
    "dd if=",
    ":(){ :|:& };:",
    "mkfs",
    "fdisk",
    "> /dev/sd",
]

[confirmation_required]
# These commands require user confirmation before execution
require_confirm = [
    "rm",
    "mv",       # If moving outside allowed_paths
    "chmod 777",
    "pip install",
]
```

---

## 10. API Protocols

### 10.1 UDS Protocol (Rust Core ↔ Python Sidecar)

Communication over Unix Domain Socket using JSON-RPC 2.0 with length-prefixed framing.

**Frame format:**
```
[4 bytes: message length, big-endian u32][JSON-RPC 2.0 message]
```

**Methods:**

```python
# --- Embedding ---

# Request: Generate embedding for text
{"jsonrpc": "2.0", "method": "embed", "params": {"text": "resize video"}, "id": 1}
# Response:
{"jsonrpc": "2.0", "result": {"embedding": [0.123, -0.456, ...], "dimensions": 384}, "id": 1}

# --- Vector Search ---

# Request: Search for similar reflexes
{"jsonrpc": "2.0", "method": "search_reflexes", 
 "params": {"embedding": [...], "top_k": 5, "threshold": 0.5}, "id": 2}
# Response:
{"jsonrpc": "2.0", "result": {
  "matches": [
    {"id": "uuid-1", "score": 0.94, "trigger": "resize video", "confidence": 0.85},
    {"id": "uuid-2", "score": 0.72, "trigger": "compress video", "confidence": 0.60}
  ]
}, "id": 2}

# --- LLM Inference ---

# Request: Generate response
{"jsonrpc": "2.0", "method": "infer",
 "params": {
   "prompt": "<|user|>\nOrganize downloads by file type\n<|assistant|">\n",
   "max_tokens": 512,
   "temperature": 0.7,
   "stop": ["<|user|>"]
 }, "id": 3}
# Response:
{"jsonrpc": "2.0", "result": {
  "text": "I'll organize your downloads by creating folders...",
  "tokens_generated": 87,
  "duration_ms": 5200
}, "id": 3}

# --- Memory ---

# Request: Write reflex
{"jsonrpc": "2.0", "method": "write_reflex",
 "params": {
   "trigger": "organize downloads by file type",
   "action_template": {"type": "shell", "template": "cd ~/Downloads && mkdir -p {categories} && find . -maxdepth 1 -type f ..."},
   "source": "learned"
 }, "id": 4}
# Response:
{"jsonrpc": "2.0", "result": {"id": "uuid-new", "status": "created"}, "id": 4}

# Request: Update reflex confidence
{"jsonrpc": "2.0", "method": "update_confidence",
 "params": {"id": "uuid-1", "success": true}, "id": 5}
# Response:
{"jsonrpc": "2.0", "result": {"id": "uuid-1", "confidence": 0.90}, "id": 5}

# --- Model Management ---

# Request: Unload LLM to free RAM
{"jsonrpc": "2.0", "method": "unload_llm", "params": {}, "id": 6}
# Response:
{"jsonrpc": "2.0", "result": {"status": "unloaded", "freed_mb": 1100}, "id": 6}

# Request: Load LLM
{"jsonrpc": "2.0", "method": "load_llm",
 "params": {"model_path": "models/phi-2-q4_k_m.gguf", "gpu_layers": 16}, "id": 7}
# Response:
{"jsonrpc": "2.0", "result": {"status": "loaded", "model_mb": 1600}, "id": 7}
```

### 10.2 CLI API (User ↔ Rust Core)

```bash
# Primary interaction
pincher "natural language request"

# Reflex management
pincher reflex list                    # List all reflexes
pincher reflex show <id>               # Show reflex details
pincher reflex delete <id>             # Delete a reflex
pincher reflex test <id>               # Test a reflex without executing

# Distillation
pincher distill --observe "command"    # Observe and learn from a command
pincher distill --from-man <cmd>       # Learn from a man page
pincher distill --from-history         # Learn from shell history

# Migration
pincher pack --output <file.nail>      # Export rigging
pincher unpack <file.nail>             # Import rigging

# Skillpack management
pincher import <file.nail>             # Import a skillpack
pincher export --reflexes <ids> --output <file.nail>  # Export selected reflexes

# System
pincher status                         # Show system status
pincher snap                           # Re-run Snap algorithm
pincher shell                          # Show current shell profile
pincher config edit                    # Open config in $EDITOR

# Daemon control
pincher daemon start                   # Start the core daemon
pincher daemon stop                    # Stop the core daemon
pincher daemon restart                 # Restart (picks up config changes)
```

---

## 11. Resource Budget

### 11.1 RAM Budget (4GB Raspberry Pi 4)

| Component | RAM (Idle) | RAM (Active) | Notes |
|-----------|-----------|-------------|-------|
| Linux OS + system | ~400MB | ~400MB | Debian minimal |
| pincher-core (Rust) | ~15MB | ~30MB | Daemon + sandbox |
| Python sidecar (base) | ~50MB | ~50MB | Python interpreter |
| Embedding model (loaded) | ~80MB | ~80MB | MiniLM-L6 ONNX |
| LLM model (loaded) | ~1.1GB | ~1.1GB | TinyLlama Q4_K_M |
| LanceDB cache | ~20MB | ~50MB | Depends on query volume |
| SQLite | ~5MB | ~10MB | Minimal |
| bwrap sandbox | ~5MB | ~50MB | Per-execution (freed after) |
| **Total** | **~675MB** | **~1.77GB** | **Leaves ~2.2GB free** |

**Key insight**: With lazy model loading, the idle state uses only ~675MB. The LLM loads on demand and unloads after 5 minutes idle. This means the Pi 4 can comfortably run PincherOS alongside other services.

### 11.2 CPU Budget

| Operation | CPU Time (Pi 4) | Frequency |
|-----------|-----------------|-----------|
| Embed input | ~30ms | Every interaction |
| LanceDB search | ~10ms | Every interaction |
| LLM inference (100 tokens) | ~12–20s | Novel situations only |
| LLM inference (reflex confirmed, 30 tokens) | ~4–6s | Low-confidence reflexes |
| Sandbox execution | Varies | Only for shell-type reflexes |
| SQLite write | ~1ms | Every interaction |
| Snap algorithm | ~500ms | Boot + migration |

### 11.3 Disk I/O Budget

| Operation | Disk Read | Disk Write |
|-----------|-----------|------------|
| Cold start (load LLM) | ~700MB | 0 |
| Cold start (load embed) | ~90MB | 0 |
| Reflex search | ~1MB | 0 |
| Memory write | 0 | ~2KB |
| LanceDB compaction (weekly) | ~150MB | ~150MB |
| SQLite vacuum (monthly) | ~20MB | ~20MB |

---

## Appendix A: Cargo.toml (pincher-core)

```toml
[package]
name = "pincher-core"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v7"] }
sysinfo = "0.30"
axum = { version = "0.7", features = ["unix-socket"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
toml = "0.8"
sha2 = "0.10"

[target.aarch64-unknown-linux-gnu.dependencies]
# ARM-specific optimizations

[profile.release]
opt-level = 3
lto = true
strip = true
```

## Appendix B: Python requirements.txt (pincher-infer)

```
lancedb>=0.6.0,<0.7.0
llama-cpp-python>=0.2.50,<0.3.0
onnxruntime>=1.17.0,<1.18.0
sentence-transformers>=2.3.0,<3.0.0
numpy>=1.26.0,<2.0.0
pydantic>=2.5.0,<3.0.0
fastapi>=0.109.0,<0.110.0
uvicorn>=0.27.0,<0.28.0
pyarrow>=14.0.0,<16.0.0
```

## Appendix C: Minimum Viable Reflex Seed

On first install, PincherOS seeds 10 built-in reflexes so the user has something to work with immediately:

```json
[
  {
    "trigger_pattern": "create a folder|make a directory|mkdir",
    "action_template": {"type": "shell", "template": "mkdir -p {path}", "params": [{"name": "path", "type": "path", "required": true}]},
    "source": "builtin",
    "confidence": 0.80
  },
  {
    "trigger_pattern": "list files|show directory contents|ls",
    "action_template": {"type": "shell", "template": "ls -la {path}", "params": [{"name": "path", "type": "path", "default": "."}]},
    "source": "builtin",
    "confidence": 0.80
  },
  {
    "trigger_pattern": "find file|search for file|locate file",
    "action_template": {"type": "shell", "template": "find {path} -name '{pattern}' 2>/dev/null", "params": [{"name": "pattern", "type": "string", "required": true}, {"name": "path", "type": "path", "default": "."}]},
    "source": "builtin",
    "confidence": 0.80
  },
  {
    "trigger_pattern": "read file|show file contents|cat",
    "action_template": {"type": "shell", "template": "cat {path}", "params": [{"name": "path", "type": "path", "required": true}]},
    "source": "builtin",
    "confidence": 0.80
  },
  {
    "trigger_pattern": "copy file|duplicate file|cp",
    "action_template": {"type": "shell", "template": "cp {source} {destination}", "params": [{"name": "source", "type": "path", "required": true}, {"name": "destination", "type": "path", "required": true}]},
    "source": "builtin",
    "confidence": 0.80
  },
  {
    "trigger_pattern": "move file|rename file|mv",
    "action_template": {"type": "shell", "template": "mv {source} {destination}", "params": [{"name": "source", "type": "path", "required": true}, {"name": "destination", "type": "path", "required": true}]},
    "source": "builtin",
    "confidence": 0.75
  },
  {
    "trigger_pattern": "download file|fetch url|wget|curl",
    "action_template": {"type": "shell", "template": "curl -L -o {output} {url}", "params": [{"name": "url", "type": "string", "required": true}, {"name": "output", "type": "path", "required": true}]},
    "source": "builtin",
    "confidence": 0.70
  },
  {
    "trigger_pattern": "system status|how is the system|resource usage",
    "action_template": {"type": "llm", "template": "Report current system resource usage from the shell profile."},
    "source": "builtin",
    "confidence": 0.90
  },
  {
    "trigger_pattern": "help|what can you do|capabilities",
    "action_template": {"type": "llm", "template": "Describe available PincherOS capabilities based on loaded reflexes and plugins."},
    "source": "builtin",
    "confidence": 0.95
  },
  {
    "trigger_pattern": "learn from this|observe and remember|distill",
    "action_template": {"type": "llm", "template": "Enter observation mode: watch the user's next command and create a reflex from it."},
    "source": "builtin",
    "confidence": 0.90
  }
]
```

---

## Appendix D: Confidence Algorithm

The confidence score is the heart of the reflex short-circuit. Here's the exact algorithm:

```python
# Confidence update formula

def update_confidence(reflex: Reflex, success: bool) -> float:
    """
    Bayesian-inspired confidence update.
    
    - Success: confidence increases, bounded at 0.99
    - Failure: confidence decreases, bounded at 0.01
    - Rate of change slows as confidence approaches extremes (resistance to flip-flopping)
    - Usage count matters: a reflex used 100 times is more stable than one used twice
    """
    ALPHA = 0.05  # Learning rate (max change per update)
    MIN_CONF = 0.01
    MAX_CONF = 0.99
    
    # Weight the update by inverse confidence distance from boundary
    if success:
        # Distance from max: the closer to max, the smaller the increment
        distance = MAX_CONF - reflex.confidence
        increment = ALPHA * distance
        new_conf = reflex.confidence + increment
    else:
        # Distance from min: the closer to min, the smaller the decrement
        distance = reflex.confidence - MIN_CONF
        decrement = ALPHA * distance
        new_conf = reflex.confidence - decrement * 2  # Failures hurt 2x more
    
    # Stabilize high-usage reflexes (resistance to single-failure regression)
    if reflex.usage_count > 10 and not success:
        stability_factor = min(0.5, reflex.usage_count / 100.0)
        new_conf = reflex.confidence + (new_conf - reflex.confidence) * (1 - stability_factor)
    
    return max(MIN_CONF, min(MAX_CONF, new_conf))


def decay_confidence(reflex: Reflex, days_since_use: int) -> float:
    """
    Time-based decay for unused reflexes.
    Reflexes lose 1% confidence per week of non-use.
    This prevents stale reflexes from short-circuiting incorrectly.
    """
    DECAY_RATE = 0.01  # 1% per 7 days
    weeks = days_since_use / 7.0
    decay = DECAY_RATE * weeks
    return max(0.01, reflex.confidence - decay)


# Path selection based on confidence
def select_path(top_match: ReflexMatch) -> str:
    if top_match.score >= 0.90:
        return "REFLEX_DIRECT"      # Muscle memory — no LLM
    elif top_match.score >= 0.70:
        return "REFLEX_CONFIRMED"    # LLM validates, then execute
    else:
        return "LLM_REASONING"       # Full LLM reasoning
```

---

*End of PincherOS MVP Architecture Specification v0.1.0*
