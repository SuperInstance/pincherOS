//! PincherOS Governance Module — Constitutional Types
//!
//! R3: The Constitution of the Hermit Crab Republic
//!
//! This module implements the governance layer that sits ABOVE Tenuo's
//! capability system and BELOW the MigrationGuard's enforcement layer.
//!
//! Architecture:
//!   Governance Layer (Rights)     ← This module
//!   Capability Layer (Tenuo)      ← tenuo crate
//!   Enforcement Layer (MigrationGuard) ← shell::migration::guard

pub mod constitution;
pub mod consent;
pub mod due_process;
pub mod erasure;
pub mod jurisdiction;
pub mod accountability;

pub use constitution::{
    Branch, ConstraintAmendment, ConstraintId, CourtDecision, CourtVerdict,
    OverrideAuthority, PrecedentWeight, Right, RightsConstraint, RightsPolicy,
};
pub use consent::{
    ConsentCondition, ConsentDenial, ConsentGrant, ConsentProof, ConsentType,
    ConsentValidity, DenialReason, PartyId,
};
pub use due_process::{
    AppealGround, ConstraintFailureNotice, ConstraintAppeal, Relief, RemediationOption,
};
pub use erasure::{ErasureRequest, ErasureResult, ErasureZone, GastrolithStatus};
pub use jurisdiction::{
    CrossBorderInfo, JurisdictionalRegime, TrustCrossingInfo,
};
pub use accountability::{
    AccountabilityLedger, ForkRecord, InheritedReflex, LedgerEntry, LiabilityType,
};
