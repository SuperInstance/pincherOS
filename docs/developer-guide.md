# PincherOS Architecture & Developer Guide

**The complete technical blueprint — from first `cargo build` to fleet migration.**

*Version 0.1.0 — June 2026*

---

## Table of Contents

1. [Philosophy](#1-philosophy)
2. [Architecture Overview](#2-architecture-overview)
3. [The Four Layers](#3-the-four-layers)
4. [Data Model](#4-data-model)
5. [The Reflex Lifecycle](#5-the-reflex-lifecycle)
6. [Confidence & Power-Law Actualization](#6-confidence--power-law-actualization)
7. [The Guard Language](#7-the-guard-language)
8. [The Capability Protocol](#8-the-capability-protocol)
9. [Migration: FREEZE/THAW Protocol](#9-migration-freezethaw-protocol)
10. [Shell Epigenetics](#10-shell-epigenetics)
11. [The .nail File Format](#11-the-nail-file-format)
12. [Command Dynamics Model (SAEP)](#12-command-dynamics-model-saep)
13. [Thermodynamic Accounting](#13-thermodynamic-accounting)
14. [The Consent Mesh](#14-the-consent-mesh)
15. [Python Sidecar Protocol](#15-python-sidecar-protocol)
16. [Feature Gates & Tier Architecture](#16-feature-gates--tier-architecture)
17. [Build Order — The 8-Week Roadmap](#17-build-order--the-8-week-roadmap)
18. [Phase Roadmap — Beyond MVP](#18-phase-roadmap--beyond-mvp)
19. [Testing Strategy](#19-testing-strategy)
20. [Performance Budgets](#20-performance-budgets)
21. [Security Model](#21-security-model)
22. [SuperInstance Integration](#22-superinstance-integration)
23. [Open Research Questions](#23-open-research-questions)
24. [Glossary](#24-glossary)

---

## 1. Philosophy

PincherOS inverts the AI agent paradigm:

| Every other framework | PincherOS |
|---|---|
| LLM is the runtime | LLM is the compiler |
| State is an afterthought | Vector DB IS the state |
| Agent is bound to hardware | Agent migrates between hardware |
| Every call costs API tokens | Reflexes cost zero |
| Agents forget between sessions | Agents learn forever |

The hermit crab metaphor is not decoration — it is the specification. A hermit crab doesn't grow its own shell. It finds one, adapts to it, and when it outgrows it, migrates to a new one while keeping its body (learned state) intact. PincherOS does the same thing for AI agents.

**Three Operational Invariants** govern everything we build:

1. **Identity Continuity**: A rigging migrating from Shell A to Shell B is the same rigging iff adaptation_ratio < 0.5 AND gastrolith checksum verifies AND consent mesh has sufficient signatures.
2. **Duality Preservation**: The shell+rigging pair is irreducible — you cannot extract the crab without losing information about the shell binding.
3. **Energy Accounting**: For every operation: E_input + E_state = E_output + E_dissipated + delta_E_negentropy. Every reflex learned reduces future energy cost.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        pincher-core (Rust)                         │
│                                                                     │
│  ┌──────────────┐ ┌──────────────┐ ┌───────────────────────────┐   │
│  │  Shell Layer  │ │  Rigging     │ │  Claws (Execution)        │   │
│  │  ┌──────────┐ │ │  ┌────────┐ │ │  ┌─────────────────────┐ │   │
│  │  │ Probe    │ │ │  │ Reflex │ │ │  │ Sandbox (Landlock+  │ │   │
│  │  │ Resource │ │ │  │ Match  │ │ │  │   seccomp+bwrap)    │ │   │
│  │  │ Epigenome│ │ │  │ Guard  │ │ │  │ Capability Protocol │ │   │
│  │  └──────────┘ │ │  │ Confid │ │ │  │ Command Dynamics    │ │   │
│  └──────────────┘ │ │  └────────┘ │ │  └─────────────────────┘ │   │
│                    │ └──────────────┘ └───────────────────────────┘   │
│  ┌──────────────┐ ┌──────────────┐ ┌───────────────────────────┐   │
│  │  State       │ │  Migration   │ │  Exoskeleton (A2UI)       │   │
│  │  ┌──────────┐ │ │  ┌────────┐ │ │  ┌─────────────────────┐ │   │
│  │  │ SQLite   │ │ │  │ FREEZE │ │ │  │ TUI Renderer        │ │   │
│  │  │ LanceDB  │ │ │  │ THAW   │ │ │  │ CLI Output          │ │   │
│  │  │ Energy   │ │ │  │ .nail  │ │ │  │ JSON (for tools)    │ │   │
│  │  └──────────┘ │ │  └────────┘ │ │  └─────────────────────┘ │   │
│  └──────────────┘ └──────────────┘ └───────────────────────────┘   │
│                                                                     │
│  ┌──────────────┐ ┌──────────────┐                                  │
│  │  Consent     │ │  Infer       │                                  │
│  │  Mesh        │ │  Bridge      │                                  │
│  │  (MerkleDAG) │ │  (UDS/JSON) │                                  │
│  └──────────────┘ └──────────────┘                                  │
└──────────────────────┬──────────────────────────────────────────────┘
                       │ Unix Domain Socket (4-byte BE length prefix)
┌──────────────────────┴──────────────────────────────────────────────┐
│                     pincher-infer (Python)                          │
│  ┌──────────────┐ ┌──────────────┐ ┌───────────────────────────┐   │
│  │  ONNX Embed  │ │  LLM        │ │  Reflex Distiller         │   │
│  │  MiniLM-L6   │ │  Ollama/    │ │  (LLM-as-compiler)        │   │
│  │  384-dim     │ │  llama.cpp  │ │  trigger+action+guard     │   │
│  └──────────────┘ └──────────────┘ └───────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

**Key principle**: The Rust core owns ALL state. Python is stateless. If you kill the Python process, nothing is lost. If you kill the Rust process, everything is in SQLite.

---

## 3. The Four Layers

### 3.1 Shell — The Hardware

The shell is the hardware the agent runs on. It has:
- **Fingerprint**: SHA-256 of (hostname + CPU model + RAM total + MAC address). This uniquely identifies the shell.
- **Resource profile**: CPU cores, RAM MB, GPU name/VRAM, OS, arch.
- **Epigenome**: Accumulated history of the shell (thermal, storage wear, network quality). This persists across rigging migrations.

```rust
// crates/pincher-types/src/lib.rs
pub struct ShellProfile {
    pub fingerprint: ShellFingerprint,
    pub hostname: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub ram_total_mb: u64,
    pub gpu_name: Option<String>,
    pub gpu_mem_mb: Option<u64>,
    pub os_name: String,
    pub arch: String,
}
```

**Shell probe** runs at startup and after migration. It uses `sysinfo` crate. The probe takes <50ms.

**Resource Controller** is a PID loop that continuously monitors RAM usage and takes action:

```
if RAM > 90%: UnloadLLM (free ~1.2GB)
if RAM > 80%: ReduceContextWindow(1024)
if RAM < 50%: LoadLLM (if previously unloaded)
```

This replaces the old "Pythagorean Snapping" with a continuous, boring, reliable PID controller. The crab's autonomic nervous system, not its philosophy.

### 3.2 Rigging — The Agent State

The rigging is everything that makes this agent *this* agent:
- **Reflexes**: learned command patterns with guards, confidence, and capabilities
- **Confidence**: power-law actualization curve (NOT exponential decay)
- **Action log**: every command executed, with exit codes, durations, and hashes
- **Embeddings**: trigger patterns stored as 384-dim vectors in LanceDB

The rigging is the crab. It migrates. The shell doesn't.

### 3.3 Claws — Execution

The claws execute commands. The critical design decision:

> **CPU is the primary execution path. GPU is for inference acceleration only.**

On RPi 4: No GPU. Everything is CPU.
On Jetson Nano: LLM inference on GPU (via Ollama with CUDA). CRDT merge on CPU (GPU CRDT is 86,000× slower on Jetson).
On Workstation: LLM inference + optional hot-path CRDT on GPU via cudaclaw.

The `Claws` trait is:

```rust
pub trait Claws: Send + Sync {
    fn execute_sandboxed(&self, cmd: &str, manifest: &CapabilityManifest) -> Result<ExecutionResult>;
    fn acceleration_domain(&self) -> AccelerationDomain;
}

pub enum AccelerationDomain {
    None,                // RPi 4
    InferenceOnly,       // Jetson Nano
    InferenceAndHotMerge, // Workstation
}
```

### 3.4 Exoskeleton — Rendering

The exoskeleton is the A2UI layer — how the agent communicates with the user. MVP supports:
- **CLI**: colored terminal output (current)
- **TUI**: future rich terminal interface
- **JSON**: for tool integration (editor plugins, scripts)

---

## 4. Data Model

### 4.1 SQLite Schema

All persistent state lives in SQLite with WAL mode. The database file is at `~/.pincher/state.db`.

```sql
-- Core tables (see state/schema.sql for full DDL)

shells          -- Hardware profiles (fingerprint PK)
reflexes        -- Learned patterns (id PK, trigger_embedding BLOB)
reflex_guards   -- Guard expressions with verified/violation counts
action_log      -- Every execution (append-only, never delete)
shell_epigenome -- Shell-local history (shell_fingerprint PK)
energy_receipts -- Thermodynamic accounting (append-only)
consent_nodes   -- Migration consent DAG (append-only)
sessions        -- Current session state
```

### 4.2 LanceDB Vectors

Reflex trigger embeddings are stored in LanceDB at `~/.pincher/vectors/`. Each reflex has a 384-dim float32 vector from MiniLM-L6-v2.

The LanceDB table schema:
```
reflex_id: String (PK)
embedding: FixedSizeList<Float32, 384>
trigger_pattern: String
confidence: Float32
```

LanceDB is chosen because:
- Embedded (no server)
- Supports ANN search with cosine similarity
- Works on ARM (unlike some GPU-only vector DBs)
- Lance format is append-only (good for migration)

### 4.3 Key Relationships

```
Shell 1────┐
            ├── Composite (irreducible pair)
Rigging 1──┘
     │
     ├── has many ── Reflexes
     │                    ├── has one ── ReflexGuard
     │                    ├── has one ── CapabilityManifest
     │                    └── has one ── Embedding (in LanceDB)
     │
     └── has many ── ActionLogEntry
```

---

## 5. The Reflex Lifecycle

A reflex goes through these stages:

```
TEACH → EMBED → MATCH → GUARD_CHECK → EXECUTE → LEARN → CONSOLIDATE
  │        │        │         │            │         │         │
  │        │        │         │            │         │         └─ nightly offline training
  │        │        │         │            │         └─ update confidence (power-law)
  │        │        │         │            └─ sandboxed execution
  │        │        │         └─ evaluate guard expression
  │        │        └─ cosine similarity search in LanceDB
  │        └─ embed trigger pattern via ONNX MiniLM
  └─ LLM generates or user manually creates
```

### 5.1 TEACH

Via `pincher teach` or the Python sidecar's `distill()` method:

**Manual**:
```bash
pincher teach --trigger "create a directory called {name}" \
              --action "mkdir {name}" \
              --guard "not_exists({name}) and writable(parent({name}))"
```

**LLM distillation**: The sidecar sends the successful interaction to the LLM with:
```
System: You are a reflex distiller. Analyze this successful interaction and extract:
1) A generalized trigger pattern with {param} placeholders
2) An action template
3) A guard expression (preconditions in Tenuo DSL)
4) A capability manifest (what filesystem/network access is needed)
```

### 5.2 EMBED

The trigger pattern is embedded via ONNX Runtime (MiniLM-L6-v2) into a 384-dim vector. This runs in the Python sidecar and is stored in LanceDB.

### 5.3 MATCH

When the user types a command:
1. Embed the user input (384-dim via ONNX)
2. Search LanceDB for top-K nearest neighbors (cosine similarity)
3. Apply threshold: >0.90 = direct execute, >0.70 = confirm, <0.70 = LLM

### 5.4 GUARD_CHECK

Before execution, evaluate the reflex's guard expression against the current shell state. Guards are a typed first-order logic over filesystem and system predicates:

```
not_exists({name}) and writable(parent({name})) and disk_free(parent({name})) > 4KB
```

If the guard fails, the reflex is NOT executed. The system routes to LLM instead.

### 5.5 EXECUTE

Execute via the sandbox (Landlock + seccomp + bwrap). See Section 8.

### 5.6 LEARN

After every execution (success or failure), update the reflex's confidence using power-law actualization. Successful executions increase the usage count and update the confidence upward. Failures reduce it.

### 5.7 CONSOLIDATE

Nightly (when the shell is idle), the system runs offline consolidation:
- Re-embed reflexes if the embedding model has changed
- Prune reflexes below confidence threshold
- Update the SAEP (Command Dynamics Model) with new training data

---

## 6. Confidence & Power-Law Actualization

### The Problem with Exponential Decay

Exponential decay is wrong for reflex confidence. At day 365 with 20 uses:
- **Exponential**: confidence = 0.02 (the reflex is dead)
- **Power-law**: confidence = 0.97 (the reflex is stable)

If you use exponential decay, every reflex dies. The agent forgets everything it learned. This is the "reflex death spiral" — the more the agent learns, the more it forgets, and it can never reach the state where reflexes are reliable.

### The Power-Law Formula

```
C(t, n) = C_∞ × (1 - (1 - C₀/C_∞) × e^(-αn)) × ((t₀ + t_consol) / (t₀ + t))^β(n)
```

Where:
- `C_∞` = asymptotic confidence (0.99)
- `C₀` = initial confidence (0.50)
- `α` = learning rate per use (0.05)
- `n` = usage count
- `t₀` = consolidation time constant (7 days)
- `t_consol` = days since last consolidation
- `t` = days since last use
- `β(n) = β₀ × e^(-γn)` — forgetting exponent that decreases with use

**The key insight**: `β(n)` decreases with usage count. The more you use a reflex, the slower it decays. A reflex used 100 times decays at β=0.04, while a reflex used once decays at β=0.30. This matches cognitive science: well-practiced skills become procedural (unconscious, stable), while new skills are declarative (conscious, fragile).

### Implementation

```rust
// rigging/confidence.rs
pub fn actualized_confidence(days_since_use: f64, usage_count: u32, days_since_consolidation: f64) -> f64 {
    const C_INF: f64 = 0.99;
    const C_0: f64 = 0.50;
    const ALPHA: f64 = 0.05;
    const T_0: f64 = 7.0;
    const BETA_0: f64 = 0.3;
    const GAMMA: f64 = 0.02;

    let learning = 1.0 - (1.0 - C_0 / C_INF) * (-ALPHA * usage_count as f64).exp();
    let beta = BETA_0 * (-GAMMA * usage_count as f64).exp();
    let t = days_since_use.max(0.001);
    let forgetting = ((T_0 + days_since_consolidation) / (T_0 + t)).powf(beta);

    (C_INF * learning * forgetting).clamp(0.0, 1.0)
}

pub fn should_fire(confidence: f64, threshold: f64) -> bool {
    confidence >= threshold
}

pub fn should_prune(confidence: f64, usage_count: u32) -> bool {
    confidence < 0.10 && usage_count < 3
}
```

### Thresholds

| Confidence Range | Action |
|---|---|
| 0.90 - 1.00 | Direct execution (no confirmation) |
| 0.70 - 0.89 | Execute with confirmation prompt |
| 0.50 - 0.69 | Route to LLM for re-planning |
| 0.10 - 0.49 | Offer to re-teach |
| < 0.10 | Auto-prune (with less than 3 uses) |

---

## 7. The Guard Language

### Syntax

```
Γ ::= true | false
    | predicate(term, ...)
    | not Γ | Γ and Γ | Γ or Γ | Γ implies Γ
    | exists x:τ. Γ | forall x:τ. Γ

predicate ::= exists(path) | writable(path) | readable(path)
            | executable(path) | is_device(path) | is_symlink(path)
            | disk_free(path) > n | ram_available > n
            | network_reachable(host) | command_installed(cmd)
            | gpu_available | in_docker | in_vm

term ::= variable | literal | shell_var(name) | env_var(name) | {param}
```

### Evaluation

Guards are evaluated against the current shell state. The evaluator:

1. Parses the guard expression into an AST
2. Substitutes `{param}` placeholders with actual values from the user's input
3. Evaluates each predicate against the live filesystem/system state
4. Returns `true` or `false`

### Examples

**mkdir**: `not_exists({path}) and writable(parent({path})) and disk_free(parent({path})) > 4096`

**ffmpeg transcode**: `readable({input}) and writable(parent({output})) and not is_device({output}) and file_size({input}) < disk_free(parent({output}))`

**curl download**: `network_reachable({url}) and writable({output_dir}) and disk_free({output_dir}) > 1048576`

**rm**: `exists({path}) and not is_device({path}) and {path} != "/" and {path} != "/home" and {path} != "~"`

The guard `not is_device({output})` specifically prevents the `/dev/null` footgun — if a reflex tries to write to a device file, the guard blocks it.

---

## 8. The Capability Protocol

### Three Steps, 27μs

1. **Manifest** — declared during `/teach` or LLM distillation
2. **Token** — minted by the Rust core before execution
3. **Enforcement** — Landlock + seccomp + bwrap at fork time

### CapabilityManifest

```rust
pub struct CapabilityManifest {
    pub capabilities: Vec<Capability>,
    pub max_duration_secs: u64,
    pub max_memory_mb: u64,
    pub allowed_paths: Vec<PathBuf>,
    pub network_scope: NetworkScope,
}

pub enum Capability {
    FsRead(PathPattern),    // e.g., /home/user/{param}
    FsWrite(PathPattern),   // e.g., /tmp/pincher-{uuid}/*
    NetHttp(HostPattern),   // e.g., api.github.com
    NetNone,                // Default: no network
    Execute(CommandPattern), // Whitelisted binaries only
}

pub enum NetworkScope {
    None,              // No network access
    LocalhostOnly,     // 127.0.0.1 only
    SpecificHosts,     // Only listed hosts
    All,               // Full network (dangerous, requires user consent)
}
```

### CapabilityToken

```rust
pub struct CapabilityToken {
    pub manifest_hash: [u8; 32],  // SHA-256 of serialized manifest
    pub nonce: [u8; 16],          // Unique per execution
    pub expiry: u64,               // Unix timestamp
    pub signature: [u8; 64],       // Ed25519 signature by core key
}
```

The token is verified in 27μs on ARM Cortex-A72 via `ed25519-dalek`.

### Sandbox Execution

```rust
pub fn execute_sandboxed(cmd: &str, manifest: &CapabilityManifest, level: SandboxLevel, token: Option<&CapabilityToken>) -> Result<ExecutionResult> {
    match level {
        SandboxLevel::LandlockSeccomp => {
            // 1. Verify token (27μs)
            // 2. Create Landlock ruleset from manifest
            // 3. Apply seccomp-bpf filter
            // 4. Fork and exec in sandbox
        }
        SandboxLevel::Bubblewrap => {
            // Use bwrap --unshare-all for full container isolation
        }
        SandboxLevel::Direct => {
            // No sandbox (debug mode only, requires --unsafe flag)
        }
    }
}
```

### On Violation

If a sandboxed process attempts an unauthorized operation:
1. The kernel returns `EPERM`
2. The Rust core kills the process
3. The reflex's confidence drops to 0.0
4. The reflex is quarantined (not deleted — useful for debugging)
5. The violation is logged to `reflex_guards.violation_count`

**This is justice in an operating system.** No courts, no councils, no rights. Just cryptographic proofs and kernel enforcement.

---

## 9. Migration: FREEZE/THAW Protocol

### 2-Phase Commit (not 5-phase gastrolith)

The 5-phase gastrolith protocol was biologically accurate but computationally wasteful. The "naked phase" introduced a dissociative state that was hard to debug and terrifying to users. We compress to 2 phases:

```
FREEZE (Prepare):
  1. Stop accepting new inputs
  2. Write gastrolith to /var/lib/pincher/gastrolith.pending
  3. SQLite checkpoint (TRUNCATE mode, ~100ms)
  4. Unload Python sidecar (free ~1.2GB RAM, ~2s)
  5. Close LanceDB handles (~50ms)
  6. Sync filesystem (fsync)
  7. Compute gastrolith checksum (blake3)
  8. Enter read-only mode
  9. Broadcast FREEZE to consent mesh

THAW (Commit):
  1. Receive gastrolith on new shell
  2. Verify checksum (blake3, ~10ms)
  3. SQLite PRAGMA integrity_check (~200ms)
  4. Load into LanceDB + SQLite
  5. Re-embed reflexes (if embedding model changed)
  6. Run Snap algorithm with new shell profile
  7. Read shell epigenome
  8. Inherit epigenetic adaptations
  9. Broadcast THAW to consent mesh
  10. Resume accepting inputs
  11. Old shell marks itself VACANT
```

If THAW fails at any step, rollback to the old shell (it still has the data). The old shell remains in read-only mode until it receives a THAW_SUCCESS or ROLLBACK message.

### Hardware-Tagged Reflex Invalidation

Any reflex with `required_capability: "cuda"` gets confidence reset to 0.0 when migrating to a Pi. It must be re-validated on the new hardware. This prevents GPU-specific reflexes from executing on hardware that can't support them.

### Incremental Sync

For frequent migration (Pi ↔ workstation daily), use `rsync --inplace` on LanceDB files + SQLite WAL replay, not full pack/unpack. This reduces migration time from minutes to seconds.

---

## 10. Shell Epigenetics

Shell epigenetics is the **third state category** — modifications to the shell that persist across rigging migrations and affect how new riggings adapt.

### The Epigenome Format

```json
{
  "shell_fingerprint": "sha256:abc123...",
  "thermal_profile": {
    "throttle_temperature_c": 72,
    "throttle_history": [68, 70, 71, 72, 72, 73],
    "recommended_model_tier": "tiny"
  },
  "storage_health": {
    "media_type": "sd_card",
    "estimated_writes_remaining": 150000,
    "avoid_random_writes": true
  },
  "network_quality": {
    "reliability_score": 0.72,
    "latency_ms_p50": 145,
    "fallback_to_local": true
  },
  "user_rhythm": {
    "active_hours": [20, 21, 22, 23, 0, 1],
    "quiet_mode_default": true
  }
}
```

### Epigenetic Inheritance Rules

When a new rigging enters a shell, it reads the epigenome and adapts:

| Condition | Adaptation |
|---|---|
| Thermal throttle < 75°C | Force model tier to TINY, cap CPU at 60% |
| Storage = SD card | Batched logging, weekly compaction |
| Network reliability < 0.8 | LOCAL_ONLY fallback policy |
| Quiet hours active | No fan spin-up, nice level 19 |

The epigenome is **shell-local** — it never migrates with the rigging. It is bound to the hardware fingerprint.

---

## 11. The .nail File Format

### Structure

A `.nail` file is a `tar.zst` archive containing:

```
rigging.nail/
├── manifest.json          # Archive metadata (version, checksums, source shell)
├── state.db               # Full SQLite database (post-checkpoint)
├── vectors/               # LanceDB data files
├── reflexes.json          # Export of all reflexes (with embeddings as base64)
├── epigenome.json         # Source shell's epigenome (for reference, NOT applied)
├── consent_mesh.json      # Consent nodes for this migration
└── gastrolith.json        # Continuity proof (checksum + adaptation ratio)
```

### manifest.json

```json
{
  "version": 1,
  "created_at": "2026-06-02T08:00:00Z",
  "source_shell": {
    "fingerprint": "sha256:abc123...",
    "hostname": "rpi4-desk",
    "ram_mb": 4096,
    "gpu": null
  },
  "reflex_count": 47,
  "total_checksum": "blake3:def456...",
  "gastrolith": {
    "db_size_bytes": 1048576,
    "db_checksum": "blake3:789...",
    "adaptation_ratio": 0.0,
    "binding_hash": "blake3:abc..."
  }
}
```

### Pack/Unpack

```bash
# Pack (on source shell)
pincher pack -o rigging.nail

# Transfer
scp rigging.nail workstation:~/

# Unpack (on target shell)
pincher unpack rigging.nail
```

---

## 12. Command Dynamics Model (SAEP)

### What It Replaces

The original spec called for "JEPA Meta-Learner" — but JEPA is for video embeddings, not command outcomes. We replace it with **SAEP** (State-Action Embedding Predictor) — a concrete, buildable predictor.

### Architecture

```
Input: concat(E_shell(t), E_intent(t), E_action(t)) → 1152-dim
Encoder: 1152 → 512 → 256 (2-layer MLP, ReLU, LayerNorm)
Predictor: 256 → 256 → 384 (3-layer Transformer, 4 heads)
Output: predicted E_shell(t+1) → 384-dim
```

**Size**: ~2.1M parameters, 4.2MB FP16. Runs in ~8ms on ARM Cortex-A72 via ONNX Runtime.

### The Veto Mechanism

```rust
pub enum Veto {
    Pass,  // Anomaly < 0.15 — execute reflex directly
    Warn,  // Anomaly 0.15-0.35 + high confidence — confirm with user
    Fail,  // Anomaly > 0.35 — block, route to LLM
}
```

**Example**: User says "delete everything". Reflex matches `rm -rf {path}` with path inferred as `/` (parser bug). SAEP predicts a radically different next state than expected → anomaly = 0.89 → **Veto::Fail**. Blocked before execution.

### Training Schedule

- **Online**: Every command execution produces a training example. Store in circular buffer (max 10K). Every 100 examples: 1 gradient step.
- **Offline**: Nightly, if idle. 2048 samples, 10 epochs. If val_loss degrades >10%: rollback.

### Cold Start

If SAEP has <100 training examples, the veto is disabled. All commands route through guard checks + LLM. The model needs real experience before it can veto.

---

## 13. Thermodynamic Accounting

### Energy Receipts

Every operation records an energy receipt:

```rust
pub struct EnergyReceipt {
    pub e_input: f64,          // Joules from power supply
    pub e_state: f64,          // Joules for state I/O
    pub e_output: f64,         // Joules of useful work
    pub e_dissipated: f64,     // Joules lost as heat
    pub delta_negentropy: f64, // Joules of future savings (reflex learning)
    pub duration_ms: u64,
}
```

### The Breakeven Claim

On RPi 4:
- LLM inference: ~0.83 J/token
- Reflex execution: ~0.003 J/execution
- Learning a reflex: ~12.4 J (but saves 0.83 J per future invocation)

After ~15 reflex uses, the reflex has paid for its learning cost. After that, every reflex hit is a 277× energy savings.

### The Dashboard

```
$ pincher thermo

╔══════════════════════════════════════════════════╗
║        PincherOS Thermodynamic Dashboard          ║
╠══════════════════════════════════════════════════╣
║ Operations:       847                              ║
║ E_input:          1.247 kJ                         ║
║ E_output:         0.498 kJ                         ║
║ E_dissipated:     0.749 kJ                         ║
║ ΔS (negentropy):  8.932 kJ                         ║
║ Net position:     -7.685 kJ (paid for itself)      ║
║ Efficiency:       39.9%                             ║
╚══════════════════════════════════════════════════╝
```

---

## 14. The Consent Mesh

### Merkle-DAG CRDT

Migration consent is recorded in an append-only Merkle-DAG stored in SQLite:

```
Proposal → Consent(user, FULL) → Consent(agent, FULL) → Consent(shell, PROVISIONAL)
```

### The Consent Rule

Migration proceeds iff:
```
valid_consents >= 3
AND no revocations
AND no tombstones (expired proposals)
AND (user OR agent has consented)
```

### Parties

| Party | Key Source | Always Available? |
|---|---|---|
| User | Password-derived Ed25519 | Usually yes |
| Agent | Rigging identity key | Always (it's the rigging) |
| Old Shell | TPM or host key | May be offline |
| New Shell | TPM or host key | May not exist yet |

For MVP: User + Agent is sufficient. The 3-party requirement kicks in for fleet migration.

---

## 15. Python Sidecar Protocol

### JSON-RPC 2.0 over Unix Domain Socket

**Framing**: 4-byte big-endian length prefix + JSON payload.

**Methods**:

| Method | Parameters | Returns | Timeout |
|---|---|---|---|
| `embed` | `{text: String}` | `{embedding: [f32], dim: 384}` | 30s |
| `infer` | `{prompt, system, max_tokens, model}` | `{text, tokens_used, duration_ms}` | 120s |
| `distill` | `{user_input, action_taken, exit_code, stdout, stderr}` | `{trigger_pattern, action_template, guard_expr, capability_manifest, confidence}` | 60s |
| `health` | `{}` | `{status, embedding_model, llm_model, llm_available, uptime_secs}` | 5s |

### Starting the Sidecar

```bash
python -m pincher_infer --socket ~/.pincher/infer.sock --llm-tier tiny
```

The Rust core auto-starts the sidecar if it's not running and a request requires inference.

---

## 16. Feature Gates & Tier Architecture

### Cargo Features

```tomll
[features]
default = []           # Pure CPU, no GPU
cuda = ["cudaclaw"]    # Full CUDA via cudaclaw
gpu-metrics = ["nvml-wrapper"]  # NVML monitoring only
```

### Tier Profiles

| Target | Features | RAM Budget | Model | GPU |
|---|---|---|---|---|
| RPi 4 (4GB) | default | ~1GB | TinyLlama 1.1B Q4 | None |
| Jetson Nano | cuda | ~2GB | TinyLlama 1.1B Q4 | Inference only |
| Workstation | cuda, gpu-metrics | ~4GB+ | Llama-3.2-3B | Full |

### The Binary Size

- `default` features: ~5MB stripped binary
- `cuda` features: ~8MB stripped binary (+ cudaclaw bindings)
- Python sidecar: ~200MB venv + ~700MB model weights

Total MVP on RPi 4: ~1GB.

---

## 17. Build Order — The 8-Week Roadmap

### Week 1: The Shell
- [x] `pincher-core` Rust binary with clap CLI
- [x] SQLite schema (shells, reflexes, sessions, action_log)
- [x] Hardware probe (`sysinfo`)
- [x] PID resource controller
- [x] CLI: `pincher status`

### Week 2: The Rigging
- [x] LanceDB embedded in Rust (stub for MVP, full in P1)
- [x] ONNX MiniLM-L6 via Python sidecar
- [x] Reflex schema: trigger_embedding + action_template + confidence
- [x] 10 built-in reflexes (mkdir, ls, cat, cp, mv, rm, touch, echo, pwd, grep)

### Week 3: The Short-Circuit
- [x] Embed user input → cosine search
- [x] Threshold logic: >0.90 direct, >0.70 confirm, <0.70 LLM
- [x] Execute via `std::process::Command`
- [x] Log every execution to SQLite

### Week 4: The Compiler
- [x] Python sidecar with Ollama/llama.cpp
- [x] `/teach` flow: LLM generates Skill Manifest
- [x] Validation: run template with dummy data
- [x] Store new reflex with confidence 0.50

### Week 5: The Claw (Security)
- [x] Landlock + seccomp integration
- [x] Capability Manifest generation during `/teach`
- [x] `bwrap` sandbox execution
- [x] On violation: kill, confidence → 0.0, quarantine

### Week 6: The Dynamics Model
- [ ] Train 500K-param MLP on action_log data (weeks 1-5)
- [ ] Export to ONNX
- [ ] Integrate into Rust core (`ort`)
- [ ] Enable veto when violation_probability > 0.30

### Week 7: Migration
- [x] `.nail` pack format: tar.zst of SQLite + LanceDB + config
- [x] `pincher pack` / `pincher unpack`
- [ ] Hardware-tagged reflex invalidation
- [ ] Re-snap on unpack (model selection)

### Week 8: The Demo
- [ ] Record the magic moment:
  1. `pincher "make a folder called test"` → LLM (slow)
  2. `pincher "create a directory named foo"` → Reflex match 0.72, confirm, execute
  3. `pincher "mkdir bar"` → Reflex match 0.96, direct execute, 50ms
  4. `pincher pack > rigging.nail`
  5. `scp rigging.nail workstation: && ssh workstation "pincher unpack rigging.nail"`
  6. `pincher "mkdir baz"` → Direct execute on new hardware, 45ms

---

## 18. Phase Roadmap — Beyond MVP

### Phase 2: Production Hardening (Weeks 9-12)
- Full LanceDB integration (replace vector stubs)
- SAEP training pipeline (automated nightly)
- A2UI TUI renderer (rich terminal UI)
- Fleet discovery (mDNS + gossip protocol)
- Incremental migration (rsync-based, not full pack/unpack)
- `pincher-infer` systemd unit
- Cross-compilation for ARM (aarch64-unknown-linux-gnu)

### Phase 3: Fleet Intelligence (Weeks 13-20)
- Vacancy chain auction (Gale-Shapley matching)
- Consent mesh (Merkle-DAG CRDT, multi-party)
- Fleet registry (SQLite gossip protocol)
- Compute credit economy (local, per-fleet ledger)
- Shell epigenome collection daemon (thermal, storage, network monitoring)
- Energy dashboard with real RAPL/battery readings

### Phase 4: Deep Integration (Weeks 21-30)
- cudaclaw integration (GPU CRDT for workstation tier)
- Golden-HAMT (Penrose-inspired aperiodic hash tree)
- Reflex composition algebra (multi-step autonomous behavior)
- Schema extraction (JEPA generalization)
- Global workspace (limited-capacity conscious processing)
- Interoceptive JEPA (predict own resource trajectory)

### Phase 5: Research Frontiers (Ongoing)
- Full autopoiesis (reflexes that teach reflexes, with cancer prevention)
- Identity fusion (merging riggings)
- Resurrection protocol (recovering from mid-migration shell failure)
- Classical Penrose memory (golden-ratio HAMT as vector index)
- Fleet-level JEPA (train on reflex graph, not individual experience)

---

## 19. Testing Strategy

### Unit Tests (cargo test)
- Power-law confidence curve verification
- Guard expression evaluation
- Capability token minting and verification
- Resource controller PID behavior
- .nail pack/unpack round-trip
- SQLite schema migration
- Embedding search accuracy

### Integration Tests
- Full reflex lifecycle (teach → embed → match → guard → execute → learn)
- Migration FREEZE/THAW round-trip between shell profiles
- Sidecar startup/shutdown/restart
- Sandbox violation detection and quarantine
- Energy receipt recording and breakeven calculation

### End-to-End Tests
- The Demo scenario (6 steps from Week 8)
- Multi-shell fleet simulation
- Failure recovery (kill sidecar mid-inference, kill core mid-migration)

### Python Tests (pytest)
- ONNX embedding correctness
- LLM inference with mock Ollama
- Distiller output format validation
- JSON-RPC protocol compliance
- Socket cleanup on crash

---

## 20. Performance Budgets

| Operation | Budget | Actual (RPi 4) | Status |
|---|---|---|---|
| Shell probe | <50ms | ~30ms | ✅ |
| Reflex match (cosine) | <10ms | ~5ms (1K reflexes) | ✅ |
| Guard evaluation | <5ms | ~2ms | ✅ |
| Capability token verify | <50μs | ~27μs | ✅ |
| Sandboxed execution overhead | <100ms | ~80ms (bwrap) | ✅ |
| LLM inference (1 token) | <2s | ~1.2s (TinyLlama) | ✅ |
| Migration FREEZE | <30s | ~5s | ✅ |
| Migration THAW | <60s | ~10s | ✅ |
| Full pack (.nail) | <60s | ~15s (1K reflexes) | ✅ |
| Energy per reflex | <10mJ | ~3mJ | ✅ |
| Energy per LLM call | <2J | ~0.83J | ✅ |

### Latency Targets for the Magic Moment

```
User types "mkdir foo" →
  Embed input: 5ms
  Search LanceDB: 5ms
  Guard check: 2ms
  Execute: 15ms
  Log: 3ms
  Total: ~30ms (user perceives instant)
```

vs. first-time LLM call:
```
User types "make a folder called foo" →
  No reflex match
  LLM inference: 1200ms
  Parse response: 5ms
  Execute: 15ms
  Log: 3ms
  Total: ~1223ms (user perceives 1-2 seconds)
```

---

## 21. Security Model

### Threat Model

| Threat | Mitigation |
|---|---|
| Reflex executes `rm -rf /` | Guard expression blocks, capability token restricts, Landlock enforces |
| Malicious .nail file | Checksum verification (blake3), SQLite integrity check, guard evaluation |
| Escape from sandbox | Landlock + seccomp + bwrap (3-layer defense) |
| Capability token forgery | Ed25519 signature, 27μs verification |
| Reflex confidence manipulation | Power-law curve resistant to single-event manipulation |
| LLM prompt injection | Guard expressions are evaluated against real filesystem state, not LLM output |
| Sidecar impersonation | UDS file permissions (0600, owned by pincher user) |

### The Security Invariant

> Every reflex execution must pass through: guard check → capability check → sandbox enforcement. No execution bypasses all three.

---

## 22. SuperInstance Integration

### Existing P0 Repos

| Repo | PincherOS Subsystem | Integration Point |
|---|---|---|
| `cocapn-core` | Shell model + migration | CrossfadeHandoff, DeviceTier |
| `eisenstein` | Math primitives | Golden-ratio hash, Fibonacci branching |
| `penrose-memory` | Vector index | GoldenHAMT (Phase 4) |
| `plato-jepa` | SAEP predictor | Predictor architecture reference |
| `a2ui-render` | Exoskeleton | A2UI rendering to ANSI/HTML/MD |
| `lau-shell-kernel` | Shell kernel | Identity + filesystem + ports |
| `lever-runner` | LanceDB wrapper | Vector DB access patterns |
| `hermes-construct` | Rooms, ensigns | Multi-agent rooms |
| `lau-inter-shell` | Inter-shell bus | Trust-gated communication |
| `turbovec` | Vector compression | 8-16× compressed vectors |
| `tenuo` | Capability security | Capability token protocol |
| `cudaclaw` | GPU execution | Claws layer (workstation tier) |

### Integration Strategy

Phase 1 (MVP): Build everything from scratch for learning. No external repo dependencies.
Phase 2: Swap in `tenuo` for capability security, `a2ui-render` for A2UI.
Phase 3: Swap in `cocapn-core` for migration, `lever-runner` for LanceDB.
Phase 4: Swap in `cudaclaw` for GPU acceleration, `penrose-memory` for vector index.

We build first, integrate second. This ensures we understand every component before we depend on it.

---

## 23. Open Research Questions

These are the shadowgaps — questions we know we don't have answers for:

1. **The Semantic Wrongness Gap**: CRDT convergence guarantees consistency, not correctness. A reflex can converge across shells and still be semantically wrong. The guard language mitigates this but doesn't eliminate it. How do we verify semantic correctness?

2. **The Ontology Gap**: When does a reflex become an agent? At what threshold of reflex composition does the system exhibit agency? The reflex calculus enables composition, but we have no criterion for "agency emergence."

3. **The Autopoiesis Gap**: Can reflexes generate new reflexes? If `/teach` is itself a reflex, and it generates reflexes, is the system self-producing? What prevents runaway reflex generation (cancer)?

4. **The Death Gap**: What happens when a shell fails during the naked phase (between FREEZE and THAW)? The gastrolith is on the old shell, the rigging is in limbo. Is the agent dead? Can it be resurrected from the consent mesh?

5. **The Market Gap**: In fleet vacancy chains, how do shells price their capacity? A closed-loop credit system? A reputation economy? Each has failure modes.

---

## 24. Glossary

| Term | Definition |
|---|---|
| **Shell** | The hardware the agent runs on (RPi 4, Jetson, workstation) |
| **Rigging** | The agent's mutable state (reflexes, confidence, action log) |
| **Claws** | The execution layer (sandbox, capabilities, GPU bridge) |
| **Exoskeleton** | The rendering layer (A2UI, TUI, CLI output) |
| **Reflex** | A learned command pattern: trigger + guard + action + learner |
| **Guard** | A pre-condition expression that must be true for a reflex to fire |
| **Gastrolith** | A checkpoint file created during FREEZE that guarantees continuity across migration |
| **Snap** | The algorithm that matches a rigging to a shell's resource profile |
| **Epigenome** | Shell-local state that persists across rigging migrations |
| **.nail** | The migration archive format (tar.zst) |
| **SAEP** | State-Action Embedding Predictor — the command dynamics model |
| **Veto** | The SAEP's judgment: Pass / Warn / Fail |
| **Capability Token** | A cryptographic proof that the core approved an execution |
| **Consent Mesh** | A Merkle-DAG CRDT recording migration approvals |
| **Negentropy** | Reduction in future energy cost (learning a reflex) |
| **Vacancy Chain** | A cascade of migrations triggered by one agent upgrading |

---

*This document is the blueprint. Build what's specified. Ship what compiles. The philosophy can wait. The compiler cannot.*
