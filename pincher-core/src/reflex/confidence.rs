//! Confidence tracking for reflexes.
//!
//! Confidence scores determine which execution path a matched reflex takes:
//! - **Direct** (> 0.90) — execute immediately
//! - **Confirm** (0.70 – 0.90) — execute but log for review
//! - **LlmRoute** (< 0.70) — fall back to the LLM sidecar

use serde::Serialize;

/// Updates a reflex's confidence score based on whether execution succeeded.
///
/// - On success, confidence increases by 5 % of the remaining gap toward 1.0.
/// - On failure, confidence decreases by 10 % of the current value.
///
/// The result is clamped to \[0.01, 0.99\].
pub fn update_confidence(current: f64, success: bool) -> f64 {
    if success {
        (current + 0.05 * (1.0 - current)).min(0.99)
    } else {
        (current - 0.10 * current).max(0.01)
    }
}

/// The execution path chosen for a reflex based on its confidence.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum ReflexPath {
    /// Confidence > 0.80 — execute immediately, no confirmation.
    Direct,
    /// Confidence 0.55 – 0.80 — execute but flag for review.
    Confirm,
    /// Confidence < 0.55 — fall back to the LLM sidecar.
    LlmRoute,
}

impl ReflexPath {
    /// Determine the path from a confidence score.
    /// Thresholds match MatchThresholds (0.80/0.55) calibrated for MiniLM-L6-v2.
    pub fn from_confidence(c: f64) -> Self {
        if c > 0.80 {
            ReflexPath::Direct
        } else if c > 0.55 {
            ReflexPath::Confirm
        } else {
            ReflexPath::LlmRoute
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_increases_on_success() {
        let c = update_confidence(0.80, true);
        assert!(c > 0.80);
        assert!(c <= 0.99);
    }

    #[test]
    fn confidence_decreases_on_failure() {
        let c = update_confidence(0.80, false);
        assert!(c < 0.80);
        assert!(c >= 0.01);
    }

    #[test]
    fn direct_path() {
        assert_eq!(ReflexPath::from_confidence(0.90), ReflexPath::Direct);
    }

    #[test]
    fn confirm_path() {
        assert_eq!(ReflexPath::from_confidence(0.70), ReflexPath::Confirm);
    }

    #[test]
    fn llm_route_path() {
        assert_eq!(ReflexPath::from_confidence(0.40), ReflexPath::LlmRoute);
    }

    #[test]
    fn confidence_clamps_high() {
        let c = update_confidence(0.999, true);
        assert!(c <= 0.99);
    }

    #[test]
    fn confidence_clamps_low() {
        let c = update_confidence(0.01, false);
        assert!(c >= 0.01);
    }
}
