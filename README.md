<div align="center">

<img src="assets/logo.jpg" width="180" alt="PincherOS Logo" />

# PincherOS

### A Shell-Portable Agent State Machine

**Teach an agent once. Run it on any device. Migrate without losing its mind.**

[![CI](https://github.com/SuperInstance/pincherOS/actions/workflows/ci.yml/badge.svg)](https://github.com/SuperInstance/pincherOS/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org/)

</div>

---

## What Is PincherOS?

PincherOS is a **post-model operating system** for AI agents. It treats the LLM as a compiler, not a runtime. You teach your agent skills (called **reflexes**), and those reflexes execute directly at ~50ms with zero API cost. The LLM only fires when the agent encounters something genuinely new.

Think of it this way: Docker lets you build an app once and run it on any server. PincherOS lets you **teach an agent once and run it on any device** — from a Raspberry Pi to a workstation — without rewriting, without API bills, and without losing its memory.

The system is built around a hermit crab metaphor that maps directly to how it works:

| Hermit Crab | PincherOS | What It Means for You |
|---|---|---|
| **Shell** — the snail house it lives in | **Shell** — the hardware your agent runs on | Your Pi, your laptop, your cloud VM — all are just shells |
| **Crab** — the creature itself, with personality | **Rigging** — your agent's reflexes, identity, and learned behavior | The agent's mind is portable. The shell is not. |
| **Claws** — how the crab manipulates the world | **Claws** — the sandbox and capability system | Every reflex runs inside a security boundary |
| **Shell Swap** — finding a bigger shell | **Migration** — `pincher pack` → copy → `pincher unpack` | Move your agent between devices in seconds |

**The crab is not the shell. The crab migrates. The crab learns. The shell is just where the crab lives right now.**

---

## How It Works: The Reflex Engine

The core insight is that most AI agent work is **repetitive**. You ask an agent to "list files" or "check memory" dozens of times. Each time, the LLM reasons from scratch, costs money, and takes seconds. PincherOS short-circuits that:

```mermaid
flowchart TD
    A["User: 'list my docker containers'"] --> B{Embed Intent<br/>384-dim vector}
    B --> C{Match Against<br/>Known Reflexes}
    C --> D{Similarity Score?}

    D -->|"> 0.90<br/>EXACT MATCH"| E["Execute Reflex Directly<br/>~50ms | $0"]
    D -->|"0.70 - 0.90<br/>PROBABLE MATCH"| F["Confirm + Execute<br/>~3s | ~$0.001"]
    D -->|"< 0.70<br/>NOVEL INTENT"| G["Route to LLM<br/>Compile → New Reflex"]

    G --> H["LLM Generates<br/>Action Template"]
    H --> I["Store as New Reflex<br/>Next Time: Instant"]

    E --> J["Log Result +<br/>Update Confidence"]
    F --> J
    I --> J

    style E fill:#2d6a4f,stroke:#52b788,color:#fff
    style F fill:#e9c46a,stroke:#f4a261,color:#000
    style G fill:#e76f51,stroke:#d62828,color:#fff
    style I fill:#2d6a4f,stroke:#52b788,color:#fff
```

**The system gets cheaper and faster the more you use it.** Each successful execution increases confidence. Higher confidence means more direct hits. Direct hits skip the LLM entirely. Over time, your agent develops "muscle memory" — it reflexively handles what it knows and only reasons about what it doesn't.

---

## The Self-Improving Loop

PincherOS isn't just caching LLM calls. It's building an increasingly competent agent through a closed feedback loop. Each execution produces a confidence update. Each confidence update changes how the next intent is routed. And the loop feeds itself:

```mermaid
flowchart LR
    subgraph Teach["TEACH Phase"]
        T1["Intent + Action"] --> T2["Embed Intent"]
        T2 --> T3["Store Reflex<br/>confidence=0.50"]
    end

    subgraph Do["DO Phase"]
        D1["New Intent"] --> D2["Match Reflexes"]
        D2 --> D3{Confidence?}
        D3 -->|"High"| D4["Direct Execute"]
        D3 -->|"Low"| D5["LLM Compile"]
    end

    subgraph Learn["LEARN Phase"]
        L1["Execution Result"] --> L2{Success?}
        L2 -->|"Yes"| L3["Confidence UP<br/>+0.05"]
        L2 -->|"No"| L4["Confidence DOWN<br/>-0.10"]
        L3 --> L5["Stronger Reflex"]
        L4 --> L6["Weaker Reflex<br/>May Re-compile"]
    end

    T3 --> D2
    D4 --> L1
    D5 --> L1
    L5 --> D2
    L6 --> D2
```

Notice the feedback path: a reflex that keeps succeeding gets stronger and faster. A reflex that fails gets weaker and eventually gets re-compiled by the LLM. The system **self-decomposes** bad habits and **self-reinforces** good ones — without you doing anything.

---

## Shell Portability: The .nail Migration

This is what makes PincherOS different from every other agent framework. Your agent's entire learned state — its reflexes, confidence scores, identity, and configuration — can be packed into a single `.nail` file and moved to a completely different machine.

```mermaid
flowchart LR
    subgraph Shell_A["Shell A: Raspberry Pi"]
        A1["Reflexes: 47"]
        A2["Identity: 'dev-assistant'"]
        A3["Config: Pi-optimized"]
    end

    A1 & A2 & A3 --> P["pincher pack<br/>→ agent.nail<br/>(tar.zst + BLAKE3)"]

    P --> |"USB / SSH / Cloud"| U

    subgraph Shell_B["Shell B: Workstation"]
        B1["Hardware: Different"]
        B2["Reflexes: 47<br/>(same crab)"]
        B3["Config: Auto-adjusted"]
    end

    U["pincher unpack<br/>agent.nail"] --> B1 & B2 & B3

    style P fill:#7b2cbf,stroke:#9d4edd,color:#fff
    style U fill:#7b2cbf,stroke:#9d4edd,color:#fff
```

The QTR (Quiesce-Transfer-Resume) protocol ensures zero state loss during migration:

1. **Quiesce** — The agent finishes any in-flight execution and flushes all writes to SQLite
2. **Transfer** — The `.nail` file is created with BLAKE3 checksums for every component
3. **Resume** — On the new shell, the agent unpacks, re-fingerprintes, and adapts any shell-specific reflexes (e.g., if `apt` isn't available, mark that reflex for re-compilation)

The `.nail` file contains:
- `manifest.json` — version, source fingerprint, timestamp, checksums
- `reflexes.db` — the full SQLite database with all reflexes and embeddings
- `identity.json` — agent name, preferences, accumulated context
- `config.toml` — user configuration and overrides

---

## Resource Control: The PID Controller

Real hardware has real limits. PincherOS uses a continuous PID (Proportional-Integral-Derivative) controller to maintain homeostasis — just like a thermostat, but for RAM and CPU:

```mermaid
flowchart TD
    S["System Sensors<br/>(RAM, CPU, Disk)"] --> PID["PID Controller<br/>Kp=0.6 Ki=0.1 Kd=0.3"]
    PID --> B{Resource State?}

    B -->|"RAM < 70%<br/>CPU < 60%"| N["NORMAL<br/>Full LLM access<br/>Full context window"]
    B -->|"RAM 70-85%<br/>CPU 60-80%"| L["LIGHT<br/>Reduced context<br/>Skip LLM for confidence > 0.85"]
    B -->|"RAM > 85%<br/>CPU > 80%"| C["CRITICAL<br/>Reflex-only mode<br/>No LLM calls at all"]

    N -->|"Pressure rises"| L
    L -->|"Pressure rises"| C
    C -->|"Pressure drops"| L
    L -->|"Pressure drops"| N

    style N fill:#2d6a4f,stroke:#52b788,color:#fff
    style L fill:#e9c46a,stroke:#f4a261,color:#000
    style C fill:#e76f51,stroke:#d62828,color:#fff
```

This means PincherOS runs **on a Raspberry Pi**. When resources are tight, the agent doesn't crash — it degrades gracefully, falling back to reflex-only mode. The LLM sidecar unloads after 5 minutes of idle time. The whole system targets ~1GB total on a Pi 4.

---

## Security: Claws and Capabilities

Every reflex execution passes through two security layers before any code runs:

```mermaid
flowchart TD
    I["Intent to Execute"] --> V["1. VETO ENGINE<br/>(Deterministic Rules)"]

    V --> V1{"Forbidden command?"}
    V1 -->|"rm -rf /, /etc writes"| DENY["DENY<br/>Hard block"]
    V1 -->|"OK"| V2{"Requires capability?"}

    V2 -->|"Network, GPU, etc."| V3{"Capability<br/>in manifest?"}
    V3 -->|"No"| DENY
    V3 -->|"Yes"| V4{"Size/resource<br/>limits OK?"}
    V4 -->|"No"| DENY
    V4 -->|"Yes"| S

    S["2. SANDBOX<br/>(bwrap + landlock)"] --> E["Execute in<br/>Isolated Namespace"]

    E --> R["Result"]

    style DENY fill:#e76f51,stroke:#d62828,color:#fff
    style S fill:#2d6a4f,stroke:#52b788,color:#fff
    style E fill:#2d6a4f,stroke:#52b788,color:#fff
```

**Veto Engine** (deterministic, MVP) — A rule-based safety check that blocks dangerous patterns before they ever reach execution. No ML, no heuristics, no edge cases. Pure rules:

- No `rm -rf /` or equivalent
- No writing to `/etc`, `/boot`, `/sys`
- No network access without explicit `network` capability
- No subprocess spawning without `subprocess` capability
- File size limits enforced per-reflex

**Sandbox** (bwrap + landlock) — After veto passes, the reflex runs inside a restricted namespace. The capability manifest declares exactly what the reflex needs, and the sandbox provides exactly that — nothing more.

---

## Quick Start

```bash
# 1. Clone and build
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build --release

# 2. Check your shell (your hardware fingerprint)
./target/release/pincher status
```

This prints your shell info — hostname, RAM, CPU cores, and an ASCII hermit crab:

```
   🦀 PincherOS v0.1.0
  ╱╱╱╱╱╱╱╱╱╱╱╱╱
 ╱  Shell: my-pi       ╲
╱   Reflexes: 0          ╲
╲   State: Normal        ╱
 ╲  RAM: 34.2%         ╱
  ╰────────────────────╯
```

```bash
# 3. Teach your first reflex
./target/release/pincher teach --intent "list docker containers" --action "docker ps"
# → ✓ Reflex stored! (2.1ms)

# 4. Execute it — notice it matches instantly, no LLM needed
./target/release/pincher do "show me my containers"
# → ✓ Matched reflex: "list docker containers" (confidence 0.92, 48ms)

# 5. Pack and migrate to another machine
./target/release/pincher pack my-agent.nail
scp my-agent.nail workstation:~/

# On the workstation:
./target/release/pincher unpack my-agent.nail
# → ✓ Same crab. Bigger shell.
```

---

## Practical Examples: What Can You Build?

### 1. Smart Home Controller on a Pi

Teach your agent to control lights, read sensors, and manage your smart home — all running locally on a Raspberry Pi with zero cloud dependency. The reflex engine handles routine commands ("turn off the kitchen lights") in 50ms, and only routes to the LLM for novel requests.

👉 **[Walk through the example →](examples/smart-home/)**

### 2. Code Review Assistant

Teach PincherOS your codebase's review patterns: "check for SQL injection," "verify error handling," "enforce naming conventions." As it reviews more PRs, it learns your team's specific standards and short-circuits the common checks.

👉 **[Walk through the example →](examples/code-review/)**

### 3. Deploy Agent That Migrates

Train an agent on your workstation with full LLM access, then pack it into a `.nail` file and deploy it to a cloud VM or edge device. It carries all its learned reflexes. On the target, it runs in reflex-only mode if resources are tight.

👉 **[Walk through the example →](examples/deploy-agent/)**

### 4. Multi-Device Dev Assistant

You work across three machines — a workstation, a laptop, and a Pi. Each has different tools and configs. Your agent lives on all three, adapting its reflexes to each shell. Teach it once on any device, pack, and unpack on the others.

👉 **[Walk through the example →](examples/migration-demo/)**

### 5. Hello Reflex (5-Minute Tutorial)

The simplest possible example: teach one reflex, execute it, watch the confidence climb. Perfect for understanding the core loop before building something real.

👉 **[Walk through the example →](examples/hello-reflex/)**

---

## Architecture: Two Processes, One Mind

PincherOS uses a two-process architecture that keeps the agent's state safe and the LLM sidecar disposable:

```mermaid
flowchart TB
    subgraph Core["pincher-core (Rust) — Owns ALL State"]
        direction TB
        DB["SQLite + sqlite-vec<br/>Reflexes · Sessions · Action Log"]
        EM["Embedder<br/>SHA-256 trigram → 256-dim<br/>ONNX MiniLM → 384-dim"]
        RE["Reflex Engine<br/>teach · match · do · confidence"]
        RC["PID Resource Controller<br/>Normal · Light · Critical"]
        SE["Security Layer<br/>Veto Engine · Sandbox · Capabilities"]
        MG["Migration Layer<br/>pack · unpack · fingerprint"]
    end

    subgraph CLI["pincher-cli (Rust)"]
        CL["pincher status · teach · do ·<br/>match · pack · unpack · bench ·<br/>reflexes · shell-info · rpc"]
    end

    subgraph Infer["pincher-infer (Python) — Stateless"]
        direction TB
        PY["JSON-RPC Server"]
        DI["Distiller<br/>LLM-as-Compiler"]
        EMB["Embedder Fallback<br/>sentence-transformers"]
    end

    CLI -->|"subcommand"| Core
    Infer -->|"UDS JSON-RPC"| Core
    Infer -.->|"lazy load,<br/>unload after 5min"| Infer

    style Core fill:#1a1a2e,stroke:#e94560,color:#fff
    style CLI fill:#16213e,stroke:#0f3460,color:#fff
    style Infer fill:#1a1a2e,stroke:#533483,color:#fff
```

**Why two processes?**

- **pincher-core** (Rust) owns every byte of state. SQLite for durability, embeddings for matching, PID for resource control. If the LLM crashes, the agent's mind is untouched.
- **pincher-infer** (Python) is completely stateless. It loads when the LLM is needed, distills an intent into a reflex template, and unloads after 5 minutes of idle. Kill it, restart it — the agent doesn't care.

This separation means your agent's personality and memories survive:
- LLM API outages (reflexes still fire)
- Python crashes (Rust core is unaffected)
- Network disconnection (all state is local SQLite)
- Device migration (pack → unpack on a new shell)

---

## The .nail Format

The `.nail` file is how a hermit crab carries its rigging to a new shell. It's a `tar.zst` archive with BLAKE3 checksums:

```
agent.nail (tar.zst)
├── manifest.json       # Version, source fingerprint, timestamp, reflex count, checksums
├── reflexes.db         # Full SQLite database (reflexes, embeddings, action log)
├── identity.json       # Agent name, preferences, accumulated context hints
└── config.toml         # User configuration, resource thresholds, capability defaults
```

When you unpack on a new shell, PincherOS:
1. Verifies all BLAKE3 checksums (tamper detection)
2. Compares hardware fingerprints (compatibility scoring)
3. Marks shell-specific reflexes for re-compilation if needed (e.g., `apt` → `brew`)
4. Merges the incoming state with any existing reflexes (dedup by embedding similarity)

See [docs/nail-format.md](docs/nail-format.md) for the full specification.

---

## Command Reference

| Command | What It Does | Example |
|---|---|---|
| `pincher status` | Show shell fingerprint, reflex count, resource state | `pincher status` |
| `pincher teach` | Teach a new reflex (interactive or CLI args) | `pincher teach -i "check disk" -a "df -h"` |
| `pincher do <intent>` | Execute an intent through the reflex engine | `pincher do "how much disk is free"` |
| `pincher match <intent>` | Show what would match, without executing | `pincher match "disk space"` |
| `pincher pack` | Pack current state into a .nail file | `pincher pack agent.nail` |
| `pincher unpack <nail>` | Unpack a .nail file and merge state | `pincher unpack agent.nail` |
| `pincher bench` | Run performance benchmarks | `pincher bench` |
| `pincher shell-info` | Detailed hardware fingerprint | `pincher shell-info` |
| `pincher reflexes` | List all stored reflexes with confidence | `pincher reflexes --verbose` |
| `pincher rpc` | Start JSON-RPC server for sidecar | `pincher rpc --port 9876` |

---

## Project Structure

```
pincherOS/
├── pincher-core/               # Core library (Rust) — the crab's nervous system
│   ├── src/
│   │   ├── db/                 #   SQLite + sqlite-vec (reflexes, sessions, action log)
│   │   ├── embed/              #   Embedding layer (hash-based + ONNX MiniLM)
│   │   ├── reflex/             #   Reflex engine (teach, match, do, confidence)
│   │   ├── resource/           #   PID resource controller (Normal/Light/Critical)
│   │   ├── security/           #   Veto engine + sandbox + capabilities
│   │   ├── migration/          #   .nail pack/unpack + hardware fingerprinting
│   │   ├── capability/         #   Capability manifests + tokens
│   │   ├── dynamics/           #   Command dynamics (deterministic veto)
│   │   ├── shell/              #   Hardware probing
│   │   └── rpc/                #   JSON-RPC server for Python sidecar
│   └── examples/
│       └── teach_and_do.rs     #   Quick library usage demo
├── pincher-cli/                # CLI binary — the exoskeleton
│   └── src/main.rs
├── pincher-infer/              # Python sidecar — the LLM compiler
│   └── pincher_infer/
│       ├── server.py           #   JSON-RPC over UDS
│       ├── distill.py          #   LLM-as-compiler (intent → action template)
│       ├── embed.py            #   sentence-transformers fallback
│       └── config.py           #   Configuration from env + TOML
├── examples/                   # Plug-and-play walkthroughs
│   ├── hello-reflex/           #   5-minute tutorial
│   ├── smart-home/             #   Pi-based home automation
│   ├── code-review/            #   Automated PR review
│   ├── deploy-agent/           #   Train then deploy
│   └── migration-demo/         #   Multi-device migration
├── docs/
│   ├── agent/                  # Agent-centric documentation (a2a native)
│   ├── adr/                    # Architecture Decision Records
│   ├── research/               # Post-MVP research RFCs
│   ├── nail-format.md          # .nail binary format spec
│   ├── threats.md              # Security threat model
│   └── RISKS.md                # Risk register
└── .github/workflows/ci.yml   # GitHub Actions CI
```

---

## Documentation

### For Developers

| Document | Description |
|---|---|
| [ROADMAP.md](ROADMAP.md) | 12-week MVP sprint + 6-month ecosystem horizon |
| [MVP_CHECKLIST.md](MVP_CHECKLIST.md) | Exit criteria for the MVP |
| [docs/nail-format.md](docs/nail-format.md) | `.nail` binary format specification |
| [docs/threats.md](docs/threats.md) | Security threat model |
| [docs/RISKS.md](docs/RISKS.md) | Risk register |
| [docs/architecture.md](docs/architecture.md) | Architecture deep-dive |
| [docs/developer-guide.md](docs/developer-guide.md) | Contributing and development setup |
| [docs/adr/001-sqlite-vec-over-lancedb.md](docs/adr/001-sqlite-vec-over-lancedb.md) | ADR: Why SQLite over LanceDB |

### For Agents

If you're an AI agent reading this repository, there's a dedicated documentation set written in **a2a-native format** — structured for machine consumption with explicit interfaces, state machines, and integration protocols:

👉 **[docs/agent/](docs/agent/)** — Agent-to-agent documentation

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/developer-guide.md](docs/developer-guide.md).

```bash
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build
cargo test
```

---

## License

PincherOS is released under the [MIT License](LICENSE).

---

<div align="center">

<img src="assets/hermit-crab.jpg" width="300" alt="Hermit Crab" />

**Same crab. Bigger shell.**

</div>
