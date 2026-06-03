//! Article V: The Right to Be Forgotten and Shell Epigenetics

use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::consent::PartyId;

/// The three-zone erasure framework.
/// Different zones have different erasure rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErasureZone {
    /// Agent data: reflexes, trust scores, JEPA predictions, session history
    /// → Full erasure right
    AgentZone,
    /// Interaction data: sandbox profiles, CRDT cells, embedding caches
    /// → Partial erasure right (anonymization, not deletion)
    InteractionZone,
    /// Shell data: thermal history, hardware wear, symbiont configurations
    /// → No erasure right (physical properties, not personal data)
    ShellZone,
}

impl ErasureZone {
    /// What erasure action is available for this zone.
    pub fn erasure_action(&self) -> ErasureAction {
        match self {
            ErasureZone::AgentZone => ErasureAction::FullErasure,
            ErasureZone::InteractionZone => ErasureAction::Anonymize,
            ErasureZone::ShellZone => ErasureAction::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErasureAction {
    /// All data in this zone is deleted
    FullErasure,
    /// Agent's contribution is anonymized (provenance stripped)
    Anonymize,
    /// No erasure right in this zone
    None,
}

/// An erasure request under GDPR Article 17 / §1.3.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureRequest {
    /// Who is requesting erasure
    pub requestor: PartyId,
    /// The agent whose data should be erased
    pub target_agent: String,
    /// The shell that holds the data
    pub target_shell: String,
    /// Which zones to erase
    pub zones: Vec<ErasureZone>,
    /// When the request was made
    #[serde(skip)]
    pub requested_at: Instant,
    /// Deadline for compliance (GDPR: 30 days)
    #[serde(skip)]
    pub compliance_deadline: Instant,
}

/// The result of an erasure request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureResult {
    pub request: ErasureRequest,
    /// Whether agent-zone data was fully erased
    pub agent_zone_erased: bool,
    /// Whether interaction-zone data was anonymized
    pub interaction_zone_anonymized: bool,
    /// Shell zone is always untouched (no erasure right)
    pub shell_zone_untouched: bool,
    /// Status of the gastrolith (migration snapshot)
    pub gastrolith_status: GastrolithStatus,
    /// When the erasure was completed
    #[serde(skip)]
    pub completed_at: Instant,
}

/// The status of the gastrolith (migration snapshot) with respect to erasure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GastrolithStatus {
    /// Within rollback window — erasure deferred until deadline
    ProtectedByRollbackWindow {
        /// When the rollback window expires and erasure can proceed
        deadline: Instant,
    },
    /// Rollback window expired — agent data in gastrolith anonymized
    Anonymized,
    /// No gastrolith exists for this agent on this shell
    NotFound,
    /// Gastrolith was erased (no rollback possible)
    Erased,
}

impl ErasureResult {
    /// Whether the erasure was fully compliant with the request.
    pub fn is_compliant(&self) -> bool {
        let agent_ok = self.request.zones.contains(&ErasureZone::AgentZone)
            == self.agent_zone_erased;
        let interaction_ok = self.request.zones.contains(&ErasureZone::InteractionZone)
            == self.interaction_zone_anonymized;
        let shell_ok = !self.request.zones.contains(&ErasureZone::ShellZone)
            || self.shell_zone_untouched;
        agent_ok && interaction_ok && shell_ok
    }
}
