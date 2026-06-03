//! Gastrolith: cognitive continuity anchor during migration.
//!
//! In biology, the gastrolith is a calcium deposit stored internally
//! before molting. After the old exoskeleton is shed, the gastrolith
//! is reabsorbed to harden the new exoskeleton.
//!
//! In PincherOS, the gastrolith is the agent-local checkpoint that
//! maintains cognitive continuity during the dissociative state of
//! migration. It prioritizes identity-critical reflexes and provides
//! the interoceptive snapshot needed to reconstruct the body schema.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The gastrolith: a cognitive continuity anchor stored in the .nail file.
/// Prioritizes identity-critical content over expendable content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gastrolith {
    /// Substance fields — identity core (must be preserved at all costs)
    pub substance: SubstanceFields,

    /// Top-K most-used reflexes at full fidelity.
    /// These are the "procedural memories" needed immediately after migration.
    pub core_reflexes: Vec<FullFidelityReflex>,

    /// Decision traces: recent decisions and outcomes.
    /// Used to reconstruct working memory after the dissociative period.
    pub decision_traces: Vec<DecisionTrace>,

    /// Interoceptive snapshot: the agent's last known bodily state.
    /// Used as a prior for interoceptive prediction on the new shell.
    pub interoceptive_snapshot: InteroceptiveSnapshot,

    /// Gastrolith creation timestamp (epoch seconds)
    pub created_at: u64,

    /// Shell fingerprint of the source shell
    pub source_shell_fingerprint: String,
}

/// Substance fields: what constitutes identity (from Greek ousia).
/// These MUST be preserved across migration; their loss = identity death.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstanceFields {
    /// Rigging UUID
    pub rigging_id: String,
    /// Accumulated personality
    pub personality: String,
    /// Number of sessions completed
    pub total_sessions: u64,
    /// Number of reflexes acquired
    pub total_reflexes: u64,
    /// Developmental stage
    pub developmental_stage: DevelopmentalStage,
}

/// Developmental stage of the agent (from biologist's ontogeny model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentalStage {
    /// No model, no reflexes — drifting through fleet
    Zoea,
    /// Model loading, first reflex acquisition — seeking stability
    Megalopa,
    /// Operating with basic reflexes, still learning
    Juvenile,
    /// Full capability, vacancy chain eligible
    Adult,
}

/// A reflex stored at full fidelity in the gastrolith.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullFidelityReflex {
    /// The reflex's unique ID
    pub reflex_id: String,
    /// The trigger pattern
    pub trigger_pattern: String,
    /// The action template (complete, not just text)
    pub action_template: String,
    /// Current cognitive trust state
    pub trust: f64,
    /// Practice count
    pub practice_count: u32,
    /// Why this reflex is in the gastrolith
    pub reason: GastrolithReason,
}

/// Why a reflex was included in the gastrolith.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GastrolithReason {
    /// Top-K by usage count
    TopUsed { rank: usize },
    /// Identity reflex (confidence > 0.99, cannot degrade without rollback)
    Identity,
    /// Recently learned but not yet consolidated
    Unconsolidated { learned_at: u64 },
    /// User-designated critical reflex
    UserCritical,
}

/// A decision trace: records a recent decision for working memory reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTrace {
    /// The input that triggered the decision
    pub input_summary: String,
    /// The reflex or LLM decision that was made
    pub decision_summary: String,
    /// The outcome (success/failure)
    pub outcome: Outcome,
    /// Timestamp (epoch seconds)
    pub timestamp: u64,
}

/// Outcome of a decision trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    PartialSuccess,
    Failure,
    Deferred,
}

/// Interoceptive snapshot: the agent's last known bodily state.
/// Used as a prior for predicting resource needs on the new shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteroceptiveSnapshot {
    /// Last known RAM usage pattern (sampled over last hour)
    pub ram_usage_samples: Vec<f64>,
    /// Last known inference latency profile
    pub latency_profile: LatencyProfile,
    /// Last known thermal state
    pub thermal_state: ThermalStatus,
    /// Predicted resource trajectory (from JEPA, if available)
    pub predicted_trajectory: Option<Vec<PredictedState>>,
    /// Time since last rehydration (epoch seconds of last rehydration)
    pub last_rehydration_at: u64,
}

/// Latency profile: inference timing statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyProfile {
    /// Median inference latency (ms)
    pub p50_ms: f64,
    /// 95th percentile latency (ms)
    pub p95_ms: f64,
    /// 99th percentile latency (ms)
    pub p99_ms: f64,
    /// Number of samples
    pub sample_count: u32,
}

/// Thermal status of the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThermalStatus {
    Nominal,
    Warm,
    Hot,
    Critical,
}

/// A predicted future state (from JEPA interoceptive channel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedState {
    /// Predicted RAM usage (fraction, 0.0-1.0)
    pub ram_fraction: f64,
    /// Predicted time (seconds from now)
    pub seconds_from_now: f64,
    /// Confidence in this prediction
    pub confidence: f64,
}

/// Builder for constructing a gastrolith before migration.
pub struct GastrolithBuilder {
    substance: Option<SubstanceFields>,
    core_reflexes: Vec<FullFidelityReflex>,
    decision_traces: Vec<DecisionTrace>,
    interoceptive: Option<InteroceptiveSnapshot>,
    source_fingerprint: String,
}

impl GastrolithBuilder {
    pub fn new(source_fingerprint: String) -> Self {
        Self {
            substance: None,
            core_reflexes: Vec::new(),
            decision_traces: Vec::new(),
            interoceptive: None,
            source_fingerprint,
        }
    }

    pub fn substance(mut self, substance: SubstanceFields) -> Self {
        self.substance = Some(substance);
        self
    }

    pub fn add_core_reflex(mut self, reflex: FullFidelityReflex) -> Self {
        self.core_reflexes.push(reflex);
        self
    }

    pub fn add_decision_trace(mut self, trace: DecisionTrace) -> Self {
        self.decision_traces.push(trace);
        self
    }

    pub fn interoceptive(mut self, snapshot: InteroceptiveSnapshot) -> Self {
        self.interoceptive = Some(snapshot);
        self
    }

    /// Build the gastrolith. The core_reflexes list is sorted by priority:
    /// Identity reflexes first, then top-used, then unconsolidated.
    pub fn build(self, now: u64) -> Result<Gastrolith, GastrolithError> {
        let substance = self.substance.ok_or(GastrolithError::MissingSubstance)?;
        let interoceptive = self.interoceptive.unwrap_or(InteroceptiveSnapshot {
            ram_usage_samples: Vec::new(),
            latency_profile: LatencyProfile {
                p50_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                sample_count: 0,
            },
            thermal_state: ThermalStatus::Nominal,
            predicted_trajectory: None,
            last_rehydration_at: now,
        });

        // Sort core_reflexes by priority: Identity > UserCritical > TopUsed > Unconsolidated
        let mut sorted = self.core_reflexes;
        sorted.sort_by(|a, b| {
            let priority = |r: &GastrolithReason| match r {
                GastrolithReason::Identity => 0,
                GastrolithReason::UserCritical => 1,
                GastrolithReason::TopUsed { .. } => 2,
                GastrolithReason::Unconsolidated { .. } => 3,
            };
            priority(&a.reason).cmp(&priority(&b.reason))
        });

        Ok(Gastrolith {
            substance,
            core_reflexes: sorted,
            decision_traces: self.decision_traces,
            interoceptive_snapshot: interoceptive,
            created_at: now,
            source_shell_fingerprint: self.source_fingerprint,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GastrolithError {
    #[error("Missing substance fields — identity core is required")]
    MissingSubstance,
}

impl Gastrolith {
    /// Compute the continuity score: how much of the agent's cognitive
    /// structure would be preserved if this gastrolith were used for migration.
    pub fn continuity_score(&self, total_reflexes: usize) -> f64 {
        if total_reflexes == 0 {
            return 1.0;
        }

        // Identity reflexes are weighted 10x (their loss = rollback)
        let identity_count = self
            .core_reflexes
            .iter()
            .filter(|r| matches!(r.reason, GastrolithReason::Identity))
            .count();
        let other_count = self.core_reflexes.len() - identity_count;

        let weighted_preserved = (identity_count as f64 * 10.0) + other_count as f64;
        let weighted_total = (total_reflexes as f64 * 0.1 * 10.0) + total_reflexes as f64 * 0.9;

        (weighted_preserved / weighted_total).min(1.0)
    }

    /// Whether this gastrolith is sufficient for migration
    /// (at least 5 core reflexes and substance is intact)
    pub fn is_migration_ready(&self) -> bool {
        !self.substance.rigging_id.is_empty() && self.core_reflexes.len() >= 5
    }
}
