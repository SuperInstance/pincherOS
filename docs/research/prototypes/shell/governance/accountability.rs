//! Article VI: Accountability and the Identity Threshold

use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::consent::{ConsentProof, PartyId};

// ═══════════════════════════════════════════
// FORK RECORD
// ═══════════════════════════════════════════

/// The fork record — created when adaptation_ratio > 0.5.
/// Documents the identity transition and carries liability information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkRecord {
    /// The old agent's UUID (predecessor)
    pub predecessor_id: String,
    /// The new agent's UUID (successor)
    pub successor_id: String,
    /// The adaptation ratio at time of fork
    pub adaptation_ratio: f64,
    /// Which reflexes were inherited (and carry reflex liability)
    pub inherited_reflexes: Vec<InheritedReflex>,
    /// Which authorizations survived the fork
    pub surviving_authorizations: Vec<String>,
    /// The consent proof for the fork (User must consent to identity change)
    pub fork_consent: ConsentProof,
    /// The predecessor's accountability ledger (immutable snapshot at fork time)
    pub predecessor_ledger_snapshot: AccountabilityLedgerSummary,
    /// When the fork occurred
    #[serde(skip)]
    pub forked_at: Instant,
}

/// A reflex inherited from the predecessor agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedReflex {
    pub reflex_id: String,
    /// Whether this reflex carries liability from predecessor
    pub carries_liability: bool,
    /// The type of liability
    pub liability_type: LiabilityType,
}

/// The type of liability carried by an inherited reflex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityType {
    /// Reflex was compiled from User's data — User retains ownership
    UserData,
    /// Reflex was learned from fleet interaction — fleet shares liability
    FleetData,
    /// Reflex was distilled from public sources — no special liability
    PublicData,
}

/// Summary of the predecessor's accountability ledger at fork time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountabilityLedgerSummary {
    /// Total number of entries in the predecessor's ledger
    pub total_entries: u64,
    /// Number of entries that involve inherited reflexes
    pub inherited_reflex_entries: u64,
    /// Number of entries that were User-authorized
    pub user_authorized_entries: u64,
    /// SHA-256 hash of the full ledger (for integrity verification)
    pub ledger_hash: [u8; 32],
}

// ═══════════════════════════════════════════
// ACCOUNTABILITY LEDGER
// ═══════════════════════════════════════════

/// The accountability ledger — CRDT-stored, append-only record of all actions.
/// Migrates with the agent via the .nail file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountabilityLedger {
    pub entries: Vec<LedgerEntry>,
}

/// A single entry in the accountability ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// What action was taken
    pub action: String,
    /// Which reflex triggered it (if any)
    pub reflex_id: Option<String>,
    /// Confidence at time of action
    pub confidence: f64,
    /// Whether the User authorized this action
    pub user_authorized: bool,
    /// When the action occurred
    #[serde(skip)]
    pub timestamp: Instant,,
    /// Which shell it occurred on
    pub shell: String,
    /// Which jurisdiction applied
    pub jurisdiction: String,
    /// Consent proof (if applicable — e.g., for migration actions)
    pub consent_proof: Option<ConsentProof>,
    /// What phase the migration was in (if during migration)
    pub migration_phase: Option<MigrationPhaseAtAction>,
}

/// The migration phase during which an action was taken.
/// Used for accountability in the CROSSFADE between-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationPhaseAtAction {
    Stable,
    Preparing,
    Crossfading,
    Finalizing,
    Finalized,
}

impl AccountabilityLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append an entry to the ledger. Append-only — no removal.
    pub fn append(&mut self, entry: LedgerEntry) {
        self.entries.push(entry);
    }

    /// Get all entries involving a specific reflex.
    pub fn entries_for_reflex(&self, reflex_id: &str) -> Vec<&LedgerEntry> {
        self.entries
            .iter()
            .filter(|e| e.reflex_id.as_deref() == Some(reflex_id))
            .collect()
    }

    /// Get all entries during a specific migration phase.
    pub fn entries_during_phase(&self, phase: MigrationPhaseAtAction) -> Vec<&LedgerEntry> {
        self.entries
            .iter()
            .filter(|e| e.migration_phase == Some(phase))
            .collect()
    }

    /// Compute a summary for fork record purposes.
    pub fn summary(&self) -> AccountabilityLedgerSummary {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for entry in &self.entries {
            entry.action.hash(&mut hasher);
            entry.confidence.to_bits().hash(&mut hasher);
        }
        let hash_bytes = hasher.finish().to_le_bytes();

        AccountabilityLedgerSummary {
            total_entries: self.entries.len() as u64,
            inherited_reflex_entries: 0, // Computed by caller
            user_authorized_entries: self.entries.iter().filter(|e| e.user_authorized).count() as u64,
            ledger_hash: {
                let mut arr = [0u8; 32];
                arr[..8].copy_from_slice(&hash_bytes);
                arr
            },
        }
    }
}

impl Default for AccountabilityLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════
// ACCOUNTABILITY RESOLUTION
// ═══════════════════════════════════════════

/// Who is accountable for an action, based on adaptation ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountabilityParty {
    /// The migrated agent (same identity, adaptation < 0.5)
    MigratedAgent,
    /// The new agent (forked identity, adaptation > 0.5)
    NewAgent,
    /// Both agents (during CROSSFADE, adaptation indeterminate)
    BothAgents,
    /// The User (vicarious liability for forked agent's inherited reflex actions)
    User,
}

/// Resolve accountability for an action taken at a given adaptation ratio.
pub fn resolve_accountability(
    adaptation_ratio: f64,
    during_crossfade: bool,
    used_inherited_reflex: bool,
    user_authorized: bool,
) -> AccountabilityParty {
    if during_crossfade {
        // Between-state: both agents share accountability
        AccountabilityParty::BothAgents
    } else if adaptation_ratio < 0.5 {
        // Pre-threshold: migrated agent is accountable
        AccountabilityParty::MigratedAgent
    } else {
        // Post-threshold: new agent + User (if inherited reflex used)
        if used_inherited_reflex && user_authorized {
            AccountabilityParty::User
        } else {
            AccountabilityParty::NewAgent
        }
    }
}
