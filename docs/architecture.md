# PincherOS Architecture

> "Docker for AI agents" — a migratable, self-learning agent state that treats the LLM as a compiler and the vector DB as the runtime.

## Overview

PincherOS is an agent runtime that learns from every interaction. After the third time you ask it to organize your downloads, it doesn't need the cloud. It just does it. In 50 milliseconds. For free.

The agent lives inside a **shell** (hardware). When the Pi is too small, the agent packs its brain into a `.nail` file, migrates to a bigger shell, and keeps working.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    pincher-core (Rust)                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Resource   │  │  Capability │  │   Command         │  │
│  │  Controller │  │  Protocol   │  │   Dynamics Model  │  │
│  │  (PID loop) │  │  (Tenuo+LL) │  │   (500K MLP, ONNX)│  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  SQLite WAL │  │  LanceDB    │  │  Migration QTR      │  │
│  │  (state)    │  │  (vectors)  │  │  (quiesce-transfer) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Reflex     │  │  A2UI       │  │  UDS Server         │  │
│  │  Matcher    │  │  Renderer   │  │  (JSON-RPC)         │  │
│  │  (cosine)   │  │  (TUI/CLI)  │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │ Unix Domain Socket
┌──────────────────────────┼──────────────────────────────────┐
│                    pincher-infer (Python)                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Ollama/    │  │  ONNX Embed │  │  Reflex Distiller   │  │
│  │  llama.cpp  │  │  (MiniLM)   │  │  (LLM-as-compiler)  │  │
│  │  (LLM)      │  │             │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Reflex (Hoare Triple)

A Reflex is the fundamental learning unit — a Hoare triple compiled from repeated LLM invocations:

- **Σ (Sigma)**: Trigger pattern + embedding — "when does this fire?"
- **Γ (Gamma)**: Guard expression (Tenuo DSL) — "is it safe to fire?"
- **Δ (Delta)**: Action template — "what gets executed?"
- **Λ (Lambda)**: Post-condition — "did it work?"

### Power-Law Confidence

**This is NON-NEGOTIABLE.** We use power-law decay, not exponential.

```
C(t, n) = C_inf * (1 - (1 - C_0/C_inf) * exp(-α*n)) * ((t_0 + t_consol) / (t_0 + t))^β(n)
```

At day 365 with 20 uses:
- **Exponential**: 0.02 (effectively dead)
- **Power-law**: 0.97 (stable, usable)

### Capability Tokens

Every reflex that accesses resources must present a signed capability manifest. This is a simple, auditable capability system — not a constitutional republic.

### 2-Phase Migration (QTR)

**FREEZE**: Stop inputs → SQLite checkpoint → Unload sidecar → Close LanceDB → Snapshot → Pack .nail

**THAW**: Verify checksum → Unpack → SQLite integrity check → LanceDB validation → Snap → Read epigenome → Resume

### PID Resource Controller

A PID control loop manages resource allocation. If the shell has 512MB RAM and the agent wants 1GB, the controller throttles.

### SAEP / Command Dynamics Model

The Command Dynamics Model predicts outcomes before execution:
1. Encode the pre-action state
2. Encode the proposed action
3. Predict energy cost and likely outcome
4. Decide: reflex or LLM?

## Crate Structure

```
pincherOS/
├── Cargo.toml (workspace)
├── crates/
│   ├── pincher-core/        # Main binary crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli.rs       # Clap CLI
│   │       ├── shell/       # Hardware probing, PID, epigenetics
│   │       ├── rigging/     # Reflexes, confidence, guards, matching
│   │       ├── state/       # SQLite WAL, LanceDB
│   │       ├── security/    # Capabilities, sandboxing
│   │       ├── migration/   # QTR FREEZE/THAW
│   │       ├── predictor/   # SAEP Command Dynamics
│   │       └── infer/       # UDS JSON-RPC bridge
│   └── pincher-types/       # Shared types
└── pincher-infer/           # Python sidecar
    └── pincher_infer/
        ├── server.py        # JSON-RPC server
        ├── embedder.py      # ONNX MiniLM-L6
        ├── distiller.py     # LLM reflex distillation
        └── llm.py           # Ollama/llama.cpp
```

## Shell Epigenetics

Like biological epigenetics, the same genome (rigging) expresses differently depending on the shell. The epigenome stores:

- Resource budgets per reflex
- Preferred tool paths
- Adaptive parameters (timeouts, batch sizes)

## Thermodynamic Model

Every operation tracks energy flow:

- **E_input**: Cost of parsing the user query
- **E_state**: Cost of loading context
- **E_output**: Cost of executing the action
- **E_dissipated**: Wasted computation
- **ΔS (negentropy)**: Organization gained/lost

When ΔS > 0, the agent is gaining organization. When ΔS < 0, it's losing it.

## CLI

```bash
pincher status           # Show shell, model, reflex count, energy
pincher "mkdir foo"      # Execute or match reflex
pincher teach            # Interactive /teach flow
pincher pack             # Pack rigging to .nail
pincher unpack FILE      # Unpack .nail to this shell
pincher thermo           # Thermodynamic dashboard
pincher reflexes list    # List all reflexes
pincher reflexes prune   # Remove low-confidence reflexes
```

## Feature Gates

- `default`: No GPU support
- `cuda`: CUDA integration (optional)
- `gpu-metrics`: NVML GPU metrics (optional)

## License

MIT
