# PincherOS Protocol Reference

This document specifies all wire protocols, file formats, and schemas used by PincherOS. An integrating agent must implement the appropriate protocol(s) based on the chosen integration pattern (see [INTEGRATION.md § Integration Patterns](./INTEGRATION.md#integration-patterns)).

---

## JSON-RPC 2.0 over TCP

### Transport

| Property | Value |
|---|---|
| Protocol | TCP |
| Default port | 9821 |
| Framing | Newline-delimited JSON (`\n` = 0x0A) |
| Encoding | UTF-8 |
| Max request size | 1 MiB (1,048,576 bytes) |
| Connection limit | 128 concurrent connections |
| Keep-alive | Yes (connections persist until closed by client or server shutdown) |
| TLS | Not supported in v0.1.0; use SSH tunnel or reverse proxy for encryption |

### Request Format

Per [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification):

```json
{
  "jsonrpc": "2.0",
  "method": "pincher.<method>",
  "params": { ... },
  "id": 1
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `jsonrpc` | string | Yes | Must be `"2.0"` |
| `method` | string | Yes | Full method name including namespace |
| `params` | object \| array | No | Method parameters. Object preferred. |
| `id` | int32 \| string \| null | No | Request identifier. Omit for notifications (no response expected). |

### Response Format (Success)

```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `jsonrpc` | string | Yes | Always `"2.0"` |
| `result` | any | Yes (on success) | Method return value |
| `id` | int32 \| string \| null | Yes | Matches request `id` |

### Response Format (Error)

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": { ... }
  },
  "id": 1
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `error.code` | int32 | Yes | Error code |
| `error.message` | string | Yes | Human-readable error description |
| `error.data` | any | No | Additional error context |

### Error Codes

#### Standard JSON-RPC 2.0 Error Codes

| Code | Message | Meaning |
|---|---|---|
| -32700 | Parse error | Invalid JSON received. |
| -32600 | Invalid Request | JSON is not a valid JSON-RPC 2.0 request. |
| -32601 | Method not found | Method does not exist or is not available. |
| -32602 | Invalid params | Invalid method parameter(s). |
| -32603 | Internal error | Internal JSON-RPC error. |

#### PincherOS Application Error Codes

| Code | Message | Meaning |
|---|---|---|
| -32001 | Database error | SQLite operation failed. `data` contains `sqlite_code` and `sqlite_message`. |
| -32002 | Reflex not found | Specified `reflex_id` does not exist. |
| -32003 | Match threshold not met | No reflex matched above the specified threshold. |
| -32004 | Execution failed | Reflex action execution returned non-zero exit code. `data` contains `exit_code`, `stdout`, `stderr`. |
| -32005 | Pack/unpack error | .nail file operation failed. `data` contains `phase` and `detail`. |
| -32006 | Migration error | Migration workflow failed. `data` contains `migration_state` and `detail`. |
| -32007 | Resource critical | Operation refused due to critical resource state. Retry after resources recover. |
| -32008 | Capability vetoed | Operation blocked by veto rule. `data` contains `veto_rule` and `capability`. |
| -32009 | Capability denied | Capability not granted. `data` contains `capability`. |
| -32010 | Migration in progress | Cannot start migration; another is already active. |
| -32011 | Invalid state transition | Requested transition is not valid from current state. `data` contains `current_state` and `requested_transition`. |
| -32012 | Embedding backend mismatch | Cannot match across different embedding backends. Regenerate embeddings. |

### Batch Requests

JSON-RPC 2.0 batch requests are supported. Send an array of request objects:

```json
[
  {"jsonrpc":"2.0","method":"pincher.status","params":{},"id":1},
  {"jsonrpc":"2.0","method":"pincher.list","params":{"limit":10},"id":2}
]
```

Response is an array of response objects in the same order. Max batch size: 32 requests.

---

## JSON-RPC 2.0 over UDS (Python Sidecar)

### Transport

| Property | Value |
|---|---|
| Protocol | Unix Domain Socket |
| Default path | `/tmp/pincher-sidecar.sock` |
| Framing | Newline-delimited JSON (`\n` = 0x0A) |
| Encoding | UTF-8 |
| Max request size | 4 MiB (4,194,304 bytes) — larger for embedding vectors |
| Connection limit | 1 concurrent connection (PincherOS is the only client) |
| Timeout | 30 seconds per request |

### Message Format

Identical to TCP JSON-RPC 2.0 format (see above). Method namespace is `sidecar.*`.

### Sidecar-Specific Error Codes

| Code | Message | Meaning |
|---|---|---|
| -32100 | Model not loaded | ONNX model failed to load. Falling back to hash-based embeddings. |
| -32101 | Inference error | ONNX inference failed. `data` contains `model_error`. |
| -32102 | Input too long | Input text exceeds model's maximum token limit. `data` contains `max_tokens` and `input_tokens`. |
| -32103 | Sidecar not running | Python sidecar process is not running or not reachable. |

### Sidecar Lifecycle

1. PincherOS starts Python sidecar process when `serve` is called with `--sidecar-socket`.
2. PincherOS connects to UDS.
3. PincherOS sends `sidecar.health` to verify.
4. If `status: "down"`, PincherOS retries 3 times with 2-second intervals.
5. If all retries fail, PincherOS logs error and proceeds without sidecar.
6. PincherOS monitors sidecar health every 30 seconds.
7. On sidecar crash, PincherOS attempts restart (max 3 restarts in 10 minutes).

---

## .nail File Format

### Overview

The `.nail` file is the migration unit for PincherOS reflexes. It contains a manifest, reflex data, embeddings, and checksums.

### Binary Layout

```
┌──────────────────────────────────────┐
│ Header (64 bytes)                    │
│   Magic:    0x50 0x4E 0x41 0x49     │  "PNAI"
│   Version:  uint16 LE               │  Currently 1
│   Flags:    uint16 LE               │  Bit flags (see below)
│   Reflexes: uint32 LE               │  Number of reflex entries
│   Manifest: uint64 LE               │  Byte offset of manifest section
│   Data:     uint64 LE               │  Byte offset of data section
│   Embeds:   uint64 LE               │  Byte offset of embedding section
│   Footer:   uint64 LE               │  Byte offset of footer section
│   Reserved: 28 bytes (zero)         │
├──────────────────────────────────────┤
│ Data Section                         │
│   [Reflex Entry 1]                  │
│   [Reflex Entry 2]                  │
│   ...                               │
│   [Reflex Entry N]                  │
├──────────────────────────────────────┤
│ Embedding Section                    │
│   [Embedding Entry 1]               │
│   [Embedding Entry 2]               │
│   ...                               │
│   [Embedding Entry N]               │
├──────────────────────────────────────┤
│ Manifest Section (JSON)              │
│   UTF-8 encoded JSON object          │
│   (see Manifest Schema below)        │
├──────────────────────────────────────┤
│ Footer (64 bytes)                    │
│   Data CRC32:    uint32 LE           │
│   Embed CRC32:   uint32 LE           │
│   Manifest CRC32:uint32 LE           │
│   Global SHA256: 32 bytes            │
│   Reserved:      20 bytes (zero)     │
└──────────────────────────────────────┘
```

### Header Flags

| Bit | Mask | Name | Description |
|---|---|---|---|
| 0 | 0x0001 | `FINGERPRINT` | Device fingerprint included in manifest. |
| 1 | 0x0002 | `ENCRYPTED` | Data section is encrypted (not implemented in v0.1.0). |
| 2 | 0x0004 | `COMPRESSED` | Data section is zstd-compressed. |
| 3–15 | — | Reserved | Must be zero. |

### Reflex Entry Layout

```
┌─────────────────────────────────┐
│ ID length:   uint16 LE          │  Length of reflex_id string
│ ID:          UTF-8 bytes        │  Variable length
│ Pattern len: uint16 LE          │
│ Pattern:     UTF-8 bytes        │
│ Action len:  uint16 LE          │
│ Action:      UTF-8 bytes        │
│ State:       uint8              │  0=created, 1=active, 2=confirmed, 3=dormant, 4=recompiled
│ Confidence:  float64 LE         │
│ Tags count:  uint16 LE          │
│ Tags:        [uint16 LE len + UTF-8 bytes] * count
│ Created at:  int64 LE           │  Unix timestamp ms
│ Updated at:  int64 LE           │  Unix timestamp ms
│ Matched:     uint32 LE          │  Match count
│ Confirmed:   uint32 LE          │  Confirm count
└─────────────────────────────────┘
```

### Embedding Entry Layout

```
┌─────────────────────────────────┐
│ Reflex ID length: uint16 LE     │  Must match a reflex entry ID
│ Reflex ID:        UTF-8 bytes   │
│ Backend:          uint8         │  0=hash, 1=onnx
│ Dimensions:       uint16 LE     │  128 (hash) or 384 (onnx)
│ Values:           float32 LE[]  │  dimensions * 4 bytes
└─────────────────────────────────┘
```

### Checksum Verification

Verification is performed in this order:

1. **Section CRC32**: Verify `Data CRC32` against data section, `Embed CRC32` against embedding section, `Manifest CRC32` against manifest section.
2. **Global SHA256**: Compute SHA-256 over the entire file excluding the `Global SHA256` field (bytes 0 to footer_offset+12). Compare against stored `Global SHA256`.
3. If any check fails, the .nail file is corrupt. Return error code -32005 with `data: { phase: "verify", detail: "checksum_mismatch" }`.

### Manifest Schema

The manifest is a JSON object embedded in the manifest section:

```json
{
  "$schema": "https://pincher.os/schema/nail-manifest-v1.json",
  "version": 1,
  "created_at": "2025-01-15T10:30:00Z",
  "source_device": {
    "hostname": "string",
    "os": "string",
    "arch": "string",
    "fingerprint": "string (SHA-256 of hardware identifiers, or null)"
  },
  "pincher_version": "0.1.0",
  "embedding_backend": "onnx | hash",
  "embedding_dimensions": 384,
  "reflex_count": 42,
  "capabilities": {
    "network": true,
    "filesystem_read": true,
    "filesystem_write": false,
    "subprocess": true,
    "gpu": false
  },
  "tags": ["production", "web-server"],
  "compatibility_notes": "string (optional)"
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `version` | uint32 | Yes | Manifest format version. Currently 1. |
| `created_at` | string (ISO 8601) | Yes | Timestamp of .nail file creation. |
| `source_device.hostname` | string | Yes | Hostname of source device. |
| `source_device.os` | string | Yes | OS of source device. |
| `source_device.arch` | string | Yes | Architecture of source device (e.g., `x86_64`, `aarch64`). |
| `source_device.fingerprint` | string \| null | Yes | Device fingerprint. `null` if `FINGERPRINT` flag not set. |
| `pincher_version` | string (semver) | Yes | PincherOS version that created the file. |
| `embedding_backend` | string | Yes | `"onnx"` or `"hash"`. Must match target backend for import. |
| `embedding_dimensions` | uint32 | Yes | Dimensionality of embeddings in file. |
| `reflex_count` | uint32 | Yes | Number of reflex entries. Must match header. |
| `capabilities` | object | Yes | Capability requirements for this reflex set. See [CAPABILITIES.md](./CAPABILITIES.md). |
| `tags` | string[] | Yes | User-defined tags for categorization. May be empty. |
| `compatibility_notes` | string | No | Free-text notes about compatibility. |

---

## Capability Manifest Schema

The capability manifest declares what a .nail file (or a running PincherOS instance) requires and what it is permitted to do.

### Full Schema

```json
{
  "$schema": "https://pincher.os/schema/capability-manifest-v1.json",
  "version": 1,
  "declared_capabilities": {
    "network": {
      "granted": true,
      "constraints": {
        "allowed_hosts": ["api.example.com"],
        "allowed_ports": [443],
        "max_connections": 10
      }
    },
    "filesystem_read": {
      "granted": true,
      "constraints": {
        "allowed_paths": ["/var/log", "/etc/pincher"],
        "max_file_size_mb": 100
      }
    },
    "filesystem_write": {
      "granted": false,
      "constraints": {}
    },
    "subprocess": {
      "granted": true,
      "constraints": {
        "allowed_commands": ["rm", "systemctl", "curl"],
        "max_runtime_seconds": 30,
        "allow_shell": false
      }
    },
    "gpu": {
      "granted": false,
      "constraints": {}
    }
  },
  "veto_rules_ref": "path/to/veto.toml"
}
```

### Capability Field Specification

| Field | Type | Required | Description |
|---|---|---|---|
| `version` | uint32 | Yes | Schema version. Currently 1. |
| `declared_capabilities` | object | Yes | Map of capability name → capability grant. |
| `declared_capabilities.<name>.granted` | bool | Yes | Whether this capability is granted. |
| `declared_capabilities.<name>.constraints` | object | Yes | Constraints on the capability. Empty object `{}` if unconstrained or not granted. |
| `veto_rules_ref` | string | No | Path to veto rules file. Relative to config directory. |

### Constraint Objects by Capability

#### `network` constraints

| Field | Type | Default | Description |
|---|---|---|---|
| `allowed_hosts` | string[] | `[]` (all allowed) | Whitelist of hostnames. Empty array = all hosts allowed. |
| `allowed_ports` | uint16[] | `[]` (all allowed) | Whitelist of ports. Empty array = all ports allowed. |
| `max_connections` | uint32 | `4294967295` | Maximum concurrent network connections. |

#### `filesystem_read` constraints

| Field | Type | Default | Description |
|---|---|---|---|
| `allowed_paths` | string[] | `[]` (all allowed) | Whitelist of readable paths. Empty array = all paths readable. |
| `max_file_size_mb` | uint32 | `1024` | Maximum single file size readable. |

#### `filesystem_write` constraints

| Field | Type | Default | Description |
|---|---|---|---|
| `allowed_paths` | string[] | `[]` (none allowed) | Whitelist of writable paths. Empty array = no paths writable. |
| `max_file_size_mb` | uint32 | `100` | Maximum single file size writable. |
| `allow_delete` | bool | `false` | Whether delete operations are permitted. |

#### `subprocess` constraints

| Field | Type | Default | Description |
|---|---|---|---|
| `allowed_commands` | string[] | `[]` (none allowed) | Whitelist of command names. Empty array = no commands allowed. |
| `max_runtime_seconds` | uint32 | `30` | Maximum execution time per subprocess. |
| `allow_shell` | bool | `false` | Whether shell expansion (pipes, redirects) is permitted. |

#### `gpu` constraints

| Field | Type | Default | Description |
|---|---|---|---|
| `max_memory_mb` | uint32 | `512` | Maximum GPU memory allocation. |
| `allowed_operations` | string[] | `["inference"]` | Allowed GPU operation types. |

### Extending Capabilities

To add a custom capability:

1. Define the capability name (must be lowercase_snake, 3–64 chars, `^[a-z][a-z0-9_]{2,63}$`).
2. Add it to `declared_capabilities` with `granted` and `constraints`.
3. Implement the enforcement in the veto engine or application layer.
4. Unknown capabilities are treated as `granted: false` by default.

---

## Veto Rule Schema

Veto rules define allow/deny policies that are evaluated before reflex execution. They override capability grants: a veto can deny an operation even if the capability is granted.

### File Format

Veto rules are stored in TOML:

```toml
[[rule]]
name = "block_remote_shell"
type = "command_pattern"
pattern = "^(ssh|telnet|nc)\\s"
action = "deny"
priority = 100

[[rule]]
name = "allow_systemctl_read"
type = "command_pattern"
pattern = "^systemctl\\s+status\\s"
action = "allow"
priority = 50

[[rule]]
name = "block_write_etc"
type = "path_pattern"
pattern = "^/etc/"
capability = "filesystem_write"
action = "deny"
priority = 200

[[rule]]
name = "block_internal_network"
type = "host_pattern"
pattern = "^(10\\.|172\\.(1[6-9]|2[0-9]|3[01])\\.|192\\.168\\.)"
capability = "network"
action = "deny"
priority = 150
```

### Rule Schema

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Unique rule identifier. Must match `^[a-z][a-z0-9_-]{2,63}$`. |
| `type` | string | Yes | Rule type. One of: `command_pattern`, `path_pattern`, `host_pattern`, `capability`, `tag`. |
| `pattern` | string | Yes (for pattern types) | Regular expression (Rust `regex` crate syntax). |
| `capability` | string | No | Capability this rule applies to. Required for `path_pattern` and `host_pattern` types. |
| `action` | string | Yes | `"allow"` or `"deny"`. |
| `priority` | uint32 | Yes | Evaluation order. Higher priority = evaluated first. Ties broken by rule order in file. |

### Rule Types

| Type | Evaluates Against | Required Fields | Description |
|---|---|---|---|
| `command_pattern` | Reflex action string | `pattern` | Matches the shell command of a reflex action. |
| `path_pattern` | File paths in reflex action | `pattern`, `capability` | Matches filesystem paths for read/write operations. |
| `host_pattern` | Hostnames in reflex action | `pattern`, `capability` | Matches network hostnames. |
| `capability` | Capability name | `capability` | Blocks or allows entire capability regardless of constraints. |
| `tag` | Reflex tags | `pattern` | Matches reflex tags. |

### Evaluation Algorithm

```
1. Collect all applicable rules for the operation.
2. Sort by priority (descending), then by file order.
3. Evaluate each rule in order:
   a. If rule matches and action = "deny" → VETO (operation blocked, error code -32008)
   b. If rule matches and action = "allow" → SKIP (operation proceeds, no further rules evaluated)
   c. If rule does not match → continue to next rule
4. If no rule matches → fall through to capability grant check.
```

### Veto Result

When a veto is triggered, the RPC response includes:

```json
{
  "error": {
    "code": -32008,
    "message": "Capability vetoed",
    "data": {
      "veto_rule": "block_remote_shell",
      "capability": "subprocess",
      "pattern_matched": "ssh user@host",
      "action": "deny"
    }
  }
}
```
