//! PincherOS Shell Module — Unified GPU/Rust Architecture
//!
//! R2 Integration: Resolves the GPU/CPU tension.
//! CRDT merge is ALWAYS on CPU. GPU is for inference only.
//!
//! R3 Integration: Thermodynamics module closes the Energy Gap (SG-4).
//! Every operation now has a joules dimension.
//! Landauer's principle applied: kT·ln(2) per bit erased.
//! CRDT merge is logically reversible → zero Landauer cost (in principle).
//! Consent is an entropy-reduction mechanism.

pub mod claws;
pub mod crdt;
// Governance module: has pre-existing serde/Instant issues — pending fix
// pub mod governance;
pub mod migration;
pub mod quality;
pub mod thermodynamics;

pub use claws::cpu_claws::{Claws, CpuClaws};
pub use crdt::engine::{PartitionConfig, PartitionedCrdtEngine};
// pub use governance::{ ... };
pub use migration::guard::{MigrationGuard, MigrationPhase};
pub use quality::ShellQuality;
pub use thermodynamics::{
    ConsentThermodynamics, CrdtMergeThermodynamics, CrdtReversibility, EnergyConservationAudit,
    EnergyPolicy, EnergyState, MigrationDecision, MigrationDecisionReason,
    MigrationEnergyCalculator, MigrationEnergyCost, PlatformThermalProfile,
    ShellStrategyComparison, ShellSwapThermodynamics, ThermalCarryingCapacity,
    BOLTZMANN, LANDAUER_ROOM_TEMP, LN2, ROOM_TEMP_K,
};
