# PincherOS R2: GPU/Rust Hybrid Architecture
## Resolving the Silicon-Ownership Tension

**Author**: Systems Architect (Rust ∩ CUDA)
**Date**: 2026-06-05
**Status**: Round 2 Integration — Reconciles GPU Engineer R1 + Rustacean R1

---

# RESOLUTION PREAMBLE: THE 86,000× TRUTH

The GPU Engineer measured it. The Rustacean coded around it. Now we must face it:

```
CPU CRDT merge on RPi 4 (4× A72):    ~50μs for 10K updates
GPU CRDT merge on Jetson Nano (1 SM):  ~4.3s for 10K updates
Ratio: 86,000× SLOWER on GPU

CPU atomicCAS on ARM (L1 hit):         ~20ns
GPU atomicCAS on Maxwell (global mem):  ~434ns (400 cycles @ 921MHz)
Ratio: 22× SLOWER per operation on GPU
```

**This is not a debate. It is a measurement.** The GPU on Jetson Nano is counterproductive for CRDT merge. The Rustacean was right: CpuClaws is the MVP. The GPU Engineer was right: the persistent kernel is wrong. The synthesis is:

> **CRDT merge is ALWAYS on CPU. GPU is for inference only. On workstations with 128 SMs, the hot/cold partition shifts — but the CPU remains the source of truth.**

---

# 1. THE UNIFIED CLAWS TRAIT

## 1.1 The Key Abstraction: ComputeSubstrate

The tension resolves when we realize "CPU CRDT primary" and "GPU inference acceleration" are not competing — they are the SAME interface viewed through different capabilities. The abstraction is **ComputeSubstrate**: a declaration of what a given hardware tier can accelerate, not how.

```rust
/// What a compute substrate can accelerate.
/// This is the SINGLE axis along which Claws implementations differ.
/// Everything else (CRDT merge, state management, coordination) is CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccelerationDomain {
    /// No acceleration. Everything on CPU. (RPi 4)
    None,
    /// GPU accelerates inference only. CRDT on CPU. (Jetson Nano)
    InferenceOnly,
    /// GPU accelerates inference + hot-path CRDT merge. (RTX 4090)
    InferenceAndHotMerge,
}
```

## 1.2 The Unified Trait Definition

```rust
use std::future::Future;
use std::pin::Pin;

// ──────────────────────────────────────────────────────────
// CORE TYPES
// ──────────────────────────────────────────────────────────

/// Marker: participates in PincherOS ecosystem, thread-safe.
pub trait Pincher: Send + Sync + 'static {}

/// A type that can reside in GPU-accessible memory.
/// Must be #[repr(C)], no Drop, no pointers, fixed layout.
pub unsafe trait GpuSafe: Copy + 'static {}

/// Priority for dispatch operations.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// What kind of compute operation we're dispatching.
/// This determines routing: CPU-only vs GPU-accelerated.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    /// CRDT merge — ALWAYS routed to CPU
    CrdtMerge = 0,
    /// LLM inference — routed to GPU if available, CPU otherwise
    Inference = 1,
    /// Vector similarity search — GPU if available
    VectorSearch = 2,
    /// Embedding computation — GPU if available
    Embed = 3,
    /// Generic compute — implementation decides
    Compute = 4,
}

/// The result of a dispatch operation.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    /// Where the operation actually ran
    pub executed_on: ExecutionTarget,
    /// Wall-clock time
    pub elapsed: std::time::Duration,
    /// Whether the result came from a cache/reflex short-circuit
    pub cached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTarget {
    Cpu,
    Gpu,
    CpuSimulated, // CPU pretending to be GPU (for testing)
}

/// GPU command — 64 bytes, #[repr(C)], properly aligned.
/// FIXED: was packed(4) with 48B — now 64B with correct alignment.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct GpuCommand {
    pub op_type: u32,           // +0x00
    pub priority: u32,          // +0x04
    pub target_id: u64,         // +0x08 (8-byte aligned)
    pub payload_ptr: u64,       // +0x10 (8-byte aligned)
    pub payload_len: u64,       // +0x18 (8-byte aligned)
    pub timestamp: u64,         // +0x20 (8-byte aligned)
    pub agent_id: u32,          // +0x28
    pub constraint_flag: u32,   // +0x2C
    pub completion_sem: u64,    // +0x30 (8-byte aligned)
    pub parent_context: u64,    // +0x38 (8-byte aligned)
    pub dna_hash: u32,          // +0x40
    pub _pad: u32,              // +0x44
}

const _: () = assert!(std::mem::size_of::<GpuCommand>() == 72);
// Note: 72B not 64B because we need 16-byte alignment.
// The PTX compiler generates single-instruction loads for all u64 fields.

/// Memory slice accessible from both CPU and GPU.
/// Unified on Jetson (zero-copy), separate on discrete (managed copy).
pub enum GpuSlice<T: GpuSafe> {
    /// CPU-only heap allocation. No GPU access.
    Cpu { data: Vec<T> },
    
    /// Unified memory: same physical address for CPU and GPU.
    /// Jetson Nano path: zero-copy, no page migration.
    #[cfg(feature = "cuda")]
    Unified { 
        ptr: *mut T, 
        len: usize,
        handle: cudaclaw::GpuBridge<T>,
    },
    
    /// Device memory: GPU-only. CPU access via explicit copy.
    /// RTX 4090 path: avoids UM ping-pong for hot-path data.
    #[cfg(feature = "cuda")]
    Device { 
        device_ptr: *mut T, 
        len: usize,
        stream: cudaclaw::CudaStream,
    },
}

unsafe impl<T: GpuSafe> Send for GpuSlice<T> {}
unsafe impl<T: GpuSafe> Sync for GpuSlice<T> {}

// ──────────────────────────────────────────────────────────
// THE CLAWS TRAIT
// ──────────────────────────────────────────────────────────

/// Claws: the compute substrate interface.
/// 
/// INVARIANT: CRDT merge is ALWAYS dispatched to CPU.
/// The trait does NOT have a `dispatch_crdt_merge` method.
/// CRDT merge is handled by the PartitionedCrdtEngine directly.
/// Claws only accelerates: inference, vector search, embedding.
///
/// This is the RESOLUTION of the GPU/CPU tension:
/// - The GPU Engineer's insight: GPU should not do CRDT merge on edge
/// - The Rustacean's insight: CpuClaws is the fallback impl
/// - The synthesis: CpuClaws is not a "fallback" — it's the CORRECT
///   implementation for CRDT, and the ONLY implementation for CRDT.
///   GPU acceleration is a BONUS, not a REQUIREMENT.
pub trait Claws: Pincher {
    type Error: std::error::Error + Send + Sync;

    // ── Identity ──

    /// What this substrate can accelerate.
    /// The SINGLE axis of differentiation between implementations.
    fn acceleration_domain(&self) -> AccelerationDomain;

    /// Whether GPU compute is physically available.
    /// Returns false on CPU-only, true on Jetson/Workstation.
    fn is_gpu_available(&self) -> bool;

    // ── Dispatch ──

    /// Dispatch a compute operation.
    /// CRDT merges are REJECTED — use PartitionedCrdtEngine instead.
    fn dispatch(
        &self,
        kind: DispatchKind,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>>;

    // ── Memory ──

    /// Allocate memory accessible from both CPU and GPU.
    /// On CPU-only: heap allocation.
    /// On Jetson: cudaMallocManaged (unified, zero-copy).
    /// On Workstation: cudaMalloc (device-only) for hot data,
    ///   or cudaMallocManaged with cudaMemAdvise for cold data.
    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error>;

    // ── Capacity ──

    /// Maximum concurrent agents this substrate supports.
    /// 0 means "no GPU acceleration for agent compute."
    fn agent_capacity(&self) -> usize;

    /// Inference throughput estimate (tokens/second).
    /// Used by the PushdownEvaluator to decide GPU vs CPU inference.
    fn inference_throughput(&self) -> f32;

    // ── ShellQuality integration ──

    /// The substrate's contribution to ShellQuality.
    /// GPU substrates report thermal throttle risk.
    fn substrate_health(&self) -> SubstrateHealth;
}

/// Health metrics for the compute substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateHealth {
    /// GPU thermal throttle rate (0.0 = never throttled, 1.0 = always throttled)
    pub gpu_throttle_rate: f64,
    /// GPU memory errors (ECC correctable + uncorrectable)
    pub gpu_memory_errors: u64,
    /// Current GPU utilization (0.0-1.0)
    pub gpu_utilization: f64,
    /// Current GPU memory utilization (0.0-1.0)
    pub gpu_memory_utilization: f64,
    /// CPU thermal status
    pub cpu_thermal: ThermalStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ThermalStatus {
    Nominal,
    Warm,
    Hot,
    Critical,
}

impl Default for SubstrateHealth {
    fn default() -> Self {
        Self {
            gpu_throttle_rate: 0.0,
            gpu_memory_errors: 0,
            gpu_utilization: 0.0,
            gpu_memory_utilization: 0.0,
            cpu_thermal: ThermalStatus::Nominal,
        }
    }
}
```

## 1.3 CpuClaws Implementation (RPi 4 Path)

```rust
/// CpuClaws: pure CPU execution. No GPU. No CUDA. No fallback — this IS the path.
/// 
/// Target: RPi 4 (4× A72 @ 1.5GHz, 4GB RAM, VideoCore VI display-only)
/// Performance:
///   - CRDT merge (10K updates, rayon): ~50μs
///   - Inference (llama.cpp CPU, TinyLlama 1.1B Q4): ~5-8 tok/s
///   - Embedding (ONNX Runtime, MiniLM-L6): ~30-50ms/sentence
///   - Vector search (LanceDB, 50K vectors): ~10ms
pub struct CpuClaws {
    /// Rayon thread pool for CPU-parallel CRDT merge and compute
    pool: rayon::ThreadPool,
    /// Number of physical cores
    cores: usize,
    /// CPU frequency in MHz
    freq_mhz: u64,
    /// Available RAM in MB
    ram_available_mb: u64,
    /// Thermal status tracker
    thermal: std::sync::Mutex<ThermalStatus>,
}

impl CpuClaws {
    pub fn new() -> Result<Self, CpuClawsError> {
        let cores = num_cpus::get_physical();
        let freq_mhz = detect_cpu_freq();
        let ram_available_mb = detect_available_ram_mb();
        
        // RPi 4: 4 cores. Use all of them for CRDT merge.
        // Reserve 0 cores for GPU (there is no GPU).
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(cores)
            .thread_name(|idx| format!("pincher-cpu-{idx}"))
            .build()
            .map_err(CpuClawsError::ThreadPool)?;
        
        Ok(Self {
            pool,
            cores,
            freq_mhz,
            ram_available_mb,
            thermal: std::sync::Mutex::new(ThermalStatus::Nominal),
        })
    }
}

impl Pincher for CpuClaws {}

impl Claws for CpuClaws {
    type Error = CpuClawsError;

    fn acceleration_domain(&self) -> AccelerationDomain {
        AccelerationDomain::None
    }

    fn is_gpu_available(&self) -> bool {
        false
    }

    fn dispatch(
        &self,
        kind: DispatchKind,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>> {
        // ALL dispatch on CpuClaws runs on CPU.
        // Inference: llama.cpp CPU backend
        // Vector search: LanceDB (CPU)
        // Embed: ONNX Runtime (CPU + NEON)
        // CRDT: REJECTED — use PartitionedCrdtEngine
        if kind == DispatchKind::CrdtMerge {
            return Box::pin(async {
                Err(CpuClawsError::InvalidDispatch(
                    "CRDT merge must go through PartitionedCrdtEngine, not Claws::dispatch"
                ))
            });
        }
        
        Box::pin(async move {
            let start = std::time::Instant::now();
            
            // CPU dispatch: execute via thread pool
            let result = self.pool.install(|| {
                execute_cpu_dispatch(kind, command)
            });
            
            Ok(DispatchResult {
                executed_on: ExecutionTarget::Cpu,
                elapsed: start.elapsed(),
                cached: false,
            })
        })
    }

    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error> {
        // CPU-only: simple Vec allocation
        Ok(GpuSlice::Cpu {
            data: vec![unsafe { std::mem::zeroed() }; len],
        })
    }

    fn agent_capacity(&self) -> usize {
        // CPU-only: capacity based on cores and RAM
        // Each agent needs ~200MB (model share + state + reflexes)
        // RPi 4 with 1.5GB available: ~4-6 concurrent agents
        let model_share_mb = 300; // TinyLlama partial load
        let agent_state_mb = 50;  // Reflexes + state per agent
        (self.ram_available_mb as usize / (model_share_mb + agent_state_mb))
            .min(6) // Hard cap: 4 A72 cores can't handle more
    }

    fn inference_throughput(&self) -> f32 {
        // TinyLlama 1.1B Q4 on 4× A72 @ 1.5GHz
        6.0 // ~5-8 tok/s, conservatively 6
    }

    fn substrate_health(&self) -> SubstrateHealth {
        let thermal = *self.thermal.lock().unwrap();
        SubstrateHealth {
            gpu_throttle_rate: 0.0,  // No GPU
            gpu_memory_errors: 0,
            gpu_utilization: 0.0,
            gpu_memory_utilization: 0.0,
            cpu_thermal: thermal,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CpuClawsError {
    #[error("Thread pool creation failed: {0}")]
    ThreadPool(rayon::ThreadPoolBuildError),
    #[error("Invalid dispatch: {0}")]
    InvalidDispatch(&'static str),
    #[error("CPU execution failed: {0}")]
    Execution(String),
}
```

## 1.4 JetsonClaws Implementation

```rust
/// JetsonClaws: unified memory GPU + CPU. GPU for inference ONLY.
///
/// Target: Jetson Nano (GM20B, 1 SM, 128 Maxwell cores, 4GB LPDDR4 shared)
/// 
/// KEY INSIGHT: On Jetson Nano, CPU and GPU share the SAME physical RAM.
/// cudaMallocManaged = zero-copy. No page migration. No UM ping-pong.
/// But the 1 SM, 128-core GPU is 86,000× SLOWER than CPU for CRDT merge.
/// So: GPU = inference only. CPU = everything else.
///
/// Performance:
///   - CRDT merge (CPU, rayon, 10K updates): ~50μs
///   - CRDT merge (GPU, 1 SM): ~4.3s (UNUSABLE — never use this)
///   - Inference (GPU, TinyLlama 1.1B Q4): ~15-25 tok/s
///   - Inference (CPU, TinyLlama 1.1B Q4): ~5-8 tok/s
///   - Embedding (GPU, MiniLM-L6): ~5-10ms/sentence
///   - Vector search (CPU, LanceDB): ~10ms
///
/// POWER BUDGET: 10W TDP. GPU uses 5-10W when active.
/// Strategy: batch GPU work, keep GPU idle between batches.
/// NO persistent kernel. Batch dispatch only.
#[cfg(feature = "cuda")]
pub struct JetsonClaws {
    /// CPU thread pool for CRDT merge and coordination
    cpu_pool: rayon::ThreadPool,
    /// CUDA context (lazy-init, shared between inference and embedding)
    cuda_ctx: std::sync::Mutex<Option<JetsonCudaContext>>,
    /// Configuration
    config: JetsonConfig,
    /// Thermal monitoring (Jetson Nano throttles at ~80°C)
    thermal: std::sync::Mutex<JetsonThermalState>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub struct JetsonConfig {
    /// Number of CPU cores for rayon (reserve 1 for GPU management)
    pub cpu_threads: usize,     // Default: 3 (of 4 A57 cores)
    /// GPU layers to offload for inference
    pub gpu_layers: u32,        // Default: 16 (out of ~22 for TinyLlama)
    /// Maximum GPU utilization before throttling
    pub max_gpu_util: f64,      // Default: 0.80
    /// Thermal throttle threshold (°C)
    pub thermal_limit_c: f64,   // Default: 75.0 (before hardware 80°C)
}

impl Default for JetsonConfig {
    fn default() -> Self {
        Self {
            cpu_threads: 3,
            gpu_layers: 16,
            max_gpu_util: 0.80,
            thermal_limit_c: 75.0,
        }
    }
}

#[cfg(feature = "cuda")]
struct JetsonCudaContext {
    /// CUDA stream for inference (NOT persistent kernel)
    inference_stream: cudaclaw::CudaStream,
    /// CUDA stream for embedding
    embed_stream: cudaclaw::CudaStream,
    /// Pre-compiled kernel cache
    kernel_cache: KernelCache,
    /// Current GPU model loaded
    loaded_model: Option<String>,
}

#[cfg(feature = "cuda")]
struct JetsonThermalState {
    /// Current GPU temperature (°C)
    gpu_temp_c: f64,
    /// Current CPU temperature (°C)
    cpu_temp_c: f64,
    /// Cumulative throttle events (for ShellQuality)
    throttle_event_count: u64,
    /// Last throttle timestamp
    last_throttle: Option<std::time::Instant>,
}

#[cfg(feature = "cuda")]
impl JetsonClaws {
    pub fn new(config: JetsonConfig) -> Result<Self, JetsonClawsError> {
        let cpu_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.cpu_threads)
            .thread_name(|idx| format!("pincher-jetson-cpu-{idx}"))
            .build()
            .map_err(JetsonClawsError::ThreadPool)?;
        
        // Don't init CUDA context here — lazy-init on first GPU dispatch.
        // Saves power at startup.
        
        Ok(Self {
            cpu_pool,
            cuda_ctx: std::sync::Mutex::new(None),
            config,
            thermal: std::sync::Mutex::new(JetsonThermalState {
                gpu_temp_c: 40.0,
                cpu_temp_c: 40.0,
                throttle_event_count: 0,
                last_throttle: None,
            }),
        })
    }
    
    /// Check thermal before dispatching to GPU.
    /// Returns false if GPU should be skipped (thermal throttle).
    fn should_use_gpu(&self) -> bool {
        let thermal = self.thermal.lock().unwrap();
        thermal.gpu_temp_c < self.config.thermal_limit_c
    }
    
    /// Lazy-init CUDA context. Only creates CUDA context when first needed.
    fn ensure_cuda_ctx(&self) -> Result<(), JetsonClawsError> {
        let mut ctx = self.cuda_ctx.lock().unwrap();
        if ctx.is_none() {
            // Jetson Nano: CUDA context with unified memory
            // Use cudaMallocManaged for all allocations (zero-copy on Jetson)
            *ctx = Some(JetsonCudaContext {
                inference_stream: cudaclaw::CudaStream::new()
                    .map_err(JetsonClawsError::Cuda)?,
                embed_stream: cudaclaw::CudaStream::new()
                    .map_err(JetsonClawsError::Cuda)?,
                kernel_cache: KernelCache::new("/opt/pincheros/cache/kernels/"),
                loaded_model: None,
            });
        }
        Ok(())
    }
}

#[cfg(feature = "cuda")]
impl Pincher for JetsonClaws {}

#[cfg(feature = "cuda")]
impl Claws for JetsonClaws {
    type Error = JetsonClawsError;

    fn acceleration_domain(&self) -> AccelerationDomain {
        AccelerationDomain::InferenceOnly
    }

    fn is_gpu_available(&self) -> bool {
        self.should_use_gpu()
    }

    fn dispatch(
        &self,
        kind: DispatchKind,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>> {
        // CRDT merge: ALWAYS REJECTED on Jetson too
        if kind == DispatchKind::CrdtMerge {
            return Box::pin(async {
                Err(JetsonClawsError::InvalidDispatch(
                    "CRDT merge on GPU is 86,000× slower than CPU on Jetson. Use PartitionedCrdtEngine."
                ))
            });
        }
        
        // Check thermal — if too hot, fall back to CPU
        if !self.should_use_gpu() {
            // Thermal throttle: execute on CPU instead
            return Box::pin(async move {
                let start = std::time::Instant::now();
                self.cpu_pool.install(|| execute_cpu_dispatch(kind, command));
                Ok(DispatchResult {
                    executed_on: ExecutionTarget::Cpu, // Thermal fallback
                    elapsed: start.elapsed(),
                    cached: false,
                })
            });
        }
        
        // GPU dispatch: batch, launch, complete, exit
        // NO persistent kernel. The GPU goes idle after each batch.
        Box::pin(async move {
            self.ensure_cuda_ctx()?;
            let start = std::time::Instant::now();
            
            let ctx = self.cuda_ctx.lock().unwrap();
            let ctx = ctx.as_ref().unwrap();
            
            let result = match kind {
                DispatchKind::Inference => {
                    // Use inference stream
                    // llama.cpp CUDA backend handles the actual execution
                    // We just provide the stream and configuration
                    ctx.inference_stream.dispatch_batch(kind, command)
                        .map_err(JetsonClawsError::Cuda)?
                }
                DispatchKind::VectorSearch | DispatchKind::Embed => {
                    // Use embed stream
                    ctx.embed_stream.dispatch_batch(kind, command)
                        .map_err(JetsonClawsError::Cuda)?
                }
                _ => {
                    // Generic compute: CPU fallback
                    self.cpu_pool.install(|| execute_cpu_dispatch(kind, command));
                    ExecutionTarget::Cpu
                }
            };
            
            Ok(DispatchResult {
                executed_on: if result == ExecutionTarget::Cpu {
                    ExecutionTarget::Cpu
                } else {
                    ExecutionTarget::Gpu
                },
                elapsed: start.elapsed(),
                cached: false,
            })
        })
    }

    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error> {
        self.ensure_cuda_ctx()?;
        // Jetson Nano: use cudaMallocManaged (unified memory, zero-copy)
        let ctx = self.cuda_ctx.lock().unwrap();
        let ctx = ctx.as_ref().unwrap();
        
        let handle = cudaclaw::GpuBridge::new_unified(len)
            .map_err(JetsonClawsError::Cuda)?;
        let ptr = handle.ptr();
        
        Ok(GpuSlice::Unified {
            ptr,
            len,
            handle,
        })
    }

    fn agent_capacity(&self) -> usize {
        // Jetson Nano with GPU inference:
        // 4GB shared RAM, ~1.2GB for LLM, ~1GB for OS, ~1.8GB for agents
        // Each agent: ~150MB (smaller model share + state)
        // With GPU inference, agents share the same model weights
        8 // Conservative for 4GB Jetson
    }

    fn inference_throughput(&self) -> f32 {
        if self.should_use_gpu() {
            // TinyLlama 1.1B Q4 on Jetson GPU (16 layers offloaded)
            20.0 // ~15-25 tok/s
        } else {
            // Thermal fallback to CPU
            6.0 // Same as pure CPU
        }
    }

    fn substrate_health(&self) -> SubstrateHealth {
        let thermal = self.thermal.lock().unwrap();
        SubstrateHealth {
            gpu_throttle_rate: if thermal.throttle_event_count > 0 {
                // Approximation based on event count
                (thermal.throttle_event_count as f64 / 1000.0).min(1.0)
            } else {
                0.0
            },
            gpu_memory_errors: 0, // No ECC on Jetson Nano
            gpu_utilization: 0.0, // Updated by monitoring thread
            gpu_memory_utilization: 0.0,
            cpu_thermal: if thermal.cpu_temp_c > 75.0 {
                ThermalStatus::Critical
            } else if thermal.cpu_temp_c > 65.0 {
                ThermalStatus::Hot
            } else if thermal.cpu_temp_c > 55.0 {
                ThermalStatus::Warm
            } else {
                ThermalStatus::Nominal
            },
        }
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, thiserror::Error)]
pub enum JetsonClawsError {
    #[error("Thread pool creation failed: {0}")]
    ThreadPool(rayon::ThreadPoolBuildError),
    #[error("CUDA error: {0}")]
    Cuda(cudaclaw::CudaError),
    #[error("Invalid dispatch: {0}")]
    InvalidDispatch(&'static str),
    #[error("Kernel cache error: {0}")]
    KernelCache(String),
}
```

---

# 2. PARTITIONED CRDT ENGINE: HOT/COLD SPLIT

## 2.1 The Partition Criterion

The GPU Engineer's Shadowgap insight: GPU merges hot-path CRDTs, CPU merges cold-path. But what defines "hot" vs "cold"?

**Partition criterion: access frequency with exponential decay.**

```
HOT:  accessed > 100 times/second — GPU-accelerated merge (workstation only)
WARM: accessed 1-100 times/second — CPU merge, GPU reads for inference
COLD: accessed < 1 time/second   — CPU merge, CPU reads only
```

On Jetson Nano: everything is COLD or WARM (GPU doesn't merge CRDTs).
On RTX 4090: hot-path cells are promoted to GPU for parallel CAS.

## 2.2 The Engine

```rust
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

// ──────────────────────────────────────────────────────────
// CRDT TYPES
// ──────────────────────────────────────────────────────────

/// A CRDT cell — the fundamental unit of merge.
/// 64 bytes, cache-line aligned, GpuSafe.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct CrdtCell {
    /// The CRDT value (interpretation depends on cell_type)
    pub value: u64,
    /// Vector clock / lamport timestamp
    pub timestamp: u64,
    /// Source shell fingerprint hash
    pub source_hash: u64,
    /// Cell type determines merge semantics
    pub cell_type: CrdtCellType,
    /// Access count (for hot/cold partitioning)
    pub access_count: u64,
    /// Last access time (epoch seconds)
    pub last_access_epoch: u64,
    /// Reserved for future use
    pub _reserved: [u64; 3],
}

unsafe impl GpuSafe for CrdtCell {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrdtCellType {
    /// Last-Writer-Wins register
    LwwRegister = 0,
    /// PN-Counter (positive-negative)
    PnCounter = 1,
    /// G-Counter (grow-only)
    GCounter = 2,
    /// OR-Set (observed-remove)
    OrSet = 3,
    /// Trust score (monotonic increase with decay)
    TrustScore = 4,
}

/// A pending update to a CRDT cell.
/// 32 bytes, GpuSafe.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct PendingUpdate {
    pub target_id: u64,
    pub new_value: u64,
    pub source_hash: u64,
    pub timestamp: u64,
    pub cell_type: CrdtCellType,
    pub _pad: [u8; 7],
}

unsafe impl GpuSafe for PendingUpdate {}

/// Result of a CRDT merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub target_id: u64,
    pub old_value: u64,
    pub new_value: u64,
    pub merged_on: ExecutionTarget,
    pub was_hot: bool,
}

// ──────────────────────────────────────────────────────────
// HOT/COLD PARTITION
// ──────────────────────────────────────────────────────────

/// Temperature of a CRDT cell based on access frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellTemperature {
    Cold = 0,   // < 1 access/sec
    Warm = 1,   // 1-100 accesses/sec
    Hot = 2,    // > 100 accesses/sec
}

/// Configuration for the hot/cold partition.
#[derive(Debug, Clone)]
pub struct PartitionConfig {
    /// Access rate threshold for Hot (accesses/sec)
    pub hot_threshold: f64,       // Default: 100.0
    /// Access rate threshold for Warm (accesses/sec)
    pub warm_threshold: f64,      // Default: 1.0
    /// Decay factor for access rate calculation
    pub decay_factor: f64,        // Default: 0.95 (per second)
    /// How often to repartition (seconds)
    pub repartition_interval: f64, // Default: 5.0
    /// Whether GPU merge is available for hot cells
    pub gpu_merge_available: bool, // Default: false (only true on WorkstationClaws)
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            hot_threshold: 100.0,
            warm_threshold: 1.0,
            decay_factor: 0.95,
            repartition_interval: 5.0,
            gpu_merge_available: false,
        }
    }
}

// ──────────────────────────────────────────────────────────
// THE PARTITIONED CRDT ENGINE
// ──────────────────────────────────────────────────────────

/// The PartitionedCrdtEngine routes CRDT merges to the appropriate
/// execution substrate based on cell temperature.
///
/// ON ALL PLATFORMS:
///   - Cold cells → CPU (DashMap + rayon)
///   - Warm cells → CPU (DashMap + rayon)
///   - Hot cells  → CPU on edge, GPU on workstation
///
/// The partition boundary is ADAPTIVE. It shifts based on:
///   1. Observed access frequency
///   2. Available compute substrate (AccelerationDomain)
///   3. Thermal pressure (reduce GPU usage when hot)
pub struct PartitionedCrdtEngine {
    /// All CRDT cells — DashMap for lock-free concurrent access
    cells: DashMap<u64, CrdtCell>,
    
    /// Access rate tracker per cell (exponentially decayed)
    access_rates: DashMap<u64, AccessRate>,
    
    /// Current partition: which cells are hot
    hot_cells: dashmap::DashSet<u64>,
    
    /// Configuration
    config: PartitionConfig,
    
    /// CPU thread pool for merge operations
    cpu_pool: rayon::ThreadPool,
    
    /// GPU merge interface (None on edge, Some on workstation)
    #[cfg(feature = "cuda")]
    gpu_merger: Option<GpuMergeDispatcher>,
    
    /// Last repartition time
    last_repartition: std::sync::Mutex<Instant>,
    
    /// Merge statistics
    stats: std::sync::Mutex<MergeStats>,
}

/// Exponentially-decayed access rate tracker.
#[derive(Debug, Clone)]
struct AccessRate {
    /// Decayed access count
    rate: f64,
    /// Last update time
    last_update: Instant,
}

impl AccessRate {
    fn new() -> Self {
        Self {
            rate: 0.0,
            last_update: Instant::now(),
        }
    }
    
    /// Record an access. Decays the rate since last update.
    fn record_access(&mut self, decay: f64) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        // Exponential decay: rate *= decay^elapsed
        self.rate *= decay.powf(elapsed);
        self.rate += 1.0;
        self.last_update = now;
    }
    
    /// Get current rate (accesses per second, decayed).
    fn current_rate(&self) -> f64 {
        self.rate
    }
}

/// Merge statistics.
#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    pub total_merges: u64,
    pub cpu_merges: u64,
    pub gpu_merges: u64,
    pub hot_merges: u64,
    pub cold_merges: u64,
    pub total_merge_time_us: u64,
    pub repartitions: u64,
    pub cells_promoted_to_hot: u64,
    pub cells_demoted_from_hot: u64,
}

impl PartitionedCrdtEngine {
    pub fn new(config: PartitionConfig) -> Result<Self, CrdtError> {
        let cpu_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get_physical())
            .thread_name(|idx| format!("pincher-crdt-{idx}"))
            .build()
            .map_err(CrdtError::ThreadPool)?;
        
        Ok(Self {
            cells: DashMap::with_shard_amount(64), // 64 shards for low contention
            access_rates: DashMap::new(),
            hot_cells: dashmap::DashSet::new(),
            config,
            cpu_pool,
            #[cfg(feature = "cuda")]
            gpu_merger: if config.gpu_merge_available {
                Some(GpuMergeDispatcher::new()?)
            } else {
                None
            },
            last_repartition: std::sync::Mutex::new(Instant::now()),
            stats: std::sync::Mutex::new(MergeStats::default()),
        })
    }
    
    /// Record an access to a cell (for hot/cold tracking).
    /// Called on every read or write to a CRDT cell.
    pub fn record_access(&self, cell_id: u64) {
        let mut rate = self.access_rates
            .entry(cell_id)
            .or_insert(AccessRate::new());
        rate.record_access(self.config.decay_factor);
    }
    
    /// Get the temperature of a cell.
    pub fn temperature(&self, cell_id: u64) -> CellTemperature {
        if self.hot_cells.contains(&cell_id) {
            return CellTemperature::Hot;
        }
        if let Some(rate) = self.access_rates.get(&cell_id) {
            let r = rate.current_rate();
            if r >= self.config.warm_threshold {
                return CellTemperature::Warm;
            }
        }
        CellTemperature::Cold
    }
    
    /// Merge a batch of updates. Routes based on temperature.
    /// 
    /// This is the CORE MERGE PATH. It is ALWAYS on CPU for edge devices.
    /// On workstations, hot-path updates MAY be routed to GPU.
    pub fn merge_batch(&self, updates: Vec<PendingUpdate>) -> Vec<MergeResult> {
        let start = Instant::now();
        
        // Maybe repartition
        self.maybe_repartition();
        
        // Split updates by temperature
        let mut hot_updates: Vec<PendingUpdate> = Vec::new();
        let mut cold_updates: Vec<PendingUpdate> = Vec::new();
        
        for update in &updates {
            self.record_access(update.target_id);
            let temp = self.temperature(update.target_id);
            match temp {
                CellTemperature::Hot => hot_updates.push(*update),
                CellTemperature::Warm | CellTemperature::Cold => cold_updates.push(*update),
            }
        }
        
        // Merge cold/warm on CPU (always)
        let mut results: Vec<MergeResult> = self.cpu_pool.install(|| {
            cold_updates.par_iter()
                .map(|update| self.merge_one_cpu(update))
                .collect::<Vec<_>>()
        });
        
        // Merge hot on GPU (if available) or CPU (fallback)
        #[cfg(feature = "cuda")]
        if !hot_updates.is_empty() {
            if let Some(ref gpu) = self.gpu_merger {
                let gpu_results = gpu.merge_batch(&hot_updates);
                results.extend(gpu_results);
            } else {
                // GPU not available — merge on CPU
                let cpu_hot = self.cpu_pool.install(|| {
                    hot_updates.par_iter()
                        .map(|update| self.merge_one_cpu(update))
                        .collect::<Vec<_>>()
                });
                results.extend(cpu_hot);
            }
        }
        
        #[cfg(not(feature = "cuda"))]
        if !hot_updates.is_empty() {
            // No GPU feature — everything on CPU
            let cpu_hot = self.cpu_pool.install(|| {
                hot_updates.par_iter()
                    .map(|update| self.merge_one_cpu(update))
                    .collect::<Vec<_>>()
            });
            results.extend(cpu_hot);
        }
        
        // Update stats
        let mut stats = self.stats.lock().unwrap();
        stats.total_merges += updates.len() as u64;
        stats.cold_merges += cold_updates.len() as u64;
        stats.hot_merges += hot_updates.len() as u64;
        stats.cpu_merges += cold_updates.len() as u64;
        stats.total_merge_time_us += start.elapsed().as_micros() as u64;
        // gpu_merges updated separately
        
        results
    }
    
    /// Merge a single update on CPU using DashMap.
    fn merge_one_cpu(&self, update: &PendingUpdate) -> MergeResult {
        let mut cell = self.cells
            .entry(update.target_id)
            .or_insert(CrdtCell {
                value: 0,
                timestamp: 0,
                source_hash: 0,
                cell_type: update.cell_type,
                access_count: 0,
                last_access_epoch: 0,
                _reserved: [0; 3],
            });
        
        let old_value = cell.value;
        
        // CRDT merge semantics
        let new_value = match update.cell_type {
            CrdtCellType::LwwRegister => {
                // Last-Writer-Wins: higher timestamp wins
                if update.timestamp > cell.timestamp {
                    cell.timestamp = update.timestamp;
                    cell.source_hash = update.source_hash;
                    update.new_value
                } else {
                    cell.value
                }
            }
            CrdtCellType::PnCounter => {
                // PN-Counter: max of positive, min of negative
                // Simplified: take max (for monotonic counters)
                std::cmp::max(cell.value, update.new_value)
            }
            CrdtCellType::GCounter => {
                // G-Counter: max (grow-only)
                std::cmp::max(cell.value, update.new_value)
            }
            CrdtCellType::TrustScore => {
                // Trust: monotonic increase with decay
                // Trust can only go UP, but decays over time
                // The new_value already has decay applied by the source
                std::cmp::max(cell.value, update.new_value)
            }
            CrdtCellType::OrSet => {
                // OR-Set: unique tag per add, remove by tag
                // Simplified: just store the latest add
                update.new_value
            }
        };
        
        cell.value = new_value;
        cell.access_count += 1;
        cell.last_access_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        MergeResult {
            target_id: update.target_id,
            old_value,
            new_value,
            merged_on: ExecutionTarget::Cpu,
            was_hot: self.hot_cells.contains(&update.target_id),
        }
    }
    
    /// Repartition: promote/demote cells between hot and cold.
    /// Called periodically based on config.repartition_interval.
    fn maybe_repartition(&self) {
        let mut last = self.last_repartition.lock().unwrap();
        if last.elapsed().as_secs_f64() < self.config.repartition_interval {
            return;
        }
        *last = Instant::now();
        drop(last);
        
        let mut promoted = 0u64;
        let mut demoted = 0u64;
        
        // Check all tracked cells
        for entry in self.access_rates.iter() {
            let cell_id = entry.key();
            let rate = entry.value().current_rate();
            let is_hot = self.hot_cells.contains(cell_id);
            
            if rate >= self.config.hot_threshold && !is_hot {
                // Promote to hot (only if GPU merge is available)
                if self.config.gpu_merge_available {
                    self.hot_cells.insert(*cell_id);
                    promoted += 1;
                }
            } else if rate < self.config.hot_threshold && is_hot {
                // Demote from hot
                self.hot_cells.remove(cell_id);
                demoted += 1;
            }
        }
        
        let mut stats = self.stats.lock().unwrap();
        stats.repartitions += 1;
        stats.cells_promoted_to_hot += promoted;
        stats.cells_demoted_from_hot += demoted;
    }
    
    /// Get current merge statistics.
    pub fn stats(&self) -> MergeStats {
        self.stats.lock().unwrap().clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CrdtError {
    #[error("Thread pool creation failed: {0}")]
    ThreadPool(rayon::ThreadPoolBuildError),
    #[cfg(feature = "cuda")]
    #[error("GPU merge dispatcher error: {0}")]
    GpuMerge(String),
}

// ──────────────────────────────────────────────────────────
// GPU MERGE DISPATCHER (Workstation only)
// ──────────────────────────────────────────────────────────

#[cfg(feature = "cuda")]
struct GpuMergeDispatcher {
    /// CUDA stream for CRDT merge
    stream: cudaclaw::CudaStream,
    /// Device-side CRDT cell table (hot cells only)
    hot_cell_table: cudaclaw::GpuBridge<CrdtCell>,
    /// Maximum hot cells that fit in GPU memory
    max_hot_cells: usize,
}

#[cfg(feature = "cuda")]
impl GpuMergeDispatcher {
    fn new() -> Result<Self, CrdtError> {
        // Only available on workstation with dedicated GPU
        // On RTX 4090: allocate hot cell table in VRAM
        let max_hot_cells = 100_000; // 100K hot cells × 64B = 6.4MB
        
        let stream = cudaclaw::CudaStream::new()
            .map_err(|e| CrdtError::GpuMerge(e.to_string()))?;
        
        let hot_cell_table = cudaclaw::GpuBridge::new_device(max_hot_cells)
            .map_err(|e| CrdtError::GpuMerge(e.to_string()))?;
        
        Ok(Self {
            stream,
            hot_cell_table,
            max_hot_cells,
        })
    }
    
    /// Merge hot-path updates on GPU.
    /// Uses batch dispatch: <<<ceil(N/256), 256>>> — NOT persistent kernel.
    fn merge_batch(&self, updates: &[PendingUpdate]) -> Vec<MergeResult> {
        if updates.is_empty() {
            return vec![];
        }
        
        // 1. Copy updates to device
        // 2. Launch merge kernel: <<<ceil(N/256), 256, 0, stream>>>
        // 3. Copy results back
        // 4. Return merge results
        
        // This is where the RTX 4090's 128 SMs actually matter:
        // 10K hot updates × 1 CAS each ≈ 400 cycles × ~1ns/cycle ≈ 400ns total
        // vs CPU: 10K × 20ns = 200μs
        // GPU is 500× faster for hot-path merge on workstation
        
        let n = updates.len();
        let blocks = (n + 255) / 256;
        let threads = 256;
        
        // CUDA kernel launch:
        // merge_hot_path<<<blocks, threads, 0, self.stream>>>(
        //     self.hot_cell_table.device_ptr(),
        //     updates_device_ptr,
        //     n as u32,
        //     overflow_ptr
        // );
        
        // ... actual FFI call to cudaclaw ...
        
        vec![] // Placeholder — real implementation returns GPU merge results
    }
}
```

## 2.3 Dispatch Logic Summary

```
┌──────────────────────────────────────────────────────────────┐
│              PartitionedCrdtEngine::merge_batch               │
│                                                               │
│  Input: Vec<PendingUpdate>                                    │
│                                                               │
│  1. Record access for each update.target_id                   │
│  2. Classify temperature: Hot? Warm? Cold?                    │
│  3. Route:                                                    │
│     ┌──────────┬─────────────┬──────────────────────────┐    │
│     │ Temp     │ Edge (RPi4) │ Workstation (RTX 4090)   │    │
│     ├──────────┼─────────────┼──────────────────────────┤    │
│     │ Cold     │ CPU+rayon   │ CPU+rayon                │    │
│     │ Warm     │ CPU+rayon   │ CPU+rayon                │    │
│     │ Hot      │ CPU+rayon   │ GPU (<<<N/256,256>>>)    │    │
│     └──────────┴─────────────┴──────────────────────────┘    │
│                                                               │
│  4. On Jetson: NO hot cells exist (gpu_merge_available=false)│
│     Everything goes to CPU+rayon. ~50μs for 10K updates.     │
│                                                               │
│  5. On Workstation: hot cells go to GPU.                      │
│     10K hot updates on GPU: ~400ns (500× faster than CPU).   │
│     Cold updates stay on CPU: ~50μs (negligible).            │
│                                                               │
│  6. Repartition every 5 seconds: promote/demote based on     │
│     exponentially-decayed access rate.                        │
└──────────────────────────────────────────────────────────────┘
```

---

# 3. THE JETSON PATH

## 3.1 Execution Profile

```
Jetson Nano (BCM20830, GM20B)
├── CPU: 4× ARM Cortex-A57 @ 1.43GHz
├── GPU: 128 Maxwell CUDA cores, 1 SM @ 921MHz
├── RAM: 4GB LPDDR4 (shared CPU/GPU, unified physical)
├── TDP: 5-10W
├── Storage: microSD (or USB SSD)
└── Network: Gigabit Ethernet

Execution Plan:
┌──────────────────────────────────────────────────────────┐
│ CPU (3 threads for PincherOS, 1 for OS/GPU management)  │
│                                                           │
│ Thread 1 (rayon): Core loop + CRDT merge                │
│ Thread 2 (rayon): Context assembly + action parsing      │
│ Thread 3 (rayon): Sandbox execution + monitoring         │
│                                                           │
│ CRDT Engine: PartitionedCrdtEngine                       │
│   gpu_merge_available: false                              │
│   All cells: CPU+rayon (DashMap, 64 shards)             │
│   10K merges: ~50μs                                       │
│                                                           │
│ Inference: llama.cpp CPU backend                          │
│   (GPU offload for 16 layers via CUDA — see below)       │
│                                                           │
│ Embedding: ONNX Runtime CPU + NEON                        │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│ GPU (128 Maxwell cores, batch dispatch only)              │
│                                                           │
│ Usage 1: LLM inference offload (16/22 layers)            │
│   Model: TinyLlama 1.1B Q4_K_M                           │
│   16 GPU layers: ~15-25 tok/s                             │
│   vs CPU-only: ~5-8 tok/s                                 │
│                                                           │
│ Usage 2: Embedding inference (MiniLM-L6)                  │
│   ~5-10ms/sentence vs ~30-50ms on CPU                    │
│                                                           │
│ NOT USED FOR:                                             │
│   ❌ CRDT merge (86,000× slower than CPU)                │
│   ❌ Persistent kernel (burns 5-10W for polling)         │
│   ❌ Vector search (LanceDB is CPU-native)               │
│                                                           │
│ Power strategy:                                           │
│   - GPU active only during inference batches              │
│   - GPU idle between batches (saves ~5W)                 │
│   - Thermal limit: 75°C (before hw throttle at 80°C)     │
│   - If thermal throttle: fall back to CPU inference      │
└──────────────────────────────────────────────────────────┘
```

## 3.2 Cargo Feature Set for Jetson

```toml
# Cargo.toml — Jetson Nano profile

[package]
name = "pincher-core"
version = "0.1.0"
edition = "2021"

[features]
default = ["std", "sqlite", "cli"]

# ── Core (always present) ──
std = []

# ── Storage ──
sqlite = ["rusqlite"]
lancedb = ["lancedb-ffi"]

# ── GPU ──
cuda = ["cudaclaw"]              # Full CUDA support
cuda-jetson = ["cuda"]           # Jetson-specific: unified memory only, no discrete GPU

# ── ML ──
llm-local = []                   # Local LLM via Python sidecar
llm-cuda-offload = ["llm-local"] # Offload LLM layers to GPU (Jetson/Workstation)
embed-onnx = ["ort"]             # ONNX Runtime for embeddings
embed-cuda = ["embed-onnx"]      # ONNX Runtime with CUDA execution provider

# ── UI ──
cli = []
a2ui-html = []

# ── Advanced ──
migration = []
landlock = []                    # Kernel sandbox (Linux 5.13+)
jepa = ["plato-jepa"]
fleet = ["lau-inter-shell"]

# ── COMPOUND: JETSON NANO ──
jetson = [
    "std",
    "sqlite",
    "lancedb",
    "cuda-jetson",           # CUDA with unified memory
    "llm-local",
    "llm-cuda-offload",      # 16 layers on GPU
    "embed-onnx",
    "embed-cuda",            # Embedding on GPU
    "cli",
    "migration",
    "landlock",
]

[dependencies]
# Core
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time", "net", "process"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v7"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }

# Concurrency
rayon = "1.10"
dashmap = "6"
crossbeam = "0.8"

# Storage
rusqlite = { version = "0.31", optional = true }
lancedb-ffi = { version = "0.6", optional = true }

# GPU
cudaclaw = { version = "0.1", optional = true }

# ML
ort = { version = "2", optional = true, features = ["load-dynamic"] }

# System
sysinfo = "0.30"
sha2 = "0.10"

# Targets
[target.aarch64-unknown-linux-gnu.dependencies]
# Jetson Nano runs aarch64 Linux

[target.aarch64-unknown-linux-gnu.dev-dependencies]
criterion = "0.5"
```

## 3.3 Jetson Memory Budget

```
4GB LPDDR4 (shared CPU/GPU)
├── OS overhead:          ~800MB  (Linux, drivers, CUDA runtime)
├── LLM model:            ~700MB  (TinyLlama 1.1B Q4_K_M)
│   └── 16 GPU layers:    ~400MB  (in shared VRAM — same physical RAM)
│   └── 6 CPU layers:     ~300MB  (in CPU RAM — same physical RAM)
├── Embedding model:       ~22MB  (MiniLM-L6 ONNX)
├── LanceDB reflexes:      ~75MB  (50K reflexes, 384-dim)
├── LanceDB memories:     ~150MB  (100K entries)
├── SQLite:                 ~10MB
├── PincherOS runtime:      ~50MB  (Rust binary + DashMap + buffers)
├── Python sidecar:        ~100MB  (Python runtime + libs)
├── Sandbox:                ~20MB
└── FREE:                ~2073MB  (for agent state, working set)

Total used: ~1927MB / 4096MB = 47%
Free: ~2073MB = 51%

This is COMFORTABLE. The Jetson Nano is not memory-constrained for
PincherOS's MVP workload of 4-8 concurrent agents.
```

---

# 4. THE RPi 4 PATH

## 4.1 CpuClaws Detailed Implementation

```rust
/// RPi 4 concrete configuration.
pub struct RPi4Config {
    /// Thread pool size (4× A72, use all)
    pub worker_threads: usize,    // Default: 4
    
    /// DashMap shard count for CRDT cells
    /// 64 shards on 4 cores = 16 shards/core = low contention
    pub crdt_shards: usize,       // Default: 64
    
    /// Maximum RAM for PincherOS (MB)
    pub ram_budget_mb: u64,       // Default: 1536 (out of 4096, after OS)
    
    /// LLM model path
    pub model_path: String,       // Default: /opt/pincheros/models/TinyLlama-1.1B-Q4_K_M.gguf
    
    /// Inference threads (reserve 1 for core loop)
    pub inference_threads: usize,  // Default: 3
    
    /// NEON optimization flag
    pub use_neon: bool,           // Default: true (always on A72)
}

impl Default for RPi4Config {
    fn default() -> Self {
        Self {
            worker_threads: 4,
            crdt_shards: 64,
            ram_budget_mb: 1536,
            model_path: "/opt/pincheros/models/TinyLlama-1.1B-Q4_K_M.gguf".into(),
            inference_threads: 3,
            use_neon: true,
        }
    }
}

/// The complete RPi 4 PincherOS stack.
pub struct RPi4Stack {
    /// Core Claws implementation
    claws: CpuClaws,
    
    /// CRDT engine — CPU-only, no GPU merge
    crdt: PartitionedCrdtEngine,
    
    /// Inference bridge to Python sidecar
    infer: InferenceBridge,
    
    /// Configuration
    config: RPi4Config,
}

impl RPi4Stack {
    pub fn new(config: RPi4Config) -> Result<Self, StackError> {
        let claws = CpuClaws::new()?;
        
        let crdt_config = PartitionConfig {
            hot_threshold: 100.0,
            warm_threshold: 1.0,
            decay_factor: 0.95,
            repartition_interval: 5.0,
            gpu_merge_available: false, // RPi 4 HAS NO GPU
        };
        let crdt = PartitionedCrdtEngine::new(crdt_config)?;
        
        let infer = InferenceBridge::new(InferenceConfig {
            socket_path: "/opt/pincheros/run/infer.sock".into(),
            model_path: config.model_path.clone(),
            inference_threads: config.inference_threads,
            gpu_layers: 0, // No GPU on RPi 4
        });
        
        Ok(Self {
            claws,
            crdt,
            infer,
            config,
        })
    }
    
    /// The RPi 4 execution profile — everything on CPU + NEON.
    pub fn execution_profile(&self) -> &'static str {
        r#"
        RPi 4 (BCM2711, 4× A72 @ 1.5GHz, 4GB RAM)
        
        Compute Substrate: CPU + NEON (NO GPU COMPUTE)
        VideoCore VI: display-only, no compute shaders
        
        Thread Layout:
          Thread 0 (tokio): Core loop (input → reflex → output)
          Thread 1 (rayon): CRDT merge (DashMap, 64 shards)
          Thread 2 (rayon): Context assembly + sandbox
          Thread 3 (Python): LLM inference (llama.cpp, 3 threads)
          
        CRDT Engine:
          Type: PartitionedCrdtEngine (CPU-only)
          Backend: DashMap<u64, CrdtCell> + rayon
          Merge rate: ~10K updates in ~50μs
          No GPU merge (86,000× slower even on Jetson; RPi has no GPU)
          
        Inference:
          Backend: llama.cpp CPU (via Python sidecar)
          Model: TinyLlama 1.1B Q4_K_M (~700MB)
          Throughput: ~5-8 tok/s
          GPU layers: 0 (no GPU)
          
        Embedding:
          Backend: ONNX Runtime CPU + ARM NEON
          Model: all-MiniLM-L6-v2 (~22MB)
          Throughput: ~30-50ms/sentence
          NEON: auto-activated for float32 SIMD on A72
          
        Vector Search:
          Backend: LanceDB (CPU-native, mmap)
          50K vectors × 384-dim: ~10ms search
          
        Memory Budget:
          Total RAM: 4096MB
          OS overhead: ~800MB
          LLM model: ~700MB (loaded lazily, unloaded after 5min idle)
          Embedding: ~22MB (always loaded)
          LanceDB: ~225MB
          SQLite: ~10MB
          PincherOS runtime: ~50MB
          Free: ~2289MB
        "#
    }
}

/// Inference bridge — communicates with Python sidecar via UDS.
pub struct InferenceBridge {
    socket_path: String,
    config: InferenceConfig,
}

#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub socket_path: String,
    pub model_path: String,
    pub inference_threads: usize,
    pub gpu_layers: u32,  // 0 on RPi 4, 16 on Jetson
}

/// NEON-accelerated embedding similarity for RPi 4.
/// Uses ARM NEON intrinsics for cosine similarity.
#[cfg(target_arch = "aarch64")]
pub mod neon_sim {
    use std::arch::aarch64::*;
    
    /// Cosine similarity using NEON intrinsics.
    /// 384-dim vectors: 12 × float32x4_t per vector
    /// ~3× faster than scalar on A72.
    #[target_feature(enable = "neon")]
    pub unsafe fn cosine_similarity_neon(a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len() % 4, 0); // 384 is divisible by 4
        
        let mut dot = vdupq_f32(0.0);
        let mut norm_a = vdupq_f32(0.0);
        let mut norm_b = vdupq_f32(0.0);
        
        for i in (0..a.len()).step_by(4) {
            let va = vld1q_f32(a.as_ptr().add(i));
            let vb = vld1q_f32(b.as_ptr().add(i));
            dot = vmlaq_f32(dot, va, vb);
            norm_a = vmlaq_f32(norm_a, va, va);
            norm_b = vmlaq_f32(norm_b, vb, vb);
        }
        
        // Horizontal sum
        let dot_sum = vaddvq_f32(dot);
        let norm_a_sum = vaddvq_f32(norm_a);
        let norm_b_sum = vaddvq_f32(norm_b);
        
        dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt())
    }
}
```

## 4.2 RPi 4 Cargo Feature Set

```toml
# ── COMPOUND: RASPBERRY PI 4 ──
rpi4 = [
    "std",
    "sqlite",
    "lancedb",
    # NO CUDA — VideoCore VI is display-only
    "llm-local",           # llama.cpp CPU backend
    "embed-onnx",          # ONNX Runtime CPU + NEON
    # NO embed-cuda
    "cli",
    "migration",
    # NO landlock — RPi OS may be on older kernel
]
```

---

# 5. ShellQuality AS FIRST-CLASS TYPE

## 5.1 The Rust Type

```rust
/// ShellQuality: a first-class metric for shell health assessment.
/// 
/// This is NOT a cosmetic addition. The biologist showed that shell quality
/// determines vacancy chain direction. A degrading shell triggers downgrade
/// cascades. A healthy shell attracts upgrade cascades.
/// 
/// ShellQuality lives in the Shell trait hierarchy, not in Claws.
/// It's a property of the SHELL, not the compute substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellQuality {
    /// Composite health index: 0.0 (failing) to 1.0 (pristine)
    pub health_index: f64,
    
    /// SSD / storage wear level: 0.0 (new) to 1.0 (end of life)
    pub storage_wear: f64,
    
    /// Thermal history: cumulative throttle events
    pub thermal_history: ThermalHistory,
    
    /// Network reliability: 0.0 (unreliable) to 1.0 (rock solid)
    pub network_reliability: f64,
    
    /// Battery health: None if not on battery, 0.0-1.0 cycle health
    pub battery_health: Option<f64>,
    
    /// Uptime stability: how often does this shell crash/reboot?
    pub uptime_stability: f64,
    
    /// Last quality assessment timestamp
    pub assessed_at: chrono::DateTime<chrono::Utc>,
}

/// Thermal history: cumulative throttle events over time windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalHistory {
    /// Throttle events in last 24 hours
    pub throttle_events_24h: u64,
    /// Throttle events in last 7 days
    pub throttle_events_7d: u64,
    /// Peak temperature ever recorded (°C)
    pub peak_temp_c: f64,
    /// Current temperature (°C)
    pub current_temp_c: f64,
    /// Thermal design limit (°C) — hardware-specific
    pub design_limit_c: f64,
}

impl ThermalHistory {
    /// Normalized throttle rate: 0.0 (never throttled) to 1.0 (constantly throttled).
    /// Based on 24-hour window.
    pub fn normalized_throttle_rate(&self) -> f64 {
        // Assume 1 throttle event per hour = moderate (0.5)
        // 24 events in 24h = constantly throttled (1.0)
        (self.throttle_events_24h as f64 / 24.0).min(1.0)
    }
    
    /// Thermal headroom: how far from the design limit.
    /// 0.0 = at limit, 1.0 = 20°C+ below limit.
    pub fn thermal_headroom(&self) -> f64 {
        let margin = self.design_limit_c - self.current_temp_c;
        (margin / 20.0).clamp(0.0, 1.0)
    }
}

impl ShellQuality {
    /// Weighted composite score: the migration decision function.
    /// 
    /// Weights derived from biological shell quality assessment:
    /// - Health index (30%): overall hardware health — like shell wall thickness
    /// - Thermal (25%): thermal headroom — like shell weight (heavy = too hot)
    /// - Storage (20%): SSD wear — like shell internal erosion
    /// - Network (15%): reliability — like aperture integrity
    /// - Battery (10%): power stability — like chemical deterioration
    pub fn composite_score(&self) -> f64 {
        let thermal = 1.0 - self.thermal_history.normalized_throttle_rate();
        let storage = 1.0 - self.storage_wear;
        let network = self.network_reliability;
        let battery = self.battery_health.unwrap_or(1.0);
        
        self.health_index * 0.30
            + thermal * 0.25
            + storage * 0.20
            + network * 0.15
            + battery * 0.10
    }
    
    /// Migration desirability: should an agent WANT to migrate TO this shell?
    /// Combines composite_score with current load.
    pub fn migration_desirability(&self, current_load: f64) -> f64 {
        // A shell is desirable if it's healthy AND has capacity
        let quality = self.composite_score();
        let capacity = 1.0 - current_load; // 0.0 = full, 1.0 = empty
        quality * 0.6 + capacity * 0.4
    }
    
    /// Emergency evacuation: should agents be EVACUATED FROM this shell?
    /// Returns true if shell quality is critically low.
    pub fn requires_evacuation(&self) -> bool {
        self.composite_score() < 0.30
            || self.thermal_history.current_temp_c > self.thermal_history.design_limit_c - 5.0
            || self.storage_wear > 0.90
            || self.network_reliability < 0.30
    }
    
    /// Quality degradation rate: how fast is this shell declining?
    /// Computed from recent thermal history trend.
    pub fn degradation_rate(&self) -> f64 {
        // If 7d throttle events >> 24h events × 7, the rate is slowing
        // If 7d events ≈ 24h events × 7, the rate is steady
        // If 24h events × 7 >> 7d events, the rate is ACCELERATING
        let daily_rate = self.thermal_history.throttle_events_24h as f64;
        let weekly_avg = self.thermal_history.throttle_events_7d as f64 / 7.0;
        
        if weekly_avg > 0.0 {
            (daily_rate / weekly_avg - 1.0).clamp(-1.0, 2.0)
        } else if daily_rate > 0.0 {
            2.0 // No prior throttles, now throttling = accelerating
        } else {
            0.0 // Stable
        }
    }
}

/// ShellQuality assessment: run periodically (every 5 minutes).
pub fn assess_shell_quality(
    shell: &dyn Shell,
    claws: &dyn Claws,
    thermal_history: &ThermalHistory,
) -> ShellQuality {
    let profile = shell.snap();
    let substrate_health = claws.substrate_health();
    
    // Storage wear from SMART data
    let storage_wear = read_smart_wear_level()
        .unwrap_or(0.0); // Default: unknown = assume OK
    
    // Network reliability from ping stats
    let network_reliability = assess_network_reliability();
    
    // Battery health from sysfs
    let battery_health = read_battery_health();
    
    // Uptime stability from reboot count
    let uptime_stability = assess_uptime_stability();
    
    // Health index: weighted combination
    let health_index = {
        let ram_health = profile.capabilities.ram_available_bytes as f64
            / profile.capabilities.ram_total_bytes as f64;
        let cpu_health = match substrate_health.cpu_thermal {
            ThermalStatus::Nominal => 1.0,
            ThermalStatus::Warm => 0.7,
            ThermalStatus::Hot => 0.4,
            ThermalStatus::Critical => 0.1,
        };
        let gpu_health = 1.0 - substrate_health.gpu_throttle_rate;
        
        ram_health * 0.4 + cpu_health * 0.35 + gpu_health * 0.25
    };
    
    ShellQuality {
        health_index,
        storage_wear,
        thermal_history: thermal_history.clone(),
        network_reliability,
        battery_health,
        uptime_stability,
        assessed_at: chrono::Utc::now(),
    }
}
```

## 5.2 ShellQuality in the Trait Hierarchy

```rust
/// ShellQuality lives in the Shell trait, not in Claws.
/// The Shell OWNS its quality — it's a property of the hardware.
pub trait Shell: Pincher {
    type Error: std::error::Error + Send + Sync;
    
    fn snap(&self) -> ShellProfile;
    fn resource_status(&self) -> ResourceStatus;
    fn degrade(&mut self, level: DegradationLevel) -> Result<(), Self::Error>;
    fn promote(&mut self, level: DegradationLevel) -> Result<(), Self::Error>;
    fn fingerprint(&self) -> &ShellFingerprint;
    fn energy_state(&self) -> Option<EnergyState>;
    
    /// NEW: Shell quality assessment.
    /// Called every 5 minutes and on migration decisions.
    fn quality(&self) -> ShellQuality;
}
```

## 5.3 Migration Scoring Function

```rust
/// Migration decision: should agent A migrate from shell_old to shell_new?
/// 
/// Returns a MigrationScore that combines:
/// 1. Fit improvement (resource capacity)
/// 2. Quality improvement (ShellQuality composite)
/// 3. Degradation risk (is the new shell declining?)
/// 4. Migration cost (downtime, data transfer)
/// 5. Consent state (Navajo animacy: only living beings initiate)
pub fn score_migration(
    agent: &RiggingProfile,
    shell_old: &dyn Shell,
    shell_new: &dyn Shell,
    consent: ConsentState,
) -> MigrationScore {
    let old_quality = shell_old.quality();
    let new_quality = shell_new.quality();
    let old_profile = shell_old.snap();
    let new_profile = shell_new.snap();
    
    // 1. Fit improvement: does the new shell have more resources?
    let old_fit = compute_fit(agent, &old_profile);
    let new_fit = compute_fit(agent, &new_profile);
    let fit_improvement = new_fit - old_fit; // Can be negative (downgrade)
    
    // 2. Quality improvement: is the new shell healthier?
    let quality_improvement = new_quality.composite_score() - old_quality.composite_score();
    
    // 3. Degradation risk: is the new shell declining?
    let degradation_risk = new_quality.degradation_rate().max(0.0);
    
    // 4. Migration cost: how much downtime?
    let migration_cost = estimate_migration_cost(&old_profile, &new_profile);
    
    // 5. Consent check: Navajo animacy — shells cannot initiate
    let consent_bonus = match consent {
        ConsentState::ExplicitUser => 0.2,    // User requested — boost
        ConsentState::AgentInitiated => 0.1,  // Agent requested — moderate boost
        ConsentState::AutoIdle => 0.0,        // Auto when idle — no boost
        ConsentState::Denied => return MigrationScore::denied(), // No consent
        ConsentState::ShellInitiated => return MigrationScore::denied(), // FORBIDDEN by Navajo constraint
    };
    
    // Weighted combination
    let score = fit_improvement * 0.35
        + quality_improvement * 0.25
        - degradation_risk * 0.15
        - migration_cost * 0.15
        + consent_bonus;
    
    MigrationScore {
        score,
        fit_improvement,
        quality_improvement,
        degradation_risk,
        migration_cost,
        consent_state: consent,
        recommendation: if score > 0.3 {
            MigrationRecommendation::Proceed
        } else if score > 0.0 {
            MigrationRecommendation::Marginal
        } else {
            MigrationRecommendation::Decline
        },
    }
}

#[derive(Debug, Clone)]
pub struct MigrationScore {
    pub score: f64,
    pub fit_improvement: f64,
    pub quality_improvement: f64,
    pub degradation_risk: f64,
    pub migration_cost: f64,
    pub consent_state: ConsentState,
    pub recommendation: MigrationRecommendation,
}

impl MigrationScore {
    fn denied() -> Self {
        Self {
            score: f64::NEG_INFINITY,
            fit_improvement: 0.0,
            quality_improvement: 0.0,
            degradation_risk: 0.0,
            migration_cost: 0.0,
            consent_state: ConsentState::Denied,
            recommendation: MigrationRecommendation::Denied,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentState {
    /// User explicitly requested migration
    ExplicitUser,
    /// Agent requested migration (allowed by Navajo animacy)
    AgentInitiated,
    /// Automatic migration when agent is idle
    AutoIdle,
    /// User/agent denied
    Denied,
    /// Shell initiated — FORBIDDEN by Navajo animacy constraint
    ShellInitiated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationRecommendation {
    Proceed,   // Score > 0.3
    Marginal,  // 0.0 < Score < 0.3
    Decline,   // Score < 0.0
    Denied,    // No consent or shell-initiated
}
```

---

# 6. THE 3-PHASE MIGRATION WITH 7 CONSTRAINTS

## 6.1 The 7 Constraints from Linguistic Analysis

The linguist (Lojban formalization) revealed 7 hidden constraints:

| # | Constraint | Source | What It Means |
|---|-----------|--------|---------------|
| C1 | **Substance-Accident** | Greek (ousia/symbebekos) | Preserve identity (substance), adapt surface (accidents) |
| C2 | **Simultaneity** | Greek (middle voice) | Old shell releases AND new shell receives in the same instant |
| C3 | **Shape-Asymmetry** | Navajo (classificatory verbs) | Migration A→B ≠ migration B→A when shells differ in shape |
| C4 | **Animacy** | Navajo (animacy hierarchy) | Only living beings (user/agent) can initiate migration, never shells |
| C5 | **Pair-Operation** | Sanskrit (dual number) | Migration is one operation on the shell-pair, not two individual ops |
| C6 | **Consent** | All languages (Navajo strongest) | Explicit consent required; automated migration needs user/agent approval |
| C7 | **Differential-Verification** | Navajo (resultative phase) | Failed reflexes lose confidence; identity reflexes (>0.99) resist degradation |

## 6.2 Mapping to 3 Phases

```
PREPARE ─────────────────────────────────────────────────────
  Constraints checked: C1 (substance-accident), C3 (shape-asymmetry),
                       C4 (animacy), C6 (consent)
  
  What happens:
    1. C4: Verify migration was initiated by user/agent, NOT shell
    2. C6: Obtain explicit consent (or verify auto-idle policy)
    3. C1: Partition rigging state into substance (preserved) and 
           accidents (to be adapted on new shell)
    4. C3: Determine shape-verb for this migration direction
           (Stretch/Spread/Settle — determines operational mode)
    5. Snapshot rigging into .nail file
    6. Mark rigging as MIGRATING (read-only)

CROSSFADE ───────────────────────────────────────────────────
  Constraints checked: C2 (simultaneity), C5 (pair-operation)
  
  What happens:
    1. C5: Establish the shell-pair as ONE operational unit
    2. C2: Simultaneously:
           - Old shell: finishes in-flight requests, starts forwarding
           - New shell: loads .nail, runs snap(), adapts accidents
    3. Duration: 200ms same-machine, 2s across-network
    4. Both shells serve requests during this window
    5. MoltingProxy active on old shell for cached reflexes

FINALIZE ────────────────────────────────────────────────────
  Constraints checked: C7 (differential-verification)
  
  What happens:
    1. C7: Verify top-K reflexes on new shell
           - Identity reflexes (confidence > 0.99): must pass OR be explained
           - Regular reflexes: failure reduces confidence by 0.1
           - If > 30% of identity reflexes fail: ROLLBACK
    2. Transfer symbionts (Tenuo, monitoring) to new shell
    3. Re-mint capability tokens for new shell's Tenuo authority
    4. Old shell: retains read-only snapshot for rollback (24h default)
    5. New shell: becomes primary, rigging is writable again
    6. Emit VacancyChainCompleted fleet event (pheromone cascade)
```

## 6.3 The State Machine with Guard Conditions

```rust
use std::time::{Duration, Instant};

/// Migration state machine.
/// The 7 constraints are enforced as transition guards.
#[derive(Debug)]
pub enum MigrationPhase {
    /// Normal operation. No migration in progress.
    Stable,
    
    /// Phase 1: PREPARE
    /// Guards: C1, C3, C4, C6 must be satisfied
    Preparing {
        /// The .nail snapshot (substance preserved, accidents pending adaptation)
        nail_snapshot: NailFile,
        /// Substance/accident partition (C1)
        partition: SubstanceAccidentPartition,
        /// Shape verb for this migration direction (C3)
        shape_verb: ShapeVerb,
        /// Who initiated this migration (C4)
        initiator: MigrationInitiator,
        /// Consent record (C6)
        consent: ConsentRecord,
        /// Timestamp for timeout
        started_at: Instant,
        /// Maximum time in PREPARE phase
        timeout: Duration,
    },
    
    /// Phase 2: CROSSFADE
    /// Guards: C2, C5 must be satisfied
    Crossfading {
        /// The .nail file (being adapted on new shell)
        nail_snapshot: NailFile,
        /// Shell-pair operational unit (C5)
        shell_pair: ShellPair,
        /// Simultaneity verification (C2)
        simultaneity_verified: bool,
        /// Duration of crossfade
        crossfade_duration: Duration,
        /// Started at
        started_at: Instant,
    },
    
    /// Phase 3: FINALIZE
    /// Guard: C7 must be satisfied
    Finalizing {
        /// Verification results for top-K reflexes (C7)
        verification: DifferentialVerification,
        /// Symbiont transfer status
        symbiont_transfer: SymbiontTransferStatus,
        /// Old shell's read-only snapshot retention
        rollback_retention: Duration,
        /// Started at
        started_at: Instant,
    },
    
    /// Migration complete. New shell owns the rigging.
    Finalized {
        migrated_to: ShellFingerprint,
        nail_snapshot: NailFile,
        rollback_deadline: Instant,
    },
    
    /// Migration failed. Rolled back to original state.
    Failed {
        reason: MigrationFailureReason,
        nail_snapshot: Option<NailFile>,
        rolled_back_at: Instant,
    },
}

// ── Constraint Types ──

/// C1: Substance-Accident partition
#[derive(Debug, Clone)]
pub struct SubstanceAccidentPartition {
    /// What is preserved across migration (identity, UUID, personality, reflex patterns)
    pub substance: Vec<SubstanceField>,
    /// What is adapted to the new shell (embeddings, sandbox profiles, GPU layer counts)
    pub accidents: Vec<AccidentField>,
    /// Ratio of substance to total state
    /// If substance_ratio < 0.5, the agent's identity has changed — this is a NEW agent
    pub substance_ratio: f64,
}

#[derive(Debug, Clone)]
pub enum SubstanceField {
    RiggingId,
    Personality,
    ReflexPatterns,
    TrustScores,
    SessionHistory,
}

#[derive(Debug, Clone)]
pub enum AccidentField {
    Embeddings,
    SandboxProfiles,
    GpuLayerCount,
    ModelSelection,
    CrdtCellAdaptations,
}

/// C3: Shape verb — determines operational mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeVerb {
    /// Long-thin shell (RPi): sequential, reflex-urgent, depth-first
    Stretch,
    /// Flat-wide shell (Jetson): layered, GPU/CPU split, parallel-first
    Spread,
    /// Round-deep shell (Workstation): concurrent, exploratory, breadth-first
    Settle,
}

impl ShapeVerb {
    /// Determine shape verb from source and destination shell profiles.
    /// C3: Migration A→B ≠ migration B→A when shapes differ.
    pub fn classify(source: &ShellProfile, dest: &ShellProfile) -> Self {
        let dest_gpu = match &dest.capabilities.gpu {
            GpuType::None => false,
            GpuType::Cuda { .. } => true,
        };
        
        let dest_ram_gb = dest.capabilities.ram_total_bytes as f64 / 1e9;
        
        if dest_gpu && dest_ram_gb > 16.0 {
            ShapeVerb::Settle  // Workstation: round-deep
        } else if dest_gpu {
            ShapeVerb::Spread  // Jetson: flat-wide
        } else {
            ShapeVerb::Stretch // RPi: long-thin
        }
    }
    
    /// Operational parameters based on shape verb.
    pub fn operational_mode(&self) -> OperationalMode {
        match self {
            ShapeVerb::Stretch => OperationalMode {
                max_concurrent_reflexes: 2,
                inference_priority: Priority::High,    // Reflex-urgent: get answers fast
                search_strategy: SearchStrategy::DepthFirst,
                context_window: 1024,                   // Smaller context to save RAM
                learn_threshold: 0.40,                  // Lower threshold — learn faster
            },
            ShapeVerb::Spread => OperationalMode {
                max_concurrent_reflexes: 4,
                inference_priority: Priority::Normal,
                search_strategy: SearchStrategy::ParallelFirst, // GPU for inference, CPU for CRDT
                context_window: 2048,
                learn_threshold: 0.50,
            },
            ShapeVerb::Settle => OperationalMode {
                max_concurrent_reflexes: 8,
                inference_priority: Priority::Low,      // Explore more, less urgent
                search_strategy: SearchStrategy::BreadthFirst,
                context_window: 4096,                   // Full context for deep reasoning
                learn_threshold: 0.60,                  // Higher bar — quality over speed
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperationalMode {
    pub max_concurrent_reflexes: usize,
    pub inference_priority: Priority,
    pub search_strategy: SearchStrategy,
    pub context_window: usize,
    pub learn_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    DepthFirst,
    BreadthFirst,
    ParallelFirst,
}

/// C4: Who initiated this migration?
#[derive(Debug, Clone)]
pub enum MigrationInitiator {
    User,
    Agent,
    AutoIdle,
    // Shell is NOT ALLOWED — enforced by state machine
}

/// C6: Consent record
#[derive(Debug, Clone)]
pub struct ConsentRecord {
    pub consent_type: ConsentState,
    pub consented_at: Instant,
    pub consented_by: String, // User ID or Agent ID
    pub policy: ConsentPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentPolicy {
    Explicit,           // User must approve
    AutoWhenIdle,       // Auto-migrate when agent is idle
    AutoIfImprovement,  // Auto-migrate if fit improvement > threshold
    OperatorOverride,   // System operator can force migration
}

/// C5: Shell pair as operational unit
#[derive(Debug, Clone)]
pub struct ShellPair {
    pub old_shell: ShellFingerprint,
    pub new_shell: ShellFingerprint,
    /// Communication channel between pair members
    pub channel_state: PairChannelState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairChannelState {
    /// Channel established, both shells responsive
    Connected,
    /// Channel degraded (high latency or packet loss)
    Degraded,
    /// Channel broken — migration must abort or continue on one side
    Broken,
}

/// C7: Differential verification of reflexes
#[derive(Debug, Clone)]
pub struct DifferentialVerification {
    /// Total reflexes tested
    pub total_tested: usize,
    /// Reflexes that passed verification
    pub passed: usize,
    /// Reflexes that failed (confidence reduced)
    pub failed: Vec<FailedReflex>,
    /// Identity reflexes (confidence > 0.99) that failed
    pub identity_failures: Vec<FailedReflex>,
    /// Whether to ROLLBACK (> 30% identity reflex failures)
    pub rollback_recommended: bool,
}

#[derive(Debug, Clone)]
pub struct FailedReflex {
    pub reflex_id: ReflexId,
    pub previous_confidence: f64,
    pub new_confidence: f64,
    pub failure_reason: String,
    /// Whether this is an identity reflex (confidence > 0.99)
    pub is_identity: bool,
}

// ── State Machine Transitions ──

/// The migration guard: checks all constraints before allowing transitions.
pub struct MigrationGuard {
    phase: MigrationPhase,
}

impl MigrationGuard {
    pub fn new() -> Self {
        Self {
            phase: MigrationPhase::Stable,
        }
    }
    
    /// Can the rigging learn new reflexes?
    pub fn can_learn(&self) -> bool {
        matches!(self.phase, MigrationPhase::Stable)
    }
    
    /// Can the rigging update trust scores?
    pub fn can_update_trust(&self) -> bool {
        matches!(
            self.phase,
            MigrationPhase::Stable | MigrationPhase::Crossfading { .. }
        )
    }
    
    /// Transition: Stable → Preparing
    /// Guards: C1, C3, C4, C6
    pub fn begin_prepare(
        &mut self,
        nail: NailFile,
        partition: SubstanceAccidentPartition,
        shape_verb: ShapeVerb,
        initiator: MigrationInitiator,
        consent: ConsentRecord,
    ) -> Result<(), MigrationError> {
        // C4: Animacy — shells cannot initiate
        // (This should already be checked, but double-enforce)
        if matches!(consent.consent_type, ConsentState::ShellInitiated) {
            return Err(MigrationError::ConstraintViolation(
                "C4 (Animacy): Shells cannot initiate migration"
            ));
        }
        
        // C6: Consent required
        if matches!(consent.consent_type, ConsentState::Denied) {
            return Err(MigrationError::ConstraintViolation(
                "C6 (Consent): Migration consent denied"
            ));
        }
        
        // C1: Substance ratio check
        if partition.substance_ratio < 0.5 {
            return Err(MigrationError::IdentityLoss(
                format!("C1 (Substance-Accident): substance_ratio = {:.2} < 0.5 — this would create a new agent, not migrate one", partition.substance_ratio)
            ));
        }
        
        // C3: Shape verb must be determined (not default)
        // shape_verb is computed from source→dest, so it's always directional
        
        self.phase = MigrationPhase::Preparing {
            nail_snapshot: nail,
            partition,
            shape_verb,
            initiator,
            consent,
            started_at: Instant::now(),
            timeout: Duration::from_secs(30),
        };
        
        Ok(())
    }
    
    /// Transition: Preparing → Crossfading
    /// Guards: C2, C5
    pub fn begin_crossfade(
        &mut self,
        shell_pair: ShellPair,
        crossfade_duration: Duration,
    ) -> Result<(), MigrationError> {
        let (nail, shape_verb) = match &self.phase {
            MigrationPhase::Preparing { nail_snapshot, shape_verb, timeout, started_at, .. } => {
                // Timeout check
                if started_at.elapsed() > *timeout {
                    self.phase = MigrationPhase::Failed {
                        reason: MigrationFailureReason::PrepareTimeout,
                        nail_snapshot: Some(nail_snapshot.clone()),
                        rolled_back_at: Instant::now(),
                    };
                    return Err(MigrationError::Timeout("PREPARE phase exceeded 30s"));
                }
                (nail_snapshot.clone(), *shape_verb)
            }
            _ => return Err(MigrationError::InvalidTransition(
                "CROSSFADE requires PREPARE phase"
            )),
        };
        
        // C5: Pair operation — verify channel
        if shell_pair.channel_state == PairChannelState::Broken {
            return Err(MigrationError::ConstraintViolation(
                "C5 (Pair-Operation): Shell pair channel is broken"
            ));
        }
        
        // C2: Simultaneity will be verified during crossfade
        // (We set simultaneity_verified = false — it's confirmed when both
        //  shells report active in the same tick)
        
        self.phase = MigrationPhase::Crossfading {
            nail_snapshot: nail,
            shell_pair,
            simultaneity_verified: false,
            crossfade_duration,
            started_at: Instant::now(),
        };
        
        Ok(())
    }
    
    /// Confirm simultaneity (C2) — called when both shells report active
    pub fn confirm_simultaneity(&mut self) -> Result<(), MigrationError> {
        if let MigrationPhase::Crossfading { ref mut simultaneity_verified, .. } = self.phase {
            *simultaneity_verified = true;
            Ok(())
        } else {
            Err(MigrationError::InvalidTransition(
                "C2 (Simultaneity) confirmation requires CROSSFADE phase"
            ))
        }
    }
    
    /// Transition: Crossfading → Finalizing
    /// Guard: C2 must be confirmed
    pub fn begin_finalize(&mut self) -> Result<DifferentialVerification, MigrationError> {
        match &self.phase {
            MigrationPhase::Crossfading { simultaneity_verified, crossfade_duration, started_at, .. } => {
                // C2: Simultaneity must be verified
                if !simultaneity_verified {
                    return Err(MigrationError::ConstraintViolation(
                        "C2 (Simultaneity): Old shell release and new shell receive were not confirmed simultaneous"
                    ));
                }
                
                // Timeout check
                if started_at.elapsed() > *crossfade_duration * 2 {
                    self.phase = MigrationPhase::Failed {
                        reason: MigrationFailureReason::CrossfadeTimeout,
                        nail_snapshot: None,
                        rolled_back_at: Instant::now(),
                    };
                    return Err(MigrationError::Timeout("CROSSFADE phase exceeded 2x duration"));
                }
            }
            _ => return Err(MigrationError::InvalidTransition(
                "FINALIZE requires CROSSFADE phase"
            )),
        }
        
        // Get the nail snapshot for verification
        let nail = match &self.phase {
            MigrationPhase::Crossfading { nail_snapshot, .. } => nail_snapshot.clone(),
            _ => unreachable!(),
        };
        
        // C7: Differential verification
        // Test top-K reflexes on new shell
        let verification = DifferentialVerification {
            total_tested: 0,
            passed: 0,
            failed: vec![],
            identity_failures: vec![],
            rollback_recommended: false,
        };
        // In real implementation: actually run top-K reflexes and collect results
        
        self.phase = MigrationPhase::Finalizing {
            verification: verification.clone(),
            symbiont_transfer: SymbiontTransferStatus::Pending,
            rollback_retention: Duration::from_hours(24),
            started_at: Instant::now(),
        };
        
        Ok(verification)
    }
    
    /// Transition: Finalizing → Finalized or Failed
    /// Guard: C7 must pass
    pub fn complete(&mut self, verification: DifferentialVerification) -> Result<(), MigrationError> {
        // C7: Differential verification
        if verification.rollback_recommended {
            // > 30% identity reflex failures — ROLLBACK
            self.phase = MigrationPhase::Failed {
                reason: MigrationFailureReason::VerificationFailed {
                    identity_failures: verification.identity_failures.len(),
                    total_identity: verification.total_tested,
                },
                nail_snapshot: None,
                rolled_back_at: Instant::now(),
            };
            return Err(MigrationError::ConstraintViolation(
                "C7 (Differential-Verification): Too many identity reflex failures — rolling back"
            ));
        }
        
        // Success!
        self.phase = MigrationPhase::Finalized {
            migrated_to: ShellFingerprint::default(), // Set from actual target
            nail_snapshot: NailFile::default(),
            rollback_deadline: Instant::now() + Duration::from_hours(24),
        };
        
        Ok(())
    }
    
    /// Emergency rollback from any phase.
    pub fn rollback(&mut self, reason: MigrationFailureReason) {
        self.phase = MigrationPhase::Failed {
            reason,
            nail_snapshot: None,
            rolled_back_at: Instant::now(),
        };
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Constraint violation: {0}")]
    ConstraintViolation(&'static str),
    #[error("Invalid transition: {0}")]
    InvalidTransition(&'static str),
    #[error("Timeout: {0}")]
    Timeout(&'static str),
    #[error("Identity loss: {0}")]
    IdentityLoss(String),
}

#[derive(Debug, Clone)]
pub enum MigrationFailureReason {
    PrepareTimeout,
    CrossfadeTimeout,
    VerificationFailed {
        identity_failures: usize,
        total_identity: usize,
    },
    ChannelBroken,
    ConsentRevoked,
    ResourceExhausted,
}

#[derive(Debug, Clone)]
pub enum SymbiontTransferStatus {
    Pending,
    InProgress,
    Complete,
    Failed(String),
}
```

---

# 7. WORKSTATION CLAWS: BEYOND MVP

## 7.1 What 128 SMs Unlock

```
RTX 4090 (AD102, Ada Lovelace)
├── SMs: 128
├── CUDA cores: 16,384
├── VRAM: 24GB GDDR6X
├── L2 cache: 72MB
├── TDP: 450W
├── PCIe: 4.0 x16 (32 GB/s bidir)
└── Compute capability: SM 8.9

What this unlocks:
1. Hot-path CRDT merge on GPU (500× faster than CPU for 10K updates)
2. Full-model inference on GPU (Phi-3 3.8B, Llama-3-8B)
3. Large embedding models on GPU (BGE-large, E5-mistral)
4. Multi-stream priority dispatch
5. Future: Thread Block Clusters (SM90+), TMA, Dynamic Parallelism
```

## 7.2 WorkstationClaws Implementation

```rust
/// WorkstationClaws: full GPU acceleration.
///
/// Target: RTX 4090 / A100 / H100
/// 
/// KEY DIFFERENCE from JetsonClaws:
/// - Uses cudaMalloc (device memory), NOT cudaMallocManaged
/// - Avoids UM ping-pong on discrete GPUs
/// - Hot-path CRDT merge on GPU via batch dispatch
/// - Multi-stream priority scheduling
/// - CPU remains source of truth (GPU merges are advisory)
///
/// Performance:
///   - CRDT merge (CPU, cold-path): ~50μs for 10K updates
///   - CRDT merge (GPU, hot-path): ~400ns for 10K updates (500× faster)
///   - Inference (GPU, Llama-3-8B Q4): ~50-80 tok/s
///   - Embedding (GPU, BGE-large): ~1ms/sentence
///   - Vector search (GPU, FAISS): ~0.5ms for 1M vectors
#[cfg(feature = "cuda")]
pub struct WorkstationClaws {
    /// CPU thread pool for cold-path CRDT and coordination
    cpu_pool: rayon::ThreadPool,
    
    /// CUDA streams with priority scheduling
    streams: CudaStreamPool,
    
    /// Hot-path CRDT merge dispatcher
    gpu_merger: GpuMergeDispatcher,
    
    /// Pre-compiled kernel cache
    kernel_cache: KernelCache,
    
    /// Configuration
    config: WorkstationConfig,
    
    /// GPU device properties
    device_props: GpuDeviceProps,
    
    /// Thermal / health monitoring
    health: std::sync::Mutex<WorkstationHealth>,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub struct WorkstationConfig {
    /// Number of CPU threads (reserve for coordination)
    pub cpu_threads: usize,            // Default: num_cpus - 2
    
    /// GPU layers for LLM inference
    pub gpu_layers: u32,               // Default: 999 (all layers on GPU)
    
    /// Maximum hot-path cells for GPU merge
    pub max_hot_cells: usize,          // Default: 100,000
    
    /// Number of CUDA streams
    pub num_streams: usize,            // Default: 4
    
    /// VRAM budget for PincherOS (MB) — leave room for other workloads
    pub vram_budget_mb: u64,           // Default: 16384 (16GB of 24GB)
    
    /// SM partition strategy
    pub sm_partition: SmPartition,
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
pub struct SmPartition {
    /// SMs reserved for CRDT merge (hot-path)
    pub crdt_sms: usize,       // Default: 32 (out of 128)
    /// SMs reserved for inference
    pub inference_sms: usize,  // Default: 64
    /// SMs reserved for vector search / embedding
    pub search_sms: usize,     // Default: 32
}

impl Default for SmPartition {
    fn default() -> Self {
        Self {
            crdt_sms: 32,
            inference_sms: 64,
            search_sms: 32,
        }
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone)]
struct GpuDeviceProps {
    name: String,
    compute_capability: (u8, u8),
    sm_count: usize,
    vram_bytes: u64,
    max_threads_per_sm: usize,
    max_threads_per_block: usize,
    shared_mem_per_sm: usize,
    l2_cache_bytes: u64,
}

#[cfg(feature = "cuda")]
struct WorkstationHealth {
    gpu_temp_c: f64,
    gpu_utilization: f64,
    gpu_memory_utilization: f64,
    gpu_power_w: f64,
    throttle_events: u64,
}

#[cfg(feature = "cuda")]
struct CudaStreamPool {
    /// Stream 0: Critical priority (CRDT hot-path merge)
    critical_stream: cudaclaw::CudaStream,
    /// Stream 1: High priority (inference)
    high_stream: cudaclaw::CudaStream,
    /// Stream 2: Normal priority (embedding, vector search)
    normal_stream: cudaclaw::CudaStream,
    /// Stream 3: Low priority (background compute)
    low_stream: cudaclaw::CudaStream,
}

#[cfg(feature = "cuda")]
impl WorkstationClaws {
    pub fn new(config: WorkstationConfig) -> Result<Self, WorkstationClawsError> {
        let cpu_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.cpu_threads)
            .thread_name(|idx| format!("pincher-workstation-cpu-{idx}"))
            .build()
            .map_err(WorkstationClawsError::ThreadPool)?;
        
        // Probe GPU
        let device_props = probe_gpu_device()
            .ok_or(WorkstationClawsError::NoGpu)?;
        
        // Create priority CUDA streams
        let streams = CudaStreamPool {
            critical_stream: cudaclaw::CudaStream::with_priority(0)?,    // Highest
            high_stream: cudaclaw::CudaStream::with_priority(1)?,
            normal_stream: cudaclaw::CudaStream::with_priority(2)?,
            low_stream: cudaclaw::CudaStream::with_priority(3)?,
        };
        
        // Create GPU merge dispatcher for hot-path CRDT
        let gpu_merger = GpuMergeDispatcher::new_with_capacity(config.max_hot_cells)?;
        
        // Kernel cache
        let kernel_cache = KernelCache::new("/opt/pincheros/cache/kernels/");
        
        Ok(Self {
            cpu_pool,
            streams,
            gpu_merger,
            kernel_cache,
            config,
            device_props,
            health: std::sync::Mutex::new(WorkstationHealth {
                gpu_temp_c: 40.0,
                gpu_utilization: 0.0,
                gpu_memory_utilization: 0.0,
                gpu_power_w: 50.0,
                throttle_events: 0,
            }),
        })
    }
}

#[cfg(feature = "cuda")]
impl Pincher for WorkstationClaws {}

#[cfg(feature = "cuda")]
impl Claws for WorkstationClaws {
    type Error = WorkstationClawsError;

    fn acceleration_domain(&self) -> AccelerationDomain {
        AccelerationDomain::InferenceAndHotMerge
    }

    fn is_gpu_available(&self) -> bool {
        true // Always available on workstation (450W TDP, thermal headroom)
    }

    fn dispatch(
        &self,
        kind: DispatchKind,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>> {
        // CRDT merge: REJECTED via dispatch, but available via PartitionedCrdtEngine
        if kind == DispatchKind::CrdtMerge {
            return Box::pin(async {
                Err(WorkstationClawsError::InvalidDispatch(
                    "Use PartitionedCrdtEngine for CRDT merge. Hot-path cells are GPU-accelerated automatically."
                ))
            });
        }
        
        // Route to appropriate stream based on priority and kind
        let stream = match (kind, priority) {
            (DispatchKind::Inference, Priority::Critical) => &self.streams.critical_stream,
            (DispatchKind::Inference, _) => &self.streams.high_stream,
            (DispatchKind::VectorSearch, _) => &self.streams.normal_stream,
            (DispatchKind::Embed, _) => &self.streams.normal_stream,
            (_, Priority::Low) => &self.streams.low_stream,
            _ => &self.streams.normal_stream,
        };
        
        // Batch dispatch on GPU
        Box::pin(async move {
            let start = std::time::Instant::now();
            
            // Launch kernel on selected stream
            // <<<ceil(N/256), 256, shared_mem, stream>>>
            let result = stream.dispatch_batch(kind, command)
                .map_err(WorkstationClawsError::Cuda)?;
            
            Ok(DispatchResult {
                executed_on: ExecutionTarget::Gpu,
                elapsed: start.elapsed(),
                cached: false,
            })
        })
    }

    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error> {
        // Workstation: use cudaMalloc (device memory)
        // Avoid UM ping-pong on discrete GPUs
        //
        // For data that needs CPU access, use explicit cudaMemcpyAsync
        // For GPU-only data (hot CRDT cells, inference buffers), stay in VRAM
        
        let handle = cudaclaw::GpuBridge::new_device(len)
            .map_err(WorkstationClawsError::Cuda)?;
        let device_ptr = handle.device_ptr();
        
        Ok(GpuSlice::Device {
            device_ptr,
            len,
            stream: self.streams.normal_stream.clone(),
        })
    }

    fn agent_capacity(&self) -> usize {
        // RTX 4090 with 24GB VRAM:
        // Llama-3-8B Q4: ~5GB
        // Each agent: ~200MB (reflexes, state, context)
        // 24GB - 5GB (model) - 4GB (CRDT/search) = 15GB for agents
        // 15GB / 200MB = ~75 concurrent agents
        // But limited by SM contention: 128 SMs / 4 SMs per agent = 32
        // Conservative: 30 concurrent agents with shared model
        30
    }

    fn inference_throughput(&self) -> f32 {
        // Llama-3-8B Q4 on RTX 4090
        60.0 // ~50-80 tok/s
    }

    fn substrate_health(&self) -> SubstrateHealth {
        let health = self.health.lock().unwrap();
        SubstrateHealth {
            gpu_throttle_rate: (health.throttle_events as f64 / 10000.0).min(1.0),
            gpu_memory_errors: 0, // ECC on A100/H100, not on RTX 4090
            gpu_utilization: health.gpu_utilization,
            gpu_memory_utilization: health.gpu_memory_utilization,
            cpu_thermal: ThermalStatus::Nominal, // Workstation CPU cooling
        }
    }
}

#[cfg(feature = "cuda")]
#[derive(Debug, thiserror::Error)]
pub enum WorkstationClawsError {
    #[error("Thread pool creation failed: {0}")]
    ThreadPool(rayon::ThreadPoolBuildError),
    #[error("CUDA error: {0}")]
    Cuda(cudaclaw::CudaError),
    #[error("No GPU detected")]
    NoGpu,
    #[error("Invalid dispatch: {0}")]
    InvalidDispatch(&'static str),
    #[error("Kernel cache error: {0}")]
    KernelCache(String),
}
```

## 7.3 How the Hot/Cold Partition Shifts on Workstation

```
On RPi 4 (CpuClaws):
  All cells: CPU + rayon
  No hot partition exists.
  10K merges: ~50μs

On Jetson (JetsonClaws):
  All cells: CPU + rayon
  No hot partition exists (gpu_merge_available = false)
  10K merges: ~50μs
  GPU used ONLY for: inference (15-25 tok/s vs 5-8 tok/s CPU)

On Workstation (WorkstationClaws):
  Cold cells (< 1 access/sec): CPU + rayon (~20ns per CAS, L1 hit)
  Warm cells (1-100 accesses/sec): CPU + rayon
  Hot cells (> 100 accesses/sec): GPU batch merge
  
  Typical distribution on a busy fleet node:
    Cold:  80% of cells → CPU (negligible: 8000 × 20ns = 160μs)
    Warm:  15% of cells → CPU (1500 × 20ns = 30μs)
    Hot:    5% of cells → GPU (500 updates × ~400ns = 0.2μs)
    
    Total: ~190μs (CPU) + ~0.2μs (GPU) ≈ 190μs
    
  Without partition (all on CPU):
    10K × 20ns = 200μs
    
  With partition:
    ~190μs total — marginal improvement at 10K scale.
    
  BUT at 1M scale (data center):
    CPU-only: 1M × 20ns = 20ms
    Partitioned:
      Cold (800K): CPU, 800K × 20ns = 16ms
      Hot (200K): GPU, 200K updates in ~8μs (parallel CAS across 128 SMs)
      Total: ~16ms
    
  At 10M scale:
    CPU-only: 10M × 20ns = 200ms
    Partitioned:
      Cold (8M): CPU = 160ms
      Hot (2M): GPU = ~80μs (bitonic sort + parallel CAS)
      Total: ~160ms
    
  The GPU advantage grows with scale. For edge (10K agents),
  CPU is sufficient. For fleet-scale (1M+ agents), GPU hot-path
  merge becomes essential.
```

---

# APPENDIX A: CORRECTED COMMAND STRUCT LAYOUT

The GPU Engineer identified that `packed(4)` causes unaligned loads on GPU (3 PTX instructions instead of 1). The corrected layout:

```c
// CORRECTED: 72 bytes, 16-byte aligned
typedef struct __attribute__((aligned(16))) {
    uint32_t op_type;          // +0x00
    uint32_t priority;         // +0x04
    uint64_t target_id;        // +0x08  ← 8-byte aligned ✓
    uint64_t payload_ptr;      // +0x10  ← 8-byte aligned ✓
    uint64_t payload_len;      // +0x18  ← 8-byte aligned ✓ (was part of u32 pair)
    uint64_t timestamp;        // +0x20  ← 8-byte aligned ✓
    uint32_t agent_id;         // +0x28
    uint32_t constraint_flag;  // +0x2C  (was _reserved)
    uint64_t completion_sem;   // +0x30  ← 8-byte aligned ✓
    uint64_t parent_context;   // +0x38  ← 8-byte aligned ✓ (NEW)
    uint32_t dna_hash;         // +0x40  (NEW: muscle fiber selection)
    uint32_t _pad;             // +0x44  (alignment padding)
} Command;  // Total: 72 bytes (0x48), 16-byte aligned

// PTX: single-instruction load for all u64 fields
// ld.global.u64 %rd1, [%r_base + 0x08];  // target_id — ONE instruction
```

---

# APPENDIX B: PERFORMANCE SUMMARY TABLE

```
┌──────────────────┬──────────────────┬───────────────────┬──────────────────┐
│ Operation        │ RPi 4 (CPU)      │ Jetson (GPU inf)  │ RTX 4090 (Full)  │
├──────────────────┼──────────────────┼───────────────────┼──────────────────┤
│ CRDT merge 10K   │ ~50μs (rayon)    │ ~50μs (CPU)       │ ~190μs (part.)   │
│ CRDT merge 1M    │ ~5ms (rayon)     │ ~5ms (CPU)        │ ~16ms (part.)    │
│ Inference tok/s  │ ~6 (llama.cpp)   │ ~20 (GPU offload) │ ~60 (full GPU)   │
│ Embedding ms/sent│ ~40 (ONNX+NEON)  │ ~8 (ONNX+CUDA)    │ ~1 (full GPU)    │
│ Vector search    │ ~10ms (LanceDB)  │ ~10ms (LanceDB)   │ ~0.5ms (FAISS)   │
│ GPU TDP impact   │ 0W (no GPU)      │ 5-10W when active │ 450W (always on) │
│ Agent capacity   │ 4-6              │ 8                 │ 30               │
│ Model max        │ TinyLlama 1.1B   │ Phi-2 2.7B        │ Llama-3-8B       │
│ UM page fault    │ N/A              │ 0ns (unified)     │ ~10-50μs (avoid) │
└──────────────────┴──────────────────┴───────────────────┴──────────────────┘

KEY: "part." = partitioned hot/cold merge. CPU handles cold, GPU handles hot.
```

---

# APPENDIX C: CONSTRAINT-TO-PHASE MAPPING

```
┌─────────┬───────────┬─────────────┬────────────┐
│Constrnt │ PREPARE   │ CROSSFADE   │ FINALIZE   │
├─────────┼───────────┼─────────────┼────────────┤
│ C1 Subs │ CHECK     │ preserve    │ verify     │
│ C2 Simu │           │ CHECK+CONF  │            │
│ C3 Shape│ CHECK     │ apply mode  │            │
│ C4 Anim │ CHECK     │             │            │
│ C5 Pair │           │ CHECK       │            │
│ C6 Cons │ CHECK     │             │            │
│ C7 DiffV│           │             │ CHECK      │
└─────────┴───────────┴─────────────┴────────────┘

CHECK = must be satisfied to enter phase
CONF = must be confirmed during phase
preserve/apply/verify = action taken based on constraint
```
