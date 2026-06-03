//! ShellQuality — First-class shell health metric WITH thermodynamics.
//!
//! The biologist showed that shell quality determines vacancy chain
//! direction. A degrading shell triggers downgrade cascades.
//! A healthy shell attracts upgrade cascades.
//!
//! R3: The thermodynamicist adds energy state as a FIRST-CLASS quality
//! dimension. Shell quality is now a function of BOTH hardware health
//! AND energy availability. A battery-powered shell with 5% charge is
//! a DIFFERENT shell than the same hardware on AC, regardless of health.

use serde::{Deserialize, Serialize};

/// ShellQuality: a first-class metric for shell health assessment.
///
/// R3 UPDATE: Now includes `energy_state` — the thermodynamic dimension.
/// The composite score now weighs energy availability (15%), reducing
/// the weight of other factors proportionally. A shell on emergency
/// battery is effectively a Littorina shell (Minimal shape-verb).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellQuality {
    /// Composite health index: 0.0 (failing) to 1.0 (pristine)
    pub health_index: f64,

    /// SSD / storage wear level: 0.0 (new) to 1.0 (end of life)
    pub storage_wear: f64,

    /// Thermal history: cumulative throttle events
    pub thermal_history: ThermalHistory,

    /// Network reliability: 0.0 (unreliable) to 1.0 (rock solid)
    pub network_reliability: f64,

    /// Battery health: None if not on battery, 0.0-1.0 cycle health
    pub battery_health: Option<f64>,

    /// Uptime stability: how often does this shell crash/reboot?
    pub uptime_stability: f64,

    /// R3: Energy state — the thermodynamic dimension.
    /// None if energy monitoring is unavailable (legacy shells).
    /// Some if the shell reports power/battery data.
    pub energy_state: Option<EnergyQuality>,

    /// Last quality assessment timestamp (epoch seconds)
    pub assessed_at_epoch: u64,
}

/// R3: Energy quality — the thermodynamic component of shell quality.
/// A shell's energy quality is independent of its hardware health.
/// A pristine RPi 4 on 5% battery is a LOW-QUALITY shell despite
/// perfect hardware — you can't USE perfection without joules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyQuality {
    /// Current power source: AC (wall), Battery, or Unknown
    pub power_source: PowerSource,

    /// Battery charge level: 0.0 (empty) to 1.0 (full)
    /// None if on AC (irrelevant)
    pub battery_pct: Option<f64>,

    /// Current power draw (watts)
    pub current_draw_watts: f64,

    /// Thermal headroom: fraction of TDP available (0.0-1.0)
    /// 1.0 = cold, can run at full power. 0.0 = at throttle limit.
    pub thermal_headroom: f64,

    /// Energy efficiency: compute-per-joule relative to platform peak
    /// 1.0 = operating at peak efficiency. <1.0 = throttled or degraded.
    pub compute_efficiency: f64,

    /// Estimated time until thermal throttle at current load (seconds)
    /// None if already throttling
    pub time_to_throttle_secs: Option<f64>,

    /// Cumulative energy consumed since boot (joules)
    /// This is the SHELL's lifetime energy budget tracker
    pub cumulative_joules: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSource {
    /// Wall power: effectively unlimited energy
    Ac,
    /// Battery: finite and depleting
    Battery,
    /// Power over Ethernet: limited but stable
    Poe,
    /// Unknown / not reporting
    Unknown,
}

/// Thermal history: cumulative throttle events over time windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalHistory {
    /// Throttle events in last 24 hours
    pub throttle_events_24h: u64,
    /// Throttle events in last 7 days
    pub throttle_events_7d: u64,
    /// Peak temperature ever recorded (°C)
    pub peak_temp_c: f64,
    /// Current temperature (°C)
    pub current_temp_c: f64,
    /// Thermal design limit (°C)
    pub design_limit_c: f64,
}

impl ThermalHistory {
    /// Normalized throttle rate: 0.0 (never throttled) to 1.0 (constantly throttled).
    pub fn normalized_throttle_rate(&self) -> f64 {
        (self.throttle_events_24h as f64 / 24.0).min(1.0)
    }

    /// Thermal headroom: how far from the design limit.
    pub fn thermal_headroom(&self) -> f64 {
        let margin = self.design_limit_c - self.current_temp_c;
        (margin / 20.0).clamp(0.0, 1.0)
    }
}

impl EnergyQuality {
    /// Energy quality score: 0.0 (no usable energy) to 1.0 (unlimited, efficient).
    ///
    /// This is NOT just battery percentage. It combines:
    /// - Power source reliability (AC > PoE > Battery)
    /// - Battery level (if applicable)
    /// - Thermal headroom (throttling kills efficiency)
    /// - Compute efficiency (throttled = wasteful)
    pub fn score(&self) -> f64 {
        let source_score = match self.power_source {
            PowerSource::Ac => 1.0,
            PowerSource::Poe => 0.85,
            PowerSource::Battery => {
                self.battery_pct.unwrap_or(0.5) * 0.7 // Battery is always less reliable
            }
            PowerSource::Unknown => 0.5,
        };

        let efficiency = self.compute_efficiency;
        let headroom = self.thermal_headroom;

        source_score * 0.40 + efficiency * 0.35 + headroom * 0.25
    }

    /// Is this shell in an energy emergency?
    pub fn is_emergency(&self) -> bool {
        match self.power_source {
            PowerSource::Battery => self.battery_pct.unwrap_or(0.0) < 0.10,
            _ => false,
        }
    }

    /// Is this shell thermally stressed?
    pub fn is_thermally_stressed(&self) -> bool {
        self.thermal_headroom < 0.2
    }
}

impl ShellQuality {
    /// Weighted composite score: the migration decision function.
    ///
    /// R3 UPDATED WEIGHTS (energy dimension added):
    /// - Health index (25%): overall hardware health — like shell wall thickness
    /// - Thermal (20%): thermal headroom — like shell weight (heavy = too hot)
    /// - Storage (15%): SSD wear — like shell internal erosion
    /// - Network (12%): reliability — like aperture integrity
    /// - Battery (8%): power stability — like chemical deterioration
    /// - Energy quality (20%): R3 — joules available = can the crab even move?
    ///
    /// The energy dimension is weighted HIGH because a shell without
    /// joules is a dead shell, regardless of hardware health.
    pub fn composite_score(&self) -> f64 {
        let thermal = 1.0 - self.thermal_history.normalized_throttle_rate();
        let storage = 1.0 - self.storage_wear;
        let network = self.network_reliability;
        let battery = self.battery_health.unwrap_or(1.0);
        let energy = self.energy_state.as_ref().map(|e| e.score()).unwrap_or(0.5);

        self.health_index * 0.25
            + thermal * 0.20
            + storage * 0.15
            + network * 0.12
            + battery * 0.08
            + energy * 0.20
    }

    /// Migration desirability: should an agent WANT to migrate TO this shell?
    ///
    /// R3: Now factors in energy quality. A thermally stressed shell
    /// on battery is NOT desirable, regardless of capacity.
    pub fn migration_desirability(&self, current_load: f64) -> f64 {
        let quality = self.composite_score();
        let capacity = 1.0 - current_load;

        // Energy penalty: don't migrate to a shell that's about to die
        let energy_penalty = match &self.energy_state {
            Some(e) if e.is_emergency() => 0.3,  // Severe penalty
            Some(e) if e.is_thermally_stressed() => 0.7, // Moderate penalty
            _ => 1.0,
        };

        quality * 0.5 + capacity * 0.3 + energy_penalty * 0.2
    }

    /// Emergency evacuation: should agents be EVACUATED FROM this shell?
    pub fn requires_evacuation(&self) -> bool {
        self.composite_score() < 0.30
            || self.thermal_history.current_temp_c
                > self.thermal_history.design_limit_c - 5.0
            || self.storage_wear > 0.90
            || self.network_reliability < 0.30
            || self.energy_state.as_ref().map(|e| e.is_emergency()).unwrap_or(false)
    }

    /// Quality degradation rate: how fast is this shell declining?
    pub fn degradation_rate(&self) -> f64 {
        let daily_rate = self.thermal_history.throttle_events_24h as f64;
        let weekly_avg = self.thermal_history.throttle_events_7d as f64 / 7.0;

        if weekly_avg > 0.0 {
            (daily_rate / weekly_avg - 1.0).clamp(-1.0, 2.0)
        } else if daily_rate > 0.0 {
            2.0 // No prior throttles, now throttling = accelerating
        } else {
            0.0 // Stable
        }
    }

    /// R3: Energy-weighted quality: quality × available_joules.
    /// This is the thermodynamicist's contribution to the migration decision.
    /// Two shells with equal quality scores may have vastly different
    /// energy profiles. This function captures that.
    pub fn energy_weighted_quality(&self) -> f64 {
        let base = self.composite_score();
        match &self.energy_state {
            Some(e) => base * e.score(),
            None => base * 0.5, // Unknown energy = assume moderate
        }
    }
}
