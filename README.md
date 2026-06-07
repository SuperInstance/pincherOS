<div align="center">
<img src="assets/hermit-crab.jpg" width="320" alt="Hermit crab" />
</div>

# pincher

*A hermit crab doesn't grow a new shell. It finds one that fits, moves in, and makes it home.*

---

> 🚀 **The Hook:**  
> *A reflex engine for AI agents — intent → action in <50ms, zero dollars, every time, without asking an LLM.*

## 📐 The la-link (Architecture)

```
                     ┌──────────────────────────────────────┐
                     │            pincher                    │
                     │   ┌──────────────────────────────┐   │
You say ────▶ [1] ──▶│   │  Reflex Engine               │   │
something     Embed  │   │  ┌─────┐   ┌────────┐       │   │
             (384D)  │   │  │Match│──▶│Execute  │       │   │
                     │   │  │≥0.80│   │Directly │       │   │
                     │   │  └─────┘   └────────┘       │   │
                     │   │  ┌─────┐   ┌────────┐       │   │
                     │   │  │Match│──▶│Confirm  │       │   │
                     │   │  │0.55-│   │+ Execute│       │   │
                     │   │  │0.80 │   └────────┘       │   │
                     │   │  └─────┘   ┌────────┐       │   │
                     │   │  ┌─────┐   │LLM     │       │   │
                     │   │  │Match│──▶│Compiles│       │   │
                     │   │  │<0.55│   │New     │       │   │
                     │   │  └─────┘   │Reflex  │       │   │
                     │   │            └────────┘       │   │
                     │   │  ┌──────────────────────┐   │   │
                     │   │  │ Veto Engine (SAEP)   │   │   │
                     │   │  │ Security → Sandbox   │   │   │
                     │   │  └──────────────────────┘   │   │
                     │   └──────────────────────────────┘   │
                     │              │                       │
                     │              ▼                       │
                     │   ┌──────────────────────────────┐   │
                     │   │    Reflex Database           │   │
                     │   │    (SQLite + sqlite-vec)    │   │
                     │   └──────────────────────────────┘   │
                     │              │                       │
                     │              ▼                       │
                     │   ┌──────────────────────────────┐   │
                     │   │    .nail Bundle               │   │
                     │   │    (Portable Agent State)     │   │
                     │   └──────────────────────────────┘   │
                     └──────────────────────────────────────┘
```

**Three-tier compute:**

```
Fast  (ms):   Embedding match + reflex execution (no LLM)
Medium (s):   Confirmation + optional execution (low confidence)
Slow   (s):   LLM compilation → new reflex (learning event)
```

---

## 🧠 What Is a Reflex?

Here's what a reflex is.

You touch a hot stove. Your hand pulls back before your brain knows why. The spinal cord handled it. No deliberation, no committee, no latency budget meeting. By the time the cortex hears about it, the hand is already safe.

That's pincher. It's the spinal cord for AI agents.

An agent says "list running containers." Pincher doesn't ask an LLM. It doesn't think. It fires the reflex — match the intent against known patterns, execute the action, return the result. Under 50 milliseconds. Zero dollars. Every time.

And when it encounters something it hasn't seen before — something that doesn't match any known reflex — *that's* when it calls the LLM. Not to answer the question. To **compile a new reflex**. The LLM writes the action, pincher stores it, and next time the answer fires from memory. The cortex teaches the spinal cord. The spinal cord gets faster.

<div align="center">
<img src="assets/logo.jpg" width="120" alt="pincher logo" />
</div>

---

## 🤔 Why This Exists

Most agent systems have a latency problem masquerading as an architecture problem.

Every intent goes to the LLM. Every response costs money and time. The same "list files" query that took 50ms the first time takes 3 seconds and $0.002 the hundredth time. The LLM isn't getting smarter about it. It's doing the same work, over and over, because nothing remembers.

Pincher remembers.

It builds a reflex database — every successful intent→action pair, stored as a vector embedding with a confidence score. The more you use it, the faster it gets. The hundredth "list files" query matches at 0.95 confidence and fires in 50ms. The LLM never hears about it.

This isn't caching. Caching returns the same answer to the same question. Pincher returns the right answer to *similar* questions, because it matches on semantic embedding, not exact string. "Show me what's running" and "what processes are active" map to the same reflex. That's not cache. That's *understanding*.

---

## 🔄 How It Works

One loop. Four outcomes.

```
You say something
       ↓
  [1] Embed it (384 dimensions, ONNX or hash fallback)
       ↓
  [2] Search the reflex database (SQLite + sqlite-vec)
       ↓
  ┌────────────────────────────────────────────────────┐
  │  Match ≥ 0.80  →  Execute directly (~50ms, free)   │
  │  Match 0.55–0.80  →  Confirm, then execute (~3s)   │
  │  Match < 0.55  →  Route to LLM → store new reflex  │
  │  Vetoed by security  →  Block, log, warn            │
  └────────────────────────────────────────────────────┘
       ↓
  [3] Execute in sandbox (bubblewrap or raw process)
       ↓
  [4] Update confidence score — success ↑, failure ↓
```

Every cycle through this loop makes the agent faster and cheaper. The reflex database grows. The match scores climb. The LLM gets called less and less. The spinal cord takes over from the cortex.

That's not a metaphor. That's literally the architecture. The hot-stove reflex works the same way — cortex teaches spinal cord, spinal cord takes over, cortex moves on to harder problems.

---

## 📦 The `.nail` File — Your Agent, Portable

A hermit crab carries its shell wherever it goes. Pincher carries its reflexes the same way.

```bash
pincher pack --output scout.nail
```

That one command bundles your agent's entire identity — every learned reflex, every confidence score, every preference — into a single `.nail` file. A compressed archive with BLAKE3 checksums. Carry it to another machine, unpack it, and the agent picks up exactly where it left off.

```
scout.nail
├── manifest.json       # version, checksums, hardware fingerprint
├── reflexes.db         # the full vector database (everything the agent learned)
├── identity.json       # agent name, preferences
└── config.toml         # resource limits, thresholds
```

The `.nail` format is the connective tissue of the [SuperInstance ecosystem](https://github.com/SuperInstance/SuperInstance). It's what makes an agent portable — not just the code, but the *experience*. The reflexes that took a hundred interactions to build travel in a single file.

There's something deeper here too. When you look at a `.nail` bundle and see `reflexes.db` (learned abilities), `identity.json` (who the agent IS), and `config.toml` (stats) — you're looking at a character sheet. The trust scores are proficiency bonuses. The skill packs are starting equipment. The migration module is multiclassing.

We followed that thread. It led to [character-build](https://github.com/SuperInstance/character-build), [character-class](https://github.com/SuperInstance/character-class), [character-sheet](https://github.com/SuperInstance/character-sheet), [character-arc](https://github.com/SuperInstance/character-arc). Pincher was always an RPG. We just saw the character sheet hiding inside the `.nail` file.

---

## 🛠️ Installation

```bash
# Build from source (the honest way)
git clone https://github.com/SuperInstance/pincher.git
cd pincher
cargo build --release -p pincher-cli
cp target/release/pincher ~/.local/bin/

# Or one-line
curl -fsSL https://raw.githubusercontent.com/SuperInstance/pincher/main/install.sh | bash
```

---

## 🚀 Quickstart — First Five Minutes

```bash
# Is it alive?
pincher status

# What does it know?
pincher reflexes

# Run an intent through the reflex engine
pincher do "list files in current directory"

# Teach it something new (interactive)
pincher teach

# Health check — embeddings, database, sandbox, disk
pincher doctor

# Pack your agent into a portable file
pincher pack --output my-agent.nail
```

Every `pincher do` is a learning event. If the intent matches an existing reflex, it executes and the confidence goes up. If it doesn't match, the LLM compiles a new reflex and stores it. Either way, the agent knows more afterward than it did before.

---

## ⌨️ The CLI

| Command | What It Does |
|---------|-------------|
| `pincher status` | Engine health, reflex count, database path |
| `pincher doctor` | Full diagnostic — ONNX model, SQLite, disk, embeddings |
| `pincher teach` | Interactive: store a new intent→action reflex |
| `pincher do "..."` | Execute natural language through the reflex engine |
| `pincher reflexes` | List all stored reflexes with confidence scores |
| `pincher compile` | Compile workspace manifest → WASM reflex |
| `pincher mature` | Adversarial fuzzing to grow vector coverage |
| `pincher bench` | Benchmark suite (embed latency, teach/match cycles) |
| `pincher shell-info` | Hardware fingerprint of the current machine |
| `pincher pack` | Bundle agent state → portable `.nail` file |
| `pincher unpack` | Load a `.nail` bundle onto this machine |
| `pincher run` | Execute a bundle against user input |
| `pincher publish` | Publish bundle to the reflex registry |
| `pincher gastrolith` | Checkpoint migration management |

---

## 🗄️ The Vector Store

Every reflex lives in SQLite, indexed by its embedding vector:

```sql
CREATE TABLE reflexes (
    id          TEXT PRIMARY KEY,
    intent      TEXT NOT NULL,        -- what the user said
    action_sql  TEXT NOT NULL,        -- what to execute
    embedding   BLOB,                -- 384-dim f32 vector
    confidence  REAL DEFAULT 0.55,   -- how well this reflex has performed
    invoke_count INTEGER DEFAULT 0,  -- how many times it's been used
    created_at  TEXT,
    updated_at  TEXT
);
```

This is production code, not a schema sketch. The database lives at `~/.pincher/reflexes.db`. Vector search is via `sqlite-vec`. The schema is in `registry_schema.sql`. The implementation is in `pincher-core/src/db/`.

---

## 🔒 Security

Pincher runs untrusted commands. It takes that seriously.

**Veto engine** — pattern-based pre-execution blocking. Before any command reaches the sandbox, it passes through a veto check. `rm -rf /`, `mkfs`, `dd if=/dev/zero`, fork bombs — all blocked at the string level. The veto list is in the code, not in a config file someone might forget to update.

**Bubblewrap sandbox** — when available, every execution runs inside a bubblewrap container. No network access. `/usr` and `/lib` mounted read-only. Only whitelisted binaries (`ls`, `cat`, `grep`, `touch`, etc.) are accessible. Falls back to raw process execution with a warning if `bwrap` isn't installed.

**Capability tokens** — signed tokens that declare what an agent is allowed to do. The capability manifest is explicit: every permission is opt-in, nothing is granted by default.

---

## 🏗️ Architecture

Rust workspace, two crates:

**`pincher-core`** — all the runtime logic. Reflex engine, vector store, embeddings, sandbox, migration, RPC, security, resource control. Feature-gated for optional components (ONNX, Landlock, Wasmtime).

**`pincher-cli`** — the `pincher` binary. Clap-based, async via tokio, thin wrapper over the core library.

```
pincher-core/src/
├── reflex/       # The reflex engine (match, execute, teach, confidence)
├── db/           # SQLite vector store with sqlite-vec
├── embed/        # ONNX embeddings (all-MiniLM-L6-v2) + hash fallback
├── sandbox/      # Bubblewrap isolation + veto engine
├── migration/    # .nail pack/unpack with BLAKE3 + tar.zst
├── rpc/          # JSON-RPC server for programmatic control
├── resource/     # PID controller with budgets
├── capability/   # Signed tokens and manifests
├── security/     # Veto engine, landlock rules
├── route/        # Spectral clustering, label propagation, room graphs
├── immunology/   # Pattern-based immune system
├── shell/        # Hardware fingerprinting
└── dynamics/     # Carapace dynamics
```

#### Feature Flags

| Flag | What It Unlocks |
|------|----------------|
| `onnx` | Real ONNX Runtime embeddings (all-MiniLM-L6-v2) |
| `landlock` | Linux Landlock sandboxing (kernel 5.13+) |
| `wasmtime` | WASM guest module execution |

Without any features, pincher uses hash-based embedding fallback. It works. It's just less semantically aware — it'll match exact intents perfectly and similar intents less well.

---

## 🔌 Where It Lives in the Ecosystem

Pincher is layer 2 of the [SuperInstance five-layer stack](https://github.com/SuperInstance/SuperInstance):

```
cudaclaw        ← deployed kernels at fleet scale
cuda-oxide      ← compile intent to GPU machine code
flux-core       ← agent cognition as bytecode IR
pincher         ← reflexes: intent → action, <1ms   ← YOU ARE HERE
open-parallel   ← ternary math: {-1, 0, +1}
```

The layers aren't a pipeline. They're a nervous system. Pincher is the spinal cord — fast reflexes, no thinking. When pincher encounters something novel, it can escalate to [flux-core](https://github.com/SuperInstance/flux-core) (the cortex) for deliberation. When a deliberated response proves reliable, it can be compiled through [cuda-oxide](https://github.com/SuperInstance/cuda-oxide) and deployed via [cudaclaw](https://github.com/SuperInstance/cudaclaw) so thousands of agents can use it at GPU speed.

The cortex teaches the spinal cord. The spinal cord gets faster. Learning becomes reflex. That's the whole stack, in one sentence.

---

## 🔗 What Connects To This

- [**agent-sync**](https://github.com/SuperInstance/agent-sync) — teaches agents *when* to fire their reflexes. Timing > quality. The reflex is the lick. The sync is the moment.
- [**character-build**](https://github.com/SuperInstance/character-build) — reads `.nail` bundles as RPG character sheets. Stats, classes, abilities — all derived from reflex data.
- [**musician-soul**](https://github.com/SuperInstance/musician-soul) — the vector DB that learns from MIDI. Same embedding architecture, different domain. Music and reflexes are both patterns stored as vectors.
- [**lever-runner**](https://github.com/SuperInstance/lever-runner) — the sandbox where pincher's reflexes execute. 70 tokens of compute. Safe, fast, disposable.
- [**ternary-types**](https://github.com/SuperInstance/ternary-types) — Z₃ math primitives used throughout pincher's routing and matching logic.
- [**SuperInstance**](https://github.com/SuperInstance/SuperInstance) — the flagship repo. Onboarding, architecture, the whole story.

---

## ⚠️ What Pincher Is Not

Honesty matters more than marketing:

- **Not a cloud service.** No API keys to buy. No SaaS subscription. It runs on your machine, in your shell, with your data.
- **Not an LLM.** Pincher calls an LLM when it needs to compile a new reflex, but the LLM is the teacher, not the brain. The reflexes are the brain.
- **Not a framework.** You don't write plugins. You teach reflexes. The interaction model is `teach → match → execute`, not `import → configure → extend`.
- **Not a database.** The vector store is an implementation detail. The interface is natural language.
- **Not vaporware.** Every module listed above has source code in this repo. The CLI compiles. The tests pass. The sandbox works. If you find something that doesn't, that's a bug, not a roadmap item.

---

## 🦀 The Hermit Crab

A hermit crab is born soft. No shell. No armor. Just a body that needs protection and an instinct to find it.

It tries shells. Some are too big — clumsy, slow, exposed. Some are too small — cramped, can't grow. When it finds one that fits, it moves in. Not forever. When the crab grows, the shell doesn't. So it finds a bigger one.

Here's the thing people miss about hermit crabs: the shell isn't the crab. The crab *is* the crab. The shell is infrastructure — important, necessary, but replaceable. The crab carries its body from shell to shell. The reflexes, not the runtime.

Pincher is the shell. The `.nail` file is the crab — everything the agent learned, everything it is, packed up and ready to move to the next machine, the next runtime, the next version of itself. The shell can change. The crab carries on.

Same crab. Bigger shell.

---

## 📚 Knowledge Path

| Path | What You'll Learn | Start Here |
|------|-------------------|------------|
| 🧭 **A: Reflex Basics** | What's a reflex, how matching works | [`TUTORIALS.md`](./TUTORIALS.md#-tutorial-1-i-want-to-teach-my-first-reflex) |
| 🧭 **B: CLI Power** | All pincher commands and flags | [`pincher-cli/src/main.rs`](./pincher-cli/src/main.rs) |
| 🧭 **C: Embeddings** | ONNX vs hash fallback | [`pincher-core/src/embed/`](./pincher-core/src/embed/) |
| 🧭 **D: Sandbox Security** | Bubblewrap, veto, capability tokens | [`pincher-core/src/security/`](./pincher-core/src/security/) |
| 🧭 **E: Building Agents** | Create portable agent `.nail` bundles | [`TEMPLATES/ONBOARDING.md`](./TEMPLATES/ONBOARDING.md) |
| 🧭 **F: API Reference** | Full API docs | [`API_REFERENCE.md`](./API_REFERENCE.md) |
| 🧭 **G: Architecture** | Deep system design | [`ARCHITECTURE.md`](./ARCHITECTURE.md) |

---

## 📄 License

MIT OR Apache-2.0

---

*The cortex teaches the spinal cord. The spinal cord gets faster.*
*The hot stove is only hot once.*
