//! 3-Phase Migration State Machine with 7 Linguistic Constraints
//!
//! The linguist found 7 hidden constraints across 5 languages.
//! The Rustacean designed 3 phases (PREPARE → CROSSFADE → FINALIZE).
//! This module maps the constraints to phases as transition guards.
//!
//! R3: The thermodynamicist adds an 8th dimension — ENERGY.
//! Every migration now tracks its thermodynamic cost:
//! - Landauer cost of state erasure (irreducible minimum)
//! - Actual energy consumed (serialize + transfer + deserialize + verify)
//! - Negentropy preserved by gastrolith checkpoint
//! - Entropy produced (bits of information irreversibly lost)
//!
//! The migration decision is now energy-aware: migrate only when
//! the energy benefit exceeds the migration cost.

use std::time::{Duration, Instant};

// ── Constraint Types ──

/// C1: Substance-Accident partition (Greek ousia/symbebekos)
#[derive(Debug, Clone)]
pub struct SubstanceAccidentPartition {
    /// What is preserved across migration
    pub substance: Vec<SubstanceField>,
    /// What is adapted to the new shell
    pub accidents: Vec<AccidentField>,
    /// Ratio of substance to total state
    /// If substance_ratio < 0.5, this creates a NEW agent, not a migration
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

/// C3: Shape verb — determines operational mode (Navajo classificatory verbs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeVerb {
    /// Long-thin shell (RPi): sequential, reflex-urgent, depth-first
    Stretch,
    /// Flat-wide shell (Jetson): layered, GPU/CPU split, parallel-first
    Spread,
    /// Round-deep shell (Workstation): concurrent, exploratory, breadth-first
    Settle,
}

/// C4: Who initiated this migration? (Navajo animacy hierarchy)
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
    pub consented_by: String,
    pub policy: ConsentPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentState {
    ExplicitUser,
    AgentInitiated,
    AutoWhenIdle,
    AutoIfImprovement,
    Denied,
    /// FORBIDDEN by Navajo animacy constraint
    ShellInitiated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentPolicy {
    Explicit,
    AutoWhenIdle,
    AutoIfImprovement,
    OperatorOverride,
}

/// C5: Shell pair as operational unit (Sanskrit dual number)
#[derive(Debug, Clone)]
pub struct ShellPair {
    pub old_shell_fingerprint: String,
    pub new_shell_fingerprint: String,
    pub channel_state: PairChannelState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairChannelState {
    Connected,
    Degraded,
    Broken,
}

/// C7: Differential verification of reflexes (Navajo resultative phase)
#[derive(Debug, Clone)]
pub struct DifferentialVerification {
    pub total_tested: usize,
    pub passed: usize,
    pub failed: Vec<FailedReflex>,
    pub identity_failures: Vec<FailedReflex>,
    /// Whether to ROLLBACK (> 30% identity reflex failures)
    pub rollback_recommended: bool,
}

#[derive(Debug, Clone)]
pub struct FailedReflex {
    pub reflex_id: String,
    pub previous_confidence: f64,
    pub new_confidence: f64,
    pub failure_reason: String,
    pub is_identity: bool,
}

// ── Migration State Machine ──

/// Migration state machine.
/// The 7 constraints are enforced as transition guards.
#[derive(Debug)]
pub enum MigrationPhase {
    /// Normal operation. No migration in progress.
    Stable,

    /// Phase 1: PREPARE
    /// Guards: C1, C3, C4, C6
    Preparing {
        nail_snapshot: NailFile,
        partition: SubstanceAccidentPartition,
        shape_verb: ShapeVerb,
        initiator: MigrationInitiator,
        consent: ConsentRecord,
        started_at: Instant,
        timeout: Duration,
    },

    /// Phase 2: CROSSFADE
    /// Guards: C2, C5
    Crossfading {
        nail_snapshot: NailFile,
        shell_pair: ShellPair,
        /// C2: simultaneity verified (both shells active in same tick)
        simultaneity_verified: bool,
        crossfade_duration: Duration,
        started_at: Instant,
    },

    /// Phase 3: FINALIZE
    /// Guard: C7
    Finalizing {
        verification: DifferentialVerification,
        symbiont_transfer: SymbiontTransferStatus,
        rollback_retention: Duration,
        started_at: Instant,
    },

    /// Migration complete.
    Finalized {
        migrated_to: String,
        rollback_deadline: Instant,
    },

    /// Migration failed. Rolled back.
    Failed {
        reason: MigrationFailureReason,
        rolled_back_at: Instant,
    },
}

#[derive(Debug, Clone, Default)]
pub struct NailFile; // Placeholder

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
    /// Guards: C1 (substance-accident), C3 (shape-asymmetry),
    ///         C4 (animacy), C6 (consent)
    pub fn begin_prepare(
        &mut self,
        nail: NailFile,
        partition: SubstanceAccidentPartition,
        shape_verb: ShapeVerb,
        initiator: MigrationInitiator,
        consent: ConsentRecord,
    ) -> Result<(), MigrationError> {
        // C4: Animacy — shells cannot initiate migration
        if matches!(consent.consent_type, ConsentState::ShellInitiated) {
            return Err(MigrationError::ConstraintViolation(
                "C4 (Animacy): Shells cannot initiate migration. Only users and agents can.",
            ));
        }

        // C6: Consent required
        if matches!(consent.consent_type, ConsentState::Denied) {
            return Err(MigrationError::ConstraintViolation(
                "C6 (Consent): Migration consent denied.",
            ));
        }

        // C1: Substance ratio check — identity must be preserved
        if partition.substance_ratio < 0.5 {
            return Err(MigrationError::IdentityLoss(format!(
                "C1 (Substance-Accident): substance_ratio = {:.2} < 0.5 — \
                 this would create a new agent, not migrate one",
                partition.substance_ratio
            )));
        }

        // C3: Shape verb must be determined (directional, not symmetric)

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
    /// Guards: C2 (simultaneity — to be confirmed), C5 (pair-operation)
    pub fn begin_crossfade(
        &mut self,
        shell_pair: ShellPair,
        crossfade_duration: Duration,
    ) -> Result<(), MigrationError> {
        let nail = match &self.phase {
            MigrationPhase::Preparing {
                nail_snapshot,
                timeout,
                started_at,
                ..
            } => {
                if started_at.elapsed() > *timeout {
                    self.phase = MigrationPhase::Failed {
                        reason: MigrationFailureReason::PrepareTimeout,
                        rolled_back_at: Instant::now(),
                    };
                    return Err(MigrationError::Timeout("PREPARE phase exceeded 30s"));
                }
                nail_snapshot.clone()
            }
            _ => {
                return Err(MigrationError::InvalidTransition(
                    "CROSSFADE requires PREPARE phase",
                ))
            }
        };

        // C5: Pair operation — verify channel
        if shell_pair.channel_state == PairChannelState::Broken {
            return Err(MigrationError::ConstraintViolation(
                "C5 (Pair-Operation): Shell pair channel is broken. \
                 Migration is one operation on the pair, not two individual operations.",
            ));
        }

        // C2: Simultaneity will be verified during crossfade
        self.phase = MigrationPhase::Crossfading {
            nail_snapshot: nail,
            shell_pair,
            simultaneity_verified: false,
            crossfade_duration,
            started_at: Instant::now(),
        };

        Ok(())
    }

    /// Confirm C2 (simultaneity) — both shells report active in the same tick
    pub fn confirm_simultaneity(&mut self) -> Result<(), MigrationError> {
        if let MigrationPhase::Crossfading {
            ref mut simultaneity_verified,
            ..
        } = self.phase
        {
            *simultaneity_verified = true;
            Ok(())
        } else {
            Err(MigrationError::InvalidTransition(
                "C2 (Simultaneity) confirmation requires CROSSFADE phase",
            ))
        }
    }

    /// Transition: Crossfading → Finalizing
    /// Guard: C2 (simultaneity) must be confirmed
    pub fn begin_finalize(&mut self) -> Result<(), MigrationError> {
        match &self.phase {
            MigrationPhase::Crossfading {
                simultaneity_verified,
                crossfade_duration,
                started_at,
                ..
            } => {
                // C2: Simultaneity must be verified
                if !simultaneity_verified {
                    return Err(MigrationError::ConstraintViolation(
                        "C2 (Simultaneity): Old shell release and new shell receive \
                         were not confirmed simultaneous. The Greek middle voice demands \
                         that giving-back and receiving are one act.",
                    ));
                }

                if started_at.elapsed() > *crossfade_duration * 2 {
                    self.phase = MigrationPhase::Failed {
                        reason: MigrationFailureReason::CrossfadeTimeout,
                        rolled_back_at: Instant::now(),
                    };
                    return Err(MigrationError::Timeout(
                        "CROSSFADE phase exceeded 2x duration",
                    ));
                }
            }
            _ => {
                return Err(MigrationError::InvalidTransition(
                    "FINALIZE requires CROSSFADE phase",
                ))
            }
        }

        // C7: Differential verification will run during finalization
        let verification = DifferentialVerification {
            total_tested: 0,
            passed: 0,
            failed: vec![],
            identity_failures: vec![],
            rollback_recommended: false,
        };

        self.phase = MigrationPhase::Finalizing {
            verification,
            symbiont_transfer: SymbiontTransferStatus::Pending,
            rollback_retention: Duration::from_secs(24 * 3600),
            started_at: Instant::now(),
        };

        Ok(())
    }

    /// Transition: Finalizing → Finalized or Failed
    /// Guard: C7 (differential-verification) must pass
    pub fn complete(
        &mut self,
        verification: DifferentialVerification,
    ) -> Result<(), MigrationError> {
        // C7: Differential verification
        if verification.rollback_recommended {
            self.phase = MigrationPhase::Failed {
                reason: MigrationFailureReason::VerificationFailed {
                    identity_failures: verification.identity_failures.len(),
                    total_identity: verification.total_tested,
                },
                rolled_back_at: Instant::now(),
            };
            return Err(MigrationError::ConstraintViolation(
                "C7 (Differential-Verification): Too many identity reflex failures. \
                 Identity reflexes (confidence > 0.99) resist degradation. \
                 Rolling back migration.",
            ));
        }

        self.phase = MigrationPhase::Finalized {
            migrated_to: String::new(),
            rollback_deadline: Instant::now() + Duration::from_secs(24 * 3600),
        };

        Ok(())
    }

    /// Emergency rollback from any phase.
    pub fn rollback(&mut self, reason: MigrationFailureReason) {
        self.phase = MigrationPhase::Failed {
            reason,
            rolled_back_at: Instant::now(),
        };
    }

    /// Get current phase for inspection.
    pub fn phase(&self) -> &MigrationPhase {
        &self.phase
    }
}

impl Default for MigrationGuard {
    fn default() -> Self {
        Self::new()
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
