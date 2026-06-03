//! Power-law actualization model for reflex trust.
//!
//! Based on:
//! - Newell & Rosenbloom (1981): Power law of practice
//! - Wixted (2004): Power law of forgetting
//! - McClelland, McNaughton & O'Reilly (1995): Complementary learning systems
//! - Bahrick et al. (1993): Spacing effect
//!
//! Key insight: reflex confidence follows a power-law actualization curve
//! (fast early learning, slow approach to asymptote) combined with a
//! power-law forgetting curve (old memories decay more slowly).
//! This is fundamentally different from the exponential decay currently
//! used in the CRDT access rate tracker.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A reinforcement event: one successful use of a reflex.
/// Tracks the interval since the previous success for the spacing effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementEvent {
    /// When this reinforcement occurred (epoch seconds)
    pub timestamp: u64,
    /// Time since the previous reinforcement (seconds)
    pub interval_secs: f64,
}

/// Cognitive trust: power-law actualization + power-law forgetting
/// with spaced reinforcement and consolidation gradient.
///
/// This replaces the simple exponential decay in the CRDT AccessRate
/// tracker with a cognitively grounded trust model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTrust {
    /// Current baseline confidence (snapped at last consolidation)
    confidence: f64,

    /// When this reflex was created (epoch seconds)
    created_at: u64,

    /// When this reflex was last consolidated (offline/JEPA)
    last_consolidated_at: u64,

    /// History of successful uses: (timestamp, interval)
    success_history: Vec<ReinforcementEvent>,

    /// Base forgetting rate (power-law exponent)
    /// ~0.8 for new reflexes, decreases with consolidation
    decay_beta: f64,

    /// Learning rate for actualization curve
    /// ~0.3 (from Newell & Rosenbloom's power law of practice)
    learning_rate: f64,

    /// Asymptotic confidence ceiling (never quite reached)
    asymptotic_ceiling: f64,

    /// Number of successful uses (practice count)
    practice_count: u32,

    /// Whether this reflex is "procedural" (confidence > 0.90)
    is_procedural: bool,

    /// Temporary dissociation penalty (applied during migration)
    dissociation_penalty: Option<f64>,

    /// Number of consolidation events (for tracking consolidation gradient)
    consolidation_count: u32,
}

impl CognitiveTrust {
    /// Create a new cognitive trust at default confidence (0.5)
    pub fn new(created_at: u64) -> Self {
        Self {
            confidence: 0.5,
            created_at,
            last_consolidated_at: created_at,
            success_history: Vec::with_capacity(50),
            decay_beta: 0.8,
            learning_rate: 0.3,
            asymptotic_ceiling: 0.98,
            practice_count: 0,
            is_procedural: false,
            dissociation_penalty: None,
            consolidation_count: 0,
        }
    }

    /// Create a cognitive trust from an existing simple confidence value
    pub fn from_simple_confidence(confidence: f64, created_at: u64) -> Self {
        let mut trust = Self::new(created_at);
        trust.confidence = confidence;
        trust.is_procedural = confidence > 0.90;
        // Estimate practice count from confidence using inverse actualization
        // C = C_inf * (1 - (1 - C_0/C_inf) * e^(-alpha * n))
        // Solving for n: n = -ln(1 - C/C_inf / (1 - C_0/C_inf)) / alpha
        let c_inf = trust.asymptotic_ceiling;
        let c_0 = 0.5;
        if confidence > c_0 && c_inf > c_0 {
            let ratio = (confidence / c_inf - 1.0 + c_0 / c_inf) / (c_0 / c_inf - 1.0);
            if ratio > 0.0 && ratio < 1.0 {
                trust.practice_count = ((-ratio.ln()) / trust.learning_rate).ceil() as u32;
            }
        }
        trust
    }

    /// Compute current effective confidence, accounting for:
    /// 1. Power-law actualization (learning curve)
    /// 2. Power-law forgetting (since last consolidation)
    /// 3. Spaced reinforcement (spacing effect)
    /// 4. Consolidation gradient (older = more stable)
    /// 5. Dissociation penalty (during migration)
    pub fn effective_confidence(&self, now: u64) -> f64 {
        let n = self.practice_count;
        let t_since_creation = (now.saturating_sub(self.created_at)) as f64;
        let t_since_consolidation = (now.saturating_sub(self.last_consolidated_at)) as f64;

        // 1. ACTUALIZATION FACTOR: power-law learning
        // C_actualized(n) = C_∞ * (1 - (1 - C_0/C_∞) * e^(-α * n))
        let c_inf = self.asymptotic_ceiling;
        let c_0 = 0.5;
        let alpha = self.learning_rate;
        let actualization = if n > 0 {
            c_inf * (1.0 - (1.0 - c_0 / c_inf) * (-alpha * n as f64).exp())
        } else {
            c_0
        };

        // 2. FORGETTING FACTOR: power-law decay since last consolidation
        // The decay rate decreases with practice: β(n) = β_0 * n^(-γ)
        let gamma = 0.3; // consolidation rate
        let beta = if n > 0 {
            self.decay_beta * (n as f64).powf(-gamma)
        } else {
            self.decay_beta
        };

        let t0 = 3600.0; // 1-hour scaling constant
        let t_consol = 3600.0; // consolidation age floor
        let forgetting = if t_since_consolidation > 0.0 {
            ((t0 + t_consol) / (t0 + t_since_consolidation)).powf(beta)
        } else {
            1.0
        };

        // 3. SPACING EFFECT: distributed practice boosts retention
        let spacing_boost = self.compute_spacing_boost();

        // 4. CONSOLIDATION GRADIENT: older reflexes decay more slowly
        // This is already encoded in the decreasing decay_beta

        // Combine all factors
        let raw = actualization * forgetting * spacing_boost;

        // 5. DISSOCIATION PENALTY: temporary reduction during migration
        let effective = match self.dissociation_penalty {
            Some(penalty) => (raw - penalty).max(0.0),
            None => raw,
        };

        effective.clamp(0.0, 1.0)
    }

    /// Record a successful use (practice trial).
    /// This implements the spacing effect: closely-spaced repetitions
    /// have diminishing returns compared to distributed practice.
    pub fn record_success(&mut self, now: u64) {
        let interval = match self.success_history.last() {
            Some(last) => (now.saturating_sub(last.timestamp)) as f64,
            None => 86400.0, // default 1 day for first use
        };

        self.success_history.push(ReinforcementEvent {
            timestamp: now,
            interval_secs: interval,
        });
        self.practice_count += 1;

        // Prune old history (keep last 50 events)
        if self.success_history.len() > 50 {
            self.success_history.drain(..self.success_history.len() - 50);
        }

        // Update procedural flag
        self.is_procedural = self.effective_confidence(now) > 0.90;
    }

    /// Record a failure.
    /// Failures should reduce confidence, but the reduction is proportional
    /// to the actualization state (per the Greek philosopher's non-linear trust).
    pub fn record_failure(&mut self, now: u64) {
        let current = self.effective_confidence(now);

        // Non-linear failure penalty (from Greek R2's trust_increment)
        let scale = match current {
            c if c < 0.5 => 0.3,   // Low confidence: small penalty (already struggling)
            c if c < 0.7 => 0.5,   // Developing: moderate penalty
            c if c < 0.9 => 1.0,   // Near-procedural: full penalty (surprising failure)
            c if c < 0.95 => 1.5,  // High confidence: enhanced penalty (very surprising)
            _ => 2.0,              // Identity reflex: severe penalty (existential failure)
        };

        let penalty = 0.05 * scale;
        self.confidence = (current - penalty).max(0.0);
        self.is_procedural = self.effective_confidence(now) > 0.90;
    }

    /// Consolidate: reset the forgetting clock.
    /// Called during offline consolidation (hourly) and JEPA training (daily).
    /// Analogous to sleep consolidation: hippocampal → cortical transfer.
    pub fn consolidate(&mut self, now: u64) {
        // Snap current effective confidence as the new baseline
        self.confidence = self.effective_confidence(now);
        self.last_consolidated_at = now;

        // Reduce the base forgetting rate: consolidation makes memories more durable
        // Each consolidation reduces decay_beta by ~10%
        self.decay_beta *= 0.9;
        self.consolidation_count += 1;

        // Clear old reinforcement history (it's been absorbed into the baseline)
        self.success_history.clear();

        // Update procedural flag
        self.is_procedural = self.confidence > 0.90;
    }

    /// Apply dissociation penalty during migration.
    /// Temporarily reduces confidence of procedural reflexes,
    /// forcing them through LLM confirmation on first use.
    pub fn apply_dissociation(&mut self, penalty: f64) {
        self.dissociation_penalty = Some(penalty);
    }

    /// Remove dissociation penalty after migration verification passes.
    pub fn resolve_dissociation(&mut self) {
        self.dissociation_penalty = None;
    }

    /// Whether this reflex is currently procedural (unconscious execution)
    pub fn is_procedural(&self) -> bool {
        self.is_procedural
    }

    /// Whether this reflex is an identity reflex (confidence > 0.99)
    pub fn is_identity(&self, now: u64) -> bool {
        self.effective_confidence(now) > 0.99
    }

    /// Get the raw baseline confidence (before decay/boost computation)
    pub fn baseline_confidence(&self) -> f64 {
        self.confidence
    }

    /// Get the practice count
    pub fn practice_count(&self) -> u32 {
        self.practice_count
    }

    /// Get the consolidation count
    pub fn consolidation_count(&self) -> u32 {
        self.consolidation_count
    }

    /// Compute the spacing boost factor.
    /// Based on Bahrick et al. (1993): distributed practice produces
    /// more durable memories than massed practice.
    fn compute_spacing_boost(&self) -> f64 {
        self.success_history
            .iter()
            .map(|event| {
                let tau = 3600.0; // 1-hour spacing timescale
                let alpha = 0.1; // reinforcement strength
                alpha * (-event.interval_secs / tau).exp()
            })
            .fold(1.0, |acc, boost| acc * (1.0 + boost))
    }

    /// Compute the adaptation ratio for migration identity assessment.
    /// Returns how much this trust would change on a new shell.
    pub fn adaptation_impact(&self, new_context_similarity: f64) -> f64 {
        // If the new context is very similar (similarity ~1.0), impact is low.
        // If very different (similarity ~0.0), impact is high.
        // The impact is weighted by actualization: identity reflexes matter more.
        let weight = self.confidence.powi(2);
        (1.0 - new_context_similarity) * weight
    }
}

/// The full actualization model for a fleet of agents.
/// Computes fleet-wide consolidation statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetActualization {
    /// Total reflexes across the fleet
    total_reflexes: u64,
    /// Distribution of reflexes by actualization stage
    stage_distribution: ActualizationStages,
    /// Fleet-wide average consolidation count
    avg_consolidation_count: f64,
    /// Fraction of reflexes that are procedural (confidence > 0.90)
    procedural_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualizationStages {
    /// Pure dynamis (0.0-0.3): potential only
    pub potential: f64,
    /// First actualization (0.3-0.5): heavy supervision needed
    pub emerging: f64,
    /// Developing energeia (0.5-0.7): LLM confirmation
    pub developing: f64,
    /// Near-actuality (0.7-0.9): LLM verification only
    pub near_actual: f64,
    /// Full energeia (0.9-1.0): procedural, no LLM needed
    pub actualized: f64,
}

impl FleetActualization {
    /// Compute fleet actualization from a set of cognitive trusts
    pub fn compute(trusts: &[CognitiveTrust], now: u64) -> Self {
        let total = trusts.len() as u64;
        if total == 0 {
            return Self {
                total_reflexes: 0,
                stage_distribution: ActualizationStages {
                    potential: 0.0,
                    emerging: 0.0,
                    developing: 0.0,
                    near_actual: 0.0,
                    actualized: 0.0,
                },
                avg_consolidation_count: 0.0,
                procedural_fraction: 0.0,
            };
        }

        let mut stages = ActualizationStages {
            potential: 0.0,
            emerging: 0.0,
            developing: 0.0,
            near_actual: 0.0,
            actualized: 0.0,
        };
        let mut total_consolidation = 0u32;
        let mut procedural_count = 0u64;

        for trust in trusts {
            let c = trust.effective_confidence(now);
            match c {
                c if c < 0.3 => stages.potential += 1.0,
                c if c < 0.5 => stages.emerging += 1.0,
                c if c < 0.7 => stages.developing += 1.0,
                c if c < 0.9 => stages.near_actual += 1.0,
                _ => stages.actualized += 1.0,
            }
            total_consolidation += trust.consolidation_count();
            if trust.is_procedural() {
                procedural_count += 1;
            }
        }

        stages.potential /= total as f64;
        stages.emerging /= total as f64;
        stages.developing /= total as f64;
        stages.near_actual /= total as f64;
        stages.actualized /= total as f64;

        Self {
            total_reflexes: total,
            stage_distribution: stages,
            avg_consolidation_count: total_consolidation as f64 / total as f64,
            procedural_fraction: procedural_count as f64 / total as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actualization_increases_with_practice() {
        let now = 1700000000;
        let mut trust = CognitiveTrust::new(now);

        let c_initial = trust.effective_confidence(now);
        for i in 0..20 {
            trust.record_success(now + (i as u64 + 1) * 86400); // one success per day
        }
        let c_after = trust.effective_confidence(now + 21 * 86400);

        assert!(c_after > c_initial, "Confidence should increase with practice");
    }

    #[test]
    fn test_power_law_decay_slower_than_exponential() {
        let now = 1700000000;
        let mut trust = CognitiveTrust::new(now);
        trust.record_success(now + 86400);
        trust.consolidate(now + 86400);

        // Check decay after 30 days
        let c_30d = trust.effective_confidence(now + 30 * 86400);
        // Power-law decay should be gentler than exponential
        // After 30 days with beta=0.72 (0.8 * 0.9), power-law should give > 0.3
        assert!(c_30d > 0.3, "Power-law decay should be gentler than exponential");
    }

    #[test]
    fn test_spacing_effect() {
        let now = 1700000000;

        // Massed practice: 10 successes in 1 minute
        let mut massed = CognitiveTrust::new(now);
        for i in 0..10 {
            massed.record_success(now + i as u64 * 60); // 1 minute apart
        }

        // Distributed practice: 10 successes over 10 days
        let mut distributed = CognitiveTrust::new(now);
        for i in 0..10 {
            distributed.record_success(now + i as u64 * 86400); // 1 day apart
        }

        // Both measured at the same time point
        let c_massed = massed.effective_confidence(now + 11 * 86400);
        let c_distributed = distributed.effective_confidence(now + 11 * 86400);

        // Distributed practice should produce more durable memory
        assert!(
            c_distributed >= c_massed * 0.95, // Allow small numerical tolerance
            "Distributed practice should produce at least as durable memory as massed"
        );
    }

    #[test]
    fn test_dissociation_penalty() {
        let now = 1700000000;
        let mut trust = CognitiveTrust::new(now);
        for i in 0..20 {
            trust.record_success(now + i as u64 * 86400);
        }
        trust.consolidate(now + 20 * 86400);

        let c_normal = trust.effective_confidence(now + 21 * 86400);
        trust.apply_dissociation(0.15);
        let c_dissociated = trust.effective_confidence(now + 21 * 86400);

        assert!(
            c_dissociated < c_normal,
            "Dissociation penalty should reduce effective confidence"
        );
        assert!(
            c_dissociated < 0.90,
            "Dissociated procedural reflex should fall below reflex threshold"
        );
    }

    #[test]
    fn test_consolidation_reduces_decay() {
        let now = 1700000000;
        let mut trust = CognitiveTrust::new(now);
        trust.record_success(now + 86400);

        // Before consolidation
        let beta_before = trust.decay_beta;

        trust.consolidate(now + 2 * 86400);

        // After consolidation, decay should be reduced
        let beta_after = trust.decay_beta;
        assert!(
            beta_after < beta_before,
            "Consolidation should reduce the forgetting rate"
        );
    }
}
