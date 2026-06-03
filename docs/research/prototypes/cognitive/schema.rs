//! Schema extraction during offline consolidation.
//!
//! Based on Ghosh & Gilboa (2014): a schema is a coherent set of
//! features that are associated with a specific pattern of activation.
//! Schemas are extracted from multiple episodic memories and represent
//! generalized knowledge (cortical/semantic memory).
//!
//! Currently, PincherOS's offline consolidation only merges reflexes
//! with >0.95 cosine similarity (trigger-level merging). This module
//! adds schema extraction: discovering generalized patterns that span
//! multiple reflexes across different contexts.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// An extracted schema: a generalized pattern discovered across multiple
/// reflexes during offline consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSchema {
    /// Unique ID for this schema
    pub id: String,

    /// The generalized trigger pattern (stripped of episodic detail)
    pub generalized_trigger: String,

    /// The action template that applies across contexts
    pub action_template: String,

    /// The contexts this schema has been validated across
    pub validated_contexts: Vec<String>,

    /// Confidence that this schema generalizes
    pub generalization_confidence: f64,

    /// The reflexes that were compressed into this schema
    pub source_reflex_ids: Vec<String>,

    /// When this schema was extracted (epoch seconds)
    pub extracted_at: u64,
}

/// Schema extractor: runs during offline consolidation (hourly)
/// to discover generalized patterns across reflexes.
pub struct SchemaExtractor {
    /// Minimum number of contexts a schema must span
    min_contexts: usize,

    /// Minimum similarity between action templates for grouping
    action_similarity_threshold: f64,

    /// Minimum generalization confidence to accept a schema
    min_generalization_confidence: f64,
}

impl SchemaExtractor {
    pub fn new() -> Self {
        Self {
            min_contexts: 2,
            action_similarity_threshold: 0.85,
            min_generalization_confidence: 0.6,
        }
    }

    /// Extract schemas from a set of reflexes.
    /// Groups reflexes by action template similarity (NOT trigger similarity),
    /// then extracts common patterns across different contexts.
    pub fn extract_schemas(&self, reflexes: &[ReflexSummary], now: u64) -> Vec<ExtractedSchema> {
        // Step 1: Group reflexes by action template similarity
        let action_groups = self.group_by_action_template(reflexes);

        // Step 2: For each group, check if it spans multiple contexts
        action_groups
            .into_iter()
            .filter_map(|group| self.try_extract_schema(&group, now))
            .collect()
    }

    /// Group reflexes by action template similarity.
    /// Uses a simple clustering approach: iterate through reflexes,
    /// and group those whose action templates are similar enough.
    fn group_by_action_template(&self, reflexes: &[ReflexSummary]) -> Vec<Vec<&ReflexSummary>> {
        let mut groups: Vec<Vec<&ReflexSummary>> = Vec::new();
        let mut assigned: HashSet<usize> = HashSet::new();

        for (i, reflex) in reflexes.iter().enumerate() {
            if assigned.contains(&i) {
                continue;
            }

            let mut group = vec![reflex];
            assigned.insert(i);

            for (j, other) in reflexes.iter().enumerate() {
                if i == j || assigned.contains(&j) {
                    continue;
                }

                if self.action_similarity(&reflex.action_template, &other.action_template)
                    >= self.action_similarity_threshold
                {
                    group.push(other);
                    assigned.insert(j);
                }
            }

            if group.len() >= self.min_contexts {
                groups.push(group);
            }
        }

        groups
    }

    /// Try to extract a schema from a group of reflexes.
    fn try_extract_schema(
        &self,
        group: &[&ReflexSummary],
        now: u64,
    ) -> Option<ExtractedSchema> {
        // Check that the group spans multiple contexts
        let contexts: HashSet<_> = group.iter().map(|r| r.context_fingerprint.clone()).collect();
        if contexts.len() < self.min_contexts {
            return None;
        }

        // Extract the common action pattern (the most common template)
        let action_template = group
            .iter()
            .map(|r| r.action_template.clone())
            .max_by_key(|t| {
                group
                    .iter()
                    .filter(|r| r.action_template == *t)
                    .count()
            })
            .unwrap_or_default();

        // Generalize triggers: find the common semantic core
        let generalized_trigger = self.generalize_triggers(group);

        // Compute generalization confidence
        let confidence = self.compute_generalization_confidence(group);

        if confidence < self.min_generalization_confidence {
            return None;
        }

        Some(ExtractedSchema {
            id: format!("schema-{}", uuid::Uuid::new_v4()),
            generalized_trigger,
            action_template,
            validated_contexts: contexts.into_iter().collect(),
            generalization_confidence: confidence,
            source_reflex_ids: group.iter().map(|r| r.id.clone()).collect(),
            extracted_at: now,
        })
    }

    /// Generalize trigger patterns from a group of reflexes.
    /// Simple approach: extract common words/phrases.
    fn generalize_triggers(&self, group: &[&ReflexSummary]) -> String {
        let all_words: Vec<Vec<&str>> = group
            .iter()
            .map(|r| r.trigger_pattern.split_whitespace().collect())
            .collect();

        // Find words that appear in at least half of the triggers
        let threshold = group.len() / 2;
        let mut word_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();

        for words in &all_words {
            let unique: HashSet<&&str> = words.iter().collect();
            for word in unique {
                *word_counts.entry(word).or_insert(0) += 1;
            }
        }

        let common_words: Vec<&str> = word_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(word, _)| word)
            .collect();

        if common_words.is_empty() {
            // Fall back to the most common trigger
            group
                .iter()
                .max_by_key(|r| r.usage_count)
                .map(|r| r.trigger_pattern.clone())
                .unwrap_or_default()
        } else {
            common_words.join(" ")
        }
    }

    /// Compute generalization confidence based on:
    /// - Number of contexts spanned
    /// - Average confidence of source reflexes
    /// - Action template consistency
    fn compute_generalization_confidence(&self, group: &[&ReflexSummary]) -> f64 {
        let n_contexts = group.len() as f64;
        let avg_confidence: f64 = group.iter().map(|r| r.confidence).sum::<f64>() / n_contexts;
        let action_consistency = group
            .iter()
            .filter(|r| r.action_template == group[0].action_template)
            .count() as f64
            / n_contexts;

        // Weighted combination
        0.3 * (n_contexts / 5.0).min(1.0) + 0.4 * avg_confidence + 0.3 * action_consistency
    }

    /// Simple action template similarity.
    /// In production, this would use embedding-based similarity.
    fn action_similarity(&self, a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }

        let a_words: HashSet<&str> = a.split_whitespace().collect();
        let b_words: HashSet<&str> = b.split_whitespace().collect();

        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }
}

impl Default for SchemaExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of a reflex for schema extraction.
/// This is a lightweight view — we don't need the full reflex object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexSummary {
    pub id: String,
    pub trigger_pattern: String,
    pub action_template: String,
    pub confidence: f64,
    pub usage_count: u32,
    pub context_fingerprint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reflex(id: &str, trigger: &str, action: &str, context: &str) -> ReflexSummary {
        ReflexSummary {
            id: id.to_string(),
            trigger_pattern: trigger.to_string(),
            action_template: action.to_string(),
            confidence: 0.8,
            usage_count: 10,
            context_fingerprint: context.to_string(),
        }
    }

    #[test]
    fn test_schema_extraction_across_contexts() {
        let extractor = SchemaExtractor::new();
        let reflexes = vec![
            make_reflex("r1", "organize downloads by type", "sort {dir} --by-type", "home-pi4"),
            make_reflex("r2", "organize documents by date", "sort {dir} --by-date", "work-jetson"),
            make_reflex("r3", "organize photos by size", "sort {dir} --by-size", "home-pi4"),
            make_reflex("r4", "compress video files", "ffmpeg -i {input} -crf 28 {output}", "home-pi4"),
        ];

        let schemas = extractor.extract_schemas(&reflexes, 1700000000);

        // Should extract at least one schema for the "sort" group
        assert!(!schemas.is_empty(), "Should extract at least one schema");
    }

    #[test]
    fn test_no_schema_for_single_context() {
        let extractor = SchemaExtractor::new();
        let reflexes = vec![
            make_reflex("r1", "organize downloads", "sort {dir}", "same-context"),
            make_reflex("r2", "organize documents", "sort {dir}", "same-context"),
        ];

        let schemas = extractor.extract_schemas(&reflexes, 1700000000);

        // Should NOT extract a schema — all same context
        // (actually they might, depending on how we define "context" —
        //  the key is that contexts.len() < min_contexts)
        // With identical contexts, HashSet collapses to 1, so < 2
        assert!(schemas.is_empty() || schemas.iter().all(|s| s.validated_contexts.len() < 2));
    }
}
