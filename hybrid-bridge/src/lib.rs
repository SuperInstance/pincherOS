//! # Hybrid Bridge
//!
//! The communication backbone of the **Hybrid Manifold** — connecting the
//! Matrix Engine, Room Agents, and Veto Engine through high-performance
//! async channels on ARM64.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     HybridEngine                                │
//! │  ┌─────────┐    ┌──────────┐    ┌──────────┐    ┌───────────┐  │
//! │  │ MATRIX   │ ──►│  ROOMS   │ ──►│  VETO    │ ──►│ EXECUTION │  │
//! │  │  Phase   │    │  Phase   │    │  Phase   │    │  Phase    │  │
//! │  └────┬─────┘    └────┬─────┘    └────┬─────┘    └─────┬─────┘  │
//! │       │               │               │               │        │
//! │       └── broadcast ──┘    mpsc ──────┘   broadcast ──┘        │
//! │                              ┌─┐                               │
//! │                              │ │  ← HybridBridge channels      │
//! │                              └─┘                               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Performance Targets (ARM64)
//!
//! | Operation | Target |
//! |-----------|--------|
//! | Matrix fast cycle | < 3ms |
//! | Room analysis (per agent) | < 100ms |
//! | Veto resolution (5000 rooms) | < 10ms |
//! | End-to-end hybrid cycle | < 1s |
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hybrid_bridge::prelude::*;
//!
//! # async fn example() {
//! // Create the communication backbone
//! let bridge = std::sync::Arc::new(HybridBridge::new());
//!
//! // Subscribe to matrix snapshots
//! let mut snapshot_rx = bridge.subscribe_matrix();
//!
//! // Submit a proposal
//! bridge.submit_proposal(RoomProposal {
//!     ticker: "AAPL".into(),
//!     gate: TernaryGate::Bullish,
//!     conviction: 0.85,
//!     confidence: 0.72,
//!     narrative_sig: "abc123".into(),
//!     matrix_agreement: 0.91,
//!     veto_override: false,
//!     timestamp: 1000,
//! }).await.unwrap();
//! # }
//! ```

// ── Module declarations ──────────────────────────────────────────────

pub mod bridge;
pub mod chaos;
pub mod cli;
pub mod datafeed;
pub mod engine;
pub mod error;
pub mod mock_matrix;
pub mod ternary_bridge;
pub mod types;

// ── Prelude ──────────────────────────────────────────────────────────

/// Convenience re-exports for downstream consumers.
pub mod prelude {
    pub use crate::bridge::{BridgeMetrics, BridgeMetricSnapshot, HybridBridge};
    pub use crate::engine::{
        DefaultVetoEngine, HybridConfig, HybridEngine, HybridEngineImpl, MatrixEngine, RoomAgent,
        VetoEngine,
    };
    pub use crate::datafeed::{CsvFileFeed, MarketDataFeed, StockTick};
    pub use crate::error::{HybridError, HybridResult};
    pub use crate::types::{
        detect_non_finite, mask_non_finite, FeatureSuggestion, FinalPosition, GovernanceLayer,
        HybridMessage, MatrixMetadata, MatrixSnapshot, PartialSnapshot, PortfolioVector,
        RoomProposal, SaepAction, SaepConstraint, TernaryGate, TopologicalSignature, Violation,
    };
}

// ── Crate-level re-exports (for ergonomic `use hybrid_bridge::*`) ────

pub use bridge::{BridgeMetrics, BridgeMetricSnapshot, HybridBridge};
pub use engine::{
    DefaultVetoEngine, HybridConfig, HybridEngine, HybridEngineImpl, MatrixEngine as MatrixEngineTrait,
    RoomAgent as RoomAgentTrait, VetoEngine as VetoEngineTrait,
};
pub use datafeed::{CsvFileFeed, MarketDataFeed, StockTick};
pub use error::{HybridError, HybridResult};
pub use types::{
    detect_non_finite, mask_non_finite, FeatureSuggestion, FinalPosition, GovernanceLayer,
    HybridMessage, MatrixSnapshot, PortfolioVector, RoomProposal, SaepAction, SaepConstraint,
    TernaryGate, TopologicalSignature, Violation,
};

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: all types can be constructed and sent across channels.
    #[tokio::test]
    async fn test_smoke_bridge_send_receive() {
        let bridge = HybridBridge::default();
        let mut system_rx = bridge.subscribe_system_events();

        // Send a system event
        bridge.emit_system_event("test".into(), "smoke test".into());

        // Receive via system channel
        match system_rx.recv().await.unwrap() {
            HybridMessage::SystemEvent { kind, payload } => {
                assert_eq!(kind, "test");
                assert_eq!(payload, "smoke test");
            }
            _ => panic!("Expected SystemEvent"),
        }
    }

    /// Test that the prelude exports everything needed (compile-time check).
    #[test]
    fn test_prelude_exports() {
        fn _check(_: prelude::HybridBridge) {}
        fn _check2(_: prelude::TernaryGate) {}
        fn _check3(_: prelude::HybridError) {}
        fn _check4(_: prelude::MatrixSnapshot) {}
        fn _check5(_: prelude::PortfolioVector) {}
        fn _check6(_: prelude::RoomProposal) {}
        fn _check7(_: prelude::FeatureSuggestion) {}
        fn _check8(_: prelude::FinalPosition) {}
        fn _check9(_: prelude::TopologicalSignature) {}
        fn _check10(_: prelude::GovernanceLayer) {}
        fn _check11(_: prelude::SaepAction) {}
        fn _check12(_: prelude::Violation) {}
        fn _check13(_: prelude::HybridConfig) {}
    }

    /// Test the default veto engine is constructable from prelude.
    #[test]
    fn test_default_veto_from_prelude() {
        let _veto = prelude::DefaultVetoEngine::new();
    }

    /// Test that HybridEngineImpl type is recognized (compile check).
    #[test]
    fn test_engine_impl_exists() {
        fn _check_engine<M: engine::MatrixEngine + 'static, V: engine::VetoEngine + 'static>(
            _engine: HybridEngineImpl<M, V>,
        ) {
        }
    }
}
