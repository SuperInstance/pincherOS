# Threat Model

## Overview

PincherOS executes commands on behalf of users based on natural-language input
matched to reflex templates. This creates several attack surfaces.

## Attack Vectors

### 1. Prompt Injection

**Threat**: Malicious text in a file or website causes the agent to execute
unintended commands.

**Example**: A file named `; rm -rf /` causes `cat "; rm -rf /"` to be interpreted
as two commands.

**Mitigation**:
- Commands run inside a `bwrap` sandbox with `--unshare-net --die-with-parent`
- Blocked pattern list catches `rm -rf /`, `dd if=/dev/zero`, etc.
- Parameter substitution uses strict `{param}` syntax, not shell interpolation

### 2. Malicious Reflex Import

**Threat**: A `.nail` file from an untrusted source contains reflexes that
execute dangerous commands.

**Mitigation**:
- Reflexes have `CapabilityManifest` declaring their permissions
- Veto engine blocks commands that exceed declared capabilities
- `pincher unpack` validates all checksums before opening the database
- Future: Sign `.nail` files with Ed25519 and verify before import

### 3. Sandbox Escape

**Threat**: A reflex finds a way to break out of the `bwrap` sandbox.

**Mitigation**:
- Defense-in-depth: 4 layers of protection
  1. Natural Language Guard — veto engine checks commands
  2. Tenuo Manifest — capability tokens restrict what the reflex can do
  3. Landlock/seccomp — kernel-level filesystem and syscall restrictions (post-MVP)
  4. OS sandbox — `bwrap` with namespace isolation
- If a sandbox escape is discovered, the reflex's confidence is set to 0.0 and
  it is quarantined

### 4. Data Exfiltration

**Threat**: A reflex reads sensitive files and sends them over the network.

**Mitigation**:
- Default sandbox: `--unshare-net` (no network access)
- Network access requires explicit `NetConnect` permission in the manifest
- Read access requires explicit `FsRead` permission

## Security Layers

```
Layer 4: OS Sandbox (bwrap)
  → Namespace isolation, unshare-net, unshare-pid
Layer 3: Landlock/seccomp (post-MVP)
  → Kernel-level filesystem and syscall restrictions
Layer 2: Capability Tokens (Tenuo)
  → HMAC-signed manifest declaring what the reflex may do
Layer 1: Veto Engine
  → Blocked pattern list + capability manifest enforcement
```
