//! PincherOS Thermodynamics — The Energy Gap Closed
//!
//! Round 3: The Thermodynamicist's perspective.
//!
//! Every computation costs joules. Landauer's principle sets the floor:
//!   E_min = k_B * T * ln(2) per bit erased.
//!
//! CRDT merge is logically reversible — Landauer does NOT mandate dissipation.
//! Shell swap erases state — Landauer DOES mandate dissipation.
//! The gastrolith protocol preserves negentropy, reducing the Landauer cost.
//! Consent topology is an entropy-reduction mechanism.
//!
//! THE CENTRAL INSIGHT: The hermit crab model is thermodynamically superior
//! to the "grow your own shell" model because shell reuse avoids the
//! irreversible initialization cost. Borrowing shells is computational recycling.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ═══════════════════════════════════════════════════════════════════
// SECTION 1: PHYSICAL CONSTANTS
// ═══════════════════════════════════════════════════════════════════

/// Boltzmann constant: 1.381 × 10⁻²³ J/K
pub const BOLTZMANN: f64 = 1.381e-23;

/// Natural log of 2: 0.6931...
pub const LN2: f64 = std::f64::consts::LN_2;

/// Room temperature: 300 K (typical operating environment)
pub const ROOM_TEMP_K: f64 = 300.0;

/// RPi 4 thermal throttle temperature: 80°C = 353.15 K
pub const RPI4_THROTTLE_TEMP_K: f64 = 353.15;

/// Jetson Nano thermal throttle: 97°C = 370.15 K (Tegra X1 junction temp)
pub const JETSON_THROTTLE_TEMP_K: f64 = 370.15;

/// Landauer limit at room temperature: k_B * T * ln(2) ≈ 2.87 × 10⁻²¹ J
/// This is the MINIMUM energy to erase one bit of information.
pub fn landauer_limit(temp_k: f64) -> f64 {
    BOLTZMANN * temp_k * LN2
}

/// Landauer limit at 300K (precomputed for convenience)
pub const LANDAUER_ROOM_TEMP: f64 = 2.869e-21; // J per bit

// ═══════════════════════════════════════════════════════════════════
// SECTION 2: PLATFORM THERMAL PROFILES
// ═══════════════════════════════════════════════════════════════════

/// Thermal and energy profile for a specific hardware platform.
/// This makes energy a FIRST-CLASS quantity in the PincherOS type system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformThermalProfile {
    /// Platform name (e.g., "rpi4", "jetson_nano_5w", "jetson_nano_10w")
    pub platform_id: String,

    /// Maximum power draw (watts)
    pub tdp_watts: f64,

    /// Idle power draw (watts) — system at rest
    pub idle_power_watts: f64,

    /// SoC thermal resistance (°C/W) — determines how fast heat builds
    pub thermal_resistance_c_per_w: f64,

    /// Throttle temperature (°C)
    pub throttle_temp_c: f64,

    /// Hard shutdown temperature (°C)
    pub shutdown_temp_c: f64,

    /// Typical ambient temperature assumption (°C)
    pub assumed_ambient_c: f64,

    /// Available RAM (bytes)
    pub ram_bytes: u64,

    /// CPU frequency under load (MHz)
    pub cpu_freq_mhz: u64,

    /// Peak FLOP/s (NEON on RPi, CUDA on Jetson)
    pub peak_flops: f64,

    /// Whether GPU compute is available
    pub gpu_compute_available: bool,

    /// Power cost of GPU if available (watts, additional to CPU load)
    pub gpu_additional_power_watts: f64,
}

impl PlatformThermalProfile {
    /// RPi 4 (4GB): The workhorse edge device.
    /// Passive cooling, Cortex-A72, no GPU compute.
    pub fn rpi4() -> Self {
        Self {
            platform_id: "rpi4_4gb".into(),
            tdp_watts: 7.5,       // Peak total board power
            idle_power_watts: 2.8, // Idle with networking
            thermal_resistance_c_per_w: 12.0, // Heatsink + case
            throttle_temp_c: 80.0,
            shutdown_temp_c: 85.0,
            assumed_ambient_c: 25.0,
            ram_bytes: 4 * 1024 * 1024 * 1024,
            cpu_freq_mhz: 1500,
            peak_flops: 12e9, // 4 cores × 1.5 GHz × 2 FLOP/cycle (NEON)
            gpu_compute_available: false,
            gpu_additional_power_watts: 0.0,
        }
    }

    /// Jetson Nano 5W mode: Power-constrained, 921 MHz, GPU available but expensive.
    pub fn jetson_nano_5w() -> Self {
        Self {
            platform_id: "jetson_nano_5w".into(),
            tdp_watts: 5.0,
            idle_power_watts: 1.5,
            thermal_resistance_c_per_w: 15.0, // Passive heatsink
            throttle_temp_c: 97.0,
            shutdown_temp_c: 102.0,
            assumed_ambient_c: 25.0,
            ram_bytes: 4 * 1024 * 1024 * 1024,
            cpu_freq_mhz: 921,
            peak_flops: 22e9, // 4 A57 + 128 Maxwell CUDA cores
            gpu_compute_available: true,
            gpu_additional_power_watts: 2.0, // GPU adds ~2W when active
        }
    }

    /// Jetson Nano 10W mode: Higher performance, more heat.
    pub fn jetson_nano_10w() -> Self {
        Self {
            platform_id: "jetson_nano_10w".into(),
            tdp_watts: 10.0,
            idle_power_watts: 2.0,
            thermal_resistance_c_per_w: 15.0,
            throttle_temp_c: 97.0,
            shutdown_temp_c: 102.0,
            assumed_ambient_c: 25.0,
            ram_bytes: 4 * 1024 * 1024 * 1024,
            cpu_freq_mhz: 1400,
            peak_flops: 48e9, // Higher clocks + GPU
            gpu_compute_available: true,
            gpu_additional_power_watts: 4.0,
        }
    }

    /// Workstation: Desktop with RTX 4090.
    pub fn workstation() -> Self {
        Self {
            platform_id: "workstation_rtx4090".into(),
            tdp_watts: 800.0, // CPU + GPU combined
            idle_power_watts: 80.0,
            thermal_resistance_c_per_w: 0.15, // Active liquid/air cooling
            throttle_temp_c: 90.0,
            shutdown_temp_c: 105.0,
            assumed_ambient_c: 22.0,
            ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_freq_mhz: 4500,
            peak_flops: 83e12, // RTX 4090: ~83 TFLOP/s FP16
            gpu_compute_available: true,
            gpu_additional_power_watts: 450.0,
        }
    }

    /// Maximum sustainable power before thermal throttle.
    /// P_max = (T_throttle - T_ambient) / R_thermal
    pub fn max_sustainable_power_watts(&self) -> f64 {
        let margin = self.throttle_temp_c - self.assumed_ambient_c;
        (margin / self.thermal_resistance_c_per_w).min(self.tdp_watts)
    }

    /// Steady-state temperature at given power draw (°C).
    pub fn temperature_at_power(&self, power_watts: f64) -> f64 {
        self.assumed_ambient_c + power_watts * self.thermal_resistance_c_per_w
    }

    /// Thermal headroom: how many watts can we add before throttle?
    pub fn thermal_headroom_watts(&self, current_power_watts: f64) -> f64 {
        self.max_sustainable_power_watts() - current_power_watts
    }

    /// Computational efficiency: FLOP per joule at peak.
    pub fn compute_efficiency_flop_per_joule(&self) -> f64 {
        if self.tdp_watts > 0.0 {
            self.peak_flops / self.tdp_watts
        } else {
            0.0
        }
    }

    /// Carnot efficiency of computation:
    /// η = (Logical work in Landauer units) / (Actual energy consumed)
    /// For 1 bit operation at peak: η = kT·ln(2) / E_actual
    pub fn carnot_efficiency(&self) -> f64 {
        let landauer = landauer_limit(self.assumed_ambient_c + 273.15);
        let energy_per_flop = self.tdp_watts / self.peak_flops;
        landauer / energy_per_flop
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 3: LANDAUER COST OF SHELL SWAP
// ═══════════════════════════════════════════════════════════════════

/// The thermodynamic cost of a shell swap (migration).
///
/// A Shell Swap erases the old composite's state in the old shell.
/// The Landauer principle mandates: E_min = kT·ln(2) per bit erased.
///
/// The gastrolith protocol preserves "negentropy" — useful information
/// is checkpointed locally, reducing the irreversibly erased bits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSwapThermodynamics {
    /// Size of the old shell's composite state (bits)
    pub old_state_bits: u64,

    /// Size of the gastrolith checkpoint (bits preserved as negentropy)
    pub gastrolith_bits: u64,

    /// Operating temperature during swap (K)
    pub temperature_k: f64,

    /// Time the swap takes (seconds)
    pub swap_duration_secs: f64,

    /// Actual power consumed during swap (watts)
    pub actual_power_watts: f64,
}

impl ShellSwapThermodynamics {
    /// Theoretical minimum energy for this shell swap (joules).
    /// Only the IRREVERSIBLY ERASED bits cost Landauer energy.
    /// Preserved bits (gastrolith) are negentropy — they avoid the cost.
    ///
    /// E_min = (old_state_bits - gastrolith_bits) × kT·ln(2)
    pub fn landauer_minimum_joules(&self) -> f64 {
        let erased_bits = self.old_state_bits.saturating_sub(self.gastrolith_bits);
        erased_bits as f64 * landauer_limit(self.temperature_k)
    }

    /// Actual energy consumed during the swap (joules).
    pub fn actual_energy_joules(&self) -> f64 {
        self.actual_power_watts * self.swap_duration_secs
    }

    /// Thermodynamic overhead factor: how many times above Landauer?
    pub fn overhead_factor(&self) -> f64 {
        let min = self.landauer_minimum_joules();
        if min > 0.0 {
            self.actual_energy_joules() / min
        } else {
            f64::INFINITY
        }
    }

    /// Negentropy preserved by the gastrolith (joules of AVOIDED cost).
    /// This is the energy the gastrolith SAVES by preventing erasure.
    pub fn negentropy_preserved_joules(&self) -> f64 {
        self.gastrolith_bits as f64 * landauer_limit(self.temperature_k)
    }

    /// Gastrolith efficiency: fraction of old state preserved as negentropy.
    pub fn gastrolith_preservation_ratio(&self) -> f64 {
        if self.old_state_bits > 0 {
            self.gastrolith_bits as f64 / self.old_state_bits as f64
        } else {
            0.0
        }
    }

    /// Entropy produced: the information-theoretic entropy of the erased bits.
    /// ΔS = H(Old | Gastrolith) = (erased_bits) × ln(2) in nats
    /// In bits: ΔS_bits = erased_bits
    pub fn entropy_produced_bits(&self) -> u64 {
        self.old_state_bits.saturating_sub(self.gastrolith_bits)
    }

    /// Entropy production rate (bits/second).
    pub fn entropy_rate_bits_per_sec(&self) -> f64 {
        if self.swap_duration_secs > 0.0 {
            self.entropy_produced_bits() as f64 / self.swap_duration_secs
        } else {
            0.0
        }
    }
}

/// Pre-computed thermodynamic costs for standard migration scenarios.
pub struct MigrationEnergyCalculator;

impl MigrationEnergyCalculator {
    /// Calculate swap thermodynamics for a typical agent on RPi 4.
    ///
    /// Assumptions:
    /// - Agent composite state: ~50 KB (reflexes + embeddings + trust + config)
    /// - Gastrolith preserves: personality + trust scores + reflex patterns
    ///   (substance), adapts embeddings + sandbox profiles (accidents)
    /// - Substance ratio ~0.65 (from the Greek substance/accident partition)
    pub fn rpi4_shell_swap(
        state_kb: u64,
        substance_ratio: f64,
    ) -> ShellSwapThermodynamics {
        let state_bits = state_kb * 1024 * 8;
        let gastrolith_bits = (state_bits as f64 * substance_ratio) as u64;

        ShellSwapThermodynamics {
            old_state_bits: state_bits,
            gastrolith_bits,
            temperature_k: 323.15, // 50°C typical operating temp on RPi 4
            swap_duration_secs: 0.5, // 500ms for pack + transfer + unpack
            actual_power_watts: 5.0, // Full CPU during swap
        }
    }

    /// Calculate swap WITHOUT gastrolith (baseline for comparison).
    /// The entire old state is discarded — maximum Landauer cost.
    pub fn rpi4_shell_swap_no_gastrolith(state_kb: u64) -> ShellSwapThermodynamics {
        let state_bits = state_kb * 1024 * 8;

        ShellSwapThermodynamics {
            old_state_bits: state_bits,
            gastrolith_bits: 0, // Nothing preserved!
            temperature_k: 323.15,
            swap_duration_secs: 0.3, // Slightly faster without checkpoint
            actual_power_watts: 5.0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 4: CRDT THERMODYNAMICS
// ═══════════════════════════════════════════════════════════════════

/// Thermodynamic classification of CRDT operations.
///
/// KEY INSIGHT: Landauer's principle only applies to LOGICALLY IRREVERSIBLE
/// operations. If all inputs are recoverable from the output, the operation
/// CAN (in principle) be performed at zero thermodynamic cost.
///
/// CRDT merge types differ in their logical reversibility:
/// - G-Counter merge: max(a, b) — IRREVERSIBLE (can't recover smaller value)
/// - PN-Counter merge: max for each component — IRREVERSIBLE
/// - LWW-Register: last-writer-wins — IRREVERSIBLE (older value lost)
/// - OR-Set merge: union + tombstone — PARTIALLY REVERSIBLE (tombstones preserve)
/// - Operation-based CRDT: log-preserving — REVERSIBLE (log retains history)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CrdtReversibility {
    /// Fully reversible: all inputs recoverable from output + metadata.
    /// Operation-based CRDTs with complete logs.
    /// Landauer cost: ZERO (in principle)
    Reversible,

    /// Partially reversible: some inputs recoverable via metadata (tombstones).
    /// OR-Sets, causal CRDTs.
    /// Landauer cost: proportional to non-recoverable fraction
    PartiallyReversible {
        /// Fraction of input information that is recoverable (0.0-1.0)
        recoverable_fraction: f64,
    },

    /// Irreversible: inputs cannot be recovered from output.
    /// G-Counters, PN-Counters, LWW-Registers.
    /// Landauer cost: FULL (kT·ln(2) per merged bit)
    Irreversible,
}

impl CrdtReversibility {
    /// The thermodynamic cost multiplier for this CRDT type.
    /// 0.0 = free (reversible), 1.0 = full cost (irreversible)
    pub fn landauer_multiplier(&self) -> f64 {
        match self {
            CrdtReversibility::Reversible => 0.0,
            CrdtReversibility::PartiallyReversible { recoverable_fraction } => {
                1.0 - recoverable_fraction
            }
            CrdtReversibility::Irreversible => 1.0,
        }
    }

    /// Minimum energy for a merge of `bits` bits at temperature `temp_k`.
    pub fn landauer_cost_joules(&self, bits: u64, temp_k: f64) -> f64 {
        bits as f64 * landauer_limit(temp_k) * self.landauer_multiplier()
    }
}

/// Thermodynamic cost of a CRDT merge operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtMergeThermodynamics {
    /// Size of the merged state (bits)
    pub merged_bits: u64,

    /// Reversibility classification of the merge
    pub reversibility: CrdtReversibility,

    /// Operating temperature (K)
    pub temperature_k: f64,

    /// Actual energy consumed (joules)
    pub actual_energy_joules: f64,
}

impl CrdtMergeThermodynamics {
    /// Theoretical minimum energy for this merge (joules).
    pub fn landauer_minimum_joules(&self) -> f64 {
        self.reversibility.landauer_cost_joules(self.merged_bits, self.temperature_k)
    }

    /// Overhead above Landauer limit.
    pub fn overhead_factor(&self) -> f64 {
        let min = self.landauer_minimum_joules();
        if min > 0.0 {
            self.actual_energy_joules / min
        } else {
            f64::INFINITY // Reversible merge: actual cost should approach zero
        }
    }

    /// Whether this merge can theoretically be done at zero thermodynamic cost.
    pub fn is_thermodynamically_free(&self) -> bool {
        matches!(self.reversibility, CrdtReversibility::Reversible)
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 5: THERMAL CARRYING CAPACITY
// ═══════════════════════════════════════════════════════════════════

/// Thermal carrying capacity: how many agents can a shell support
/// before thermal throttling degrades performance below usable thresholds?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalCarryingCapacity {
    /// The platform profile
    pub platform: PlatformThermalProfile,

    /// Power consumed by the base system (OS, networking, etc.)
    pub base_power_watts: f64,

    /// Power consumed by a single active agent (LLM inference + CRDT)
    pub active_agent_power_watts: f64,

    /// Power consumed by a single idle agent (CRDT heartbeat only)
    pub idle_agent_power_mw: f64,

    /// Ratio of idle to active agents (typical workload)
    pub idle_to_active_ratio: f64,

    /// Minimum acceptable inference rate (tokens/second) below which
    /// the agent is considered non-functional
    pub min_tokens_per_sec: f64,

    /// LLM inference throughput at full power (tokens/second)
    pub peak_tokens_per_sec: f64,
}

impl ThermalCarryingCapacity {
    /// Create for RPi 4 with realistic parameters.
    pub fn rpi4() -> Self {
        Self {
            platform: PlatformThermalProfile::rpi4(),
            base_power_watts: 2.8,
            active_agent_power_watts: 2.0, // Full CPU for inference
            idle_agent_power_mw: 0.5,      // Just data structures + heartbeat
            idle_to_active_ratio: 10.0,     // 10 idle per 1 active
            min_tokens_per_sec: 1.0,
            peak_tokens_per_sec: 6.0,
        }
    }

    /// Maximum sustainable power budget (watts).
    /// This is the power at which the system reaches throttle temperature.
    pub fn power_budget_watts(&self) -> f64 {
        self.platform.max_sustainable_power_watts()
    }

    /// Available power for agents after base system overhead.
    pub fn agent_power_budget_watts(&self) -> f64 {
        (self.power_budget_watts() - self.base_power_watts).max(0.0)
    }

    /// Maximum number of active agents before thermal throttle.
    /// N_active × P_active + N_idle × P_idle ≤ P_agent_budget
    /// With N_idle = ratio × N_active:
    /// N_active × (P_active + ratio × P_idle_mW/1000) ≤ P_budget
    pub fn max_active_agents(&self) -> f64 {
        let budget = self.agent_power_budget_watts();
        let idle_power_w = self.idle_agent_power_mw / 1000.0;
        let per_active_total = self.active_agent_power_watts
            + self.idle_to_active_ratio * idle_power_w;
        if per_active_total > 0.0 {
            budget / per_active_total
        } else {
            0.0
        }
    }

    /// Total carrying capacity (active + idle agents).
    pub fn total_carrying_capacity(&self) -> f64 {
        self.max_active_agents() * (1.0 + self.idle_to_active_ratio)
    }

    /// Thermal margin at a given number of agents.
    /// Returns remaining watts before throttle.
    pub fn thermal_margin_at(&self, n_active: usize, n_idle: usize) -> f64 {
        let idle_power_w = self.idle_agent_power_mw / 1000.0;
        let total_power = self.base_power_watts
            + n_active as f64 * self.active_agent_power_watts
            + n_idle as f64 * idle_power_w;
        self.power_budget_watts() - total_power
    }

    /// Performance degradation factor at given agent count.
    /// When approaching thermal throttle, the CPU slows down.
    /// This models the non-linear relationship between load and throughput.
    ///
    /// Returns a factor 0.0-1.0 where 1.0 = full performance.
    pub fn performance_factor(&self, n_active: usize, n_idle: usize) -> f64 {
        let margin = self.thermal_margin_at(n_active, n_idle);
        let budget = self.agent_power_budget_watts();

        if margin >= budget * 0.3 {
            // Plenty of headroom: full performance
            1.0
        } else if margin > 0.0 {
            // Approaching throttle: performance degrades quadratically
            let fraction = margin / (budget * 0.3);
            fraction * fraction
        } else {
            // At or past throttle: severe degradation
            // RPi 4 throttles to 1.0 GHz from 1.5 GHz = 67% performance
            0.67 * (1.0 + margin / budget).max(0.1)
        }
    }

    /// Usable carrying capacity: agents above minimum performance threshold.
    /// This accounts for the fact that throttled agents may be non-functional.
    pub fn usable_carrying_capacity(&self) -> usize {
        // Binary search for the largest N where performance_factor > threshold
        let perf_threshold = self.min_tokens_per_sec / self.peak_tokens_per_sec;
        let mut lo = 0usize;
        let mut hi = 10000usize;

        while lo < hi {
            let mid = lo + (hi - lo + 1) / 2;
            let n_idle = (mid as f64 * self.idle_to_active_ratio) as usize;
            let perf = self.performance_factor(mid, n_idle);
            if perf >= perf_threshold {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        let n_idle = (lo as f64 * self.idle_to_active_ratio) as usize;
        lo + n_idle
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 6: ENERGY-AWARE MIGRATION DECISIONS
// ═══════════════════════════════════════════════════════════════════

/// Energy cost of a migration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEnergyCost {
    /// Energy to serialize rigging to .nail format (joules)
    pub serialize_joules: f64,

    /// Energy to transfer .nail over network (joules)
    pub network_transfer_joules: f64,

    /// Energy to deserialize + snap on target (joules)
    pub deserialize_joules: f64,

    /// Landauer cost of state erasure at source (joules)
    pub erasure_joules: f64,

    /// Energy for CRDT state reconciliation post-migration (joules)
    pub crdt_reconciliation_joules: f64,

    /// Energy for verification (top-5 reflex test) (joules)
    pub verification_joules: f64,
}

impl MigrationEnergyCost {
    /// Total migration energy (joules).
    pub fn total_joules(&self) -> f64 {
        self.serialize_joules
            + self.network_transfer_joules
            + self.deserialize_joules
            + self.erasure_joules
            + self.crdt_reconciliation_joules
            + self.verification_joules
    }

    /// Estimate migration cost for a given platform and state size.
    pub fn estimate(
        platform: &PlatformThermalProfile,
        state_kb: u64,
        substance_ratio: f64,
        network_latency_ms: f64,
    ) -> Self {
        // Serialize: CPU-bound, ~1ms per 10KB on ARM
        let serialize_time = (state_kb as f64 / 10000.0).max(0.001);
        let serialize_j = platform.tdp_watts * 0.5 * serialize_time;

        // Network: power for WiFi/Ethernet (~0.5W) × transfer time
        let network_bandwidth_kbps = 5000.0; // 5 MB/s typical
        let transfer_time = state_kb as f64 / network_bandwidth_kbps;
        let network_j = 0.5 * (transfer_time + network_latency_ms / 1000.0);

        // Deserialize: similar to serialize
        let deserialize_j = serialize_j;

        // Landauer erasure cost (the irreducible minimum)
        let erased_bits = (state_kb * 1024 * 8) as f64 * (1.0 - substance_ratio);
        let erasure_j = erased_bits * landauer_limit(platform.assumed_ambient_c + 273.15);

        // CRDT reconciliation: ~10ms of CPU
        let crdt_j = platform.tdp_watts * 0.3 * 0.01;

        // Verification: ~50ms of CPU for top-5 reflex test
        let verify_j = platform.tdp_watts * 0.3 * 0.05;

        Self {
            serialize_joules: serialize_j,
            network_transfer_joules: network_j,
            deserialize_joules: deserialize_j,
            erasure_joules: erasure_j,
            crdt_reconciliation_joules: crdt_j,
            verification_joules: verify_j,
        }
    }
}

/// The energy-optimal migration decision.
///
/// Should an agent stay or migrate? This is an energy balance:
///   Migrate if: E_stay(t_remaining) > E_migrate + E_operate_at_new(t_remaining)
///
/// The key variable is THERMAL EFFICIENCY: a throttled CPU does the same
/// work in more time, consuming MORE total energy for the same result.
/// Migration to a cooler shell is thermodynamically advantageous.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationDecision {
    /// Energy cost of migrating (joules)
    pub migration_cost_joules: f64,

    /// Projected energy cost of STAYING for the next hour (joules)
    pub stay_cost_joules: f64,

    /// Projected energy cost at the NEW shell for the next hour (joules)
    pub new_shell_cost_joules: f64,

    /// Net energy SAVINGS from migration (joules, negative = migration is wasteful)
    pub net_energy_savings_joules: f64,

    /// Whether migration is thermodynamically justified
    pub should_migrate: bool,

    /// The thermal efficiency at current shell (0.0-1.0)
    pub current_thermal_efficiency: f64,

    /// The thermal efficiency at new shell (0.0-1.0)
    pub new_thermal_efficiency: f64,

    /// Reason for the decision
    pub reason: MigrationDecisionReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationDecisionReason {
    /// Thermal headroom at current shell is sufficient; no benefit to migration
    ThermallyComfortable,

    /// Current shell is approaching throttle; migration saves energy
    /// by avoiding throttled (inefficient) computation
    ThermalEscape,

    /// New shell has significantly better compute-per-joule
    /// (e.g., moving from RPi to Jetson for GPU inference)
    BetterComputeEfficiency,

    /// Migration cost exceeds projected savings; stay put
    MigrationTooExpensive,

    /// Battery critically low; must migrate to AC-powered shell
    BatteryEmergency,

    /// Consent denied; migration blocked regardless of energy benefit
    ConsentBlocked,
}

impl MigrationDecision {
    /// Compute the energy-optimal migration decision.
    pub fn evaluate(
        current: &PlatformThermalProfile,
        target: &PlatformThermalProfile,
        state_kb: u64,
        substance_ratio: f64,
        current_temp_c: f64,
        current_load_watts: f64,
        hours_ahead: f64,
    ) -> Self {
        // Migration cost
        let migration_cost = MigrationEnergyCost::estimate(
            current,
            state_kb,
            substance_ratio,
            10.0, // 10ms network latency
        );

        // Thermal efficiency at current shell
        let current_temp_fraction =
            (current_temp_c - current.assumed_ambient_c)
            / (current.throttle_temp_c - current.assumed_ambient_c);
        let current_eff = if current_temp_fraction < 0.7 {
            1.0
        } else {
            // Efficiency degrades as we approach throttle
            1.0 - 0.5 * ((current_temp_fraction - 0.7) / 0.3).powi(2)
        };

        // Projected energy cost of staying
        let secs = hours_ahead * 3600.0;
        let stay_joules = current_load_watts * secs / current_eff.max(0.1);

        // Thermal efficiency at target shell (assume lower temp)
        let target_power = current_load_watts * (current.peak_flops / target.peak_flops).min(1.0);
        let target_temp = target.temperature_at_power(target_power);
        let target_temp_fraction =
            (target_temp - target.assumed_ambient_c)
            / (target.throttle_temp_c - target.assumed_ambient_c);
        let target_eff = if target_temp_fraction < 0.7 {
            1.0
        } else {
            1.0 - 0.5 * ((target_temp_fraction - 0.7) / 0.3).powi(2)
        };

        // Projected energy cost at new shell
        let new_shell_joules = target_power * secs / target_eff.max(0.1);

        // Net savings
        let net_savings = stay_joules - (migration_cost.total_joules() + new_shell_joules);

        let should_migrate = net_savings > 0.0;

        let reason = if current_temp_fraction > 0.85 {
            MigrationDecisionReason::ThermalEscape
        } else if target_eff > current_eff + 0.2 {
            MigrationDecisionReason::BetterComputeEfficiency
        } else if net_savings <= 0.0 {
            MigrationDecisionReason::MigrationTooExpensive
        } else {
            MigrationDecisionReason::ThermallyComfortable
        };

        Self {
            migration_cost_joules: migration_cost.total_joules(),
            stay_cost_joules: stay_joules,
            new_shell_cost_joules: new_shell_joules,
            net_energy_savings_joules: net_savings,
            should_migrate,
            current_thermal_efficiency: current_eff,
            new_thermal_efficiency: target_eff,
            reason,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 7: CONSENT AS ENTROPY REDUCTION
// ═══════════════════════════════════════════════════════════════════

/// Consent as an entropy-reduction mechanism.
///
/// The category theorist showed that consent constrains the shadowgap
/// topology. The thermodynamicist shows that consent IS a thermodynamic
/// constraint: it limits entropy production by restricting migrations.
///
/// More consent restrictions → fewer allowed migrations → less entropy produced.
/// The consent topology is analogous to a thermal insulator.
///
/// But consent itself has a thermodynamic cost (Maxwell's demon):
/// - Evaluating consent rules costs energy
/// - Maintaining consent data structures costs energy
/// - The cost of the "demon" must be less than the entropy reduction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentThermodynamics {
    /// Number of migrations BLOCKED by consent in the last hour
    pub migrations_blocked: u64,

    /// Average entropy per migration (bits) that would have been produced
    pub entropy_per_migration_bits: f64,

    /// Total entropy AVOIDED by consent in the last hour (bits)
    pub entropy_avoided_bits: f64,

    /// Energy cost of consent evaluation (joules per evaluation)
    pub consent_evaluation_cost_joules: f64,

    /// Number of consent evaluations in the last hour
    pub consent_evaluations: u64,

    /// Total energy cost of running consent (the demon's cost)
    pub consent_overhead_joules: f64,
}

impl ConsentThermodynamics {
    /// Is consent thermodynamically net-positive?
    /// The entropy avoided must exceed the demon's energy cost (in Landauer units).
    pub fn is_net_positive(&self, temp_k: f64) -> bool {
        let avoided_energy = self.entropy_avoided_bits as f64 * landauer_limit(temp_k);
        avoided_energy > self.consent_overhead_joules
    }

    /// Consent efficiency: entropy avoided per joule of consent overhead.
    pub fn consent_efficiency_bits_per_joule(&self) -> f64 {
        if self.consent_overhead_joules > 0.0 {
            self.entropy_avoided_bits as f64 / self.consent_overhead_joules
        } else {
            f64::INFINITY
        }
    }

    /// The consent insulator factor: how much entropy production is
    /// reduced relative to unconstrained migration.
    /// 0.0 = no insulation (all migrations allowed)
    /// 1.0 = perfect insulation (no migrations allowed)
    pub fn insulation_factor(&self, total_migration_requests: u64) -> f64 {
        if total_migration_requests > 0 {
            self.migrations_blocked as f64 / total_migration_requests as f64
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 8: HERMIT CRAB vs. GROW-YOUR-OWN THERMODYNAMICS
// ═══════════════════════════════════════════════════════════════════

/// The deep comparison: is borrowing shells thermodynamically superior
/// to growing them?
///
/// GROW-YOUR-OWN MODEL (like snails):
/// - Each agent initializes its own container from scratch
/// - E_init = full cost of container setup (OS bootstrap, dependency loading,
///   memory allocation, capability provisioning)
/// - This is LOGICALLY IRREVERSIBLE: the uninitialized state is destroyed
/// - Landauer cost: E_init_landauer = init_bits × kT × ln(2)
///
/// HERMIT CRAB MODEL (PincherOS):
/// - Shells are pre-built and reused
/// - Migration cost: E_migrate = serialize + transfer + deserialize + adapt
/// - The gastrolith preserves negentropy
/// - Only adaptation is irreversible (accidents change, substance preserved)
///
/// THEOREM: If E_migrate < E_init, the hermit crab model is thermodynamically
/// superior. This is almost always true because:
///   1. Shell configuration is preserved (no re-initialization)
///   2. Dependencies are pre-loaded
///   3. Network state is maintained by the shell
///   4. The gastrolith reduces irreversible erasure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStrategyComparison {
    /// Energy to initialize a new container from scratch (joules)
    pub grow_init_joules: f64,

    /// Bits irreversibly erased during initialization
    pub grow_erased_bits: u64,

    /// Energy to migrate to an existing shell (joules)
    pub borrow_migrate_joules: f64,

    /// Bits irreversibly erased during migration
    pub borrow_erased_bits: u64,

    /// Bits preserved as negentropy by gastrolith
    pub gastrolith_preserved_bits: u64,

    /// Temperature (K)
    pub temperature_k: f64,
}

impl ShellStrategyComparison {
    /// Compare strategies for a typical agent.
    pub fn typical_agent(temp_k: f64) -> Self {
        // GROW: Initialize a container from scratch
        // - OS bootstrap: ~2 seconds at 5W = 10 J
        // - Model loading: ~10 seconds at 5W = 50 J
        // - Dependency resolution: ~5 seconds at 3W = 15 J
        // - Sandbox setup: ~1 second at 3W = 3 J
        // Total: ~78 J
        // Erased bits: entire uninitialized state (~10 MB of zero-initialized memory
        // overwritten with actual state = 80,000,000 bits of irreversible erasure)
        let grow_init = 78.0;
        let grow_erased = 80_000_000u64; // ~10 MB

        // BORROW: Migrate to existing shell
        // - Serialize: ~0.5 seconds at 2W = 1.0 J
        // - Network transfer: ~0.2 seconds at 0.5W = 0.1 J
        // - Deserialize + snap: ~0.5 seconds at 2W = 1.0 J
        // - Adapt (re-embed): ~1 second at 3W = 3.0 J
        // - Verify: ~0.5 seconds at 3W = 1.5 J
        // Total: ~6.6 J
        // Erased bits: only the accident fields that change (~1 MB = 8,000,000 bits)
        let borrow_migrate = 6.6;
        let borrow_erased = 8_000_000u64;
        let gastrolith_preserved = 32_000_000u64; // ~4 MB of substance preserved

        Self {
            grow_init_joules: grow_init,
            grow_erased_bits: grow_erased,
            borrow_migrate_joules: borrow_migrate,
            borrow_erased_bits: borrow_erased,
            gastrolith_preserved_bits: gastrolith_preserved,
            temperature_k: temp_k,
        }
    }

    /// Energy savings of the hermit crab model (joules).
    pub fn energy_savings_joules(&self) -> f64 {
        self.grow_init_joules - self.borrow_migrate_joules
    }

    /// Entropy savings of the hermit crab model (bits not erased).
    pub fn entropy_savings_bits(&self) -> u64 {
        self.grow_erased_bits.saturating_sub(self.borrow_erased_bits)
    }

    /// Landauer cost of the grow model (joules).
    pub fn grow_landauer_joules(&self) -> f64 {
        self.grow_erased_bits as f64 * landauer_limit(self.temperature_k)
    }

    /// Landauer cost of the borrow model (joules).
    pub fn borrow_landauer_joules(&self) -> f64 {
        self.borrow_erased_bits as f64 * landauer_limit(self.temperature_k)
    }

    /// Thermodynamic superiority ratio: how many times more efficient is borrowing?
    pub fn superiority_ratio(&self) -> f64 {
        if self.borrow_migrate_joules > 0.0 {
            self.grow_init_joules / self.borrow_migrate_joules
        } else {
            f64::INFINITY
        }
    }

    /// Negentropy preserved by the gastrolith (in Landauer joules).
    pub fn gastrolith_negentropy_joules(&self) -> f64 {
        self.gastrolith_preserved_bits as f64 * landauer_limit(self.temperature_k)
    }

    /// Amortization: how many borrows before the shell's creation cost
    /// is amortized below the grow cost per use?
    ///
    /// If a shell is created once (E_create) and borrowed N times (E_borrow each),
    /// the amortized cost per use is: (E_create + N × E_borrow) / N
    /// The grow cost per use is always: E_init
    ///
    /// Break-even: N = E_create / (E_init - E_borrow)
    pub fn amortization_break_even(&self, shell_creation_joules: f64) -> f64 {
        let savings_per_use = self.grow_init_joules - self.borrow_migrate_joules;
        if savings_per_use > 0.0 {
            shell_creation_joules / savings_per_use
        } else {
            f64::INFINITY
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 9: ENERGY STATE (for ShellQuality integration)
// ═══════════════════════════════════════════════════════════════════

/// Energy state of a shell — first-class thermodynamic metadata.
///
/// This closes the Energy Gap (SG-4 from the Polyformalist Synthesis).
/// Every shell operation now has a joules dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyState {
    /// Whether the shell is running on battery
    pub on_battery: bool,

    /// Battery charge level: 0.0 (empty) to 1.0 (full)
    pub battery_pct: f64,

    /// Estimated remaining battery life (minutes)
    pub estimated_remaining_mins: u64,

    /// Current power draw (watts, positive = consuming)
    pub power_draw_watts: f64,

    /// Power source max capacity (watts)
    pub power_source_max_watts: f64,

    /// Cumulative energy consumed since boot (joules)
    pub total_energy_joules: f64,

    /// Thermal profile of the platform
    pub platform: PlatformThermalProfile,

    /// Energy policy
    pub policy: EnergyPolicy,
}

/// Energy policy — determines how aggressively to conserve energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyPolicy {
    /// Maximum performance. AC power assumed.
    Performance,

    /// Balance performance and battery life.
    Balanced,

    /// Conserve battery. Reflex-only mode (no LLM inference).
    Saver,

    /// Battery < 10%. Minimal operation. Emergency.
    Emergency,
}

impl EnergyState {
    /// Determine the appropriate energy policy based on current state.
    pub fn auto_policy(&self) -> EnergyPolicy {
        if !self.on_battery {
            return EnergyPolicy::Performance;
        }
        match self.battery_pct {
            p if p > 0.50 => EnergyPolicy::Balanced,
            p if p > 0.20 => EnergyPolicy::Saver,
            _ => EnergyPolicy::Emergency,
        }
    }

    /// Available compute power budget (watts).
    /// On battery: limited by battery discharge rate.
    /// On AC: limited by thermal throttle.
    pub fn compute_budget_watts(&self) -> f64 {
        match self.policy {
            EnergyPolicy::Performance => {
                self.platform.max_sustainable_power_watts()
            }
            EnergyPolicy::Balanced => {
                self.platform.max_sustainable_power_watts() * 0.7
            }
            EnergyPolicy::Saver => {
                // Reflex-only: no LLM, minimal CPU
                self.platform.max_sustainable_power_watts() * 0.3
            }
            EnergyPolicy::Emergency => {
                // Bare minimum: heartbeat + CRDT sync only
                self.platform.idle_power_watts * 1.2
            }
        }
    }

    /// Maximum LLM model size allowed by current energy policy (bytes).
    pub fn max_model_bytes(&self, base_limit: u64) -> u64 {
        match self.policy {
            EnergyPolicy::Performance => base_limit,
            EnergyPolicy::Balanced => (base_limit as f64 * 0.7) as u64,
            EnergyPolicy::Saver => 0, // No LLM
            EnergyPolicy::Emergency => 0,
        }
    }

    /// Energy per token of LLM inference (joules/token).
    pub fn energy_per_token(&self, tokens_per_sec: f64) -> f64 {
        if tokens_per_sec > 0.0 {
            self.power_draw_watts / tokens_per_sec
        } else {
            f64::INFINITY
        }
    }

    /// How many tokens can be generated before battery exhaustion?
    pub fn tokens_until_battery_death(&self, tokens_per_sec: f64) -> f64 {
        if !self.on_battery || self.power_draw_watts <= 0.0 {
            return f64::INFINITY;
        }
        let remaining_joules = self.estimated_remaining_mins as f64 * 60.0 * self.power_draw_watts;
        let joules_per_token = self.power_draw_watts / tokens_per_sec;
        remaining_joules / joules_per_token
    }
}

// ═══════════════════════════════════════════════════════════════════
// SECTION 10: CONSERVATION LAW #8 — ENERGY CONSERVATION
// ═══════════════════════════════════════════════════════════════════

/// The 8th Conservation Law: Energy is conserved across migration.
///
/// The Polyformalist Synthesis identified 7 conservation laws.
/// The Thermodynamicist adds the 8th:
///
///   E_agent(shell_A) + E_migration = E_agent(shell_B) + E_dissipated
///
/// Energy is neither created nor destroyed — only transformed.
/// The energy "gap" between shells is the DISSIPATED energy
/// (heat, network transmission loss, irreversible erasure).
///
/// This is NOT a trivial restatement of the First Law.
/// It's a DESIGN CONSTRAINT: the system must ACCOUNT for all energy flows.
/// Every migration produces a traceable energy budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConservationAudit {
    /// Agent energy state before migration (joules stored in RAM, cache, etc.)
    pub energy_before_joules: f64,

    /// Energy consumed by the migration operation (joules)
    pub migration_energy_joules: f64,

    /// Agent energy state after migration (joules)
    pub energy_after_joules: f64,

    /// Energy dissipated as heat during migration (joules)
    pub dissipated_joules: f64,

    /// Energy stored as negentropy in gastrolith (joules, Landauer units)
    pub negentropy_stored_joules: f64,
}

impl EnergyConservationAudit {
    /// Verify conservation: E_before + E_migration = E_after + E_dissipated + E_negentropy
    pub fn is_conserved(&self, tolerance_pct: f64) -> bool {
        let lhs = self.energy_before_joules + self.migration_energy_joules;
        let rhs = self.energy_after_joules + self.dissipated_joules + self.negentropy_stored_joules;
        if lhs == 0.0 {
            return rhs.abs() < 1e-10;
        }
        ((lhs - rhs).abs() / lhs) < tolerance_pct / 100.0
    }

    /// Dissipation fraction: what fraction of migration energy is wasted as heat?
    pub fn dissipation_fraction(&self) -> f64 {
        if self.migration_energy_joules > 0.0 {
            self.dissipated_joules / self.migration_energy_joules
        } else {
            0.0
        }
    }

    /// Negentropy recovery: what fraction of erased information is preserved?
    pub fn negentropy_recovery_fraction(&self) -> f64 {
        let total = self.dissipated_joules + self.negentropy_stored_joules;
        if total > 0.0 {
            self.negentropy_stored_joules / total
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landauer_limit_room_temp() {
        let e = landauer_limit(300.0);
        // kT·ln(2) = 1.381e-23 * 300 * 0.6931 ≈ 2.87e-21
        assert!((e - 2.87e-21).abs() < 0.01e-21);
    }

    #[test]
    fn rpi4_max_sustainable_power() {
        let p = PlatformThermalProfile::rpi4();
        // P_max = (80 - 25) / 12 ≈ 4.58W
        let max_p = p.max_sustainable_power_watts();
        assert!(max_p > 4.0 && max_p < 5.0);
    }

    #[test]
    fn shell_swap_with_gastrolith_saves_energy() {
        let with = MigrationEnergyCalculator::rpi4_shell_swap(50, 0.65);
        let without = MigrationEnergyCalculator::rpi4_shell_swap_no_gastrolith(50);

        // With gastrolith: less Landauer cost
        assert!(with.landauer_minimum_joules() < without.landauer_minimum_joules());

        // Negentropy preserved
        assert!(with.negentropy_preserved_joules() > 0.0);

        // Gastrolith preserves 65% of state
        assert!((with.gastrolith_preservation_ratio() - 0.65).abs() < 0.01);
    }

    #[test]
    fn hermit_crab_model_superior() {
        let comp = ShellStrategyComparison::typical_agent(300.0);
        // Borrowing should be much cheaper than growing
        assert!(comp.borrow_migrate_joules < comp.grow_init_joules);
        // Superiority ratio should be significant (>5×)
        assert!(comp.superiority_ratio() > 5.0);
        // Less entropy produced
        assert!(comp.borrow_erased_bits < comp.grow_erased_bits);
    }

    #[test]
    fn reversible_crdt_has_zero_landauer_cost() {
        let rev = CrdtReversibility::Reversible;
        assert_eq!(rev.landauer_multiplier(), 0.0);

        let cost = rev.landauer_cost_joules(1000, 300.0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn irreversible_crdt_has_full_landauer_cost() {
        let irrev = CrdtReversibility::Irreversible;
        assert_eq!(irrev.landauer_multiplier(), 1.0);

        let cost = irrev.landauer_cost_joules(1000, 300.0);
        let expected = 1000.0 * landauer_limit(300.0);
        // Both should be approximately 2.87e-18 J
        // Use generous relative tolerance for floating-point
        assert!((cost - expected).abs() / expected < 0.01,
            "cost = {cost:e}, expected = {expected:e}");
    }

    #[test]
    fn thermal_carrying_capacity_rpi4() {
        let cap = ThermalCarryingCapacity::rpi4();
        let max_active = cap.max_active_agents();
        // Should be modest — RPi 4 is thermally constrained
        assert!(max_active > 0.0 && max_active < 50.0);
    }

    #[test]
    fn energy_conservation_audit() {
        let audit = EnergyConservationAudit {
            energy_before_joules: 100.0,
            migration_energy_joules: 10.0,
            energy_after_joules: 95.0,
            dissipated_joules: 12.0,
            negentropy_stored_joules: 3.0,
        };
        // 100 + 10 = 95 + 12 + 3 = 110
        assert!(audit.is_conserved(1.0));
    }

    #[test]
    fn crdt_reversibility_classification() {
        // G-Counter: max(a, b) — irreversible
        let g_counter = CrdtReversibility::Irreversible;
        assert_eq!(g_counter.landauer_multiplier(), 1.0);

        // OR-Set with tombstones — partially reversible
        let or_set = CrdtReversibility::PartiallyReversible { recoverable_fraction: 0.6 };
        assert!((or_set.landauer_multiplier() - 0.4).abs() < 0.001);

        // Op-based CRDT — reversible
        let op_based = CrdtReversibility::Reversible;
        assert_eq!(op_based.landauer_multiplier(), 0.0);
    }
}
