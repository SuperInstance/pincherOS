//! CpuClaws — Pure CPU execution. No GPU. No fallback — this IS the path.
//!
//! Target: RPi 4 (4× A72 @ 1.5GHz, 4GB RAM, VideoCore VI display-only)
//! Performance:
//!   - CRDT merge (10K updates, rayon): ~50μs
//!   - Inference (llama.cpp CPU, TinyLlama 1.1B Q4): ~5-8 tok/s
//!   - Embedding (ONNX Runtime, MiniLM-L6): ~30-50ms/sentence
//!   - Vector search (LanceDB, 50K vectors): ~10ms

use crate::shell::claws::types::*;
use std::future::Future;
use std::pin::Pin;

/// CpuClaws: pure CPU execution. No GPU. No CUDA. No fallback — this IS the path.
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
        let freq_mhz = 1500; // RPi 4 default, should probe
        let ram_available_mb = 1536; // Conservative after OS

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

/// The Claws trait — unified compute substrate interface.
///
/// INVARIANT: CRDT merge is ALWAYS dispatched to CPU.
/// The trait REJECTS DispatchKind::CrdtMerge.
/// CRDT merge is handled by PartitionedCrdtEngine directly.
pub trait Claws: Pincher {
    type Error: std::error::Error + Send + Sync;

    /// What this substrate can accelerate.
    fn acceleration_domain(&self) -> AccelerationDomain;

    /// Whether GPU compute is physically available.
    fn is_gpu_available(&self) -> bool;

    /// Dispatch a compute operation.
    /// CRDT merges are REJECTED — use PartitionedCrdtEngine instead.
    fn dispatch(
        &self,
        kind: DispatchKind,
        command: GpuCommand,
        priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>>;

    /// Allocate memory accessible from both CPU and GPU.
    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error>;

    /// Maximum concurrent agents this substrate supports.
    fn agent_capacity(&self) -> usize;

    /// Inference throughput estimate (tokens/second).
    fn inference_throughput(&self) -> f32;

    /// The substrate's contribution to ShellQuality.
    fn substrate_health(&self) -> SubstrateHealth;
}

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
        _command: GpuCommand,
        _priority: Priority,
    ) -> Pin<Box<dyn Future<Output = Result<DispatchResult, Self::Error>> + Send + '_>> {
        if kind == DispatchKind::CrdtMerge {
            return Box::pin(async {
                Err(CpuClawsError::InvalidDispatch(
                    "CRDT merge must go through PartitionedCrdtEngine, not Claws::dispatch",
                ))
            });
        }

        Box::pin(async move {
            let start = std::time::Instant::now();

            // CPU dispatch: execute via thread pool
            // In real implementation: route to inference bridge / vector search / etc.

            Ok(DispatchResult {
                executed_on: ExecutionTarget::Cpu,
                elapsed: start.elapsed(),
                cached: false,
            })
        })
    }

    fn allocate<T: GpuSafe>(&self, len: usize) -> Result<GpuSlice<T>, Self::Error> {
        Ok(GpuSlice::Cpu {
            data: vec![unsafe { std::mem::zeroed() }; len],
        })
    }

    fn agent_capacity(&self) -> usize {
        // RPi 4 with 1.5GB available: ~4-6 concurrent agents
        let model_share_mb = 300;
        let agent_state_mb = 50;
        (self.ram_available_mb as usize / (model_share_mb + agent_state_mb)).min(6)
    }

    fn inference_throughput(&self) -> f32 {
        // TinyLlama 1.1B Q4 on 4× A72 @ 1.5GHz
        6.0
    }

    fn substrate_health(&self) -> SubstrateHealth {
        let thermal = *self.thermal.lock().unwrap();
        SubstrateHealth {
            gpu_throttle_rate: 0.0,
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
