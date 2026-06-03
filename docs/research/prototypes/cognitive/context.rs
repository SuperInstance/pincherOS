//! Context-tagged reflexes: encoding specificity and context-dependent memory.
//!
//! Based on Tulving (1983): encoding specificity principle — memory is
//! most effective when retrieval conditions match encoding conditions.
//! Godden & Baddeley (1975): context-dependent memory in divers.
//!
//! Reflexes should be tagged with their encoding context (shell fingerprint,
//! resource state, developmental stage) and validated across contexts.

use serde::{Deserialize, Serialize};

/// A reflex tagged with encoding context and cross-context validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTaggedReflex {
    /// The reflex's unique ID
    pub reflex_id: String,

    /// Original encoding context (where and when it was learned)
    pub encoding_context: EncodingContext,

    /// Validated contexts: where it has been successfully used
    pub validated_contexts: Vec<ContextValidation>,

    /// The trigger pattern
    pub trigger_pattern: String,

    /// The action template
    pub action_template: String,

    /// Base confidence (at encoding context)
    pub base_confidence: f64,
}

/// The context in which a reflex was encoded or validated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingContext {
    /// Shell fingerprint at encoding time
    pub shell_fingerprint: String,

    /// Shell species (Nassarius, Busycotypus, etc.)
    pub shell_species: String,

    /// Resource state at encoding
    pub resource_state: ResourceSnapshot,

    /// Developmental stage at encoding
    pub developmental_stage: String,

    /// Whether the encoding was on GPU or CPU
    pub compute_path: ComputePath,

    /// Trust boundary (personal, work, shared)
    pub trust_boundary: String,
}

/// Snapshot of resource state at encoding/validation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// Available RAM (MB)
    pub available_ram_mb: u64,
    /// GPU available
    pub gpu_available: bool,
    /// Model loaded at encoding time
    pub model_loaded: bool,
    /// Thermal state
    pub thermal_state: String,
}

/// The compute path used for encoding/execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputePath {
    /// Reflex short-circuit (no LLM, ~50ms)
    ReflexDirect,
    /// Reflex + LLM confirmation (~3s)
    ReflexWithConfirmation,
    /// Full LLM reasoning (~5s)
    FullLlmReasoning,
}

/// Validation of a reflex in a specific context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextValidation {
    /// Context where validation occurred
    pub context: EncodingContext,
    /// Number of successful uses in this context
    pub success_count: u32,
    /// Number of failed uses in this context
    pub failure_count: u32,
    /// Confidence in this context
    pub confidence: f64,
    /// Last validation timestamp
    pub last_validated_at: u64,
}

impl ContextTaggedReflex {
    /// Compute effective confidence for the current context.
    /// Follows Tulving's encoding specificity principle:
    /// - Full confidence if validated in current context
    /// - Reduced confidence proportional to context similarity if not
    /// - Floor of 0.5 * base_confidence (semantic value persists)
    pub fn effective_confidence(&self, current_context: &EncodingContext) -> f64 {
        // Check if we've been validated in the current context
        if let Some(validation) = self
            .validated_contexts
            .iter()
            .find(|v| v.context.shell_fingerprint == current_context.shell_fingerprint)
        {
            return validation.confidence;
        }

        // Not validated in current context: modulate by context similarity
        let context_match = self.encoding_context.similarity_to(current_context);
        let floor = self.base_confidence * 0.5;
        (self.base_confidence * context_match).max(floor)
    }

    /// Record a successful validation in the current context.
    pub fn record_success(&mut self, current_context: &EncodingContext, now: u64) {
        if let Some(validation) = self
            .validated_contexts
            .iter_mut()
            .find(|v| v.context.shell_fingerprint == current_context.shell_fingerprint)
        {
            validation.success_count += 1;
            // Update confidence using Bayesian update
            let alpha = 0.1; // learning rate
            validation.confidence = validation.confidence * (1.0 - alpha) + 1.0 * alpha;
            validation.last_validated_at = now;
        } else {
            // First validation in this context
            self.validated_contexts.push(ContextValidation {
                context: current_context.clone(),
                success_count: 1,
                failure_count: 0,
                confidence: self.base_confidence * 0.7, // Start at 70% of base
                last_validated_at: now,
            });
        }
    }

    /// Record a failure in the current context.
    pub fn record_failure(&mut self, current_context: &EncodingContext, now: u64) {
        if let Some(validation) = self
            .validated_contexts
            .iter_mut()
            .find(|v| v.context.shell_fingerprint == current_context.shell_fingerprint)
        {
            validation.failure_count += 1;
            let alpha = 0.15; // failure learning rate (higher than success)
            validation.confidence = (validation.confidence * (1.0 - alpha)).max(0.1);
            validation.last_validated_at = now;
        } else {
            // First use in this context, and it failed
            self.validated_contexts.push(ContextValidation {
                context: current_context.clone(),
                success_count: 0,
                failure_count: 1,
                confidence: self.base_confidence * 0.3, // Low confidence after failure
                last_validated_at: now,
            });
        }
    }

    /// How many distinct contexts has this reflex been validated in?
    pub fn context_span(&self) -> usize {
        self.validated_contexts.len()
    }

    /// Is this reflex a schema (validated across multiple contexts)?
    pub fn is_generalized(&self) -> bool {
        self.context_span() >= 2
    }
}

impl EncodingContext {
    /// Compute similarity to another encoding context.
    /// Returns 0.0-1.0 based on matching features.
    pub fn similarity_to(&self, other: &EncodingContext) -> f64 {
        let mut score = 0.0;
        let mut weight_sum = 0.0;

        // Shell species match: high weight (same species = similar capabilities)
        let species_weight = 0.3;
        if self.shell_species == other.shell_species {
            score += species_weight;
        }
        weight_sum += species_weight;

        // GPU availability match
        let gpu_weight = 0.25;
        if self.resource_state.gpu_available == other.resource_state.gpu_available {
            score += gpu_weight;
        }
        weight_sum += gpu_weight;

        // Trust boundary match
        let trust_weight = 0.2;
        if self.trust_boundary == other.trust_boundary {
            score += trust_weight;
        }
        weight_sum += trust_weight;

        // RAM similarity (within 2x)
        let ram_weight = 0.15;
        let ram_ratio = if self.resource_state.available_ram_mb > 0 && other.resource_state.available_ram_mb > 0 {
            let r = self.resource_state.available_ram_mb as f64
                / other.resource_state.available_ram_mb as f64;
            if r >= 0.5 && r <= 2.0 { 1.0 } else { 0.5 }
        } else {
            0.5
        };
        score += ram_weight * ram_ratio;
        weight_sum += ram_weight;

        // Compute path match
        let path_weight = 0.1;
        if self.compute_path == other.compute_path {
            score += path_weight;
        }
        weight_sum += path_weight;

        score / weight_sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context(shell: &str, species: &str, gpu: bool) -> EncodingContext {
        EncodingContext {
            shell_fingerprint: shell.to_string(),
            shell_species: species.to_string(),
            resource_state: ResourceSnapshot {
                available_ram_mb: 4096,
                gpu_available: gpu,
                model_loaded: true,
                thermal_state: "nominal".to_string(),
            },
            developmental_stage: "adult".to_string(),
            compute_path: ComputePath::ReflexDirect,
            trust_boundary: "personal".to_string(),
        }
    }

    #[test]
    fn test_same_context_full_confidence() {
        let context = make_context("shell-a", "Nassarius", false);
        let mut reflex = ContextTaggedReflex {
            reflex_id: "r1".to_string(),
            encoding_context: context.clone(),
            validated_contexts: vec![ContextValidation {
                context: context.clone(),
                success_count: 10,
                failure_count: 0,
                confidence: 0.95,
                last_validated_at: 1700000000,
            }],
            trigger_pattern: "test".to_string(),
            action_template: "test".to_string(),
            base_confidence: 0.95,
        };

        let effective = reflex.effective_confidence(&context);
        assert!((effective - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_different_context_reduced_confidence() {
        let encoding_context = make_context("shell-a", "Nassarius", false);
        let different_context = make_context("shell-b", "Busycotypus", true);

        let reflex = ContextTaggedReflex {
            reflex_id: "r1".to_string(),
            encoding_context: encoding_context.clone(),
            validated_contexts: vec![],
            trigger_pattern: "test".to_string(),
            action_template: "test".to_string(),
            base_confidence: 0.9,
        };

        let effective = reflex.effective_confidence(&different_context);
        // Should be reduced (different species, different GPU)
        assert!(effective < 0.9, "Different context should reduce confidence");
        // But should not go below floor (0.5 * base)
        assert!(
            effective >= 0.9 * 0.5,
            "Should not go below semantic floor"
        );
    }

    #[test]
    fn test_validation_increases_cross_context_confidence() {
        let encoding_context = make_context("shell-a", "Nassarius", false);
        let new_context = make_context("shell-b", "Busycotypus", true);

        let mut reflex = ContextTaggedReflex {
            reflex_id: "r1".to_string(),
            encoding_context,
            validated_contexts: vec![],
            trigger_pattern: "test".to_string(),
            action_template: "test".to_string(),
            base_confidence: 0.9,
        };

        // Before validation
        let before = reflex.effective_confidence(&new_context);

        // Record success in new context
        reflex.record_success(&new_context, 1700000000);

        // After validation
        let after = reflex.effective_confidence(&new_context);

        assert!(after >= before, "Validation should increase confidence in context");
    }
}
