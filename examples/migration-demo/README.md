# Migration Demo — One Crab, Many Shells

This walkthrough demonstrates PincherOS migration without needing a second physical device. You'll simulate two "shells" using two separate database paths, pack an agent on Shell A, and unpack it on Shell B. You'll see exactly what moves, how compatibility scoring works, and what the `.nail` file contains.

---

## The Concept

A hermit crab outgrows its shell and finds a new one. The crab doesn't change — it just moves. PincherOS does the same thing for AI agents. Your agent's learned reflexes, confidence scores, and identity all live in a portable `.nail` file. When you unpack on a new device, the agent re-adapts to the new shell without losing anything it learned.

The QTR (Quiesce-Transfer-Resume) protocol ensures zero state loss:

1. **Quiesce** — Finish any in-flight execution, flush all writes to SQLite
2. **Transfer** — Create the `.nail` file with BLAKE3 checksums for every component
3. **Resume** — On the new shell, unpack, re-fingerprint, adapt shell-specific reflexes

---

## Step 1: Train Agent on "Shell A"

We'll use two separate database paths to simulate two devices:

```bash
# Shell A: the workstation
export PINCHER_DB_A="/tmp/pincher-shell-a/reflexes.db"
mkdir -p /tmp/pincher-shell-a

# Teach some reflexes
pincher teach --intent "list running processes" --action "ps aux"
pincher teach --intent "check memory usage" --action "free -h"
pincher teach --intent "show disk space" --action "df -h"
pincher teach --intent "check network connections" --action "ss -tlnp"
pincher teach --intent "view recent logs" --action "journalctl --since '10 min ago' --no-pager"
pincher teach --intent "find large files" --action "du -sh /* | sort -rh | head -10"
pincher teach --intent "check docker status" --action "docker ps -a"
pincher teach --intent "git status" --action "git status --short"
```

Verify what the agent knows:

```bash
pincher reflexes
```

You should see 8 reflexes, all with initial confidence around 0.50.

---

## Step 2: Pack into a .nail File

```bash
pincher pack /tmp/agent-migration.nail
```

Output:

```
✓ Packed 8 reflexes into /tmp/agent-migration.nail
  Size:       ~45 KB (compressed with zstd)
  Checksum:   blake3:a7f3b2c1d4e5f6...
  Source:     your-hostname
```

The `.nail` file is small — typically under 100KB for a moderate reflex set. It's designed to be small enough to transfer over slow connections (SSH to a Pi, etc.).

---

## Step 3: Inspect the .nail File

The `.nail` file is a `tar.zst` archive. You can inspect it:

```bash
# Using the inspect script (if available)
./inspect-nail.sh /tmp/agent-migration.nail

# Or manually:
cp /tmp/agent-migration.nail /tmp/inspect.nail.tar.zst
zstd -d /tmp/inspect.nail.tar.zst -o /tmp/inspect.nail.tar
tar -tf /tmp/inspect.nail.tar
```

You'll see the contents:

```
manifest.json
reflexes.db
identity.json
config.toml
```

Let's look at `manifest.json`:

```bash
tar -xf /tmp/inspect.nail.tar -C /tmp/nail-inspect manifest.json
cat /tmp/nail-inspect/manifest.json
```

Example output:

```json
{
  "version": 1,
  "created_at": "2025-01-15T14:30:00Z",
  "source_device": {
    "hostname": "my-workstation",
    "os": "linux",
    "arch": "x86_64",
    "fingerprint": "blake3:abc123..."
  },
  "pincher_version": "0.1.0",
  "embedding_backend": "hash",
  "embedding_dimensions": 256,
  "reflex_count": 8
}
```

And `identity.json`:

```json
{
  "agent_name": "default",
  "preferences": {
    "output_format": "text",
    "confirm_threshold": 0.70
  }
}
```

---

## Step 4: Unpack into "Shell B"

Simulate a different device by using a different database path:

```bash
# Shell B: the "target device"
export PINCHER_DB_B="/tmp/pincher-shell-b/reflexes.db"
mkdir -p /tmp/pincher-shell-b

# Unpack the .nail file
pincher unpack /tmp/agent-migration.nail
```

Output:

```
✓ Unpacked 8 reflexes from /tmp/agent-migration.nail
  Checksum verified: blake3:a7f3b2c1d4e5f6... ✓

  Source shell:  my-workstation (x86_64, linux)
  Target shell:  my-workstation (x86_64, linux)
  Compatibility: 1.00 (EXACT — same device)

  Reflexes imported: 8
  Flagged for re-compilation: 0
```

Since we're on the same machine, compatibility is 1.00. In a real migration (Pi → laptop), the compatibility would be lower and shell-specific reflexes might be flagged.

---

## Step 5: Verify the Reflexes Carried Over

```bash
pincher reflexes
```

You should see the same 8 reflexes, with the same confidence scores. The agent's "muscle memory" is intact.

Test a match:

```bash
pincher do "how much RAM is free"
# → Matches "check memory usage" (confidence 0.50)
```

The reflex works on Shell B exactly as it did on Shell A.

---

## Step 6: Observe Compatibility Scoring

In a real migration between different hardware, the compatibility score tells you how well the agent's reflexes will work on the new shell. Let's understand what affects it:

The hardware fingerprint includes:

| Component | What's Collected | Affects Compatibility? |
|---|---|---|
| Hostname | Machine's hostname | No (informational) |
| OS | Linux, macOS, etc. | Yes (different OS = different commands) |
| Architecture | x86_64, aarch64 | Yes (different binaries) |
| CPU cores | Number of cores | No (performance only) |
| RAM | Total RAM in MB | No (affects resource state) |
| GPU | GPU model or "none" | No (affects gpu capability) |
| MAC hash | BLAKE3 hash of first MAC | Yes (network identity) |

**Compatibility scoring**:

```
score = 1.0
if source.os != target.os:     score -= 0.3
if source.arch != target.arch: score -= 0.3
if source.gpu != target.gpu:   score -= 0.1
score = max(0.0, score)
```

| Score Range | Classification | Behavior |
|---|---|---|
| 0.90 – 1.00 | EXACT | All reflexes imported, no adaptation needed |
| 0.70 – 0.89 | PROBABLE | Reflexes imported, shell-specific ones flagged for re-compilation |
| 0.50 – 0.69 | POSSIBLE | Only OS-independent reflexes imported, rest flagged |
| < 0.50 | UNLIKELY | Import blocked, manual review required |

---

## Step 7: Shell-Specific Reflex Re-compilation

When a reflex depends on tools that might not exist on the new shell, it gets flagged for re-compilation. For example:

- On Ubuntu: `apt list --installed` works
- On macOS: `brew list` is the equivalent
- On Alpine: `apk info` is the equivalent

If you pack an agent on Ubuntu and unpack on macOS, any reflex using `apt` would be flagged. PincherOS detects this by checking if the command in the reflex action exists on the target shell.

When a reflex is flagged:
1. It's still imported with its original action
2. Its confidence is reset to 0.30 (below the execution threshold)
3. The next time the intent is matched, the LLM re-compiles the reflex with the correct command for the new shell
4. The reflex's confidence then climbs normally from 0.30

This is the self-healing mechanism — the agent adapts to a new shell automatically.

---

## Real-World: Pi → Laptop → Cloud VM

Here's how a real three-device migration would look:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Raspberry Pi 4  │     │    Laptop        │     │   Cloud VM       │
│  aarch64, 4GB    │────▶│  x86_64, 16GB   │────▶│  x86_64, 4GB    │
│  Linux           │     │  macOS           │     │  Linux           │
│                  │     │                  │     │                  │
│  Compatibility:  │     │  Compatibility:  │     │  Compatibility:  │
│  (source)        │     │  0.70 (PROBABLE) │     │  0.90 (EXACT)   │
│                  │     │  2 reflexes      │     │  0 flagged      │
│                  │     │  flagged         │     │                  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                        │                       │
   agent.nail              agent.nail              agent.nail
   (45 KB)                 (47 KB)                 (46 KB)
```

The Pi→Laptop migration flags macOS-incompatible reflexes (like `apt` and `systemctl`). The Laptop→Cloud migration is smoother because both run Linux.

---

## Simulate with the Script

```bash
chmod +x simulate-migration.sh
./simulate-migration.sh
```

This script automates the entire walkthrough above, including cleanup.

---

## Next Steps

- **[Hello Reflex](../hello-reflex/)** — The 5-minute basics tutorial
- **[Deploy Agent](../deploy-agent/)** — Train on workstation, deploy to production
- **[Smart Home Controller](../smart-home/)** — Reflexes for home automation on a Pi
