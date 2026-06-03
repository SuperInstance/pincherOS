//! Article III: Consent as a Cryptographic Protocol

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// The parties in the consent protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartyKind {
    User,
    Agent,
    OldShell,
    NewShell,
    Symbiont,
}

/// Identifier for a party in the consent protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartyId {
    User { id: String },
    Agent { rigging_id: String },
    Shell { fingerprint: String },
    Symbiont { name: String },
}

/// The type of consent granted.
/// Maps to the developmental stage consent model from R2 Philosopher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentType {
    /// Direct, informed, unambiguous (GDPR Art. 4(11))
    Explicit,
    /// JEPA predicts User preference (Megalopa agents only)
    Proxied,
    /// Agent proposes, User confirms (Juvenile agents only)
    Assisted,
    /// Agent decides, User informed (Adult agents only)
    Autonomous,
    /// Fleet operator forces migration (logged, auditable)
    OperatorOverride,
}

impl ConsentType {
    /// Which developmental stages may use this consent type.
    pub fn valid_for_stage(&self, stage: crate::shell::migration::guard::MigrationInitiator) -> bool {
        match self {
            ConsentType::Explicit => true,
            ConsentType::Proxied => {
                matches!(stage, crate::shell::migration::guard::MigrationInitiator::Agent)
            }
            ConsentType::Assisted => {
                matches!(stage, crate::shell::migration::guard::MigrationInitiator::Agent)
            }
            ConsentType::Autonomous => {
                matches!(stage, crate::shell::migration::guard::MigrationInitiator::Agent)
            }
            ConsentType::OperatorOverride => {
                matches!(stage, crate::shell::migration::guard::MigrationInitiator::AutoIdle)
            }
        }
    }
}

/// How long the consent remains valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentValidity {
    /// Valid for one migration only
    SingleUse,
    /// Valid until expiry time
    TimeBounded { expiry_secs: u64 },
    /// Valid while predicate holds
    Conditional { description: String },
}

/// Conditions attached to a consent grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentCondition {
    /// Migration must improve fit by at least this threshold
    FitImprovementAbove { threshold: f64 },
    /// Must stay within same trust boundary
    NoPrivacyBoundaryCrossing,
    /// New shell must have at least this much symbiont budget
    SymbiontBudgetAvailable { min_mb: u64 },
    /// Old shell must retain rollback snapshot for this duration
    RollbackGuaranteed { retention_secs: u64 },
    /// C7 verification must not exceed this many identity failures
    VerificationThreshold { max_identity_failures: usize },
}

/// A consent grant from one party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentGrant {
    pub consenter: PartyId,
    pub consent_type: ConsentType,
    pub validity: ConsentValidity,
    pub conditions: Vec<ConsentCondition>,
    #[serde(skip)]
    pub timestamp: Instant,
}

/// Reason for consent denial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenialReason {
    /// Previously granted consent was withdrawn
    ConsentRevoked,
    /// Fit improvement below threshold
    InsufficientImprovement,
    /// Migration crosses trust boundary without authorization
    PrivacyViolation,
    /// A symbiont refused transfer
    SymbiontRejection,
    /// Agent (≥Juvenile) exercises §2.4 right of refusal
    AgentRefusal,
    /// Predicted adaptation > 0.5 (identity threshold)
    AdaptationTooHigh,
}

/// A consent denial from one party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentDenial {
    pub denier: PartyId,
    pub reason: DenialReason,
    #[serde(skip)]
    pub timestamp: Instant,
}

/// The consent proof — cryptographic attestation that all required
/// consents were obtained. Stored in CRDT as immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentProof {
    pub proposal_id: [u8; 16], // UUID as bytes (no external dep)
    pub grants: Vec<ConsentGrant>,
    pub denials: Vec<ConsentDenial>,
    /// The constraint verification status
    pub constraint_verdict: ConstraintConsentStatus,
    /// Consent Court ruling (if appealed)
    pub court_verdict: Option<super::constitution::CourtDecision>,
    #[serde(skip)]
    pub completed_at: Instant,
    /// Merkle root of all consent messages for efficient verification
    pub merkle_root: [u8; 32],
}

/// Whether the constraint check passed for consent purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintConsentStatus {
    AllPass,
    HasWarnings,
    HasFailures,
}

impl ConsentProof {
    /// Whether the proof represents a valid consent (all required parties consented, no denials).
    pub fn is_valid(&self) -> bool {
        self.denials.is_empty()
            && !self.grants.is_empty()
            && self.constraint_verdict != ConstraintConsentStatus::HasFailures
    }

    /// Whether a specific party consented.
    pub fn party_consented(&self, party: &PartyId) -> bool {
        self.grants.iter().any(|g| g.consenter == *party)
    }

    /// Whether a specific party denied.
    pub fn party_denied(&self, party: &PartyId) -> bool {
        self.denials.iter().any(|d| d.denier == *party)
    }
}
