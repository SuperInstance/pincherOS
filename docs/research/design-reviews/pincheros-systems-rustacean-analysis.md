# PincherOS: Systems Rustacean Analysis
## Or: Why I Stopped Worrying and Learned to Love the Borrow Checker on a Pi 4

> *"If it compiles, it's correct. If it doesn't, your architecture is wrong."*
> — Every Rust programmer, at 3 AM, fighting the borrow checker

---

# 0. GROUND RULES

I'm not here to write poetry about hermit crabs. I'm here to talk about **who owns what, for how long, and who's allowed to touch it**. That's not a metaphor — that's Rust's entire value proposition, and it happens to be the only question that matters when you're shipping an OS that runs on 1GB of RAM.

The hermit crab metaphor is *accidentally perfect* for Rust:
- **Shell** = the hardware resource owner. The crab doesn't own the shell; it *inhabits* it. This is `&mut Shell` — exclusive borrow, not ownership.
- **Rigging** = the crab itself. It owns its state, carries it between shells. This is `Rigging: Owned` — move semantics, no copy.
- **Claws** = the execution interface. Claws borrow the GPU, they don't own it. This is `&Claws` — shared borrow of a capability.
- **Exoskeleton** = the rendering surface. It's a projection of state, not state itself. This is `&self → RenderOutput` — pure function of the rigging.

Every single one of these maps to a Rust ownership pattern. Let me prove it.

---

# 1. TRAIT HIERARCHY: The Vtable of the Crab

## The Core Trait Hierarchy

```
                    ┌──────────────┐
                    │   Pincher    │  (top-level marker)
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
     ┌────────┴──┐  ┌──────┴────┐  ┌───┴──────────┐
     │   Shell   │  │  Rigging  │  │ Exoskeleton  │
     │ (provide) │  │  (own)    │  │  (project)   │
     └─────┬─────┘  └─────┬─────┘  └──────────────┘
           │              │
    ┌──────┴──────┐  ┌────┴─────┐
    │   Claws    │  │  Reflex  │
    │  (borrow)  │  │ (owned)  │
    └────────────┘  └──────────┘
```

## Actual Rust Trait Signatures

```rust
use std::future::Future;
use std::pin::Pin;

/// Marker trait: this type participates in the PincherOS ecosystem.
/// All PincherOS types must be Send + Sync because:
/// - Shell runs on a background thread
/// - Rigging state is accessed from async tasks
/// - Claws may be shared across warp schedulers
pub trait Pincher: Send + Sync + 'static {}

// ──────────────────────────────────────────────────────────
// SHELL: The Hardware Abstraction
// ──────────────────────────────────────────────────────────

/// A Shell provides hardware resources. It does NOT own the rigging.
/// The rigging borrows the shell's capabilities.
///
/// Key insight: Shell is the ONLY trait that varies per hardware.
/// Everything else is hardware-agnostic.
pub trait Shell: Pincher {
    type Error: std::error::Error + Send + Sync;

    /// Probe the current hardware and return an immutable snapshot.
    /// This is called at boot, on hardware change, and on migration.
    /// Returns a snapshot, not a reference — the profile must be 'static
    /// because it's used across async boundaries.
    fn snap(&self) -> ShellProfile;

    /// Monitor current resource usage. Called on a 5-second tick.
    fn resource_status(&self) -> ResourceStatus;

    /// Request the shell to adapt to a new degradation level.
    /// Returns Ok(()) if the shell accepted the new level.
    /// Returns Err if the shell cannot comply (e.g., already at minimum).
    fn degrade(&mut self, level: DegradationLevel) -> Result<(), Self::Error>;

    /// Restore from a degraded state. Called when resources free up.
    fn promote(&mut self, level: DegradationLevel) -> Result<(), Self::Error>;

    /// Returns the shell's unique fingerprint.
    /// Stable across reboots, changes on hardware swap.
    /// Used for CRDT context tagging.
    fn fingerprint(&self) -> &ShellFingerprint;

    /// The shell's energy state (battery, AC, etc.)
    /// None if not applicable (desktop, server).
    fn energy_state(&self) -> Option<EnergyState>;
}

/// Shell profile: a 'static snapshot of hardware capabilities.
/// This is the OUTPUT of snap(), not the shell itself.
/// It's cloned freely — it's just data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellProfile {
    pub fingerprint: ShellFingerprint,
    pub device_type: DeviceType,
    pub capabilities: Capabilities,
    pub limits: Limits,
    pub energy: Option<EnergyState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// Raspberry Pi 4: no GPU, 4GB RAM, ARM Cortex-A72
    TurboShell,
    /// Jetson Nano: 128 CUDA cores, 4GB shared RAM
    CudaShell,
    /// Workstation with discrete GPU
    BigConch,
    /// Unknown / custom
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub ram_total_bytes: u64,
    pub ram_available_bytes: u64,
    pub cpu_cores: usize,
    pub cpu_freq_mhz: u64,
    pub gpu: GpuType,
    pub disk_free_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuType {
    None,
    Cuda {
        compute_capability: (u8, u8),  // e.g., (5, 3) for Maxwell
        sm_count: usize,
        vram_bytes: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub max_model_bytes: u64,
    pub max_concurrent_reflexes: usize,
    pub inference_threads: usize,
    pub gpu_layers: u32,
    pub sandbox_mem_bytes: u64,
    pub sandbox_cpu_secs: u64,
}

// ──────────────────────────────────────────────────────────
// RIGGING: The Mutable Agent State (THE OWNER)
// ──────────────────────────────────────────────────────────

/// The Rigging OWNS the agent's state. It is the crab itself.
/// It carries: reflexes, embeddings, trust scores, personality.
/// It migrates between shells.
///
/// CRITICAL DESIGN DECISION: Rigging is NOT generic over Shell.
/// A Rigging can inhabit ANY shell. The Shell is a runtime parameter,
/// not a type parameter. This is intentional — if Rigging<JetsonShell>
/// and Rigging<PiShell> were different types, migration would require
/// transmute, which is a code smell that says your type system is wrong.
pub trait Rigging: Pincher {
    type Error: std::error::Error + Send + Sync;

    /// The rigging's unique identity. Persists across migrations.
    fn identity(&self) -> &RiggingId;

    /// Match an input against known reflexes.
    /// Returns candidates sorted by cosine similarity, capped at top_k.
    /// This is the REFLEX SHORT-CIRCUIT entry point.
    fn match_reflexes(
        &self,
        input: &InputEvent,
        top_k: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ReflexMatch>, Self::Error>> + Send + '_>>;

    /// Store a new reflex. The rigging takes ownership of the reflex data.
    /// Returns the new reflex's ID.
    fn learn_reflex(
        &mut self,
        trigger: &str,
        action: ActionTemplate,
        source: ReflexSource,
    ) -> Pin<Box<dyn Future<Output = Result<ReflexId, Self::Error>> + Send + '_>>;

    /// Update a reflex's trust score after execution.
    /// This is the actualization engine: dynamis → energeia.
    fn update_trust(
        &mut self,
        reflex_id: &ReflexId,
        outcome: &ExecutionOutcome,
    ) -> Result<(), Self::Error>;

    /// Pack the rigging into a portable .nail file.
    /// The rigging remains valid after packing — it's a snapshot, not a move.
    /// BUT: the caller should mark the rigging as MIGRATING to prevent
    /// state divergence between the snapshot and live state.
    fn pack(&self) -> Pin<Box<dyn Future<Output = Result<NailFile, Self::Error>> + Send + '_>>;

    /// Unpack a .nail file and merge into this rigging via CRDT semantics.
    /// Migration = merge. No conflicts, only convergence.
    fn unpack_and_merge(
        &mut self,
        nail: NailFile,
        source_context: &ShellFingerprint,
    ) -> Pin<Box<dyn Future<Output = Result<MigrationReport, Self::Error>> + Send + '_>>;

    /// Compact the rigging's storage. Called when LanceDB exceeds threshold.
    fn compact(&mut self) -> Pin<Box<dyn Future<Output = Result<CompactionStats, Self::Error>> + Send + '_>>;
}

/// A reflex match result from the vector DB search.
#[derive(Debug, Clone)]
pub struct ReflexMatch {
    pub reflex_id: ReflexId,
    pub cosine_similarity: f32,  // f32, not f64 — 384-dim vectors don't need f64
    pub confidence: f32,         // The trust-derived confidence
    pub context_tags: Vec<String>,
}

/// The source of a reflex. Affects initial confidence and expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflexSource {
    /// Learned from direct interaction with LLM
    Learned,
    /// Distilled from observing a CLI command
    Distilled,
    /// Imported from a .nail skillpack
    Imported,
    /// Inherited from fleet CRDT merge
    FleetMerged,
}

// ──────────────────────────────────────────────────────────
// CLAWS: The GPU Execution Bridge (BORROWED CAPABILITY)
// ──────────────────────────────────────────────────────────

/// Claws BORROW GPU access. They do not own the GPU.
/// On a Pi with no GPU, Claws is a no-op.
/// On a Jetson, Claws borrows the CUDA context.
/// On an RTX 4090, Claws creates a persistent kernel.
///
/// The key insight: Claws is ALWAYS available, but its
/// implementation varies. Feature gates select the impl,
/// not the trait.
pub trait Claws: Pincher {
    type Error: std::error::Error + Send + Sync;

    /// Dispatch a compute command to the GPU.
    /// Returns a handle that can be polled for completion.
    /// On CPU-only systems, this executes synchronously.
    fn dispatch(
        &self,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchHandle, Self::Error>> + Send + '_>>;

    /// Check if GPU execution is available.
    /// Returns false on CPU-only systems or when GPU is in degraded mode.
    fn is_available(&self) -> bool;

    /// Returns the current GPU tier for pushdown evaluation.
    fn tier(&self) -> ExecutionTier;

    /// Allocate unified memory on the GPU. Returns a GpuBridge<T>-like handle.
    /// On CPU-only, this is a heap allocation.
    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error>;

    /// Returns the number of concurrent agents the GPU can support.
    /// 0 on CPU-only systems.
    fn agent_capacity(&self) -> usize;
}

/// A type that can safely reside in GPU memory.
/// Must be #[repr(C)], no Drop impl, no pointers.
pub unsafe trait GpuSafe: Copy + 'static {}

/// Priority levels for GPU dispatch.
/// Maps directly to cudaclaw's 4-tier priority queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Execution tier for pushdown evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    /// CPU-only: reflex short-circuit, LLM via llama.cpp
    LocalCpu,
    /// GPU warp-level: 32 concurrent agents
    GpuWarp,
    /// GPU block-level: 512 agents per block
    GpuBlock,
    /// GPU grid-level: full GPU coordination
    GpuGrid,
    /// Cloud LLM: fallback for novel reasoning
    CloudLlm,
}

/// A GPU command. 48 bytes, #[repr(C, packed(4))].
/// This matches cudaclaw's Command struct exactly.
#[repr(C, packed(4))]
#[derive(Debug, Clone, Copy)]
pub struct GpuCommand {
    pub kernel_id: u32,
    pub variant: KernelVariant,
    pub priority: Priority,
    pub payload_ptr: u64,   // Pointer into unified memory
    pub payload_len: u32,
    pub flags: u16,
    pub _reserved: u16,
}

const _: () = assert!(std::mem::size_of::<GpuCommand>() == 48);

#[derive(Debug, Clone, Copy, repr(u8))]
pub enum KernelVariant {
    Baseline = 0,
    L1 = 1,
    Shmem = 2,
    L1Equal = 3,
}

/// A slice of GPU-accessible memory.
/// On GPU systems: unified memory, zero-copy between CPU and GPU.
/// On CPU-only: a Vec<T> in host memory.
pub enum GpuSlice<T: GpuSafe> {
    #[cfg(feature = "cuda")]
    Cuda { ptr: *mut T, len: usize, handle: cudaclaw::GpuBridge<T> },
    Cpu { data: Vec<T> },
}

// Safety: GpuSlice is Send + Sync because:
// - CUDA unified memory is accessible from both CPU and GPU
// - Access is mediated through the GpuBridge which handles synchronization
unsafe impl<T: GpuSafe> Send for GpuSlice<T> {}
unsafe impl<T: GpuSafe> Sync for GpuSlice<T> {}

// ──────────────────────────────────────────────────────────
// EXOSKELETON: The Rendering Projection (PURE FUNCTION)
// ──────────────────────────────────────────────────────────

/// The Exoskeleton is a PURE PROJECTION of Rigging state.
/// It does not own state. It does not mutate state.
/// It takes &self and produces a renderable output.
///
/// This is the most important design constraint in the trait hierarchy:
/// rendering is PURE. If you need to mutate state to render,
/// your architecture is wrong. Go fix the architecture.
pub trait Exoskeleton: Pincher {
    type Output: RenderOutput;
    type Error: std::error::Error + Send + Sync;

    /// Render the current state to a display format.
    /// This MUST be side-effect-free.
    fn render(&self, state: &RiggingState, format: RenderFormat) -> Result<Self::Output, Self::Error>;

    /// Render a migration proposal for user consent.
    /// This is the ONLY time the exoskeleton produces interactive output.
    fn render_migration_proposal(
        &self,
        proposal: &MigrationProposal,
    ) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Ansi,   // Terminal with ANSI codes
    Html,   // Web browser
    Markdown, // Piped to a pager
    Raw,    // No formatting (for scripting)
}

pub trait RenderOutput: Send + Sync + 'static {
    fn as_bytes(&self) -> &[u8];
    fn content_type(&self) -> &str;
}

// ──────────────────────────────────────────────────────────
// THE TOP-LEVEL COMPOSITION: PincherOS Runtime
// ──────────────────────────────────────────────────────────

/// The PincherOS runtime composes all four traits.
/// This is the single entry point for the entire system.
pub struct PincherOs<S, R, C, E>
where
    S: Shell,
    R: Rigging,
    C: Claws,
    E: Exoskeleton,
{
    shell: S,
    rigging: R,
    claws: C,
    exoskeleton: E,
    monitor: ResourceMonitor,
    degradation: DegradationLevel,
}

impl<S, R, C, E> PincherOs<S, R, C, E>
where
    S: Shell,
    R: Rigging,
    C: Claws,
    E: Exoskeleton,
{
    /// Boot the system. Runs snap(), restores rigging, starts monitors.
    pub async fn boot(shell: S, rigging: R, claws: C, exoskeleton: E) -> Result<Self, BootError> {
        let profile = shell.snap();
        let monitor = ResourceMonitor::new(profile);
        Ok(Self {
            shell,
            rigging,
            claws,
            exoskeleton,
            monitor,
            degradation: DegradationLevel::Normal,
        })
    }

    /// Process a single user input. This is the CORE LOOP.
    pub async fn process(&mut self, input: InputEvent) -> Result<Output, CoreError> {
        // 1. Match reflexes (borrow rigging)
        let matches = self.rigging.match_reflexes(&input, 5).await?;

        // 2. Check for reflex short-circuit
        if let Some(best) = matches.first() {
            if best.confidence > 0.90 {
                // REFLEX SHORT-CIRCUIT: skip LLM entirely
                let result = self.execute_reflex(&best.reflex_id, &input).await?;
                self.rigging.update_trust(&best.reflex_id, &result.outcome)?;
                return Ok(Output::from_result(result));
            }
        }

        // 3. No high-confidence match — consult LLM
        // ... (full path: context assembly → LLM → parse → execute → store)
        todo!("Implement full LLM path")
    }
}
```

## Trait Bounds Rationale

| Trait | Bounds | Why |
|-------|--------|-----|
| `Pincher` | `Send + Sync + 'static` | Crosses async boundaries. Shared between tasks. |
| `Shell` | `Pincher` + `Degrade` | Must be mutable for degradation, but snap is immutable. |
| `Rigging` | `Pincher` + async methods | Owns mutable state, accessed from multiple tasks. |
| `Claws` | `Pincher` + `is_available()` | May be unavailable (no GPU). Must handle gracefully. |
| `Exoskeleton` | `Pincher` + pure render | **No mutation.** If you need to mutate to render, redesign. |

The critical insight: **`Shell` and `Rigging` have fundamentally different ownership semantics.** `Shell` is borrowed — the rigging inhabits it temporarily. `Rigging` is owned — it persists across shells. This is exactly the hermit crab metaphor, and it's exactly Rust's ownership model.

---

# 2. OWNERSHIP MODEL: Who Owns What, For How Long

This is the section where the hermit crab metaphor stops being cute and starts being load-bearing. Rust's ownership model isn't a suggestion — it's a compile-time guarantee. If we get the ownership model wrong, the borrow checker will tell us, and we'll be grateful for it.

## The Ownership Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      PincherOs { }                              │
│                   OWNS: shell, rigging, claws, exo              │
│                                                                  │
│  ┌─────────────────┐     ┌──────────────────────────────────┐  │
│  │   Shell          │     │   Rigging                        │  │
│  │   OWNED by OS    │     │   OWNED by OS                    │  │
│  │                  │     │                                   │  │
│  │  ┌──────────┐   │     │  ┌──────────┐  ┌──────────────┐  │  │
│  │  │Profile   │   │     │  │Reflexes  │  │Trust Scores  │  │  │
│  │  │(snapshot)│   │     │  │(LanceDB) │  │(SQLite)      │  │  │
│  │  │  Clone   │   │     │  │ owned    │  │ owned        │  │  │
│  │  └──────────┘   │     │  └──────────┘  └──────────────┘  │  │
│  │                  │     │  ┌──────────┐  ┌──────────────┐  │  │
│  │  BORROWED by:   │     │  │Person-   │  │JEPA State    │  │  │
│  │  - Rigging      │     │  │ality     │  │(if present)  │  │  │
│  │  - Claws        │     │  │ owned    │  │ owned        │  │  │
│  │  - Monitor      │     │  └──────────┘  └──────────────┘  │  │
│  └─────────────────┘     └──────────────────────────────────┘  │
│                                                                  │
│  ┌─────────────────┐     ┌──────────────────────────────────┐  │
│  │   Claws          │     │   Exoskeleton                    │  │
│  │   OWNED by OS    │     │   OWNED by OS                    │  │
│  │                  │     │                                   │  │
│  │  BORROWS:        │     │  BORROWS:                        │  │
│  │  - GPU context   │     │  - &RiggingState (immutable)     │  │
│  │    (if present)  │     │                                   │  │
│  │                  │     │  PRODUCES:                        │  │
│  │  PRODUCES:       │     │  - RenderOutput (owned)          │  │
│  │  - GpuSlice<T>   │     │  - No state mutation             │  │
│  │    (owned)       │     │                                   │  │
│  └─────────────────┘     └──────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## The Lifetime Model in Code

```rust
/// The PincherOS runtime. Owns everything.
/// Lifetimes are ELIDED because PincherOs owns all its components.
/// There are no references into PincherOs from outside.
pub struct PincherOs<S, R, C, E>
where
    S: Shell,
    R: Rigging,
    C: Claws,
    E: Exoskeleton,
{
    shell: S,
    rigging: R,
    claws: C,
    exoskeleton: E,
    // ...
}

/// The core loop's ownership flow:
///
/// 1. PincherOs OWNS Shell → snap() produces a ShellProfile (owned, Clone)
/// 2. PincherOs OWNS Rigging → match_reflexes() borrows &self
/// 3. PincherOs OWNS Claws → dispatch() borrows &self
/// 4. PincherOs OWNS Exoskeleton → render() borrows &RiggingState
///
/// There is NO shared mutable state.
/// The only mutation path is: &mut self.rigging.update_trust()
/// Which requires &mut PincherOs — i.e., exclusive access.
impl<S, R, C, E> PincherOs<S, R, C, E>
where
    S: Shell,
    R: Rigging,
    C: Claws,
    E: Exoskeleton,
{
    /// The core loop demonstrates the ownership model.
    /// Pay attention to what borrows what.
    pub async fn process(&mut self, input: InputEvent) -> Result<Output, CoreError> {
        // BORROW: &self.rigging (immutable, for matching)
        let matches = self.rigging.match_reflexes(&input, 5).await?;

        // BORROW: &self.shell (immutable, for profile)
        let profile = self.shell.snap();

        // BORROW: &self.claws (immutable, for tier check)
        let tier = self.claws.tier();

        if let Some(best) = matches.first() {
            if best.confidence > 0.90 {
                // MUTABLE BORROW: &mut self.rigging (for trust update)
                // This is the ONLY mutation in the reflex short-circuit path.
                self.rigging.update_trust(&best.reflex_id, &outcome)?;

                // BORROW: &self.exoskeleton (immutable, for rendering)
                let state = self.rigging_state();
                let output = self.exoskeleton.render(&state, RenderFormat::Ansi)?;
                return Ok(Output::Rendered(output));
            }
        }

        // Full path: context assembly → LLM → parse → execute → store
        // Each step borrows a specific component. No overlapping mutable borrows.
        let context = self.assemble_context(&input, &matches, &profile)?;
        let inference = self.infer(&context).await?;
        let actions = self.parse_actions(&inference)?;
        let result = self.execute_sandboxed(&actions, &profile).await?;

        // MUTABLE BORROW: learning a new reflex
        self.rigging.learn_reflex(&input.text, result.action_template, ReflexSource::Learned).await?;

        Ok(Output::from_result(result))
    }
}
```

## The Migration Ownership Transfer

This is where it gets interesting. Migration is a **three-phase ownership transfer**, modeled on a database transaction:

```rust
/// Migration state machine.
/// The key insight: during migration, BOTH shells have a BORROW
/// on the rigging state. This is safe because:
/// - Phase 1 (PREPARE): Shell-A owns, Shell-B borrows nothing
/// - Phase 2 (CROSSFADE): Shell-A owns, Shell-B borrows a snapshot
/// - Phase 3 (FINALIZE): Shell-B owns, Shell-A retains a snapshot (read-only)
///
/// The "half-migrated" state is IMPOSSIBLE because the state machine
/// enforces atomic transitions.
pub enum MigrationPhase {
    /// Normal operation. Shell-A fully owns the rigging.
    Stable,

    /// Shell-A has snapshotted the rigging. The rigging is READ-ONLY.
    /// No new reflexes can be learned. No trust updates.
    /// Shell-B has received the snapshot and is running snap().
    Preparing {
        snapshot: NailFile,
        prepared_at: Instant,
        timeout: Duration,  // Default: 30s
    },

    /// Both shells are running. Shell-A is "fading" (finishing in-flight
    /// requests only). Shell-B is "warming" (processing forwarded intents).
    /// This phase lasts 200ms on same-machine, 2s across machines.
    Crossfading {
        snapshot: NailFile,
        started_at: Instant,
        crossfade_duration: Duration,
    },

    /// Shell-B now owns the rigging. Shell-A retains a read-only snapshot
    /// for rollback (default: 24 hours).
    Finalized {
        migrated_to: ShellFingerprint,
        snapshot: NailFile,
        retention_deadline: Instant,
    },
}

/// The rigging's migration state. Encoded in SQLite.
pub struct MigrationGuard {
    phase: MigrationPhase,
}

impl MigrationGuard {
    /// Can the rigging accept new reflexes?
    /// Only during Stable phase.
    pub fn can_learn(&self) -> bool {
        matches!(self.phase, MigrationPhase::Stable)
    }

    /// Can the rigging update trust scores?
    /// Yes during Stable, no during Preparing/Finalized,
    /// limited during Crossfading (only for in-flight requests).
    pub fn can_update_trust(&self) -> bool {
        matches!(self.phase, MigrationPhase::Stable | MigrationPhase::Crossfading { .. })
    }
}
```

## The Borrowing Rules Summary

| Component | Owned By | Borrowed By | Mutable? |
|-----------|----------|-------------|----------|
| `Shell` | `PincherOs` | `Rigging` (for limits), `Monitor` (for status) | `&mut self` for degradation |
| `Rigging` | `PincherOs` | `Exoskeleton` (read-only), `CoreLoop` (read/write) | `&mut self` for learn/trust |
| `Claws` | `PincherOs` | `CoreLoop` (dispatch), `PushdownEvaluator` (tier check) | `&self` only — GPU state is managed by CUDA runtime |
| `Exoskeleton` | `PincherOs` | Nobody — it's a pure projection | `&self` only — no mutation ever |
| `ShellProfile` | Nobody (Clone) | Freely copied — it's just data | N/A — immutable snapshot |
| `NailFile` | Migration subsystem | Temporary during migration | Consumed on unpack |

**The golden rule: the only `&mut` in the entire core loop is `&mut self.rigging` for trust updates and reflex learning.** Everything else is `&self`. This is not an accident — it's the architecture.

---

# 3. ZERO-COST ABSTRACTIONS: What Compiles Away

This is the section where I earn my Rust merit badge. Not every abstraction in PincherOS can be zero-cost. Let me be honest about which ones are and which ones aren't.

## Can Shell Swap Compile Away?

**No. And that's the right answer.**

Shell Swap is a *runtime* operation. You can't know at compile time whether the rigging will migrate from a Pi to a Jetson. If you try to make Shell a const generic, you'd need `PincherOs<JetsonShell, ...>` and `PincherOs<PiShell, ...>` as separate types — and then migration is literally impossible because you can't transmute between generic monomorphizations.

**But: the cost of Shell Swap can be made trivial.** Here's how:

```rust
/// The Shell trait object. This IS a vtable dispatch — one indirect call
/// per snap() invocation. Cost: ~5ns on ARM.
///
/// Is this zero-cost? No. Is this acceptable? Yes.
/// snap() is called once at boot and once per migration.
/// At those frequencies, 5ns is irrelevant.
pub type DynShell = Box<dyn Shell>;

/// What IS zero-cost: the ShellProfile.
/// It's a plain struct with no trait objects.
/// All downstream code operates on ShellProfile, not on Shell.
/// The trait dispatch happens ONCE (in snap()), and then
/// everything else is monomorphized over the profile data.
pub struct ShellProfile {
    pub fingerprint: ShellFingerprint,
    pub device_type: DeviceType,
    pub capabilities: Capabilities,
    pub limits: Limits,
}
```

The principle: **pay for abstraction once, at the boundary.** Shell is a trait object at the boundary (where the hardware is probed). Everything downstream is concrete data (ShellProfile). This is the same pattern as `std::io::Read` — the trait dispatch is at the `read()` call, not at every byte processed.

## Can A2UI Rendering Be Zero-Cost?

**Yes, at the trait level. At the implementation level, of course not — you're producing UTF-8 strings.**

```rust
/// The Exoskeleton trait IS zero-cost because:
/// 1. render() takes &self — no mutation, no state dependency
/// 2. The output is a concrete type (RenderOutput)
/// 3. The trait can be monomorphized if the concrete type is known at compile time
///
/// On a Pi with CLI only: AnsiExoskeleton is known at compile time.
/// The trait dispatch compiles away entirely.
pub trait Exoskeleton: Pincher {
    type Output: RenderOutput;
    fn render(&self, state: &RiggingState, format: RenderFormat) -> Result<Self::Output, Self::Error>;
}

/// On a Pi, we use the concrete type directly — no vtable.
pub struct AnsiExoskeleton;

impl Exoskeleton for AnsiExoskeleton {
    type Output = AnsiOutput;

    fn render(&self, state: &RiggingState, _format: RenderFormat) -> Result<AnsiOutput, Self::Error> {
        // Pure function. No allocation beyond the output string.
        // Stack-allocated formatting for small outputs.
        Ok(AnsiOutput(state.to_ansi()))
    }
}

/// If you need dynamic dispatch (e.g., user switches between ANSI and HTML):
pub type DynExoskeleton = Box<dyn Exoskeleton<Output = Box<dyn RenderOutput>>>;
/// This costs one vtable dispatch per render. ~2ns. Acceptable.
```

## What IS Zero-Cost in PincherOS

| Abstraction | Zero-Cost? | Mechanism |
|-------------|-----------|-----------|
| `ReflexMatch` struct | **Yes** | Stack-allocated, no heap. `f32` for cosine similarity. |
| `GpuCommand` 48B struct | **Yes** | `#[repr(C, packed(4))]`, const size assertion. No padding. |
| `Priority` enum | **Yes** | `repr(u8)`, 1 byte, no vtable. |
| `DeviceType` enum | **Yes** | `Copy + 'static`. Matches on ARM via `cmp`. |
| `ShellProfile` snapshot | **Yes** | Plain data, Clone. No trait objects after snap(). |
| Reflex short-circuit path | **Yes** | Branch is predictable (95%+ hit rate on learned systems). CPU branch predictor handles it. |
| `DegradationLevel` dispatch | **Yes** | `match` on an enum. Compiles to a jump table. |
| Shell trait dispatch | **No** | One vtable call per snap(). ~5ns. Acceptable. |
| Rigging trait dispatch | **No** | Async trait dispatch on match_reflexes. ~50ns. Acceptable. |
| Claws trait dispatch | **No** | One vtable call per dispatch(). ~5ns. Acceptable. |
| LanceDB vector search | **No** | This is a full database query. ~10ms on Pi. This is your bottleneck. |

**The takeaway: PincherOS's internal abstractions are zero-cost. The external interfaces (trait objects) are near-zero-cost. The actual work (vector search, LLM inference) is where your time goes — and no amount of abstraction can change that.**

---

# 4. FEARLESS CONCURRENCY: The Runtime Model

Here's where I have an opinion, and it's a strong one.

## The Concurrency Model

**PincherOS should NOT be lock-free everywhere.** Lock-free is harder to reason about, harder to debug, and on ARM Cortex-A72 (Pi 4), the atomic operations are expensive enough that a `Mutex` is often faster for low-contention paths.

Here's my model:

```
┌──────────────────────────────────────────────────────┐
│                tokio::runtime::Runtime                │
│                  (multi-thread, 4 workers)            │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐                  │
│  │ Core Loop    │  │ Monitor      │                  │
│  │ (exclusive)  │  │ (periodic)   │                  │
│  │              │  │              │                  │
│  │ Owns:        │  │ Borrows:     │                  │
│  │ &mut Rigging │  │ &Shell       │                  │
│  │ &Claws       │  │              │                  │
│  │ &Exoskeleton │  │ Produces:    │                  │
│  │              │  │ Degradation  │                  │
│  │ Channel:     │  │ commands     │                  │
│  │ mpsc::Sender │  │              │                  │
│  └──────────────┘  └──────────────┘                  │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐                  │
│  │ Infer Bridge │  │ Sandbox      │                  │
│  │ (UDS client) │  │ (subprocess) │                  │
│  │              │  │              │                  │
│  │ Communicates │  │ bwrap        │                  │
│  │ with Python  │  │ isolated     │                  │
│  │ sidecar via  │  │ execution    │                  │
│  │ UDS          │  │              │                  │
│  └──────────────┘  └──────────────┘                  │
└──────────────────────────────────────────────────────┘
```

## Why Not Lock-Free Everywhere?

The GpuDispatcher's SPSC ring buffer is lock-free because it HAS to be — GPU dispatch is latency-critical, and the producer (CPU) and consumer (persistent CUDA kernel) are on different execution units. That's the correct use of lock-free.

But the Rigging's match_reflexes doesn't need lock-free access. It's called once per user input, and user inputs are serial (you type one command at a time). A `tokio::sync::Mutex<Rigging>` is perfectly fine here.

```rust
/// The core loop runs as a single task with exclusive access to Rigging.
/// This is NOT a bottleneck because:
/// - User inputs are serial (one at a time)
/// - The reflex short-circuit path is ~50ms (no contention)
/// - The full LLM path is ~5s (the mutex is held for microseconds
///   of actual CPU time; the rest is .await points)
pub struct CoreLoop<R: Rigging, C: Claws> {
    rigging: Arc<Mutex<R>>,
    claws: C,
    infer: InferenceBridge,
    sandbox: SandboxExecutor,
}

impl<R: Rigging, C: Claws> CoreLoop<R, C> {
    pub async fn process(&self, input: InputEvent) -> Result<Output, CoreError> {
        // Lock rigging for matching. Lock is held for ~1ms (vector search).
        let matches = {
            let rigging = self.rigging.lock().await;
            rigging.match_reflexes(&input, 5).await?
        };
        // Lock is RELEASED here. The LLM inference happens without holding the lock.

        if let Some(best) = matches.first() {
            if best.confidence > 0.90 {
                // Re-lock for trust update. Lock held for ~10μs.
                let mut rigging = self.rigging.lock().await;
                rigging.update_trust(&best.reflex_id, &outcome)?;
                return Ok(Output::from_result(result));
            }
        }

        // Full path: no lock held during LLM inference (~5s)
        let context = self.assemble_context(&input, &matches)?;
        let inference = self.infer(&context).await?;  // .await — lock NOT held
        let actions = self.parse_actions(&inference)?;

        // Re-lock for learning. Lock held for ~5ms (LanceDB write).
        let mut rigging = self.rigging.lock().await;
        rigging.learn_reflex(&input.text, result.action_template, ReflexSource::Learned).await?;

        Ok(Output::from_result(result))
    }
}
```

## The Concurrency Strategy Summary

| Component | Concurrency Model | Why |
|-----------|------------------|-----|
| Core Loop | `tokio::sync::Mutex` | Serial user input, low contention |
| Resource Monitor | `tokio::time::interval` | Periodic tick, sends degradation commands via channel |
| Inference Bridge | `tokio::net::UnixStream` + JSON-RPC | IPC with Python sidecar |
| Sandbox Executor | `tokio::process::Command` | Subprocess management |
| GPU Dispatch (cudaclaw) | Lock-free SPSC ring buffer | Latency-critical, cross-execution-unit |
| CRDT Merge | `tokio::task::spawn_blocking` | CPU-intensive merge, offload from async runtime |
| Migration Protocol | State machine + channels | Three-phase commit, no shared mutable state |

**The principle: use the weakest concurrency primitive that works.** Mutex for shared state. Channels for communication. Lock-free only when latency demands it. `spawn_blocking` for CPU-heavy work. Never roll your own lock-free data structure unless you can prove with a model checker that it's correct.

---

# 5. FEATURE GATES: The Hierarchy of Absence

Feature gates are how Rust says "this code doesn't exist." On a Pi 4 with 1GB, code that doesn't exist is the best kind of code. Let me design the feature hierarchy.

## The Feature Gate Tree

```toml
[features]
default = ["std", "sqlite", "cli"]

# ── TIER 0: Core (always present) ──
# No feature gate. These types are used everywhere.
# ShellProfile, GpuCommand, Priority, etc.
# Compiles to ~50KB.

# ── TIER 1: Standard library ──
std = []  # Enables std::fs, std::net, std::process, etc.

# ── TIER 2: Storage backends ──
sqlite = ["rusqlite"]           # SQLite for metadata + audit log
lancedb = ["lancedb-ffi"]      # LanceDB for vector storage (via FFI)

# ── TIER 3: GPU execution ──
cuda = ["cudaclaw", "nvidia-driver"]  # Full CUDA support
cuda-metrics = ["cudaclaw"]           # CUDA metrics only (no kernel dispatch)
# cuda-metrics: for machines with nvidia-smi but no CUDA runtime
# (e.g., monitoring a Jetson from a Pi)

# ── TIER 4: ML backends ──
llm-local = []    # Local LLM inference (via Python sidecar)
llm-cloud = ["reqwest"]  # Cloud LLM fallback
embed-onnx = ["ort"]  # ONNX Runtime for local embeddings

# ── TIER 5: UI renderers ──
cli = []          # ANSI terminal output (always available)
a2ui-html = []    # HTML rendering via a2ui-render
a2ui-websocket = ["tokio-tungstenite"]  # WebSocket for browser-based UI

# ── TIER 6: Advanced features ──
jepa = ["plato-jepa"]            # JEPA predictive model
penrose = ["penrose-memory"]     # Penrose tensor memory
tenuo = ["tenuo-core"]           # Cryptographic capability tokens
fleet = ["lau-inter-shell"]      # Multi-shell coordination
migration = []                   # .nail file import/export
landlock = []                    # Kernel-level sandbox (Linux 5.13+)

# ── TIER 7: no_std (eisenstein only) ──
# This is for the eisenstein crate, not pincher-core.
# See Section 6 for details.

# ── COMPOUND FEATURES ──
rpi4 = ["std", "sqlite", "lancedb", "llm-local", "embed-onnx", "cli", "migration"]
jetson = ["std", "sqlite", "lancedb", "cuda", "llm-local", "embed-onnx", "cli", "migration", "landlock"]
workstation = ["std", "sqlite", "lancedb", "cuda", "llm-local", "llm-cloud", "embed-onnx",
               "a2ui-html", "a2ui-websocket", "jepa", "penrose", "tenuo", "fleet", "migration", "landlock"]
```

## How Feature Gates Affect the Code

```rust
// src/claws/mod.rs

#[cfg(feature = "cuda")]
mod cuda_claw;
#[cfg(not(feature = "cuda"))]
mod cpu_fallback;

// The trait is the same. The impl changes.
#[cfg(feature = "cuda")]
pub use cuda_claw::CudaClaws as DefaultClaws;
#[cfg(not(feature = "cuda"))]
pub use cpu_fallback::CpuClaws as DefaultClaws;

// Usage: always through the trait, never through the concrete type
pub fn create_claws(profile: &ShellProfile) -> Box<dyn Claws> {
    match profile.capabilities.gpu {
        #[cfg(feature = "cuda")]
        GpuType::Cuda { .. } => Box::new(CudaClaws::new(profile)),
        _ => Box::new(CpuClaws::new(profile)),
    }
}
```

## The Feature Hierarchy Principle

**Features are additive, never subtractive.** You can add `cuda` to `rpi4`, but you can't remove `sqlite` from `jetson`. This is a Cargo convention and it's the correct one — it means feature resolution is commutative and you never get conflicts.

**The MVP feature set for RPi 4: `rpi4`**. This compiles in: std, sqlite, lancedb (via FFI), llm-local (Python sidecar), embed-onnx, CLI, migration. Total binary size: ~5MB. Total memory: ~1GB with models loaded.

---

# 6. THE NO_STD QUESTION

eisenstein is `#![no_std]`. Can pincher-core be `#![no_std]`?

**No. And here's why that's the right answer.**

## What Requires `std`

| Dependency | Needs `std` | Why | Alternative |
|------------|-------------|-----|-------------|
| `tokio` | **Yes** | Async runtime, TCP, UDS | `embassy` for embedded, but overkill for Pi |
| `rusqlite` | **Yes** | Filesystem access, dynamic loading | `sqlx` with `sqlite` feature |
| `serde_json` | **Yes** | String allocation, collections | `serde_json_core` (no_std, limited) |
| `sysinfo` | **Yes** | OS-level hardware probing | Raw `/proc` and `/sys` parsing |
| `axum` | **Yes** | HTTP server, TCP listener | Custom UDS server |
| `clap` | **Yes** | CLI parsing | `getopts` or hand-rolled |

The minimum `std` dependency for PincherOS MVP is: **filesystem** (for SQLite and LanceDB), **networking** (for UDS), **threads** (for tokio), **process spawning** (for bwrap).

On a Pi 4 running Linux, all of these are available. `no_std` would gain us nothing — the Pi has a full Linux kernel. `no_std` matters for microcontrollers (STM32, ESP32), which are not PincherOS targets.

## What CAN Be `no_std`

| Crate | `no_std` Viable | Why |
|-------|----------------|-----|
| `eisenstein` | **Already is** | Pure integer math, no allocations |
| `pincher-types` | **Yes** | ShellProfile, GpuCommand, etc. — no allocations needed |
| `penrose-memory` | **Maybe** | Golden-ratio hashing is pure math, but storage needs `std` |
| `plato-jepa` | **Maybe** | Predictor is pure math, but model loading needs `std` |

## The `no_std` Architecture: What It Would Look Like

If you wanted to run PincherOS on a bare-metal ARM core (e.g., a real-time control system on a Jetson's realtime CPU):

```rust
#![no_std]
#![no_main]

// The no_std PincherOS is NOT the full OS.
// It's a REFLEX EXECUTOR — it can match and execute reflexes,
// but it cannot learn new ones or run LLM inference.
// Think of it as the crab's spinal cord, not its brain.

use panic_halt as _;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    // Boot: load ShellProfile from flash
    let profile: ShellProfile = load_from_flash();

    // Boot: load Rigging from flash (pre-compiled reflexes)
    let rigging: StaticRigging = load_rigging_from_flash();

    // Main loop: match input → execute reflex
    loop {
        let input = read_sensor_input();
        if let Some(reflex) = rigging.match_reflex(&input) {
            execute_reflex(&reflex);
        }
        // No LLM fallback. No learning. Just reflexes.
        // This is the CRITICAL degradation mode made permanent.
    }
}

/// StaticRigging: reflexes stored in fixed-size arrays, no heap.
pub struct StaticRigging {
    /// Fixed-size reflex table. Max 256 reflexes on a microcontroller.
    reflexes: [ReflexEntry; 256],
    /// Embedding vectors stored in flash (read-only).
    embeddings: &'static [[f32; 384]],
    /// Trust scores stored in SRAM.
    trust_scores: [f32; 256],
}
```

**The verdict: PincherOS core requires `std`. But the `pincher-types` crate should be `no_std`-compatible so that eisenstein and the snap math can be reused in embedded contexts. The `StaticRigging` pattern above is the escape hatch for truly constrained environments.**

---

# 7. ERROR HANDLING: The Type System Is Your Safety Net

cudaclaw's `ConstraintValidator` returns `Pass/Warn/Fail`. That's a good start. But it's not enough for an OS. Let me design the error hierarchy.

## The Error Philosophy

**Rule 1: `Result<T, E>` everywhere. No panics in production.**
**Rule 2: No error hierarchies. Use enums, not trait objects.**
**Rule 3: Error chains must be explicit, not implicit.**

```rust
/// The top-level error type for PincherOS.
/// This is NOT a trait object. It's an enum.
/// Every variant is exhaustive — you must handle all of them.
#[derive(Debug, thiserror::Error)]
pub enum PincherError {
    #[error("shell error: {0}")]
    Shell(#[from] ShellError),

    #[error("rigging error: {0}")]
    Rigging(#[from] RiggingError),

    #[error("claw error: {0}")]
    Claw(#[from] ClawError),

    #[error("exoskeleton error: {0}")]
    Exoskeleton(#[from] ExoskeletonError),

    #[error("inference error: {0}")]
    Inference(#[from] InferenceError),

    #[error("migration error: {0}")]
    Migration(#[from] MigrationError),

    #[error("resource exhausted: {resource} needed {needed}MB, available {available}MB")]
    ResourceExhausted {
        resource: String,
        needed: u64,
        available: u64,
    },
}

/// Shell-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("hardware probe failed: {0}")]
    ProbeFailed(String),

    #[error("degradation to {level:?} rejected: already at minimum")]
    DegradationRejected { level: DegradationLevel },

    #[error("energy critical: battery at {pct}%")]
    EnergyCritical { pct: f64 },
}

/// Rigging-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum RiggingError {
    #[error("vector DB search failed: {0}")]
    VectorSearchFailed(String),

    #[error("reflex {id} not found")]
    ReflexNotFound { id: ReflexId },

    #[error("LanceDB write failed: {0}")]
    LanceWriteFailed(String),

    #[error("SQLite write failed: {0}")]
    SqliteWriteFailed(String),

    #[error("rigging is in migration state: {phase:?}")]
    MigrationLocked { phase: MigrationPhase },
}

/// Claw-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum ClawError {
    #[error("GPU not available")]
    GpuUnavailable,

    #[error("GPU dispatch failed: {0}")]
    DispatchFailed(String),

    #[error("GPU allocation failed: requested {requested} bytes, available {available}")]
    AllocationFailed { requested: usize, available: usize },

    #[error("GPU timeout after {ms}ms")]
    Timeout { ms: u64 },
}

/// Migration-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("version skew: source {source_ver}, target {target_ver}")]
    VersionSkew { source_ver: String, target_ver: String },

    #[error("embedding model mismatch: source {source_model}, target {target_model}")]
    EmbeddingModelMismatch { source_model: String, target_model: String },

    #[error("snap overflow: rigging needs {needed}MB, shell has {available}MB")]
    SnapOverflow { needed: u64, available: u64 },

    #[error("verification failed: {failed_count}/{total} reflexes failed")]
    VerificationFailed { failed_count: usize, total: usize },

    #[error("handoff timeout after {ms}ms")]
    HandoffTimeout { ms: u64 },

    #[error("consent rejected: {reason}")]
    ConsentRejected { reason: String },
}

/// Constraint validation result (from cudaclaw).
/// This is NOT an error — it's a tri-state that the caller decides how to handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintResult {
    /// Constraint satisfied. Proceed.
    Pass,
    /// Constraint borderline. Log warning, proceed.
    Warn(String),
    /// Constraint violated. Abort or degrade.
    Fail(String),
}

impl ConstraintResult {
    /// Convert to a Result, treating Warn as Ok and Fail as Err.
    pub fn into_result(self) -> Result<(), ConstraintViolation> {
        match self {
            ConstraintResult::Pass => Ok(()),
            ConstraintResult::Warn(msg) => {
                tracing::warn!("constraint warning: {msg}");
                Ok(())
            }
            ConstraintResult::Fail(msg) => Err(ConstraintViolation(msg)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("constraint violation: {0}")]
pub struct ConstraintViolation(String);
```

## The Error Handling Strategy

| Error Category | Strategy | Why |
|---------------|----------|-----|
| Shell errors | Degrade gracefully | The shell is a resource; if it fails, reduce demands |
| Rigging errors | Retry with backoff | Vector DB is eventually consistent; transient failures are normal |
| Claw errors | Fall back to CPU | GPU is always optional; CPU fallback is the safety net |
| Migration errors | Rollback to snapshot | Never leave the system in a half-migrated state |
| Inference errors | Queue and retry | LLM inference may fail; the query can wait |
| Resource exhaustion | Degrade to reflex-only | The system MUST function, even minimally |

**The principle: every error has a recovery path. There is no `unwrap()` in production code. There is no `panic!()` except in test code. The `?` operator is your friend — use it liberally, and let the error propagate to the point that knows how to handle it.**

---

# 8. THE MVP: Cargo.toml Dependency Tree

You have 1GB on a Pi 4. Here's what compiles in.

```toml
# pincher-core/Cargo.toml
[package]
name = "pincher-core"
version = "0.1.0"
edition = "2021"

[dependencies]
# ── CORE (non-negotiable) ──
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "process", "time", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v7"] }
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "1"

# ── STORAGE ──
rusqlite = { version = "0.31", features = ["bundled"], optional = true }
# Note: LanceDB is accessed via Python sidecar, not Rust FFI in MVP.
# The Rust core sends JSON-RPC commands over UDS.

# ── CLI ──
clap = { version = "4", features = ["derive"] }

# ── HARDWARE PROBING ──
sysinfo = "0.30"
sha2 = "0.10"

# ── SANDBOX ──
# bubblewrap is an external binary, not a Rust crate.
# We invoke it via tokio::process::Command.

# ── IPC ──
# JSON-RPC over UDS is hand-rolled with tokio::net::UnixStream.
# No crate needed — it's just length-prefixed JSON frames.

# ── OPTIONAL: GPU ──
cudaclaw = { path = "../cudaclaw", optional = true }

# ── OPTIONAL: Security ──
tenuo-core = { path = "../tenuo", optional = true }

# ── OPTIONAL: Advanced ──
plato-jepa = { path = "../plato-jepa", optional = true }
penrose-memory = { path = "../penrose-memory", optional = true }
a2ui-render = { path = "../a2ui-render", optional = true }
eisenstein = { path = "../eisenstein" }  # Always included: snap math
cocapn-core = { path = "../cocapn-core" }  # Always included: fleet types

[features]
default = ["sqlite", "cli"]
sqlite = ["rusqlite"]
cuda = ["cudaclaw"]
tenuo = ["tenuo-core"]
jepa = ["plato-jepa"]
penrose = ["penrose-memory"]
a2ui = ["a2ui-render"]

# Compound features
rpi4 = ["sqlite", "cli"]
jetson = ["sqlite", "cli", "cuda"]
workstation = ["sqlite", "cli", "cuda", "tenuo", "jepa", "penrose", "a2ui"]
```

## Binary Size Targets

| Feature Set | Binary Size | Dependencies | RAM at Idle |
|-------------|------------|-------------|-------------|
| `rpi4` (MVP) | **~5MB** | tokio, rusqlite, serde, clap, sysinfo, sha2, eisenstein | ~15MB |
| `jetson` | **~7MB** | + cudaclaw, CUDA runtime | ~25MB |
| `workstation` | **~12MB** | + tenuo, jepa, penrose, a2ui | ~40MB |

## What's OUT of the MVP

| Crate | Why OUT | When IN |
|-------|---------|---------|
| `plato-jepa` | No action-conditioned JEPA exists yet | Phase 2 (after custom training) |
| `penrose-memory` | Novel research, not battle-tested | Phase 3 (after POC) |
| `tenuo-core` | Capability security is P1, not P0 | Phase 3 (after basic sandbox works) |
| `a2ui-render` | CLI is sufficient for MVP | Phase 2 (after core loop is solid) |
| `lau-inter-shell` | Single-shell MVP first | Phase 4 (fleet coordination) |
| `turbovec` | Compression is optimization, not MVP | Phase 3 (after storage pressure testing) |

## The Python Sidecar (Not Cargo)

```
pincher-infer/requirements.txt
    lancedb>=0.6
    llama-cpp-python>=0.2
    onnxruntime>=1.17
    sentence-transformers>=2.0
    numpy>=1.26
    pydantic>=2.0
```

This is ~200MB in a venv. It loads lazily and unloads when idle. Total RAM with models: ~1.2GB. Fits on a 4GB Pi with OS overhead.

---

# 9. INTER-PROCESS BOUNDARIES

Shell ↔ Rigging ↔ Claws ↔ Exoskeleton — what are these? Processes? Threads? Async tasks?

**Answer: they're all async tasks in a single process, EXCEPT the Python sidecar and the sandbox executor.**

## The Process Model

```
┌─────────────────────────────────────────────────────────────────┐
│ PROCESS 1: pincher-core (Rust, ~5MB binary)                     │
│                                                                  │
│  THREAD: tokio runtime (4 worker threads)                        │
│  ├── TASK: CoreLoop          (processes user input)              │
│  ├── TASK: ResourceMonitor   (5-second tick)                     │
│  ├── TASK: InferenceBridge   (UDS client to pincher-infer)       │
│  ├── TASK: MigrationManager  (if migration in progress)          │
│  └── TASK: PluginHost        (runs plugin hooks)                 │
│                                                                  │
│  THREAD: stdout writer (CLI output, non-blocking)                │
│  THREAD: signal handler (SIGTERM, SIGINT → graceful shutdown)    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PROCESS 2: pincher-infer (Python, ~200MB venv)                   │
│                                                                  │
│  THREAD: uvicorn (ASGI server on UDS)                            │
│  ├── HANDLER: /embed          (MiniLM-L6 via ONNX)              │
│  ├── HANDLER: /search         (LanceDB vector search)            │
│  ├── HANDLER: /infer          (TinyLlama 1.1B via llama.cpp)    │
│  ├── HANDLER: /learn          (Write reflex to LanceDB)          │
│  └── HANDLER: /health         (Model status, memory usage)       │
│                                                                  │
│  THREAD: model unloader (background, 5-minute idle timeout)      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│ PROCESS 3-N: bwrap sandbox (ephemeral, ~200KB binary)            │
│                                                                  │
│  Created per action execution. Destroyed after exit.              │
│  Communication: stdout/stderr capture via pipe.                   │
│  No shared memory. No network. No persistent state.               │
└─────────────────────────────────────────────────────────────────┘
```

## The FFI Boundaries

| Boundary | Mechanism | Data Format | Latency |
|----------|-----------|-------------|---------|
| Core → Infer | Unix Domain Socket | JSON-RPC 2.0, length-prefixed | ~0.1ms |
| Core → Sandbox | `tokio::process::Command` | stdout/stderr capture | ~50ms (process spawn) |
| Core → GPU (cudaclaw) | `extern "C"` FFI | `#[repr(C)]` structs, unified memory | ~5μs (persistent kernel) |
| Infer → LanceDB | Python FFI | PyArrow + Lance format | ~10ms (disk I/O) |
| Infer → LLM | C FFI (llama.cpp) | GGUF tensors | ~200ms per token (Pi) |
| Core → Fleet (future) | `lau-inter-shell` bus | Custom protocol over TCP | ~1ms (LAN) |

## Why Two Processes (Not One)

The Python sidecar is a separate process because:
1. **Memory isolation**: Python's GC doesn't affect the Rust core's memory
2. **Crash isolation**: A Python segfault (ONNX, llama.cpp) doesn't take down the OS
3. **Model lifecycle**: Python can load/unload models without affecting the core
4. **GIL containment**: The GIL is contained within the Python process

## Why NOT Microservices

PincherOS is NOT a microservice architecture. There are exactly 2 processes (3 if you count the sandbox). If you need more than 2 processes, you've over-engineered it.

The reason: **on a Pi 4, every process costs memory.** A Python process is ~30MB at baseline. Two of them would be 60MB — that's 5% of your total RAM on process overhead alone. One Python process is the right number.

---

# 10. THE CHALLENGE QUESTION

> **"You have a category where every object is a shell and every morphism is a migration. Composition is CRDT merge. You've proven that merge is associative, commutative, and idempotent — so your category is a commutative idempotent monoid, and you've built a beautiful semilattice.**
>
> **Now: a morphism fails. Not 'does not exist' — fails. The network drops mid-migration. Your commuting diagram has a broken arrow. Your cocycle condition is violated.**
>
> **In pure math, you'd say 'the diagram doesn't commute' and move on. But I'm running on a Pi in a basement, and my crab is half-moved between two shells, and my user is waiting.**
>
> **What's the limit of this category? And more importantly: is the limit computable in bounded time on hardware that has 512MB of free RAM and no swap?"**

The category theorist will reach for sheaf cohomology or van Kampen's theorem. The systems programmer knows: **the limit is a timeout, and it's computable in O(1) time because I set the timeout to 30 seconds and I'm done waiting.**

The deep insight: **mathematics doesn't have deadlines. Systems do.** A category-theoretic model of migration that can't handle a broken arrow is a model that doesn't describe reality. The correct formalism isn't a category — it's a **petri net with timeouts**, where every transition has a deadline, and the system is always in a recoverable state regardless of which transitions complete.

In Rust terms:

```rust
/// The migration protocol is NOT a morphism in a category.
/// It's a FINITE STATE MACHINE with TIMEOUTS.
/// Every state has a timeout. Every timeout has a fallback.
/// There is no "broken arrow" — there's only "timeout expired, rolling back."
enum MigrationState {
    Stable { rigging: Rigging },
    Preparing { rigging: Rigging, snapshot: NailFile, deadline: Instant },
    Crossfading { rigging: Rigging, snapshot: NailFile, deadline: Instant },
    Finalized { snapshot: NailFile, retention_deadline: Instant },
    RollingBack { snapshot: NailFile, reason: RollbackReason },
    Failed { reason: RollbackReason, snapshot: NailFile },
}

impl MigrationState {
    fn tick(&mut self, now: Instant) -> Option<MigrationAction> {
        match self {
            MigrationState::Preparing { deadline, .. } if now > *deadline => {
                Some(MigrationAction::Rollback { reason: RollbackReason::PrepareTimeout })
            }
            MigrationState::Crossfading { deadline, .. } if now > *deadline => {
                Some(MigrationAction::Rollback { reason: RollbackReason::CrossfadeTimeout })
            }
            _ => None,
        }
    }
}
```

**The category theorist proves that the diagram commutes. The systems programmer proves that the system recovers when it doesn't.**

---

# APPENDIX: The Const Assertions

Because if it compiles, it's correct. And if the size is wrong, it doesn't compile.

```rust
// GpuCommand is exactly 48 bytes — matches cudaclaw's CUDA layout
const _: () = assert!(std::mem::size_of::<GpuCommand>() == 48);

// Priority fits in a u8 — used in GpuCommand's packed representation
const _: () = assert!(std::mem::size_of::<Priority>() == 1);

// KernelVariant fits in a u8
const _: () = assert!(std::mem::size_of::<KernelVariant>() == 1);

// ReflexMatch is cache-friendly — fits in a cache line (64 bytes)
const _: () = assert!(std::mem::size_of::<ReflexMatch>() <= 64);

// ShellProfile is small enough to clone freely
const _: () = assert!(std::mem::size_of::<ShellProfile>() <= 512);

// DegradationLevel is a single byte
const _: () = assert!(std::mem::size_of::<DegradationLevel>() == 1);
```

---

# SUMMARY: The 10-Point Opinion

1. **Trait Hierarchy**: Four traits — Shell (provide), Rigging (own), Claws (borrow), Exoskeleton (project). The top-level composition is `PincherOs<S, R, C, E>` with generic type parameters. No trait objects in the hot path.

2. **Ownership**: PincherOs owns everything. The core loop takes `&mut self`. The only mutable borrow in the reflex short-circuit path is `&mut self.rigging` for trust updates. Everything else is `&self`.

3. **Zero-Cost**: Internal abstractions (enums, structs, const generics) are zero-cost. Trait dispatch at boundaries (snap, dispatch, render) is near-zero (~5ns). The real cost is in LanceDB vector search (~10ms) and LLM inference (~5s). Optimize those, not the abstractions.

4. **Concurrency**: `tokio::sync::Mutex` for Rigging. Lock-free SPSC only for GPU dispatch. Channels for inter-task communication. `spawn_blocking` for CPU-heavy CRDT merge. Never roll your own lock-free data structure.

5. **Feature Gates**: Seven-tier hierarchy. `rpi4`, `jetson`, and `workstation` as compound features. Additive only. The MVP is `rpi4` = std + sqlite + lancedb + llm-local + embed-onnx + cli + migration.

6. **no_std**: Pincher-core requires `std`. Pincher-types should be `no_std`-compatible. The `StaticRigging` pattern (fixed-size arrays, no heap) is the escape hatch for microcontrollers.

7. **Error Handling**: Enum-based errors, not trait objects. `thiserror` for Display and Error impls. `ConstraintResult` (Pass/Warn/Fail) for cudaclaw. Every error has a recovery path. No `unwrap()` in production.

8. **MVP Binary**: ~5MB for `pincher-core`, ~200MB venv for `pincher-infer`, ~700MB for TinyLlama GGUF. Total: ~1GB. Fits on a Pi 4.

9. **Process Boundaries**: Two processes (Rust core + Python sidecar) + ephemeral sandbox processes. UDS for IPC. `extern "C"` for GPU FFI. No microservices.

10. **The Challenge**: Categories don't have timeouts. Systems do. The correct formalism for migration is a finite state machine with deadlines, not a commuting diagram. The category theorist proves commutativity; the systems programmer proves recoverability.

---

*"The borrow checker is not your enemy. It's the only friend who will tell you the truth about your architecture at 3 AM."*

*— End of Analysis*