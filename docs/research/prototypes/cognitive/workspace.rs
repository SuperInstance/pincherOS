//! Global Workspace: the system's limited-capacity broadcast mechanism.
//!
//! Based on Baars (1988) Global Workspace Theory and Dehaene & Changeux
//! (2011) neuronal workspace model. Only N agents can be "conscious"
//! (engaged in LLM deliberation) simultaneously, where N = max_concurrent.
//!
//! The attentional blink (Raymond, Shapiro & Arnell, 1992) is modeled:
//! when the LLM is processing one agent's request, another must wait.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Priority for workspace admission (mirrors claws::types::Priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// The Global Workspace: a limited-capacity mechanism for "conscious"
/// processing (LLM deliberation). Only N agents can be admitted at once.
#[derive(Debug)]
pub struct GlobalWorkspace {
    /// Maximum concurrent LLM contexts
    capacity: usize,

    /// Currently active contexts (agents being served)
    active_contexts: Vec<WorkspaceSlot>,

    /// Queue of agents waiting for workspace access
    waiting_queue: VecDeque<WaitingAgent>,

    /// Statistics
    stats: WorkspaceStats,
}

/// A slot in the global workspace: an agent currently engaged in
/// "conscious" (LLM) processing.
#[derive(Debug, Clone)]
pub struct WorkspaceSlot {
    /// The agent occupying this slot
    pub agent_id: String,
    /// When the agent entered the workspace
    pub entered_at: Instant,
    /// Priority of this workspace entry
    pub priority: Priority,
}

/// An agent waiting for workspace access.
#[derive(Debug, Clone)]
pub struct WaitingAgent {
    /// The agent requesting workspace access
    pub agent_id: String,
    /// When the agent was enqueued
    pub enqueued_at: Instant,
    /// Priority of this request
    pub priority: Priority,
    /// Whether this agent can fall back to procedural (reflex-only) mode
    pub can_degrade_to_procedural: bool,
}

/// Statistics about global workspace usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceStats {
    /// Total admissions to the workspace
    pub total_admissions: u64,
    /// Total rejections (workspace full, agent couldn't degrade)
    pub total_rejections: u64,
    /// Total degradations (agent fell back to procedural mode)
    pub total_degradations: u64,
    /// Total time agents spent waiting (microseconds)
    pub total_wait_time_us: u64,
    /// Peak cognitive load observed
    pub peak_cognitive_load: f64,
    /// Attentional blink events (agent had to wait > 1 second)
    pub attentional_blinks: u64,
}

impl GlobalWorkspace {
    /// Create a new global workspace with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            active_contexts: Vec::with_capacity(capacity),
            waiting_queue: VecDeque::new(),
            stats: WorkspaceStats::default(),
        }
    }

    /// Cognitive load: how full is the global workspace?
    /// 0.0 = empty, 1.0 = full, >1.0 = overloaded
    pub fn cognitive_load(&self) -> f64 {
        self.active_contexts.len() as f64 / self.capacity.max(1) as f64
    }

    /// Whether the workspace is at capacity
    pub fn is_full(&self) -> bool {
        self.active_contexts.len() >= self.capacity
    }

    /// Admit an agent to the global workspace (make it "conscious").
    /// Returns Some(slot) if admitted, None if the workspace is full.
    pub fn admit(&mut self, agent: WaitingAgent) -> Option<WorkspaceSlot> {
        if self.active_contexts.len() < self.capacity {
            let slot = WorkspaceSlot {
                agent_id: agent.agent_id,
                entered_at: Instant::now(),
                priority: agent.priority,
            };
            self.active_contexts.push(slot);
            self.stats.total_admissions += 1;
            self.update_peak_load();
            Some(self.active_contexts.last().cloned().unwrap())
        } else {
            // Workspace full: attentional blink
            self.stats.total_rejections += 1;

            if agent.can_degrade_to_procedural {
                // Agent falls back to reflex-only execution (unconscious)
                self.stats.total_degradations += 1;
                None
            } else {
                // Agent must wait
                self.waiting_queue.push_back(agent);
                None
            }
        }
    }

    /// Release a workspace slot (agent finishes deliberation).
    /// Admits the highest-priority waiting agent.
    pub fn release(&mut self, agent_id: &str) -> Option<WorkspaceSlot> {
        let removed = self
            .active_contexts
            .iter()
            .position(|s| s.agent_id == agent_id);
        if let Some(idx) = removed {
            self.active_contexts.remove(idx);
        }

        // Admit next waiting agent (priority-based scheduling)
        if let Some(best_idx) = self
            .waiting_queue
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                // Check for attentional blink: waiting > 1 second
                if a.enqueued_at.elapsed() > Duration::from_secs(1) {
                    // This is a blink event
                }
                true
            })
            .max_by(|(_, a), (_, b)| a.priority.cmp(&b.priority))
            .map(|(i, _)| i)
        {
            let next = self.waiting_queue.remove(best_idx)?;
            self.admit(next)
        } else {
            None
        }
    }

    /// Get the number of agents currently in the workspace
    pub fn active_count(&self) -> usize {
        self.active_contexts.len()
    }

    /// Get the number of agents waiting for workspace access
    pub fn waiting_count(&self) -> usize {
        self.waiting_queue.len()
    }

    /// Get workspace statistics
    pub fn stats(&self) -> &WorkspaceStats {
        &self.stats
    }

    /// Update peak cognitive load statistic
    fn update_peak_load(&mut self) {
        let load = self.cognitive_load();
        if load > self.stats.peak_cognitive_load {
            self.stats.peak_cognitive_load = load;
        }
    }

    /// Check for attentional blink events (agents waiting too long)
    pub fn check_attentional_blinks(&mut self) -> usize {
        let blink_threshold = Duration::from_secs(1);
        let blink_count = self
            .waiting_queue
            .iter()
            .filter(|a| a.enqueued_at.elapsed() > blink_threshold)
            .count();
        self.stats.attentional_blinks += blink_count as u64;
        blink_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_admission() {
        let mut workspace = GlobalWorkspace::new(2);

        let agent1 = WaitingAgent {
            agent_id: "agent-1".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::Normal,
            can_degrade_to_procedural: false,
        };

        let result = workspace.admit(agent1);
        assert!(result.is_some());
        assert_eq!(workspace.active_count(), 1);
    }

    #[test]
    fn test_workspace_capacity() {
        let mut workspace = GlobalWorkspace::new(2);

        for i in 0..3 {
            let agent = WaitingAgent {
                agent_id: format!("agent-{i}"),
                enqueued_at: Instant::now(),
                priority: Priority::Normal,
                can_degrade_to_procedural: false,
            };
            workspace.admit(agent);
        }

        assert_eq!(workspace.active_count(), 2);
        assert_eq!(workspace.waiting_count(), 1);
    }

    #[test]
    fn test_cognitive_load() {
        let mut workspace = GlobalWorkspace::new(4);

        assert!((workspace.cognitive_load() - 0.0).abs() < 0.01);

        let agent = WaitingAgent {
            agent_id: "agent-1".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::Normal,
            can_degrade_to_procedural: false,
        };
        workspace.admit(agent);

        assert!((workspace.cognitive_load() - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_procedural_degradation() {
        let mut workspace = GlobalWorkspace::new(1);

        // Fill the workspace
        let agent1 = WaitingAgent {
            agent_id: "agent-1".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::Normal,
            can_degrade_to_procedural: false,
        };
        workspace.admit(agent1);

        // Agent 2 can degrade to procedural
        let agent2 = WaitingAgent {
            agent_id: "agent-2".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::Normal,
            can_degrade_to_procedural: true,
        };
        let result = workspace.admit(agent2);

        assert!(result.is_none());
        assert_eq!(workspace.stats().total_degradations, 1);
        assert_eq!(workspace.waiting_count(), 0); // Not queued — degraded
    }

    #[test]
    fn test_release_admits_next() {
        let mut workspace = GlobalWorkspace::new(1);

        let agent1 = WaitingAgent {
            agent_id: "agent-1".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::Normal,
            can_degrade_to_procedural: false,
        };
        workspace.admit(agent1);

        let agent2 = WaitingAgent {
            agent_id: "agent-2".to_string(),
            enqueued_at: Instant::now(),
            priority: Priority::High,
            can_degrade_to_procedural: false,
        };
        workspace.admit(agent2);

        assert_eq!(workspace.active_count(), 1);
        assert_eq!(workspace.waiting_count(), 1);

        // Release agent-1, should admit agent-2
        workspace.release("agent-1");
        assert_eq!(workspace.active_count(), 1);
        assert_eq!(workspace.waiting_count(), 0);
    }
}
