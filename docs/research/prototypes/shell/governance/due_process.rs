//! Article IV: Due Process for Constraint Failures

use serde::{Deserialize, Serialize};

use super::constitution::ConstraintId;

/// A constraint failure notice (Step 1: Notice).
/// Delivered to the agent and User when the MigrationGuard returns Fail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintFailureNotice {
    pub constraint_id: ConstraintId,
    /// What was required
    pub threshold: f64,
    /// What was observed
    pub actual: f64,
    /// Human-readable explanation (§1.5: Right of Explanation)
    pub explanation: String,
    /// Seconds until appeal deadline (default: 60)
    pub appeal_deadline_secs: u64,
    /// Actions the agent can take to remedy the failure
    pub remediation_options: Vec<RemediationOption>,
}

/// Actions the agent can take to potentially satisfy the constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationOption {
    /// Re-measure the constraint (e.g., re-probe shell capabilities)
    ReMeasure { description: String },
    /// Reduce the adaptation ratio (e.g., preserve more substance)
    ReduceAdaptation { target_ratio: f64 },
    /// Re-embed reflexes from trigger_text (addresses version skew)
    ReEmbedReflexes,
    /// Request explicit User consent for an exception
    RequestExplicitConsent,
}

/// Grounds for appealing a constraint failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppealGround {
    /// The constraint was measured incorrectly
    IncorrectMeasurement,
    /// The threshold is too strict for this context
    IncorrectThreshold,
    /// C1 misclassified identity-significant state as accident
    SubstanceAccidentError,
    /// The agent's context makes the constraint inapplicable
    ContextualOverride,
    /// A previous Consent Court decision would allow this
    PrecedentConflict,
}

/// Relief requested from the Consent Court.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relief {
    /// Allow the migration despite the constraint failure
    AllowMigration,
    /// Reduce the threshold for this specific case
    ReduceThreshold,
    /// Re-measure and re-evaluate with better data
    ReMeasureAndEvaluate,
    /// Allow with additional conditions
    ConditionalAllow,
}

/// A due process appeal (Step 2: Appeal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAppeal {
    pub notice: ConstraintFailureNotice,
    /// Who is filing the appeal
    pub appellant: super::consent::PartyId,
    /// Legal grounds for the appeal
    pub grounds: AppealGround,
    /// Evidence supporting the appeal
    pub evidence: Vec<String>,
    /// What relief the appellant seeks
    pub requested_relief: Relief,
    /// When the appeal was filed
    #[serde(skip)]
    pub filed_at: std::time::Instant,
}

/// Anti-abuse: appeal rate limiter.
/// Maximum 3 appeals per 5-minute window per appellant per constraint.
#[derive(Debug, Clone)]
pub struct AppealRateLimiter {
    /// Appeals filed in the current window, keyed by (appellant, constraint_id)
    appeals: std::collections::HashMap<(String, ConstraintId), Vec<std::time::Instant>>,
    /// Maximum appeals per window
    max_appeals: usize,
    /// Window duration in seconds
    window_secs: u64,
}

impl AppealRateLimiter {
    pub fn new() -> Self {
        Self {
            appeals: std::collections::HashMap::new(),
            max_appeals: 3,
            window_secs: 300, // 5 minutes
        }
    }

    /// Check whether an appeal is allowed under the rate limit.
    pub fn may_appeal(&self, appellant: &str, constraint_id: ConstraintId) -> bool {
        let key = (appellant.to_string(), constraint_id);
        match self.appeals.get(&key) {
            None => true,
            appeals => {
                let count = appeals
                    .unwrap()
                    .iter()
                    .filter(|t| t.elapsed().as_secs() < self.window_secs)
                    .count();
                count < self.max_appeals
            }
        }
    }

    /// Record that an appeal was filed.
    pub fn record_appeal(&mut self, appellant: &str, constraint_id: ConstraintId) {
        let key = (appellant.to_string(), constraint_id);
        let now = std::time::Instant::now();
        self.appeals.entry(key).or_default().push(now);
    }
}

impl Default for AppealRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
