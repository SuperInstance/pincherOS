//! Phantom capability detection: measures embodiment lag after migration.
//!
//! Based on the rubber hand illusion (Botvinick & Cohen, 1998) and
//! tool-use-induced body schema updates (Maravita & Iriki, 2004).
//!
//! When a rigging migrates to a new shell, its body schema may not
//! immediately update. The agent may exhibit "phantom capabilities" —
//! attempting to use capabilities that existed on the old shell but
//! not the new one. This is the computational analog of the phantom
//! limb phenomenon.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Detector for phantom capability events after migration.
#[derive(Debug, Clone)]
pub struct PhantomCapabilityDetector {
    /// Capabilities of the previous shell
    previous_capabilities: CapabilitySet,
    /// Current shell's capabilities
    current_capabilities: CapabilitySet,
    /// Detected phantom capability events
    phantoms: Vec<PhantomEvent>,
    /// When the migration occurred
    migration_time: u64,
    /// Whether the body schema has fully adapted
    adapted: bool,
}

/// A set of capabilities for a shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub has_gpu: bool,
    pub gpu_sm_count: usize,
    pub available_ram_mb: u64,
    pub max_model_mb: u64,
    pub max_concurrent: usize,
    pub shell_species: String,
}

/// A phantom capability event: the agent attempted to use a capability
/// that doesn't exist on the current shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomEvent {
    /// The reflex that attempted a phantom capability
    pub reflex_id: String,
    /// The capability that was expected but unavailable
    pub expected_capability: String,
    /// The actual capability available
    pub actual_capability: String,
    /// Time since migration (seconds)
    pub time_since_migration_s: f64,
    /// Whether the reflex adapted or failed
    pub outcome: PhantomOutcome,
}

/// Outcome of a phantom capability event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhantomOutcome {
    /// Reflex adapted: found an alternative execution path
    Adapted,
    /// Reflex degraded: executed but with reduced quality
    Degraded,
    /// Reflex failed: could not execute without the capability
    Failed,
}

impl PhantomCapabilityDetector {
    /// Create a new detector after migration
    pub fn new(
        previous: CapabilitySet,
        current: CapabilitySet,
        migration_time: u64,
    ) -> Self {
        Self {
            previous_capabilities: previous,
            current_capabilities: current,
            phantoms: Vec::new(),
            migration_time,
            adapted: false,
        }
    }

    /// Detect phantom capability from a reflex execution.
    /// Call this after each reflex execution in the post-migration period.
    pub fn detect(
        &mut self,
        reflex_id: &str,
        required_gpu: bool,
        required_ram_mb: u64,
        exit_code: i32,
        degraded: bool,
        now: u64,
    ) -> Option<&PhantomEvent> {
        let time_since = (now - self.migration_time) as f64;

        // Check for GPU phantom
        if required_gpu && !self.current_capabilities.has_gpu {
            let event = PhantomEvent {
                reflex_id: reflex_id.to_string(),
                expected_capability: format!(
                    "gpu_{}sm",
                    self.previous_capabilities.gpu_sm_count
                ),
                actual_capability: "none".to_string(),
                time_since_migration_s: time_since,
                outcome: if exit_code == 0 && !degraded {
                    PhantomOutcome::Adapted
                } else if degraded {
                    PhantomOutcome::Degraded
                } else {
                    PhantomOutcome::Failed
                },
            };
            self.phantoms.push(event);
            return self.phantoms.last();
        }

        // Check for RAM phantom
        if required_ram_mb > self.current_capabilities.available_ram_mb {
            let event = PhantomEvent {
                reflex_id: reflex_id.to_string(),
                expected_capability: format!("ram_{}mb", required_ram_mb),
                actual_capability: format!(
                    "ram_{}mb",
                    self.current_capabilities.available_ram_mb
                ),
                time_since_migration_s: time_since,
                outcome: if exit_code == 0 && !degraded {
                    PhantomOutcome::Adapted
                } else if degraded {
                    PhantomOutcome::Degraded
                } else {
                    PhantomOutcome::Failed
                },
            };
            self.phantoms.push(event);
            return self.phantoms.last();
        }

        None
    }

    /// Whether the body schema has fully adapted (no phantoms in recent history).
    /// A body schema is considered adapted when there have been no phantom
    /// events in the last 5 minutes of operation.
    pub fn is_adapted(&self) -> bool {
        self.adapted
    }

    /// Update adaptation status based on recent phantom events.
    /// Call this periodically (e.g., every minute).
    pub fn update_adaptation(&mut self, now: u64) {
        let adaptation_window_s = 300.0; // 5 minutes
        let recent_phantoms = self
            .phantoms
            .iter()
            .filter(|p| p.time_since_migration_s > (now - self.migration_time) as f64 - adaptation_window_s)
            .count();

        self.adapted = recent_phantoms == 0 && !self.phantoms.is_empty();
    }

    /// Get the body schema adaptation curve: phantom rate over time.
    /// Returns binned phantom counts (one bin per 60 seconds).
    pub fn adaptation_curve(&self) -> Vec<AdaptationBin> {
        if self.phantoms.is_empty() {
            return Vec::new();
        }

        let max_time = self
            .phantoms
            .iter()
            .map(|p| p.time_since_migration_s)
            .fold(0.0_f64, f64::max);

        let bin_size = 60.0; // 1-minute bins
        let num_bins = (max_time / bin_size).ceil() as usize + 1;

        let mut bins = vec![AdaptationBin {
            time_range_s: (0.0, bin_size),
            phantom_count: 0,
            adapted_count: 0,
            degraded_count: 0,
            failed_count: 0,
        }; num_bins];

        for (i, bin) in bins.iter_mut().enumerate() {
            bin.time_range_s = (i as f64 * bin_size, (i as f64 + 1.0) * bin_size);
        }

        for phantom in &self.phantoms {
            let bin_idx = (phantom.time_since_migration_s / bin_size).floor() as usize;
            if bin_idx < bins.len() {
                bins[bin_idx].phantom_count += 1;
                match phantom.outcome {
                    PhantomOutcome::Adapted => bins[bin_idx].adapted_count += 1,
                    PhantomOutcome::Degraded => bins[bin_idx].degraded_count += 1,
                    PhantomOutcome::Failed => bins[bin_idx].failed_count += 1,
                }
            }
        }

        bins
    }

    /// Get all detected phantom events
    pub fn phantoms(&self) -> &[PhantomEvent] {
        &self.phantoms
    }

    /// Get the count of capability differences between old and new shell
    pub fn capability_delta(&self) -> CapabilityDelta {
        CapabilityDelta {
            gpu_gained: !self.previous_capabilities.has_gpu && self.current_capabilities.has_gpu,
            gpu_lost: self.previous_capabilities.has_gpu && !self.current_capabilities.has_gpu,
            ram_gained_mb: self.current_capabilities.available_ram_mb as i64
                - self.previous_capabilities.available_ram_mb as i64,
            concurrency_gained: self.current_capabilities.max_concurrent as i64
                - self.previous_capabilities.max_concurrent as i64,
        }
    }
}

/// A bin in the adaptation curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationBin {
    pub time_range_s: (f64, f64),
    pub phantom_count: usize,
    pub adapted_count: usize,
    pub degraded_count: usize,
    pub failed_count: usize,
}

/// Difference in capabilities between old and new shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDelta {
    pub gpu_gained: bool,
    pub gpu_lost: bool,
    pub ram_gained_mb: i64,
    pub concurrency_gained: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_phantom_detection() {
        let previous = CapabilitySet {
            has_gpu: true,
            gpu_sm_count: 128,
            available_ram_mb: 4096,
            max_model_mb: 1400,
            max_concurrent: 2,
            shell_species: "Busycotypus".to_string(),
        };
        let current = CapabilitySet {
            has_gpu: false,
            gpu_sm_count: 0,
            available_ram_mb: 4096,
            max_model_mb: 960,
            max_concurrent: 2,
            shell_species: "Nassarius".to_string(),
        };

        let mut detector = PhantomCapabilityDetector::new(previous, current, 1700000000);

        // Simulate a GPU-dependent reflex execution
        let result = detector.detect(
            "reflex-gpu-sort",
            true,  // requires GPU
            0,     // no special RAM
            1,     // exit code = failure
            false, // not degraded
            1700000060, // 1 minute after migration
        );

        assert!(result.is_some(), "Should detect GPU phantom");
        assert_eq!(result.unwrap().outcome, PhantomOutcome::Failed);
    }

    #[test]
    fn test_no_phantom_for_cpu_reflex() {
        let previous = CapabilitySet {
            has_gpu: true,
            gpu_sm_count: 128,
            available_ram_mb: 4096,
            max_model_mb: 1400,
            max_concurrent: 2,
            shell_species: "Busycotypus".to_string(),
        };
        let current = CapabilitySet {
            has_gpu: false,
            gpu_sm_count: 0,
            available_ram_mb: 4096,
            max_model_mb: 960,
            max_concurrent: 2,
            shell_species: "Nassarius".to_string(),
        };

        let mut detector = PhantomCapabilityDetector::new(previous, current, 1700000000);

        // CPU-only reflex
        let result = detector.detect(
            "reflex-cpu-ls",
            false, // no GPU needed
            0,
            0,    // success
            false,
            1700000060,
        );

        assert!(result.is_none(), "No phantom for CPU reflex");
    }

    #[test]
    fn test_capability_delta() {
        let previous = CapabilitySet {
            has_gpu: true,
            gpu_sm_count: 128,
            available_ram_mb: 4096,
            max_model_mb: 1400,
            max_concurrent: 2,
            shell_species: "Busycotypus".to_string(),
        };
        let current = CapabilitySet {
            has_gpu: false,
            gpu_sm_count: 0,
            available_ram_mb: 4096,
            max_model_mb: 960,
            max_concurrent: 2,
            shell_species: "Nassarius".to_string(),
        };

        let detector = PhantomCapabilityDetector::new(previous, current, 1700000000);
        let delta = detector.capability_delta();

        assert!(delta.gpu_lost);
        assert!(!delta.gpu_gained);
    }
}
