# 🦀 pincher — TUTORIALS

## The Hacker's la-link: From 0 to Your First Reflex in 15 Minutes

---

### 🎯 Tutorial 1: "I want to teach my first reflex"

**Goal:** Teach pincher how to respond to "list files" and watch it learn.

**Time:** 5 minutes

```bash
# 1. Check if pincher is alive
pincher status

# 2. Teach a new reflex interactively
pincher teach

# When prompted:
# Intent: "list files in current directory"
# Action: "ls -la"
# Category: "filesystem"

# 3. Now fire the reflex — pincher matches the intent
pincher do "what files are here"

# 4. See it learned
pincher reflexes
```

**What happened:**
1. `pincher teach` stored `"list files in current directory" → "ls -la"` in the reflex database
2. The intent was embedded as a 384-dimensional vector
3. `pincher do "what files are here"` matched at >0.80 confidence
4. The reflex fired in <50ms — no LLM involved

**If it doesn't match yet:**
```bash
# Low confidence? Confirm manually
pincher do --confirm "what files are here"

# Or lower the threshold temporarily
pincher do --threshold 0.55 "what files are here"
```

---

### 🎯 Tutorial 2: "I want to chain reflexes together"

**Goal:** Create a workflow that lists files, counts them, and reports.

**Time:** 10 minutes

```bash
# Teach the building blocks
pincher teach "list files" "ls -la"
pincher teach "count items" "ls | wc -l"
pincher teach "disk usage" "df -h ."

# Now combine them in a script
cat > inventory.sh << 'SCRIPT'
echo "=== Files ==="
ls -la
echo "=== Count ==="
ls | wc -l
echo "=== Disk ==="
df -h .
SCRIPT

chmod +x inventory.sh

# Teach the compound reflex
pincher teach "run inventory check" "./inventory.sh"

# Fire it
pincher do "run inventory check"
```

Or using pincher's built-in compound intent matching:

```bash
# pincher can match multi-intent strings
pincher do "list files and check disk usage"
# This fires both reflexes in sequence if they exist
```

---

### 🎯 Tutorial 3: "I want to pack an agent and move it"

**Goal:** Create a `.nail` bundle and unpack it on another machine.

**Time:** 5 minutes

```bash
# On machine A — train some reflexes
pincher teach "ping google" "ping -c 4 google.com"
pincher teach "check memory" "free -h"
pincher teach "check uptime" "uptime"

# Pack everything into a .nail
pincher pack --output my-agent.nail

# Copy to machine B
scp my-agent.nail user@machine-b:~/

# On machine B — load the agent
pincher unpack my-agent.nail

# The agent knows everything it learned on machine A
pincher do "ping google"
pincher do "how long has the system been up"
```

**Inspect the nail:**
```bash
# Peel open the onion
pincher pack --inspect my-agent.nail

# Output:
# 📦 my-agent.nail
# ├── manifest.json       # Version, checksums, hardware fingerprint
# ├── reflexes.db         # Full vector database
# ├── identity.json       # Agent name, preferences
# └── config.toml         # Resource limits, thresholds
```

---

### 🎯 Tutorial 4: "I want to see what pincher is doing in real-time"

**Goal:** Watch the reflex engine live.

**Time:** 5 minutes

```bash
# Start the RPC server
pincher daemon

# In another terminal, watch the log
tail -f ~/.pincher/pincher.log

# Send intents programmatically
pincher do "list files"
pincher do "check memory"
pincher do "what time is it"

# See each reflex fire with match score and execution time
# [REFLEX] Match: list-files (0.92_conf) — 47ms
# [REFLEX] Match: check-memory (0.88_conf) — 52ms
# [REFLEX] TEACH: time-query (0.35_conf) — LLM compile — 3.2s
```

**Real-time dashboard:**
```bash
# Watch the confidence grow
watch -n 2 'pincher reflexes | sort -k3 -t"|" -rn'
```

---

### 🎯 Tutorial 5: "I want to create a custom capability token"

**Goal:** Restrict an agent to only run file-reading commands.

**Time:** 10 minutes

```bash
# 1. Create a capability manifest
cat > read-only.toml << 'EOF'
[capability]
name = "read-only-agent"
version = "1.0"

[permissions]
commands = ["ls", "cat", "head", "tail", "grep", "find", "wc", "echo"]
files = ["read"]
network = false
system = false

[limits]
max_executions = 100
max_memory_mb = 128
EOF

# 2. Apply the capability token
pincher apply-capability read-only.toml

# 3. Now test — allowed
pincher do "list files"           # ✅ ls — allowed
pincher do "check memory"          # ❌ free -h — blocked by veto

# 4. See the veto log
pincher doctor
# VETO: Command 'free' not in capability whitelist
```

---

### 🎯 Tutorial 6: "I want to use pincher from Python"

**Goal:** Drive pincher programmatically.

**Time:** 5 minutes

```python
import subprocess
import json

def pincher_do(intent: str) -> dict:
    result = subprocess.run(
        ["pincher", "do", "--json", intent],
        capture_output=True, text=True
    )
    return json.loads(result.stdout)

def pincher_teach(intent: str, action: str):
    subprocess.run(
        ["pincher", "teach", intent, action],
        check=True
    )

# Use it
pincher_teach("list files", "ls -la")
response = pincher_do("what's in this folder")
print(f"Executed: {response['action']}")
print(f"Confidence: {response['confidence']}")
print(f"Time: {response['elapsed_ms']}ms")
```

---

### 🎯 Tutorial 7: "Benchmark my reflex database"

**Goal:** Stress-test pincher's matching performance.

**Time:** 5 minutes

```bash
# Teach 100 random reflexes
for i in $(seq 1 100); do
    pincher teach "query-$i" "echo reflex-$i"
done

# Benchmark
pincher bench --iterations 1000

# Sample output:
# Embedding latency: 1.2ms avg
# Match latency:     0.8ms avg (100-reflex DB)
# Execute latency:   0.5ms avg
# Total pipeline:    2.5ms avg
#
# Throughput: ~400 intentions/second

# Test with fuzzy matching
pincher bench --fuzzy "what is query 50" --iterations 100
```

---

## 🔬 Quick Reference

| Tutorial | Skill | Commands | Time |
|----------|-------|----------|------|
| 1 | First reflex | `teach`, `do`, `reflexes` | 5 min |
| 2 | Chain reflexes | Scripts, multi-intent | 10 min |
| 3 | Agent portability | `pack`, `unpack` | 5 min |
| 4 | Real-time monitoring | `daemon`, logs | 5 min |
| 5 | Security tokens | `apply-capability` | 10 min |
| 6 | Python integration | `pincher do --json` | 5 min |
| 7 | Benchmarking | `bench` | 5 min |

---

## 🏆 From Here

- ➡️ [ONBOARDING.md](./TEMPLATES/ONBOARDING.md) — Day 1 → Day 5 plan
- ➡️ [README.md](./README.md) — Full docs & philosophy
- ➡️ [API_REFERENCE.md](./API_REFERENCE.md) — All API details
- ➡️ [ARCHITECTURE.md](./ARCHITECTURE.md) — System design
- ➡️ [GETTING_STARTED.md](./GETTING_STARTED.md) — First-time setup
- ➡️ [CONTRIBUTING.md](./CONTRIBUTING.md) — How to contribute
- ➡️ [examples/](./examples/) — Full example projects
