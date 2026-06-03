# Deploy Agent — Train on Workstation, Deploy to Cloud

The full migration story: train your agent with full LLM access on a powerful workstation, pack it into a `.nail` file, and deploy it to a resource-constrained cloud VM where it runs in reflex-only mode. The LLM is a compiler at training time; at runtime, it's optional.

---

## The Concept

In production, you want your agent to be:

1. **Fast** — 50ms responses, not 3-second LLM round-trips
2. **Cheap** — zero API cost for known operations
3. **Resilient** — works even when the LLM is down or network is flaky
4. **Portable** — move between servers without retraining

PincherOS achieves this by separating **training** from **deployment**:

- **Training phase**: On your workstation, with plenty of RAM and fast internet. The LLM sidecar is active. You teach reflexes interactively, and the LLM compiles novel intents into new reflexes. This is where the agent learns.

- **Deployment phase**: On the target server, possibly with limited resources. The LLM may not be available (or may be in reflex-only mode due to resource constraints). The agent runs entirely on its accumulated reflexes. It's fast, cheap, and reliable.

The `.nail` file is the bridge between the two phases. It carries everything the agent learned during training.

---

## Phase 1: Train on Workstation

### Build PincherOS

```bash
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build --release
```

### Teach Deployment Reflexes

You need to teach reflexes for everything your production agent will encounter. Think of this as writing a runbook — except the agent memorizes it as muscle memory.

Here's a set of reflexes for a typical deployment operations agent:

```bash
# Health checks
pincher teach --intent "check service health" --action "curl -sf http://localhost:8080/health"
pincher teach --intent "check database connection" --action "pg_isready -h localhost -p 5432"

# Logs
pincher teach --intent "show recent logs" --action "journalctl -u myapp --since '10 minutes ago' --no-pager"
pincher teach --intent "show error logs" --action "journalctl -u myapp -p err --no-pager | tail -50"
pincher teach --intent "search logs for keyword" --action "journalctl -u myapp --no-pager | rg -i"

# Deployment
pincher teach --intent "deploy latest version" --action "cd /opt/myapp && git pull && cargo build --release && systemctl restart myapp"
pincher teach --intent "rollback to previous version" --action "cd /opt/myapp && git checkout HEAD~1 && cargo build --release && systemctl restart myapp"
pincher teach --intent "restart the service" --action "systemctl restart myapp"
pincher teach --intent "stop the service" --action "systemctl stop myapp"
pincher teach --intent "start the service" --action "systemctl start myapp"

# Monitoring
pincher teach --intent "check disk usage" --action "df -h"
pincher teach --intent "check memory usage" --action "free -h"
pincher teach --intent "check cpu load" --action "uptime"
pincher teach --intent "check running processes" --action "ps aux | head -20"
pincher teach --intent "check open ports" --action "ss -tlnp"

# Database
pincher teach --intent "list database tables" --action "psql -h localhost -c '\\dt'"
pincher teach --intent "count rows in users table" --action "psql -h localhost -c 'SELECT count(*) FROM users'"
pincher teach --intent "show active connections" --action "psql -h localhost -c 'SELECT * FROM pg_stat_activity'"

# Backups
pincher teach --intent "create database backup" --action "pg_dump -h localhost myapp > /backup/myapp-$(date +%Y%m%d).sql"
pincher teach --intent "list available backups" --action "ls -lh /backup/"
pincher teach --intent "restore from backup" --action "psql -h localhost myapp < /backup/myapp-latest.sql"

# SSL / Certificates
pincher teach --intent "check certificate expiry" --action "echo | openssl s_client -connect localhost:443 2>/dev/null | openssl x509 -noout -dates"
pincher teach --intent "renew certificates" --action "certbot renew"

# Network
pincher teach --intent "test external connectivity" --action "curl -sf https://httpbin.org/ip"
pincher teach --intent "check dns resolution" --action "dig +short myapp.example.com"
```

That's 24 reflexes. Or use the batch script:

```bash
chmod +x train.sh
./train.sh
```

### Verify Performance

```bash
pincher bench
```

On a workstation, you should see:

```
PincherOS Benchmark Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━
Embed (trigram):     0.3ms
Embed (MiniLM):      8.2ms
Match (24 reflexes):  0.4ms
Match (1000 sim):    2.4ms
Total end-to-end:     ~55ms
```

### Review What the Agent Knows

```bash
pincher reflexes
```

Scroll through all 24 reflexes and verify each one. Check that the actions are correct and the intents are natural. Fix any that look wrong:

```bash
# If a reflex needs updating, just re-teach it (duplicates are merged by embedding similarity)
pincher teach --intent "check service health" --action "curl -sf http://localhost:8080/healthz"
```

---

## Phase 2: Pack the Agent

Once you're satisfied with the reflex set, pack everything into a `.nail` file:

```bash
pincher pack production-agent.nail
```

Output:

```
✓ Packed 24 reflexes into production-agent.nail
  Size:       142 KB (compressed with zstd)
  Checksum:   blake3:7f3a2b1c4d5e6f...
  Source:     my-workstation (8 cores, 32GB RAM, linux)
  
  Contents:
    manifest.json    0.2 KB
    reflexes.db     89.4 KB
    identity.json    0.1 KB
    config.toml      0.3 KB
```

The `.nail` file is small — typically under 200KB for a well-trained agent. It's a `tar.zst` archive with BLAKE3 checksums for every component.

### Verify the Pack

Before deploying, test the pack locally:

```bash
# Test that common intents match correctly
pincher match "is the service healthy?"
# → Best match: "check service health" (similarity 0.91)

pincher match "how much disk is left"
# → Best match: "check disk usage" (similarity 0.89)

pincher match "show me the error logs"
# → Best match: "show error logs" (similarity 0.94)
```

If any match looks wrong, fix it before packing again.

---

## Phase 3: Deploy to Target

### Build PincherOS on the Target

If the target is a cloud VM, you can either build from source or cross-compile:

```bash
# Option A: Build on the VM
ssh deploy@prod-server
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS && cargo build --release

# Option B: Cross-compile on your workstation
# (faster for ARM targets, etc.)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
scp target/x86_64-unknown-linux-musl/release/pincher deploy@prod-server:~/
```

### Transfer the .nail File

```bash
scp production-agent.nail deploy@prod-server:~/
```

Or use the deploy script:

```bash
chmod +x deploy.sh
./deploy.sh deploy@prod-server production-agent.nail
```

### Unpack on the Target

```bash
# On the production server
pincher unpack production-agent.nail
```

Output:

```
✓ Unpacked 24 reflexes from production-agent.nail
  Checksum verified: blake3:7f3a2b1c4d5e6f... ✓
  
  Source shell:  my-workstation (8 cores, 32GB RAM)
  Target shell:  prod-server (2 cores, 4GB RAM)
  Compatibility: 0.72 (PROBABLE)
  
  Reflexes imported: 24
  Flagged for re-compilation: 0
  Shell-adapted: 2 (systemctl paths adjusted)
```

### Verify on the Target

```bash
pincher status
```

```
   🦀 PincherOS v0.1.0
  ╱╱╱╱╱╱╱╱╱╱╱╱╱
 ╱  Shell: prod-server     ╲
╱   Reflexes: 24            ╲
╲   State: Light            ╱
 ╲  RAM: 72.1%           ╱
  ╰───────────────────────╯
```

Note the **Light** resource state — the VM has less RAM. This means the LLM sidecar will only load for low-confidence matches (< 0.85). Most reflexes will still fire directly.

```bash
pincher bench
```

The benchmarks will be slower on the smaller VM, but reflex matching should still be under 5ms:

```
PincherOS Benchmark Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━
Embed (trigram):     0.4ms
Embed (MiniLM):      12.1ms
Match (24 reflexes):  0.6ms
Total end-to-end:     ~60ms
```

---

## Phase 4: Run in Production

### Start the RPC Server

For programmatic access (from monitoring systems, Slack bots, etc.), start the JSON-RPC server:

```bash
pincher rpc --port 9876
```

This starts a persistent server that listens for intents and returns results:

```bash
# Send an intent via JSON-RPC
curl -s -X POST http://localhost:9876 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "pincher.do",
    "params": {"intent": "check service health"},
    "id": 1
  }'
```

Response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "matched_reflex": "check service health",
    "confidence": 0.87,
    "similarity": 0.94,
    "action": "curl -sf http://localhost:8080/health",
    "output": "OK",
    "exit_code": 0,
    "duration_ms": 52,
    "confidence_delta": +0.05
  },
  "id": 1
}
```

### Reflex-Only Mode

When the production server is under heavy load (RAM > 85% or CPU > 80%), the PID controller shifts to **Critical** state. In this state:

- The LLM sidecar is **not loaded** — it would use too much RAM
- All matching is done via the fast trigram embedder (~0.4ms) + cosine similarity (~0.6ms)
- Only reflexes with confidence > 0.70 are executed directly
- Intents that don't match a known reflex return a "no match" error instead of routing to the LLM

This means your production agent **never crashes due to LLM overhead**. It degrades gracefully, handling what it knows and deferring what it doesn't.

### Updating the Agent

When you need to add new reflexes:

1. **On your workstation**: Teach the new reflexes
2. **Pack**: `pincher pack updated-agent.nail`
3. **Transfer**: `scp updated-agent.nail deploy@prod-server:~/`
4. **Unpack**: On the server, `pincher unpack updated-agent.nail`

The unpack process merges new reflexes with existing ones. Reflexes that already exist (by embedding similarity) are updated with the new action. Reflexes that are new are added. No reflexes are deleted.

This is the **pack → unpack cycle** — the deployment pattern for PincherOS agents in production.

---

## Quick Reference

| Phase | Command | What It Does |
|---|---|---|
| **Train** | `pincher teach -i "..." -a "..."` | Teach a reflex |
| | `pincher reflexes` | Review what the agent knows |
| | `pincher bench` | Verify performance |
| **Pack** | `pincher pack agent.nail` | Create portable .nail file |
| | `pincher match "..."` | Verify match quality |
| **Deploy** | `scp agent.nail server:~/` | Transfer to target |
| | `pincher unpack agent.nail` | Unpack on target |
| | `pincher status` | Verify shell and reflex count |
| | `pincher bench` | Verify performance on target hardware |
| **Run** | `pincher rpc --port 9876` | Start RPC server |
| | `pincher do "..."` | Execute an intent directly |
| **Update** | `pincher pack` → `scp` → `pincher unpack` | Add new reflexes |

---

## Next Steps

- **[Hello Reflex](../hello-reflex/)** — The 5-minute basics tutorial
- **[Smart Home Controller](../smart-home/)** — Reflexes for home automation on a Pi
- **[Migration Demo](../migration-demo/)** — Deep dive into .nail migration mechanics
