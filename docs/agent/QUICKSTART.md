# PincherOS Agent Quickstart

Fastest path from zero to working integration. Every command and payload is exact — copy and run.

Cross-references: [INTEGRATION.md](./INTEGRATION.md) | [PROTOCOLS.md](./PROTOCOLS.md) | [CAPABILITIES.md](./CAPABILITIES.md)

---

## Step 1: Build PincherOS

```bash
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build --release
```

Binary location: `./target/release/pincher`

Verification:

```bash
./target/release/pincher --version
# Expected output: pincher 0.1.0
```

---

## Step 2: Initialize and Check Status

```bash
./target/release/pincher status
```

Expected output fields:

| Field | Type | Example |
|---|---|---|
| `version` | string | `"0.1.0"` |
| `hostname` | string | `"my-machine"` |
| `os` | string | `"linux"` |
| `arch` | string | `"x86_64"` |
| `cpu_cores` | uint32 | `8` |
| `total_ram_gb` | float64 | `31.45` |
| `ram_usage_percent` | float64 | `34.2` |
| `reflex_count` | uint32 | `0` |

If `reflex_count` > 0, a database already exists at `~/.pincher/reflexes.db`.

---

## Step 3: Teach Your First Reflex Programmatically

### Via CLI

```bash
./target/release/pincher teach --intent "check disk space" --action "df -h"
```

### Via JSON-RPC

First, start the RPC server:

```bash
./target/release/pincher rpc --port 9876 &
```

Then send a request:

```bash
curl -s -X POST http://localhost:9876 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "teach",
    "params": {"intent": "check disk space", "action": "df -h"},
    "id": 1
  }'
```

Expected response:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "id": "a1b2c3d4",
    "intent": "check disk space",
    "action": "df -h",
    "confidence": 0.5,
    "created_at": "2025-01-15T10:30:00Z"
  },
  "id": 1
}
```

---

## Step 4: Execute via JSON-RPC

```bash
curl -s -X POST http://localhost:9876 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "do",
    "params": {"intent": "how much disk is free"},
    "id": 2
  }'
```

Expected response (if match found):

```json
{
  "jsonrpc": "2.0",
  "result": {
    "executed": true,
    "action": "df -h"
  },
  "id": 2
}
```

### Available RPC Methods

| Method | Params | Description |
|---|---|---|
| `status` | `{}` | System status |
| `teach` | `{"intent": string, "action": string}` | Create reflex |
| `match` | `{"intent": string}` | Dry-run match |
| `do` | `{"intent": string}` | Match and execute |
| `list` | `{}` | List all reflexes |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Missing 'intent' param"
  },
  "id": 2
}
```

| Error Code | Meaning |
|---|---|
| `-32700` | Parse error (invalid JSON) |
| `-32600` | Invalid request |
| `-32601` | Method not found |
| `-32602` | Invalid params |
| `-32603` | Internal error |

---

## Step 5: Pack and Migrate

### Pack

```bash
./target/release/pincher pack --output my-agent.nail
```

The `.nail` file is a `tar.zst` archive containing:
- `manifest.json` — version, source fingerprint, timestamp, checksums
- `reflexes.db` — SQLite database with all reflexes and embeddings
- `identity.json` — agent name, preferences
- `config.toml` — configuration

### Transfer

```bash
scp my-agent.nail target-host:~/
```

### Unpack on Target

```bash
# On the target machine
./target/release/pincher unpack my-agent.nail
```

### Verify

```bash
./target/release/pincher status
# reflex_count should reflect imported reflexes
./target/release/pincher reflexes
# list all imported reflexes with confidence scores
```

---

## Common Gotchas

| Issue | Cause | Fix |
|---|---|---|
| `reflex_count` stays 0 after teach | Database path mismatch | Use `--db` flag or set `PINCHER_DB` env var |
| RPC returns "Method not found" | Wrong method name | Use exact names: `teach`, `do`, `match`, `list`, `status` |
| Unpack fails with checksum error | Corrupted .nail file | Re-pack and transfer again |
| Match returns no results | Embedding dimension mismatch | Both source and target must use same embedding backend |
| `pincher` binary not found | Not in PATH | Use full path `./target/release/pincher` |

---

## Next Steps

- Read [INTEGRATION.md](./INTEGRATION.md) for all interface details and integration patterns
- Read [STATE_MACHINE.md](./STATE_MACHINE.md) for state transitions and confidence algorithm
- Read [PROTOCOLS.md](./PROTOCOLS.md) for wire format specifications
- Read [CAPABILITIES.md](./CAPABILITIES.md) for capability model and enforcement
