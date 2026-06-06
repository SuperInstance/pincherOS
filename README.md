<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/hermit-crab.jpg">
    <img src="assets/hermit-crab.jpg" width="640" alt="Pincher — the hermit crab finds the right shell for every situation">
  </picture>
</p>

<h1 align="center">🦀 Pincher — Vector DB as Runtime, LLM as Compiler</h1>

<p align="center">
  <strong>A reflex runtime for agents.</strong> Teach once, match instantly, execute safely — every time faster.
</p>

<p align="center">
  <a href="#-the-reflex-engine"><img src="https://img.shields.io/badge/reflex_engine-live-brightgreen" alt="Reflex Engine"></a>
  <a href="./PLUG_AND_PLAY.md"><img src="https://img.shields.io/badge/docs-PLUG_AND_PLAY-blue" alt="Plug & Play"></a>
  <a href="./GETTING_STARTED.md"><img src="https://img.shields.io/badge/docs-GETTING_STARTED-blue" alt="Getting Started"></a>
  <a href="./ARCHITECTURE.md"><img src="https://img.shields.io/badge/docs-ARCHITECTURE-blue" alt="Architecture"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-lightgrey" alt="License"></a>
  <a href="./install.sh"><img src="https://img.shields.io/badge/install-one_line-success" alt="One-line install"></a>
</p>

<br>

---

## The Elevator Pitch

**Pincher is the runtime that makes the "conversation is the building" pattern actually work.**

You say what you want. Pincher matches it against learned reflexes — sub-millisecond for known intents, ~3 seconds for novel ones that need LLM reasoning. Every interaction makes the system faster. No daemon. No cloud dependency. No configuration.

It snaps into any shell on any Linux machine and adds adaptive, battery-powered cognition — the same Teach → Match → Execute loop that powers Nebula at the edge, but running locally with SQLite, ONNX embeddings, and bubblewrap sandboxing.

---

## 🧠 The Reflex Engine

```mermaid
flowchart TB
    subgraph Input["🗣️ Intent"]
        I["pincher do 'show running containers'"]
    end

    subgraph Engine["⚙️ Reflex Engine"]
        direction TB
        E["Embed intent\n(all-MiniLM-L6-v2 / hash fallback)\n384-dim vector"] --> M["Match against\nSQLite vector store"]
        
        M -->|"≥ 0.80<br/>Exact match"| F["⚡ Fast Path\nExecute known action\n~50ms • $0"]
        M -->|"0.55–0.80<br/>Similar match"| S["🔍 Similar Path\nLLM confirms + adapts\n~3s • ~$0.001"]
        M -->|"< 0.55<br/>Novel intent"| N["🚀 Slow Path\nLLM reasons + stores\nnew reflex"]
        
        F --> V
        S --> V
        N --> V
        
        V["🛡️ Veto Engine\nPattern check\nbefore sandbox"]
        V -->|"Blocked"| X["⛔ Rejected:\n'rm -rf /' blocked\nby veto pattern"]
        V -->|"Allowed"| B["📦 Bubblewrap Sandbox\nRead-only /usr + /lib\nNo network by default"]
        B --> R["✅ Result logged\nConfidence updated\nCache populated"]
    end

    subgraph Store["💾 SQLite Store"]
        DB[("reflexes.db\n384-dim vectors\nconfidence scores\ninvoke counts")]
    end

    R --> DB
    DB -->|"Fast path lookup"| M

    style I fill:#1a1a2e,stroke:#e94560,color:#fff
    style F fill:#0f3460,stroke:#16c79a,color:#fff
    style S fill:#0f3460,stroke:#e8d44d,color:#fff
    style N fill:#0f3460,stroke:#e94560,color:#fff
    style V fill:#1a1a2e,stroke:#e8d44d,color:#fff
    style B fill:#16213e,stroke:#16c79a,color:#fff
    style X fill:#3d0000,stroke:#e94560,color:#fff
    style R fill:#16213e,stroke:#16c79a,color:#fff
    style DB fill:#1a1a2e,stroke:#533483,color:#fff
```

## 📈 The Confidence Loop

Every reflex has a confidence score. Every execution changes it:

```mermaid
flowchart LR
    subgraph Cycle["🔄 Confidence Feedback Cycle"]
        A["⚡ Execute reflex"] --> B{"Success?"}
        B -->|"✅ Yes"| C["⬆️ x1.15\nConfidence boost"]
        B -->|"❌ No"| D["⬇️ x0.85\nConfidence penalty"]
        B -->|"⛔ Vetoed"| E["⬇️ x0.50\nVeto penalty"]
        C --> F{"Confidence ≥ 0.80?"}
        D --> F
        E --> F
        F -->|"✅ Yes"| G["🏆 Promoted to Fast Path\nFuture matches skip LLM"]
        F -->|"❌ No"| H["🔁 Stays in Similar/Slow\nKeeps learning"]
    end

    G -->|"next match"| A
    H -->|"next match"| A

    style A fill:#16213e,stroke:#16c79a,color:#fff
    style G fill:#0f3460,stroke:#16c79a,color:#fff
    style H fill:#1a1a2e,stroke:#e8d44d,color:#fff
```

**Three paths. One engine. No daemon needed.**

| Path | Confidence | Latency | Cost | When |
|------|-----------|---------|------|------|
| ⚡ **Fast** | ≥ 0.80 | ~50ms | $0 | Exact match — known reflex |
| 🔍 **Similar** | 0.55–0.80 | ~3s | ~$0.001 | Close match — LLM confirms + adapts |
| 🚀 **Slow** | < 0.55 | ~3-8s | ~$0.01 | Novel intent — full LLM reasoning + store |

> The system gets faster and more reliable the more you use it. Teach once, match instantly forever.

---

## 🎯 Built-in Reflexes

Every Pincher install ships with these ready to go:

```mermaid
mindmap
  root((Pincher\nReflexes))
    System
      system.info
      env.get
      pincher.status
      pincher.doctor
    Files
      file.read
      file.write
      file.list
    Processes
      process.list
      process.kill
    Network
      network.ping
      network.ports
    Git
      git.status
      git.diff
      git.log
    Docker
      docker.ps
      docker.logs
    Reflex Mgmt
      pincher.teach
      pincher.reflexes
      pincher.compile
```

---

## 🚀 Quick Start

```bash
# Install (checks for Rust, builds from source)
curl -fsSL https://raw.githubusercontent.com/SuperInstance/pincher/main/install.sh | bash

# Verify
pincher status
# → Engine: healthy · Reflexes: 12 · DB: ~/.pincher/reflexes.db

# Run a health check
pincher doctor
# → ONNX model: ✅ · SQLite: ✅ · Embedding: ✅ · Disk: 18G free

# Execute an intent
pincher do "list files in current directory"
# → [fast] matched 'file.list' at 0.92 confidence → executed in 47ms

# Teach a new reflex
pincher teach
# → Intent: show disk usage
# → Action: df -h /
# → Stored at confidence 0.55

# List all reflexes
pincher reflexes
# → 13 reflexes · avg confidence 0.67
```

---

## 🏗️ Under the Hood

```mermaid
graph TB
    subgraph Workspace["📦 Cargo Workspace"]
        CLI["pincher-cli\nClap CLI\nTokio async"]
        CORE["pincher-core\nRuntime Library"]
        INFER["pincher-infer\nPython Inference"]
    end

    subgraph Core["pincher-core modules"]
        REF["reflex/\nEngine · Matcher\nConfidence"]
        DB["db/\nSQLite + sqlite-vec\nVector store"]
        EMB["embed/\nONNX all-MiniLM-L6-v2\nHash fallback"]
        SAN["sandbox/\nBubblewrap\nVeto engine"]
        MIG["migration/\n.nail pack/unpack\nBLAKE3 + tar.zst"]
        RPC["rpc/\nJSON-RPC server\nProgrammatic control"]
        RES["resource/\nPID budgets\nMemory limits"]
        CAP["capability/\nSigned tokens\nManifests"]
    end

    CLI --> CORE
    CORE --> REF
    CORE --> DB
    CORE --> EMB
    CORE --> SAN
    CORE --> MIG
    CORE --> RPC
    CORE --> RES
    CORE --> CAP
    INFER --> EMB

    style CLI fill:#0f3460,stroke:#16c79a,color:#fff
    style CORE fill:#16213e,stroke:#e94560,color:#fff
    style INFER fill:#1a1a2e,stroke:#533483,color:#fff
    style Core fill:#1a1a2e,stroke:#533483,color:#fff
    style REF fill:#16213e,stroke:#16c79a,color:#fff
    style DB fill:#16213e,stroke:#e8d44d,color:#fff
    style EMB fill:#16213e,stroke:#16c79a,color:#fff
    style SAN fill:#16213e,stroke:#e94560,color:#fff
```

**Three crates. One philosophy. Zero daemons.**

| Crate | Lines | Role |
|-------|-------|------|
| [`pincher-core`](./pincher-core/) | ~8K | All runtime logic — reflex engine, vector store, sandbox, migration, RPC |
| [`pincher-cli`](./pincher-cli/) | ~1.5K | Clap-based CLI — all subcommands wired to core |
| [`pincher-infer`](./pincher-infer/) | ~500 | Python inference module for ONNX embeddings |

---

## 🛡️ Safety First

Pincher runs untrusted intent in a **hardened sandbox**:

```mermaid
flowchart LR
    subgraph Check["🔍 Pre-execution"]
        V["Veto Engine\nPattern matching\non command string"]
        P["PID Controller\nBudget check\nResource limits"]
    end

    subgraph Runtime["📦 Execution"]
        B["Bubblewrap\nRead-only /usr + /lib\nNo network\nWhitelisted bins"]
        L["Landlock (optional)\nKernel 5.13+\nFine-grained FS rules"]
        W["WASM (optional)\nwasmtime guest\nMemory sandboxed"]
    end

    subgraph Result["✅ Output"]
        O["stdout captured\nstderr captured\nexit code checked"]
    end

    I["Intent parsed"] --> V
    V --> P
    P -->|"Blocked"| X["⛔ Rejected:\n'rm -rf /' blocked\nby veto pattern"]
    P -->|"Allowed"| B
    B --> L
    L --> W
    W --> O

    style I fill:#1a1a2e,stroke:#e94560,color:#fff
    style V fill:#16213e,stroke:#e8d44d,color:#fff
    style P fill:#16213e,stroke:#e8d44d,color:#fff
    style X fill:#3d0000,stroke:#e94560,color:#fff
    style B fill:#0f3460,stroke:#16c79a,color:#fff
    style L fill:#0f3460,stroke:#16c79a,color:#fff
    style W fill:#0f3460,stroke:#16c79a,color:#fff
    style O fill:#16213e,stroke:#16c79a,color:#fff
```

- **Bubblewrap** — read-only system directories, no network by default, whitelisted binaries
- **Veto Engine** — pattern-based blocking *before* execution (catches `rm -rf /`, fork bombs, etc.)
- **PID Controller** — resource budgets per reflex, OOM protection
- **Landlock** (optional) — kernel-level filesystem restrictions (5.13+)
- **WASM** (optional) — web assembly guest execution via wasmtime

---

## 📦 The Rig: Portable Agent Identity

Pack your entire agent into a **`.nail` file** — a portable archive with BLAKE3 integrity:

```mermaid
packet-beta
0-15: "📦 .nail Archive"
16-31: "manifest.json"
32-47: "reflexes.db"
48-63: "identity.json"
64-79: "config.toml"
80-95: "BLAKE3 Checksums"
96-111: "Hardware Fingerprint"
```

```bash
# Pack it up
pincher pack --output my-agent.nail

# Ship it
scp my-agent.nail user@server:~/

# Unpack anywhere
pincher unpack --bundle my-agent.nail

# Run against the new machine
pincher run --bundle my-agent.nail "show disk usage"
```

---

## 🧪 CLI Reference

```text
pincher ─── status ─────── Engine health, reflex count, DB path
     │     doctor ─────── Full health check (ONNX, SQLite, disk)
     │     teach ──────── Interactive reflex teaching
     │     do "..." ───── Execute intent through reflex engine
     │     reflexes ───── List reflexes + confidence scores
     │     compile ────── Workspace → WASM reflex
     │     mature ─────── Fuzzing for coverage expansion
     │     bench ──────── Latency benchmark suite
     │
     ├── pack ─────────── Pack → .nail file
     ├── unpack ───────── Unpack .nail ←─ file
     ├── run ──────────── Execute .nail bundle
     │
     ├── publish ──────── Publish to registry (stub)
     ├── update ───────── Check registry updates (stub)
     │
     ├── shell-info ───── Hardware fingerprint
     └── gastrolith ───── Checkpoint migration
```

---

## 🧭 What Pincher Is (and Isn't)

**Pincher is a focused, portable reflex runtime that works right now.** Here's the honest scope of v0.1.0:

<div align="center">

| ✅ Is | ❌ Isn't |
|-------|----------|
| CLI-driven reflex engine | Cloud fleet manager ("Lighthouse Keeper") |
| SQLite-backed vector store | Edge-sync protocol ("Tender") |
| Bubblewrap sandbox for safety | Docker image on Docker Hub |
| Teach → Match → Execute loop | Holodeck MUD or fantasy |
| `.nail` portable bundles | ESP32 or WebAssembly builds |
| Local, offline capable | Five deployment modes |
| ONNX + hash embeddings | Instant-boot guarantees |

</div>

> The project deploys in exactly **one mode**: build from source, run the binary. That's the honest scope of v0.1.0.

---

## 🗺️ Near-Term Roadmap

```mermaid
gantt
    title Pincher v0.1.0 → v0.2.0
    dateFormat  YYYY-MM-DD
    axisFormat  %b %d
    
    section Core
    WASM guest execution       :done, 2026-05-20, 2026-06-05
    Landlock sandbox           :done, 2026-05-25, 2026-06-05
    Reflex publish/update      :active, 2026-06-01, 2026-06-20
    Multi-process pipelines    :2026-06-15, 2026-07-01

    section Quality
    Benchmark rigor            :active, 2026-06-01, 2026-06-15
    Confidence tuning          :2026-06-10, 2026-06-25
    Fuzz testing               :2026-06-15, 2026-07-01

    section Ecosystem
    Reflex registry API        :2026-06-20, 2026-07-10
    Nebula sync integration    :2026-07-01, 2026-07-15
    Make-me-app template       :2026-07-01, 2026-07-15
```

---

## 🧬 Lineage

Pincher is the descendant of a long line of agent infrastructure experiments:

```mermaid
timeline
    title From PLATO to Pincher
    2024 : PLATO Evennia MUD — 380 rooms, ensign training
    2025Q1 : LAU — Rust construct CLI + AI tutor
    2025Q3 : cocapn-runtime — constraint theory, spatial math
    2025Q4 : PincherOS — reflex engine, `.nail` rigs, veto engine
    2026Q1 : fleet I2I — agent-to-agent bottle protocol
    2026Q2 : Pincher v0.1.0 — standalone reflex runtime
             Nebula — edge-reflex twin at Cloudflare Workers
             Make Me A... — instant apps from conversation
```

Every shell fits. Every situation has the right shape. That's the hermit crab way.

---

## 🧪 Development

```bash
# Prerequisites: Rust toolchain
# (see rust-toolchain.toml for pinned version)

# Debug build
cargo build

# Release build (fast)
cargo build --release

# Full feature set
cargo build --release --features "onnx,landlock,wasmtime"

# Run all tests
cargo test --workspace
```

### Project Structure

```
/
├── Cargo.toml                 # Workspace root
├── rust-toolchain.toml        # Toolchain pinning
├── pincher-core/              # ~8K lines — all runtime logic
│   ├── src/reflex/            # Engine · Matcher · Confidence
│   ├── src/db/                # SQLite vector store (sqlite-vec)
│   ├── src/embed/             # ONNX all-MiniLM-L6-v2 + hash
│   ├── src/sandbox/           # Bubblewrap + Landlock + veto
│   ├── src/migration/         # .nail pack/unpack (tar.zst)
│   ├── src/rpc/               # JSON-RPC server
│   └── src/resource/          # PID budgets + memory limits
├── pincher-cli/               # ~1.5K lines — Clap CLI
├── pincher-infer/             # Python ONNX inference
├── tools/                     # Shell scripts (reflex-engine, fleet-scout, gc)
├── docs/                      # Architecture, roadmap, ADRs, checklists
├── assets/                    # Logo and hermit crab images
├── examples/                  # Code review, hello-reflex, smart-home
├── install.sh                 # One-line installer
└── .devcontainer/             # Codespace config
```

---

## 📚 Documentation

| Doc | What it covers |
|-----|---------------|
| [`PLUG_AND_PLAY.md`](./PLUG_AND_PLAY.md) | Quickest path from zero to running |
| [`GETTING_STARTED.md`](./GETTING_STARTED.md) | Detailed setup and first reflex |
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | Full system architecture and design decisions |
| [`API_REFERENCE.md`](./API_REFERENCE.md) | Complete CLI and library API reference |
| [`LOW_LEVEL.md`](./LOW_LEVEL.md) | Internal module walkthrough |
| [`docs/ROADMAP.md`](./docs/ROADMAP.md) | What's coming next |
| [`docs/FLEET_ARCHITECTURE.md`](./docs/FLEET_ARCHITECTURE.md) | How Pincher fits in the fleet |
| [`docs/COGNITIVE_REFLEXES.md`](./docs/COGNITIVE_REFLEXES.md) | Advanced reflex patterns |
| [`docs/PLATO-LINEAGE.md`](./docs/PLATO-LINEAGE.md) | The full history from PLATO to Pincher |

---

## 🤝 Contributing

PRs welcome. The project is in active development and the architecture is still settling. Good places to start:

- Add a built-in reflex dispatcher
- Improve veto pattern coverage
- Write benchmark harness tests
- Fix a `TODO` in the code

---

## 📜 License

**MIT OR Apache-2.0** — see `LICENSE`.

---

<p align="center">
  <img src="assets/logo.jpg" width="200" alt="Pincher logo">
</p>

<p align="center">
  <em>🦀 Same crab. Bigger shell.</em><br>
  <em>The hermit crab finds the right shell for every situation — but it starts with the one it's in.</em>
</p>
