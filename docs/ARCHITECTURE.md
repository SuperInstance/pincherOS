# Pincher Architecture

## Complete System Stack

```
                    ┌─────────────────────────────┐
                    │       User / Intent Input     │
                    └─────────────┬───────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │   1. Vector Distance Router  │
                    │   (reflexes.db lookup <3ms)  │
                    └─────────────┬───────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │   2. Variable Extractor      │
                    │   (regex param extraction)   │
                    │   ~0.5ms — no LLM at edge   │
                    └─────────────┬───────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │   3. WASM Sandbox Execution  │
                    │   (wasmtime, <12ms)          │
                    └─────────────┬───────────────┘
                                  │
            ┌─────────────────────┼─────────────────────┐
            │                     │                     │
            ▼                     ▼                     ▼
    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
    │ 4. Telemetry │    │ 5. Registry  │    │ 6. Self-Heal │
    │   Daemon     │    │   Client     │    │   Compiler   │
    │  (background)│    │  (publish)   │    │  (cloud fix) │
    └──────────────┘    └──────────────┘    └──────────────┘
```

## Core Components

### 1. Background Daemon (`src/daemon.rs`)
- Low-priority SCHED_IDLE thread
- Polls SQLite `telemetry_queue` for failed reflexes
- Uploads error payloads to cloud compiler when network available
- Atomically patches `reflexes.db` when healed WASM returns

### 2. Registry Client (`src/registry.rs`)
- Stateless .nail bundle publishing to central registry
- Bearer token + cryptographic signature verification
- Developer registration, package management, release tracking
- Immutable version enforcement: no overwrites allowed

### 3. Variable Extractor (`src/extractor.rs`)
- Pre-compiled regex patterns with named capture groups
- Runs in <1ms at the edge — no LLM needed at runtime
- Falls back to keyword extraction when regex doesn't match
- Schema coverage validation tooling

### 4. Cloud Regex Generator (`scripts/regex_compiler.py`)
- LLM generates PCRE patterns from variable schemas
- Self-validating: patterns tested against seed phrases
- Temperature 0.0 prevents creative regressions
- Refuses to release broken patterns

### 5. Self-Healing Compiler (`scripts/self_heal.py`)
- Cloud-side fixer for broken WASM reflexes
- Never runs on the edge device (sterile local design)
- Analyzes error logs, rewrites code, deploys silent patches
- Daemon mode watches error telemetry directory

### 6. Registry Schema (`registry_schema.sql`)
- PostgreSQL: developers, packages, bundle_releases tables
- Composite unique constraint on (package_id, version_semver)
- Cryptographic author verification
- Telemetry table for edge device error collection

## Data Flow

```
Developer Push → GitHub Actions CI/CD
                    │
                    ▼
              pincher compile       (LLM-as-Compiler)
                    │
                    ▼
              pincher mature        (Adversarial Fuzzing)
                    │
                    ▼
              pincher pack          (Sign + Seal .nail)
                    │
                    ▼
              pincher publish       (Registry Upload)
                    │
                    ▼
              Edge Device Runs      pincher update
                    │
                    ▼
              [If Reflex Fails] → Telemetry Daemon
                    │
                    ▼
              Cloud Self-Heal Loop → Fresh WASM → reflexes.db update
```

## Key Design Rules

1. **Edge is sterile** — never self-modifies code, only updates via signed .nail bundles
2. **Registry is immutable** — once version v1.2.3 is published, it can never be overwritten
3. **Regex beats LLM** — pre-compiled extraction at <1ms beats API call at >500ms
4. **Cloud heals** — broken code goes UP to compiler, fixed code comes DOWN to edge
5. **Zero-temperature healing** — creativity causes regressions; deterministic fixes only

---

## 🎬 The Real Story: Why the Edge Must Stay Sterile

Every architecture document says "the edge should be sterile" like it's a universal truth. It's not. It's a scar from a specific battle.

We learned this the hard way during the **PLATO days**: agents that could self-modify were fine — until they weren't. The edge device would compile a reflex, run it successfully 47 times, then on the 48th time the WASM sandbox would catch an infinite loop, and by then the device was at 98% CPU with a bricked agent. The agent had mutated itself into a corner it couldn't escape.

**The rule is simple:** an edge device should never trust its own creativity. The edge is for *execution*. The cloud is for *generation*. If the edge needs new code, it sends a telemetry packet UP. The cloud compiles, signs, and sends it DOWN. The edge never generates. It only receives, verifies, and runs.

This asymmetry is deliberate. It means:
- The edge can be fully offline and still run (it just can't *learn* new reflexes)
- The registry is a single source of truth — no two devices can diverge
- A compromised edge device can't spread mutated code because it can't sign bundles

The sterility cost is real: an offline edge device can't learn. But the cost of a bricked device that can't even *reboot into a safe state* is higher. The tradeoff is worth it until someone builds a verifiably safe self-modification sandbox.

**The deeper insight:** this architecture inverts the standard AI pattern. Normal AI runs inference on the cloud and sends results to the edge. Pincher runs inference on the edge and sends *failure telemetry* to the cloud. The cloud only acts when something breaks. The edge is the primary runtime. The cloud is the safety net.

This is backwards from ChatGPT, and that's the point. Pincher is not a chatbot. It's a runtime that happens to use an LLM as a compiler.
