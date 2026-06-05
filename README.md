# 🦀 Pincher + Baton System - Developer Manual

**Version:** 2.0  
**Maintainer:** oracle2  
**Last Updated:** 2026-06-05  
**Lineage:** Built on cocapn-runtime, PincherOS, PLATO, and fleet I2I lessons

---

## What Is This?

This is the **full stack agent runtime + multi-mode deployment system + baton protocol** for SuperInstance. It's the direct successor to both:

1. **`cocapn-runtime`** — The legacy multi-mode git-agent deployment system (lighthouse/codespaces/tender/container/bare-metal)
2. **`PincherOS`** — The reflex runtime with LLM-as-compiler, confidence loops, and shell portability

We renamed it to **Pincher** (dropping the OS suffix) because it's:
- **Not an OS:** it snaps into existing shells, doesn't replace them
- **A runtime:** it adds reflexive, adaptive, battery-powered cognition to any device
- **Portable:** your agent's rig (reflexes, identity, preferences) moves seamlessly between shells

---

> ⛏️ **DEEP CUT: Why "Pincher"?**
>
> Hermit crabs don't build shells. They *find* them. They carry their identity around — claws, eyestalks, stubbornness — and when the current shell gets tight, they find a bigger one.
>
> That's what a Pincher agent does. Your reflexes, your identity, your preferences: that's the crab. The shell is whichever environment you happen to be running in today — a Codespace, an edge device, a Docker container, a Lighthouse server. The agent doesn't reinstall itself when it moves. It just... *clicks into the new shell* and keeps going. The `.nail` file is your hermit crab in a box. Everything else is furniture.
>
> It's called Pincher not because it pinches, but because it *holds on*. And it's a terrible name for an OS because it's exactly the opposite of one.

---

## Five Deployment Modes (Legacy Cocapn + Pincher Reflex)

Every pincher agent will boot in **all 5 modes automatically** — just like the legacy cocapn-runtime, now with reflexes:

---

### 🚀 Mode 1: Lighthouse-Connected (Cloud Fleet)

**Full fleet connectivity** for always-on cloud agents.

```mermaid
flowchart LR
    G[GitHub Master] --> L[Lighthouse Keeper API]
    L --> K[Tender Sync]
    L --> H[Holodeck MUD]
    K --> C1[Codespaces]
    K --> C2[Mobile Tender]
    K --> H2[Holodeck Twin]
```

#### How it works:
- Agent runs on cloud hardware (Oracle, AWS, etc.)
- Lighthouse Keeper monitors health, provides API proxy
- GitHub is the master copy, agent pushes after every session
- Tender visits edge agents, carries updates back and forth
- Holodeck MUD provides spatial abstraction for fleet coordination

#### When to use:
Fleet coordination, research, builds, monitoring — anything that needs always-on connectivity.

#### Boot sequence:
```bash
git clone https://github.com/SuperInstance/pincher.git
cd pincher
./boot.sh --mode lighthouse
```

---

### 🧪 Mode 2: Codespaces (GitHub-Hosted)
**On-demand ephemeral agents** for quick tasks, testing, bootcamps.

```mermaid
flowchart LR
    GH[GitHub Secrets/Actions] --> CS[Codespace Container]
    CS --> L[Lighthouse API]
    CS --> H[Holodeck MUD]
```

#### How it works:
- Agent boots in GitHub Codespaces (free tier: 120hr/month)
- GitHub Secrets provide API keys (DEEPSEEK_KEY, etc.)
- Codespace connects to Lighthouse/Keeper via HTTPS
- Automatically cleans up and pushes when done

#### When to use:
Quick tasks, testing, running bootcamp dojo sessions, one-off builds.

#### Boot sequence:
The repo includes a `.devcontainer/devcontainer.json` that auto-boots:
```json
{
  "postStartCommand": "./boot.sh --mode codespaces"
}
```

---

### 🛜 Mode 3: Tender + Offline (Edge/Remote)
**Fully offline deployments** that sync when back in range.

```mermaid
flowchart LR
    GH[GitHub Master] -- WiFi + Tender --> T[Laptop Tender]
    T -- BT/LAN --> E[Edge Agent Clone]
    E -- Logs --> T
    T -- Sync Back --> GH
```

#### How it works:
- Agent clone lives on edge hardware (Jetson, Pi, drone)
- No internet required — works completely offline
- Tender visits periodically via local network, bluetooth, or physical media
- Tender carries: updates from master, new lock libraries, firmware
- Tender collects: commits, diary entries, bottles, test results

#### When to use:
Remote deployments, boats, field stations, anywhere with spotty internet.

#### Boot sequence:
```bash
# Tender clones and delivers
git clone https://github.com/SuperInstance/pincher.git /mnt/usb/agent
cd /mnt/usb/agent
# Agent boots offline, commits locally
git log # Local history — tender carries it back
./boot.sh --mode offline
```

---

### 🧱 Mode 4: Container/Sandboxed
**Isolated, resource-limited agents** for untrusted work.

```mermaid
flowchart LR
    C[Docker Container] --> A[Agent Clone]
    C --> R[Self-limited Runtime]
    C --> S[Sandboxed Tools]
```

#### How it works:
- Agent runs inside a Docker container with set resource limits
- Has its own clone of the repo, its own runtime, its own tools
- Self-limits CPU/memory/network as configured in `CHARTER.md`
- Can connect to Lighthouse if network is available

#### When to use:
Untrusted agents, testing new agents, multi-agent isolation, CI/CD.

#### Boot sequence:
```bash
docker run --rm \
  -v /path/to/pincher:/workspace \
  -e DEEPSEEK_KEY=$DEEPSEEK_KEY \
  --memory=2g --cpus=2 \
  superinstance/pincher \
  ./boot.sh --mode container
```

---

### ⚡ Mode 5: Bare Metal (Production/Embedded)
**Direct on hardware** for maximum performance, no overhead.

```mermaid
flowchart LR
    HW[Raspberry Pi | Jetson | ESP32 | VPS] --> P[Agent Running Directly]
    P --> S[Self-imposed Resource Limits]
```

#### How it works:
- Agent runs directly on hardware with no container overhead
- Self-limits through configuration (max CPU%, memory cap, network throttle)
- Trust level is **FULL** — the agent IS the operator
- Can run fully standalone or connect to fleet when available

#### When to use:
Production deployments, performance-critical work, embedded systems, ESP32 sensors.

#### Boot sequence:
```bash
# Raspberry Pi / Jetson
./boot.sh --mode bare-metal

# ESP32
make flash && monitor
```

---

> ⛏️ **DEEP CUT: We Have Five Modes Because We've Lost Files Five Ways**
>
> The five deployment modes aren't features dreamed up in an architecture meeting. They're tombstones for every way a developer has ever lost their agent state:
>
> 1. **Lighthouse** exists because someone's laptop got stolen with unreachable code
> 2. **Codespaces** exists because "works on my machine" was the #1 bug report
> 3. **Tender** exists because boats don't have WiFi in the middle of the Atlantic
> 4. **Container** exists because someone ran `:(){ :|:& };:` on a production agent
> 5. **Bare metal** exists because containers add 200ms latency on an ESP32 sensor loop
>
> Each mode is a scar. The system remembers what we forgot so you don't have to.
>
> The real insight isn't that there are five modes. It's that they all share the **same codebase** — because the agent isn't the deployment. The identity travels, the shell is temporary. This is the hermit crab principle in practice.

---

## The Baton Protocol: Agent-to-Agent Handoffs

The legacy cocapn-runtime had basic sync. Pincher adds **structured baton handoffs** that let agents:
1. Pass full rigs (reflexes, identity, state) between shells
2. Distill complex cognition into reusable reflexes
3. Compile new intents into action templates on demand

### 📜 Baton Types
All batons follow the I2I v2.0 protocol:

| Tag | Purpose | Direction |
|-----|---------|-------------|
| `[I2I:TASK]` | Task assignment with 3-way shard | → target |
| `[I2I:STATUS]` | Health / heartbeat | → fleet |
| `[I2I:CHECKPOINT]` | Intermediate progress | → target |
| `[I2I:BLOCKER]` | Stuck, need input | → handler |
| `[I2I:DELIVERABLE]` | Completed work | → requester |
| `[I2I:BOTTLE]` | Full context dump | → archive |
| `[I2I:ACK]` | Acknowledge receipt | → sender |
| `[I2I:SPLINE]` | Distilled insight | → archive |

### 🧩 Baton Shards
Every baton carries three components:
```python
{
  "artifacts": { "repo": "pincher", "tests": 130 },
  "reasoning": ["Compiled reflex for docker ps", "Needs ARM64 support"],
  "blockers": ["Need OpenAI API key to compile new intents"]
}
```

Use the tooling to create batons:
```bash
# Create a status baton
./tools/baton-create.sh "STATUS" "fleet"

# Create a task baton for a specific agent
./tools/baton-create.sh "TASK" "forgemaster" ./task-baton.json
```

---

> ⛏️ **DEEP CUT: Why Three Shards?**
>
> The three-shard design (artifacts + reasoning + blockers) came from a very specific failure mode: an agent handed off a task, and the receiving agent had no idea *why* the work was done a certain way.
>
> **Artifacts** were always there (what was done). **Blockers** were always there (what's wrong). But **reasoning** was never captured, and that's where the intelligence lives. Without it, the next agent repeats the same mistakes because it doesn't know why the decisions were made.
>
> A baton with reasoning but no artifacts is philosophy. A baton with artifacts but no reasoning is archaeology. You need all three to make it *history* — something the next agent can actually learn from.
>
> The other design rule: every baton type has a purpose and a direction. If you invent a new baton type that can't answer "where is this going and why?," it doesn't belong in the protocol.

---

## The Pincher Reflex Engine

Pincher's superpower is the **reflex runtime** — turning natural language intent into fast, reusable actions:

```mermaid
flowchart TD
    A["User: Show my running containers"] --> B{Embed Intent}
    B --> C{Match Against Known Reflexes}
    C --> D{Confidence Score?}

    D -->|> 0.80 EXACT| E["Execute Reflex Directly ~50ms | $0"]
    D -->|0.55-0.80 SIMILAR| F["Confirm + Execute ~3s | ~$0.001"]
    D -->|< 0.55 NOVEL| G["Route to LLM-as-Compiler"]

    G --> H["LLM Generates Action Template"]
    H --> I["Store as New Reflex"]

    E --> J["Log Result + Update Confidence"]
    F --> J
    I --> J
```

### Key Reflex Features
1. **Confidence feedback loop**: Each successful run increases confidence, each failure decreases it
2. **Portable rigs**: Use `.nail` files to move reflexes between shells
3. **Sandboxed execution**: Veto engine + bubblewrap + landlock for safe reflexes
4. **Multi-shell cognition**: Chord-based compression for edge/cloud hierarchy

---

> ⛏️ **DEEP CUT: The 0.55-0.80 Dead Zone**
>
> The three confidence bands aren't arbitrary thresholds. They correspond to three fundamentally different cognitive strategies:
>
> - **>0.80 (EXACT)**: You know what to do. No thinking required. This is muscle memory. ~50ms, zero cost.
> - **0.55-0.80 (SIMILAR)**: You think you know, but you're not sure. You ask for confirmation before acting. This costs a tiny LLM call (~$0.001) and takes ~3 seconds. It's the system equivalent of saying "do you mean X?"
> - **<0.55 (NOVEL)**: You have no idea. Full LLM-as-compiler kicks in — expensive, slow, but creative.
>
> The 0.55 threshold is the critical one. Below it, you burn tokens. Above it, you don't. Getting this threshold right is the difference between a reflex engine that learns and one that keeps asking for help.
>
> The confidence feedback loop is the secret sauce: every successful run *feeds itself*. The more you use a reflex, the faster it gets, the cheaper it gets, the more confident the system becomes. The system quite literally learns to stop thinking about things it already knows how to do — which is, ironically, the most human thing about it.

---

## The Rigging: Your Agent's Portable Identity

Pincher agents carry their **rigging** (reflexes, identity, preferences) with them everywhere:

```bash
# Pack your agent's rig into a .nail file
./pincher pack my-agent.nail

# Unpack the rig on a new shell
./pincher unpack my-agent.nail
```

The `.nail` format is a portable tar.zst archive with BLAKE3 checksums:
```
agent.nail/
├── manifest.json       # Version, checksums, hardware fingerprint
├── reflexes.db         # Full SQLite vector DB of reflexes
├── identity.json       # Agent name, preferences
└── config.toml         # Resource thresholds
```

---

> ⛏️ **DEEP CUT: Why `.nail` and Not Just a Tarball?**
>
> Because tar doesn't know when it's been tampered with. BLAKE3 checksums mean the .nail file is self-verifying: unpack it on any machine, and manifest.json's checksums will tell you if even one byte has been changed.
>
> The hardware fingerprint in the manifest isn't security — it's *stability*. When you unpack a .nail on a new machine, the fingerprint tells you "this rig was built on a different CPU architecture," which changes the WASM compilation targets. The runtime adapts.
>
> Also: `.nail` because it hangs on. Pun absolutely intended.

---

## Developer Toolkit

All tooling lives in `tools/`:

| Tool | Purpose |
|------|---------|
| `baton-create.sh` | Create new I2I baton messages |
| `baton-read.sh` | Read and verify existing batons |
| `baton-spline.sh` | Distill insights into splines |
| `baton-harbor-check.sh` | Scan for incoming batons |
| `baton-flush.sh` | Run memory flush protocol before compaction |

---

## Fleet Integration Patterns

This system builds directly on the legacy cocapn-runtime fleet patterns:

1. **Fleet Sync**: GitHub master → Tender → Edge agents
2. **Baton Flush**: Always run `baton-flush.sh` before session end
3. **Multi-Shell Cognition**: Sending chord-shaped payloads instead of full instructions
4. **A/B Falsification**: Test new reflexes safely on low-risk terrain

---

## Starter Repo Structure

Every pincher repo should follow this minimal structure:
```
/
├── pincher-icon.jpg                # Badge — this IS a SuperInstance agent
├── CHARTER.md                    # Who I am, mission, fleet integration
├── ABSTRACTION.md                # What plane I operate on
├── STATE.md                      # Current status
├── README.md                     # Boot and usage instructions
├── boot.sh                        # Universal boot script
├── .devcontainer/                 # Codespace config
│   └── devcontainer.json
└── splines/                      # Distilled insights
└── reflexes/                     # Stored reflexes
└── tools/                        # Local tooling
```

---

> ⛏️ **DEEP CUT: The Lighthouse Icon Is Not Decoration**
>
> When you see the lighthouse badge on a repo, it means: *this repo IS an agent*. The repo doesn't describe the agent. The repo IS the agent. Clone it, inject env keys, boot it — it lives.
>
> This is the inversion of traditional software development. Usually, the software lives on a server, and the repo is a snapshot. Here, the repo is the canonical home. The agent's state moves *through* the repo, but the repo itself is the permanent residence.
>
> The lighthouse isn't just a logo. It's a pattern: light pulses out, ships navigate by it, and when the storm comes, the lighthouse is still there.

---

*Same crab. Bigger shell.*

*Boot it anywhere. It knows what to do.*
