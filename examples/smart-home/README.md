# Smart Home Controller — PincherOS on a Raspberry Pi

Turn a Raspberry Pi into a reflex-driven smart home controller. Routine commands — lights, sensors, thermostat, locks — become reflexes that execute in ~50ms with zero API cost. Novel requests ("turn everything off and lock up, I'm leaving") still get routed to the LLM for compilation.

---

## The Concept

Home automation has a long tail of repetitive commands. You say "turn on the kitchen lights" dozens of times. Each time, a cloud-based voice assistant:

1. Sends audio to a remote server
2. Runs it through an LLM
3. Sends a command back to your device
4. Latency: 1–3 seconds. Cost: a fraction of a cent. Privacy: your voice leaves your house.

With PincherOS on a Pi, the same command:

1. Matches a local reflex in ~2ms
2. Executes the action in ~50ms
3. Latency: under 100ms. Cost: $0.00. Privacy: nothing leaves your house.

The LLM is only invoked for genuinely new requests. After it compiles a new reflex, that reflex is available instantly next time. Your smart home gets faster and cheaper the more you use it.

---

## Setting Up on a Raspberry Pi

### Option A: Build from Source on the Pi

```bash
# SSH into your Pi
ssh pi@homebridge.local

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and build
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build --release

# This takes ~15 minutes on a Pi 4. Go get coffee.
```

### Option B: Cross-Compile from Your Workstation

```bash
# On your workstation (much faster)
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu

cd pincherOS
cargo build --release --target aarch64-unknown-linux-gnu

# Copy the binary to your Pi
scp target/aarch64-unknown-linux-gnu/release/pincher pi@homebridge.local:~/
scp -r examples/smart-home/ pi@homebridge.local:~/smart-home/
```

### Verify It Works

```bash
pincher status
```

On a Pi 4 with 4GB RAM, you should see:

```
   🦀 PincherOS v0.1.0
  ╱╱╱╱╱╱╱╱╱╱╱╱╱
 ╱  Shell: homebridge     ╲
╱   Reflexes: 0            ╲
╲   State: Normal          ╱
 ╲  RAM: 18.5%           ╱
  ╰──────────────────────╯
```

---

## Teaching Smart Home Reflexes

Each reflex maps a natural-language intent to an HTTP call against your Homebridge (or similar) API.

### Kitchen Lights

```bash
pincher teach \
  --intent "turn on the kitchen lights" \
  --action "curl -s http://homebridge.local:8581/api/lights/kitchen/on"
```

```bash
pincher teach \
  --intent "turn off the kitchen lights" \
  --action "curl -s http://homebridge.local:8581/api/lights/kitchen/off"
```

### Temperature Sensor

```bash
pincher teach \
  --intent "what's the temperature" \
  --action "curl -s http://homebridge.local:8581/api/sensors/temp"
```

### Thermostat

```bash
pincher teach \
  --intent "set thermostat to 72" \
  --action "curl -s 'http://homebridge.local:8581/api/thermostat?temp=72'"
```

### Front Door Lock

```bash
pincher teach \
  --intent "lock the front door" \
  --action "curl -s http://homebridge.local:8581/api/locks/front/lock"
```

```bash
pincher teach \
  --intent "unlock the front door" \
  --action "curl -s http://homebridge.local:8581/api/locks/front/unlock"
```

### Camera Feed

```bash
pincher teach \
  --intent "show me camera feed" \
  --action "curl -s http://homebridge.local:8581/api/cameras/front/snapshot -o /tmp/camera-snapshot.jpg"
```

### Verify All Reflexes

```bash
pincher reflexes
```

You should see 6 reflexes, each at confidence 0.50 (initial). Now try executing one:

```bash
pincher do "turn on kitchen light"
# → ✓ Matched reflex: "turn on the kitchen lights" (confidence 0.93, 48ms)
# → {"status": "ok", "lights": {"kitchen": "on"}}
# → Confidence updated: 0.50 → 0.55
```

Or use the batch script (see below):

```bash
cd ~/smart-home
chmod +x teach-reflexes.sh
./teach-reflexes.sh
```

---

## How the Reflex Short-Circuit Saves Latency and Cost

Consider the path of a typical smart home command through different systems:

| System | Path | Latency | Cost | Privacy |
|---|---|---|---|---|
| Cloud voice assistant | Audio → Cloud → LLM → Command → Device | 1–3s | ~$0.002 | Voice leaves home |
| Local LLM (Ollama on Pi) | Text → LLM → Command | 2–10s | $0 (but high RAM) | Local |
| PincherOS reflex | Text → Embed → Match → Execute | ~50ms | $0 | Local |

The reflex path is **20–60× faster** than cloud and **40–200× faster** than a local LLM. And it uses a fraction of the RAM — the ONNX embedder needs ~50MB vs. 4–8GB for a local LLM.

When the LLM sidecar is needed (for a novel intent like "set up a movie scene with dim lights and jazz"), it loads on demand, compiles the reflex, and unloads after 5 minutes of idle. The Pi's RAM is free the rest of the time.

---

## Resource Degradation: What Happens Under Load

The PID resource controller monitors RAM and CPU. On a Pi 4 with 4GB:

| State | RAM | CPU | Behavior |
|---|---|---|---|
| **Normal** | < 70% | < 60% | Full LLM access. New reflexes can be compiled. |
| **Light** | 70–85% | 60–80% | Reduced context. LLM only fires for confidence < 0.85. |
| **Critical** | > 85% | > 80% | Reflex-only mode. No LLM calls at all. |

If you're running Homebridge + PincherOS + a browser on the Pi, you might hit **Light** during peak use. That's fine — your existing reflexes still fire at full speed. Only novel intents get queued until resources free up.

If another process hogs RAM (looking at you, Chrome), the PID controller shifts to **Critical**. All existing reflexes continue working. You just can't teach new ones until resources recover. The LLM sidecar is automatically unloaded.

You can monitor this in real time:

```bash
watch -n 2 pincher status
```

---

## Security: Capability Manifests for Network Access

Every reflex that makes network calls needs the `network` capability. This is declared in a **capability manifest** — a TOML file that specifies exactly what a reflex is allowed to do.

See `smart-home-capabilities.toml` in this directory for the full manifest. The key sections:

```toml
[capabilities.network]
allowed = true
allowed_hosts = ["homebridge.local:8581"]
max_request_size = "1KB"
max_response_size = "100KB"

[capabilities.filesystem]
write_paths = ["/tmp/camera-snapshot.jpg"]
read_paths = []
```

This means:
- Network access is **allowed**, but only to `homebridge.local:8581`
- Requests can be at most 1KB, responses at most 100KB
- The camera reflex can write to `/tmp/camera-snapshot.jpg`, but nowhere else
- No other filesystem reads or writes are permitted

If a reflex tries to `curl` an external URL, the **veto engine** blocks it before execution. If it somehow gets past the veto, the **sandbox** (bwrap + landlock) prevents the network call at the OS level.

To apply the capability manifest:

```bash
pincher teach \
  --intent "turn on the kitchen lights" \
  --action "curl -s http://homebridge.local:8581/api/lights/kitchen/on" \
  --capabilities ./smart-home-capabilities.toml
```

---

## Migration: Pack and Test on Your Laptop

One of PincherOS's superpowers: you can train on one device and run on another. Let's pack the Pi agent and test it on your laptop:

```bash
# On the Pi
pincher pack pi-home-agent.nail
scp pi-home-agent.nail my-laptop:~/
```

```bash
# On your laptop
pincher unpack pi-home-agent.nail
pincher status
```

The laptop will show a different shell fingerprint but the same reflexes. When you run `pincher reflexes`, all 6 smart home reflexes will be there. However, `homebridge.local` might not resolve from your laptop's network — shell-specific reflexes get flagged for re-compilation if the action fails.

The compatibility scoring is visible in `pincher shell-info`:

```
Shell Fingerprint:
  Hostname:     my-laptop
  CPU cores:    8
  RAM:          32 GB
  OS:           linux

Migration Compatibility:
  Source shell: homebridge (Pi 4, 4GB)
  Compatibility: 0.78 (PROBABLE)
  Flagged reflexes: 0 (all actions are portable curl commands)
  Re-compilation needed: 0
```

When you're done testing, pack it back and deploy to the Pi:

```bash
# On the laptop (after testing and adding new reflexes)
pincher pack updated-agent.nail
scp updated-agent.nail pi@homebridge.local:~/

# On the Pi
pincher unpack updated-agent.nail
```

The crab migrated. Same reflexes. Bigger shell (temporarily).

---

## Quick Reference

| Command | What It Does |
|---|---|
| `pincher status` | Check Pi resources and reflex count |
| `pincher teach -i "..." -a "..."` | Teach a new smart home reflex |
| `pincher do "turn on kitchen lights"` | Execute via reflex (or LLM fallback) |
| `pincher match "kitchen lights on"` | Preview what would match |
| `pincher reflexes` | List all smart home reflexes |
| `pincher bench` | Benchmark reflex speed on Pi hardware |
| `pincher pack agent.nail` | Pack for migration |
| `pincher unpack agent.nail` | Unpack on another device |

---

## Next Steps

- **[Hello Reflex](../hello-reflex/)** — The 5-minute basics tutorial
- **[Code Review Assistant](../code-review/)** — Reflexes for developer tooling
- **[Deploy Agent](../deploy-agent/)** — Full train-then-deploy workflow
- **[Migration Demo](../migration-demo/)** — Deep dive into .nail migration
