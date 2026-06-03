//! Article III (Jurisdiction): Cross-Border Migration and Conflict of Laws

use serde::{Deserialize, Serialize};

/// The jurisdictional regime for a migration.
/// Determined by the `determine_applicable_law` function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JurisdictionalRegime {
    /// Same trust boundary, same jurisdiction — no data transfer
    NoTransfer,

    /// Same trust boundary, different jurisdiction
    CrossBorder {
        source: String,
        destination: String,
        /// Whether the destination has GDPR adequacy decision
        adequacy: bool,
        /// Whether Standard Contractual Clauses are required
        safeguards_required: bool,
    },

    /// Different trust boundary, same jurisdiction
    TrustBoundaryCrossing {
        source_boundary: String,
        destination_boundary: String,
        /// Privacy impact score (0.0 = none, 1.0 = severe)
        privacy_impact: f64,
    },

    /// Both trust boundary AND jurisdiction change
    /// → ALWAYS requires Explicit consent (auto-migration FORBIDDEN)
    FullTransfer {
        cross_border: CrossBorderInfo,
        trust_crossing: TrustCrossingInfo,
        /// Always ConsentType::Explicit for full transfers
        consent_required: super::consent::ConsentType,
    },
}

/// Cross-border transfer information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBorderInfo {
    pub source: String,
    pub destination: String,
    /// Whether the destination has an adequacy decision (GDPR Art. 45)
    pub adequacy: bool,
    /// Whether Standard Contractual Clauses are required (GDPR Art. 46(2)(c))
    pub sccs_required: bool,
    /// Whether a Transfer Impact Assessment is required (Schrems II)
    pub transfer_impact_assessment: bool,
}

/// Trust boundary crossing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCrossingInfo {
    pub source: String,
    pub destination: String,
    /// Privacy impact score
    pub impact: f64,
    /// Number of reflexes that would cross the boundary
    pub reflexes_crossing: usize,
    /// Number of reflexes filtered out (stayed behind)
    pub reflexes_filtered: usize,
}

/// Determine the applicable jurisdictional regime for a migration.
pub fn determine_applicable_law(
    old_jurisdiction: &str,
    new_jurisdiction: &str,
    old_trust_boundary: &str,
    new_trust_boundary: &str,
    privacy_impact: f64,
) -> JurisdictionalRegime {
    let trust_crossing = old_trust_boundary != new_trust_boundary;
    let geo_crossing = old_jurisdiction != new_jurisdiction;

    match (trust_crossing, geo_crossing) {
        (false, false) => JurisdictionalRegime::NoTransfer,

        (false, true) => {
            let adequacy = check_adequacy(old_jurisdiction, new_jurisdiction);
            JurisdictionalRegime::CrossBorder {
                source: old_jurisdiction.to_string(),
                destination: new_jurisdiction.to_string(),
                adequacy,
                safeguards_required: !adequacy,
            }
        }

        (true, false) => JurisdictionalRegime::TrustBoundaryCrossing {
            source_boundary: old_trust_boundary.to_string(),
            destination_boundary: new_trust_boundary.to_string(),
            privacy_impact,
        },

        (true, true) => {
            let adequacy = check_adequacy(old_jurisdiction, new_jurisdiction);
            JurisdictionalRegime::FullTransfer {
                cross_border: CrossBorderInfo {
                    source: old_jurisdiction.to_string(),
                    destination: new_jurisdiction.to_string(),
                    adequacy,
                    sccs_required: !adequacy,
                    transfer_impact_assessment: true,
                },
                trust_crossing: TrustCrossingInfo {
                    source: old_trust_boundary.to_string(),
                    destination: new_trust_boundary.to_string(),
                    impact: privacy_impact,
                    reflexes_crossing: 0,   // Filled in by caller
                    reflexes_filtered: 0,   // Filled in by caller
                },
                consent_required: super::consent::ConsentType::Explicit,
            }
        }
    }
}

/// Check whether a destination jurisdiction has GDPR adequacy.
/// Based on European Commission adequacy decisions as of 2025.
fn check_adequacy(_source: &str, destination: &str) -> bool {
    // Jurisdictions with GDPR adequacy decisions
    const ADEQUATE_JURISDICTIONS: &[&str] = &[
        "JP", // Japan (adequacy decision 2019)
        "UK", // United Kingdom (adequacy decision 2021)
        "KR", // South Korea (adequacy decision 2021)
        "CH", // Switzerland
        "IL", // Israel
        "NZ", // New Zealand
        "AR", // Argentina
        "CA", // Canada (commercial organizations)
        "UY", // Uruguay
        "AD", // Andorra
        "FO", // Faroe Islands
        "GG", // Guernsey
        "IM", // Isle of Man
        "JE", // Jersey
        "GB", // United Kingdom (post-Brexit)
        "EU", // EU member states
    ];

    ADEQUATE_JURISDICTIONS.contains(&destination)
}
