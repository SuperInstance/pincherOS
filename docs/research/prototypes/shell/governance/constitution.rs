//! Article I & II: Rights and Separation of Powers

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════
// ARTICLE I: BILL OF RIGHTS
// ═══════════════════════════════════════════

/// The inalienable rights of all PincherOS persons.
/// These are NOT capabilities — they cannot be revoked by Tenuo.
/// They can only be overridden by the Consent Court (Article IV).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Right {
    // User Rights (§1)
    /// §1.1: User controls agent lifecycle
    SovereignDeployment,
    /// §1.2: User owns agent data
    DataOwnership,
    /// §1.3: User can demand erasure (GDPR Art. 17)
    Erasure,
    /// §1.5: User can demand explanation of constraint failures
    Explanation,
    /// §1.6: User can challenge constraints
    Appeal,

    // Agent Rights (§2)
    /// §2.1: Agent identity persists through migration
    IdentityContinuity,
    /// §2.2: Consent capacity matches developmental stage
    DevelopmentalProgression,
    /// §2.3: Identity reflexes (confidence > 0.99) are inalienable
    ReflexIntegrity,
    /// §2.4: Agent (≥Juvenile) can refuse migration
    Refusal,
    /// §2.5: Agent's autobiographical memory cannot be erased by non-User
    Memory,

    // Shell Rights (§3)
    /// §3.1: Hardware boundaries are inviolable
    Inviolability,
    /// §3.2: Symbionts have guaranteed resource allocation
    SymbiontBudget,
    /// §3.3: Shell keeps its epigenetics
    EpigeneticRetention,

    // Symbiont Rights (§4)
    /// §4.1: Symbionts have right to allocated resources
    MinimalFootprint,
    /// §4.2: Tenuo operates independently of the agent
    AutonomousOperation,
    /// §4.3: Symbionts can reject transfer to new shell
    TransferNegotiation,
}

/// A rights policy constrains how Tenuo may issue capabilities.
/// Tenuo cannot violate a RightsPolicy — it is checked BEFORE capability issuance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsPolicy {
    /// The right being protected
    pub right: Right,
    /// The constraint on capability issuance
    pub constraint: RightsConstraint,
    /// Who may override this policy (and under what conditions)
    pub override_authority: OverrideAuthority,
}

/// How a right constrains capability issuance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RightsConstraint {
    /// Capability cannot be issued at all (inalienable right)
    Absolute,
    /// Capability can be issued only with explicit User consent
    RequiresExplicitConsent,
    /// Capability can be issued only within specified constraints
    Conditional { description: String },
    /// Capability can be overridden by the Consent Court
    OverridableByCourt,
}

/// Who may override a rights policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverrideAuthority {
    /// Cannot be overridden
    None,
    /// Only the Consent Court (§7)
    ConsentCourt,
    /// Constraint Council with supermajority (0.667 operational, 0.8 foundational)
    ConstraintCouncil { supermajority: f64 },
    /// Only explicit User consent
    UserExplicit,
}

// ═══════════════════════════════════════════
// ARTICLE II: SEPARATION OF POWERS
// ═══════════════════════════════════════════

/// The three branches of PincherOS governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    /// Constraint Council (§5): defines constraints
    Legislative,
    /// MigrationGuard (§6): enforces constraints
    Executive,
    /// Consent Court (§7): adjudicates disputes
    Judicial,
}

/// The seven foundational constraints from Lojban analysis.
/// These are ENTRENCHED — require 4/5 supermajority to amend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintId {
    C1SubstanceAccident,
    C2Simultaneity,
    C3ShapeAsymmetry,
    C4AnimacyOfInitiation,
    C5PairOperation,
    C6ConsentRequirement,
    C7DifferentialVerification,
}

impl ConstraintId {
    /// Whether this constraint is foundational (entrenched, requires 4/5 supermajority).
    pub fn is_foundational(&self) -> bool {
        true // All seven are foundational
    }

    /// The supermajority required to amend this constraint.
    pub fn amendment_threshold(&self) -> f64 {
        if self.is_foundational() {
            0.8 // 4/5
        } else {
            0.667 // 2/3
        }
    }
}

/// A constraint amendment proposed to the Constraint Council.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAmendment {
    pub amendment_id: uuid::Uuid,
    pub constraint_id: ConstraintId,
    pub change_type: AmendmentType,
    pub votes_for: u32,
    pub votes_against: u32,
    pub supermajority_required: f64,
    #[serde(skip)]
    pub enacted_at: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmendmentType {
    Add { description: String },
    Modify { old: String, new: String },
    Remove { reason: String },
}

impl ConstraintAmendment {
    /// Whether the amendment has achieved the required supermajority.
    pub fn is_approved(&self) -> bool {
        let total = self.votes_for + self.votes_against;
        if total == 0 {
            return false;
        }
        (self.votes_for as f64 / total as f64) >= self.supermajority_required
    }
}

/// A Consent Court decision (§7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtDecision {
    pub case_id: uuid::Uuid,
    pub appeal_id: uuid::Uuid,
    pub verdict: CourtVerdict,
    pub reasoning: String,
    pub precedent_weight: PrecedentWeight,
    #[serde(skip)]
    pub effective_until: Option<std::time::Instant>,
    /// Three arbiter votes: [User proxy, Agent advocate, Shell witness]
    pub votes: [Option<ArbiterVote>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CourtVerdict {
    /// Constraint was correct, migration remains blocked
    Uphold,
    /// Constraint was wrong, migration proceeds
    Override,
    /// Insufficient evidence, re-measure and re-evaluate
    Remand,
    /// Migration proceeds with additional conditions
    Conditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecedentWeight {
    /// Must be followed by all future similar cases
    Binding,
    /// Should be considered but not binding
    Persuasive,
    /// Specific to this case, no future weight
    NonPrecedential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbiterVote {
    For,
    Against,
    Abstain,
}

impl CourtDecision {
    /// Compute the verdict from the three arbiter votes.
    /// 2/3 majority required. Split (1-1-1) defaults to status quo (Uphold).
    pub fn compute_verdict(votes: [Option<ArbiterVote>; 3]) -> CourtVerdict {
        let for_count = votes.iter().filter(|v| matches!(v, Some(ArbiterVote::For))).count();
        let against_count = votes.iter().filter(|v| matches!(v, Some(ArbiterVote::Against))).count();

        if for_count >= 2 {
            CourtVerdict::Override
        } else if against_count >= 2 {
            CourtVerdict::Uphold
        } else {
            // Split decision or too many abstentions → status quo
            CourtVerdict::Uphold
        }
    }
}
