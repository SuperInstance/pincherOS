# PincherOS Capability Reference

This document specifies all capabilities PincherOS exposes, the permission model, resource constraints, enforcement chain, and manifest declaration format.

Cross-references: [INTEGRATION.md § Interface Specification](./INTEGRATION.md#interface-specification) | [PROTOCOLS.md § Capability Manifest Schema](./PROTOCOLS.md#capability-manifest-schema) | [STATE_MACHINE.md § Resource States](./STATE_MACHINE.md#resource-states)

---

## Capability Model Overview

PincherOS uses a **declare-then-enforce** capability model:

1. **Declare**: The capability manifest (TOML or JSON) lists what the agent needs.
2. **Grant**: At startup or migration, capabilities are granted or denied based on the manifest and the shell's security policy.
3. **Enforce**: Before every reflex execution, the enforcement chain checks capabilities.

```mermaid
flowchart LR
    M["Manifest<br/>(declare)"] --> G["Grant Phase<br/>(startup/migrate)"]
    G --> E["Enforce Phase<br/>(per-execution)"]
    E --> V["Veto Engine<br/>(rule check)"]
    V --> S["Sandbox<br/>(isolation)"]
    S --> X["Execute"]
```

---

## Core Capabilities

These are the built-in capabilities that PincherOS recognizes. Each capability has a name, a set of constraints, and a default enforcement behavior.

| Capability | Description | Default Grant | Enforcement |
|---|---|---|---|
| `network` | Outbound network access (TCP/UDP) | Denied | bwrap network namespace isolation |
| `filesystem_read` | Read files from disk | Granted (unconstrained) | Landlock read-only rules |
| `filesystem_write` | Write files to disk | Denied | Landlock write rules + path whitelist |
| `subprocess` | Spawn child processes | Granted (constrained) | bwrap + seccomp filter |
| `gpu` | Access GPU device | Denied | Device namespace isolation |

### Capability: `network`

**Description**: Allows the agent to make outbound network connections. Required for any reflex that uses `curl`, `wget`, `ssh`, database clients, or HTTP APIs.

**Default**: Denied. Must be explicitly declared in the manifest.

**Constraints**:

| Constraint | Type | Default | Description |
|---|---|---|---|
| `allowed_hosts` | `string[]` | `[]` (all hosts) | Whitelist of hostnames. Empty = all allowed. |
| `allowed_ports` | `uint16[]` | `[]` (all ports) | Whitelist of ports. Empty = all allowed. |
| `max_connections` | `uint32` | `4294967295` | Maximum concurrent connections. |
| `allow_dns` | `bool` | `true` | Whether DNS resolution is permitted. |

**Enforcement**: When `network` is not granted, bwrap creates a new network namespace with only loopback. All outbound connections fail immediately.

**When to grant**: Smart home controllers (HTTP APIs), deployment agents (health checks), monitoring agents (metric submission).

### Capability: `filesystem_read`

**Description**: Allows the agent to read files from the filesystem. Required for reflexes that inspect logs, configs, or source code.

**Default**: Granted with no constraints. This is safe because read-only access cannot modify state.

**Constraints**:

| Constraint | Type | Default | Description |
|---|---|---|---|
| `allowed_paths` | `string[]` | `[]` (all paths) | Whitelist of readable paths. Empty = all readable. |
| `max_file_size_mb` | `uint32` | `1024` | Maximum single file size. |
| `allow_symlinks` | `bool` | `true` | Whether symlinks are followed. |

**Enforcement**: Landlock rules create read-only access controls. When `allowed_paths` is non-empty, only those paths are mounted in the sandbox.

**When to constrain**: Code review agents (only read source directories), log analyzers (only read `/var/log`).

### Capability: `filesystem_write`

**Description**: Allows the agent to write, create, or delete files. Required for reflexes that modify configs, write backups, or generate reports.

**Default**: Denied. Must be explicitly declared. This is the most dangerous capability.

**Constraints**:

| Constraint | Type | Default | Description |
|---|---|---|---|
| `allowed_paths` | `string[]` | `[]` (none) | Whitelist of writable paths. Empty = no paths writable. Must be non-empty to grant. |
| `max_file_size_mb` | `uint32` | `100` | Maximum single file size writable. |
| `allow_delete` | `bool` | `false` | Whether delete/unlink operations are permitted. |
| `allow_create` | `bool` | `true` | Whether creating new files is permitted. |
| `allow_rename` | `bool` | `false` | Whether renaming files is permitted. |

**Enforcement**: Landlock write rules. Only declared paths are mounted writable in the sandbox. The veto engine also blocks writes to `/etc`, `/boot`, `/sys`, `/proc`, and `/dev` regardless of manifest settings.

**When to grant**: Backup agents (write to `/backup`), deployment agents (write to `/opt`), report generators (write to `/tmp/reports`).

### Capability: `subprocess`

**Description**: Allows the agent to spawn child processes. Required for reflexes that execute shell commands.

**Default**: Granted with constraints. Without this, PincherOS cannot execute any reflex action.

**Constraints**:

| Constraint | Type | Default | Description |
|---|---|---|---|
| `allowed_commands` | `string[]` | `[]` (none) | Whitelist of command names. Empty = no commands allowed. Must include at least one command. |
| `max_runtime_seconds` | `uint32` | `30` | Maximum execution time per subprocess. Killed after timeout. |
| `allow_shell` | `bool` | `false` | Whether shell expansion (pipes, redirects, `&&`) is permitted. |
| `max_processes` | `uint32` | `4` | Maximum concurrent subprocesses. |

**Enforcement**: bwrap creates a minimal mount namespace. Only `/bin`, `/usr/bin`, `/lib`, `/usr/lib` are mounted. The `allowed_commands` list is checked before execution. `allow_shell: false` prevents `sh -c` wrapping.

**When to constrain**: Production agents (only allow specific commands), CI agents (only allow build tools).

### Capability: `gpu`

**Description**: Allows the agent to access GPU devices for inference acceleration.

**Default**: Denied. Only needed for ONNX model inference on GPU.

**Constraints**:

| Constraint | Type | Default | Description |
|---|---|---|---|
| `max_memory_mb` | `uint32` | `512` | Maximum GPU memory allocation. |
| `allowed_operations` | `string[]` | `["inference"]` | Allowed GPU operation types. |
| `device_ids` | `uint32[]` | `[0]` | Which GPU devices can be used. |

**Enforcement**: Device namespace isolation. Only declared GPU devices are accessible.

**When to grant**: ONNX inference sidecar (accelerated embedding), ML training agents.

---

## Enforcement Chain

Every reflex execution passes through this enforcement chain, in order:

```
1. Veto Engine (deterministic rules)
   └─ If DENY → abort, return error code -32008
2. Capability Check (manifest grants)
   └─ If capability not granted → abort, return error code -32009
3. Resource Check (PID controller state)
   └─ If CRITICAL → abort, return error code -32007
4. Sandbox Construction (bwrap + landlock)
   └─ Build isolated namespace per capability constraints
5. Execution (inside sandbox)
   └─ If timeout → kill subprocess, return error code -32004
6. Result Validation (exit code + output check)
   └─ If non-zero exit → log failure, update confidence negatively
```

### Enforcement Order Matters

The veto engine runs **before** capability checks because:
- Veto rules can block operations that capabilities would allow (safety override)
- Veto evaluation is O(n_rules) and fast (~1ms)
- Capability check requires sandbox construction which is slower (~50ms)

### Sandbox Construction Details

For each execution, the sandbox is constructed based on the granted capabilities:

| Capability | Sandbox Effect |
|---|---|
| `network` not granted | New network namespace (loopback only) |
| `filesystem_read` granted | Mount `allowed_paths` read-only |
| `filesystem_write` granted | Mount `allowed_paths` read-write |
| `subprocess` granted | Mount `/bin`, `/usr/bin`, `/lib`, `/usr/lib` |
| `gpu` granted | Mount `/dev/nvidia*` devices |

The sandbox is created by `bwrap` with these flags:
- `--unshare-net` (if `network` not granted)
- `--ro-bind /usr /usr`
- `--ro-bind /lib /lib`
- `--proc /proc`
- `--dev /dev`
- `--bind <write_path> <write_path>` (per `filesystem_write.allowed_paths`)

---

## Declaring Capabilities in a Manifest

### TOML Format (recommended for CLI)

```toml
[capabilities.network]
granted = true
allowed_hosts = ["api.homebridge.local", "httpbin.org"]
allowed_ports = [443, 8080]

[capabilities.filesystem_read]
granted = true
allowed_paths = ["/var/log", "/etc/pincher"]
max_file_size_mb = 50

[capabilities.filesystem_write]
granted = true
allowed_paths = ["/tmp/pincher-output"]
max_file_size_mb = 10
allow_delete = false

[capabilities.subprocess]
granted = true
allowed_commands = ["curl", "systemctl", "journalctl", "df", "free", "ps"]
max_runtime_seconds = 15
allow_shell = false

[capabilities.gpu]
granted = false
```

### JSON Format (for .nail manifests and RPC)

```json
{
  "declared_capabilities": {
    "network": {
      "granted": true,
      "constraints": {
        "allowed_hosts": ["api.homebridge.local"],
        "allowed_ports": [443]
      }
    },
    "filesystem_read": {
      "granted": true,
      "constraints": {}
    },
    "filesystem_write": {
      "granted": false,
      "constraints": {}
    },
    "subprocess": {
      "granted": true,
      "constraints": {
        "allowed_commands": ["curl", "df"],
        "max_runtime_seconds": 15
      }
    },
    "gpu": {
      "granted": false,
      "constraints": {}
    }
  }
}
```

---

## Extending Capabilities

To add a custom capability:

1. **Define the name**: Must match `^[a-z][a-z0-9_]{2,63}$` (lowercase, snake_case, 3-64 chars).
2. **Add to manifest**: Include in `declared_capabilities` with `granted` and `constraints`.
3. **Implement enforcement**: Add enforcement logic in the veto engine or application layer.
4. **Handle unknown**: Unknown capabilities are treated as `granted: false` by default.

### Custom Capability Example

```toml
[capabilities.bluetooth]
granted = true
allowed_devices = ["00:11:22:33:44:55"]
max_range_meters = 10
```

The veto engine will pass this to the application layer for enforcement. PincherOS does not natively understand `bluetooth` — the application must check the capability before performing Bluetooth operations.

---

## Resource Constraints

Capabilities interact with the resource state machine. See [STATE_MACHINE.md § Resource States](./STATE_MACHINE.md#resource-states).

| Resource State | Effect on Capabilities |
|---|---|
| `normal` | All granted capabilities available. |
| `light` | `gpu` capability suspended. `network` connections limited to `max_connections / 2`. |
| `critical` | All capabilities suspended except `filesystem_read` and `subprocess` (for reflex-only execution). |

This means resource pressure can override capability grants. An agent with `network` granted will lose network access in `critical` state.

---

## Security Considerations

1. **Principle of least privilege**: Grant only the capabilities the agent needs. A monitoring agent should not have `filesystem_write`.
2. **Veto overrides grants**: Even if a capability is granted, veto rules can block specific operations. Use veto rules for dangerous patterns.
3. **Sandbox is defense-in-depth**: Even if the veto engine has a bug, the sandbox prevents the reflex from accessing resources outside its namespace.
4. **Migration merges capabilities**: When unpacking a `.nail` file, the resulting capability set is the intersection of the file's declared capabilities and the target's security policy. The target can never grant more than it allows.
