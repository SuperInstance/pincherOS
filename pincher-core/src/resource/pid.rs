//! PID resource controller for RAM homeostasis.
//!
//! The controller adjusts the runtime mode of PincherOS based on current
//! RAM utilisation.  When RAM pressure increases it transitions from
//! [`RuntimeMode::Normal`] → [`RuntimeMode::Light`] → [`RuntimeMode::Critical`]
//! (reflex-only, no LLM).

/// A control action recommended by the PID controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ControlAction {
    /// No change — RAM is within target range.
    Maintain,
    /// Unload the LLM sidecar to free RAM.
    UnloadLLM,
    /// Reduce the LLM context window to reduce RAM pressure.
    ReduceContextWindow,
    /// Load the LLM sidecar (RAM headroom available).
    LoadLLM,
}

/// The current runtime mode of PincherOS, dictated by resource pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RuntimeMode {
    /// Full operation — LLM + reflex engine available.
    Normal,
    /// Reduced context window — LLM available but limited.
    Light,
    /// Reflex-only — LLM is unloaded, only pre-compiled reflexes execute.
    Critical,
}

/// A PID controller tuned for RAM homeostasis.
///
/// The controller tracks the error between current RAM usage and a target
/// ratio, and outputs a [`ControlAction`] each tick.
///
/// # Default gains
/// - `kp` = 2.0 (proportional — reacts strongly to current error)
/// - `ki` = 0.1 (integral — eliminates steady-state error slowly)
/// - `kd` = 0.5 (derivative — dampens oscillation)
/// - `ram_target` = 0.75 (aim for 75 % utilisation)
pub struct ResourceController {
    /// Proportional gain.
    pub kp: f64,
    /// Integral gain.
    pub ki: f64,
    /// Derivative gain.
    pub kd: f64,
    /// Target RAM utilisation ratio (0.0 – 1.0).
    pub ram_target: f64,
    integral: f64,
    prev_error: f64,
}

impl ResourceController {
    /// Create a new PID controller with default gains.
    pub fn new() -> Self {
        Self {
            kp: 2.0,
            ki: 0.1,
            kd: 0.5,
            ram_target: 0.75,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    /// Create a PID controller with custom gains.
    pub fn with_gains(kp: f64, ki: f64, kd: f64, ram_target: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            ram_target,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    /// Process one tick of the controller.
    ///
    /// Returns a [`ControlAction`] based on the PID output.
    pub fn tick(&mut self, ram_used_ratio: f64) -> ControlAction {
        let error = ram_used_ratio - self.ram_target;

        // PID computation.
        self.integral += error;
        // Anti-windup clamp.
        self.integral = self.integral.clamp(-1.0, 1.0);
        let derivative = error - self.prev_error;
        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;
        self.prev_error = error;

        // Map PID output to control action.
        if output > 0.15 {
            // RAM pressure is high — reduce footprint.
            if ram_used_ratio > 0.90 {
                ControlAction::UnloadLLM
            } else {
                ControlAction::ReduceContextWindow
            }
        } else if output < -0.15 {
            // RAM headroom available — can load LLM.
            ControlAction::LoadLLM
        } else {
            ControlAction::Maintain
        }
    }

    /// Determine the current [`RuntimeMode`] based on RAM utilisation.
    pub fn current_mode(&self, ram_used_ratio: f64) -> RuntimeMode {
        if ram_used_ratio > 0.90 {
            RuntimeMode::Critical
        } else if ram_used_ratio > 0.80 {
            RuntimeMode::Light
        } else {
            RuntimeMode::Normal
        }
    }

    /// Reset the controller state (e.g. after migration).
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

impl Default for ResourceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_at_low_ram() {
        let ctrl = ResourceController::new();
        assert_eq!(ctrl.current_mode(0.50), RuntimeMode::Normal);
    }

    #[test]
    fn critical_mode_at_high_ram() {
        let ctrl = ResourceController::new();
        assert_eq!(ctrl.current_mode(0.95), RuntimeMode::Critical);
    }

    #[test]
    fn light_mode_at_moderate_ram() {
        let ctrl = ResourceController::new();
        assert_eq!(ctrl.current_mode(0.85), RuntimeMode::Light);
    }

    #[test]
    fn tick_recommends_unload_at_95_percent() {
        let mut ctrl = ResourceController::new();
        let action = ctrl.tick(0.95);
        assert_eq!(action, ControlAction::UnloadLLM);
    }
}
