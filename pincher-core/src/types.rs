//! Core types for PincherOS

use serde::{Deserialize, Serialize};

/// A stored reflex — an intent-action pair with confidence and usage stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflex {
    pub id: i64,
    pub intent: String,
    pub action: String,
    pub confidence: f64,
    pub created_at: String,
    pub invoked_count: i64,
}

/// Hardware and OS fingerprint of the current shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellFingerprint {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_ram_gb: f64,
    pub ram_usage_percent: f64,
}

/// Database statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub reflex_count: usize,
    pub db_size_bytes: u64,
}

/// The .nail file format for packing and migrating PincherOS state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NailFile {
    pub version: String,
    pub exported_at: String,
    pub shell_fingerprint: ShellFingerprint,
    pub reflexes: Vec<Reflex>,
}
