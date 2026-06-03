//! PincherOS Core Types — Shared across all Claws implementations
//!
//! The 86,000× truth: GPU CRDT merge on Jetson Nano is 86,000× SLOWER
//! than CPU. CRDT merge is ALWAYS on CPU. GPU is for inference only.

use serde::{Deserialize, Serialize};

/// Marker: participates in PincherOS ecosystem, thread-safe.
pub trait Pincher: Send + Sync + 'static {}

/// A type that can reside in GPU-accessible memory.
/// Must be #[repr(C)], no Drop, no pointers, fixed layout.
pub unsafe trait GpuSafe: Copy + 'static {}

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
    CpuSimulated,
}

/// GPU command — 72 bytes, #[repr(C)], 16-byte aligned.
/// FIXED from R1: was packed(4) 48B with unaligned u64 fields.
/// Now properly aligned — PTX generates single-instruction loads.
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

// With align(16), struct size rounds up to next 16-byte multiple: 72 → 80 bytes
// Or we can add more padding to keep it at 80 explicitly.
// For now: 72 bytes of fields, but Rust will pad to 80 with align(16).
// We accept this — the GPU gets properly aligned loads.
const _: () = assert!(std::mem::size_of::<GpuCommand>() == 80);

/// Memory slice accessible from both CPU and GPU.
pub enum GpuSlice<T: GpuSafe> {
    /// CPU-only heap allocation. No GPU access.
    Cpu { data: Vec<T> },

    /// Unified memory: same physical address for CPU and GPU.
    /// Jetson Nano path: zero-copy, no page migration.
    #[cfg(feature = "cuda")]
    Unified {
        ptr: *mut T,
        len: usize,
        // handle: cudaclaw::GpuBridge<T>,
    },

    /// Device memory: GPU-only. CPU access via explicit copy.
    /// RTX 4090 path: avoids UM ping-pong for hot-path data.
    #[cfg(feature = "cuda")]
    Device {
        device_ptr: *mut T,
        len: usize,
        // stream: cudaclaw::CudaStream,
    },
}

// Safety: GpuSlice is Send + Sync because access is mediated
unsafe impl<T: GpuSafe> Send for GpuSlice<T> {}
unsafe impl<T: GpuSafe> Sync for GpuSlice<T> {}

/// Health metrics for the compute substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateHealth {
    /// GPU thermal throttle rate (0.0 = never throttled, 1.0 = always)
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

// ── Shell types (referenced by Claws trait) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellFingerprint(pub String);

impl Default for ShellFingerprint {
    fn default() -> Self {
        Self(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellProfile {
    pub fingerprint: ShellFingerprint,
    pub device_type: DeviceType,
    pub capabilities: Capabilities,
    pub limits: Limits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    TurboShell,  // RPi 4
    CudaShell,   // Jetson Nano
    BigConch,    // Workstation
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
        compute_capability: (u8, u8),
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

// ── Migration types ──

#[derive(Debug, Clone, Default)]
pub struct NailFile; // Placeholder

#[derive(Debug, Clone)]
pub struct ReflexId(pub String);

#[derive(Debug, Clone)]
pub struct RiggingProfile; // Placeholder
