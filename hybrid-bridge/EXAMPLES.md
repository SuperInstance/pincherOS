# hybrid-bridge Examples

> Runnable code examples for the Hybrid Manifold communication backbone.

Each example is a standalone `#[tokio::test]` that can be run with `cargo test`. Import `hybrid_bridge::prelude::*` for all examples.

---

## Contents

- [Example 1: Bridge Creation & Channel Inspection](#example-1-bridge-creation--channel-inspection)
- [Example 2: Snapshot Broadcast & Receive](#example-2-snapshot-broadcast--receive)
- [Example 3: Proposal Submission (Async & Non-Blocking)](#example-3-proposal-submission-async--non-blocking)
- [Example 4: Full Proposal Pipeline (Room → Veto → Execute)](#example-4-full-proposal-pipeline-room--veto--execute)
- [Example 5: One Hybrid Cycle (Matrix → Rooms → Veto → Execution)](#example-5-one-hybrid-cycle-matrix--rooms--veto--execution)
- [Example 6: Feature Suggestion Flow](#example-6-feature-suggestion-flow)
- [Example 7: SAEP Constraint Registration](#example-7-saep-constraint-registration)
- [Example 8: Chaos Injection & Recovery](#example-8-chaos-injection--recovery)
- [Example 9: Bridge Metrics & Observability](#example-9-bridge-metrics--observability)
- [Example 10: Freeze / Unfreeze Cycle](#example-10-freeze--unfreeze-cycle)
- [Example 11: System Event Handling](#example-11-system-event-handling)
- [Example 12: Shutdown & Graceful Teardown](#example-12-shutdown--graceful-teardown)
- [Example 13: Custom Veto Engine Implementation](#example-13-custom-veto-engine-implementation)

---

## Example 1: Bridge Creation & Channel Inspection

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_bridge_creation() {
    // Default channel capacities:
    //   matrix broadcast: 256
    //   veto broadcast:   256
    //   proposal mpsc:    8192
    //   feature mpsc:     1024
    let bridge = Arc::new(HybridBridge::new());
    assert_eq!(bridge.subscriber_count(), 0);

    // Custom channel capacities for high-throughput setups:
    let bridge = Arc::new(HybridBridge::with_capacities(
        1024,   // matrix broadcast
        1024,   // veto broadcast
        16384,  // proposal mpsc
        4096,   // feature mpsc
    ));

    // Subscribe multiple receivers
    let rx1 = bridge.subscribe_matrix();
    let rx2 = bridge.subscribe_matrix();
    let rx3 = bridge.subscribe_portfolio();
    assert_eq!(bridge.subscriber_count(), 2); // only matrix subscribers count

    println!("Bridge created. Matrix subscribers: {}", bridge.subscriber_count());
}
```

---

## Example 2: Snapshot Broadcast & Receive

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;
use ndarray::array;

#[tokio::test]
async fn example_snapshot_broadcast() {
    let bridge = Arc::new(HybridBridge::new());
    let mut rx = bridge.subscribe_matrix();

    // --- Matrix side: broadcast ---
    let snapshot = MatrixSnapshot {
        tick: 42,
        n_stocks: 3,
        eigenvalues: vec![0.91, 0.06, 0.03],
        eigenvectors: array![[0.7071, 0.7071], [0.7071, -0.7071]],
        topologies: vec![
            TopologicalSignature {
                ticker: "AAPL".into(),
                betti_numbers: vec![1, 0, 0],
                persistence_landscape: vec![0.5, 0.1],
                wasserstein_distance_centroid: 0.05,
                regime_label: "stable".into(),
                confidence: 0.92,
            },
        ],
        universe_betti: [3, 1, 0],
        regime: "stable".into(),
        condition_number: 2.5,
    };

    let recipients = bridge.broadcast_snapshot(snapshot);
    assert_eq!(recipients, 1, "One subscriber should receive");

    // --- Room side: receive ---
    match rx.recv().await.unwrap() {
        HybridMessage::SnapshotBroadcast(snap) => {
            assert_eq!(snap.tick, 42);
            assert_eq!(snap.regime, "stable");
            assert!((snap.condition_number - 2.5).abs() < 1e-10);

            // Inspect topological signatures
            let topo = &snap.topologies[0];
            assert_eq!(topo.ticker, "AAPL");
            assert_eq!(topo.betti_numbers, vec![1, 0, 0]);
            assert!(topo.confidence > 0.9);
        }
        other => panic!("Expected SnapshotBroadcast, got {:?}", other),
    }
}
```

---

## Example 3: Proposal Submission (Async & Non-Blocking)

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

async fn make_proposal(ticker: &str, gate: TernaryGate, conviction: f64) -> RoomProposal {
    RoomProposal {
        ticker: ticker.into(),
        gate,
        conviction,
        confidence: 0.75,
        narrative_sig: format!("narrative_{}", ticker),
        matrix_agreement: 0.8,
        veto_override: false,
        timestamp: 1000,
    }
}

#[tokio::test]
async fn example_proposal_submission() {
    let bridge = Arc::new(HybridBridge::new());

    // --- Async submission (preferred) ---
    let proposal = make_proposal("AAPL", TernaryGate::Bullish, 0.85).await;
    bridge.submit_proposal(proposal).await.unwrap();

    // --- Non-blocking submission ---
    let fast_proposal = make_proposal("GOOGL", TernaryGate::Neutral, 0.0).await;
    bridge.try_submit_proposal(fast_proposal).unwrap();

    // --- Verify both proposals arrived ---
    let mut rx = bridge.take_proposal_receiver().await.unwrap();

    let p1 = rx.recv().await.unwrap();
    assert_eq!(p1.ticker, "AAPL");
    assert_eq!(p1.gate, TernaryGate::Bullish);

    let p2 = rx.recv().await.unwrap();
    assert_eq!(p2.ticker, "GOOGL");
    assert_eq!(p2.gate, TernaryGate::Neutral);
}
```

---

## Example 4: Full Proposal Pipeline (Room → Veto → Execute)

```rust
use std::sync::Arc;
use std::collections::HashMap;
use hybrid_bridge::prelude::*;
use hybrid_bridge::engine::DefaultVetoEngine;

#[tokio::test]
async fn example_proposal_pipeline() {
    let bridge = Arc::new(HybridBridge::new());
    let mut portfolio_rx = bridge.subscribe_portfolio();

    // --- Step 1: Rooms submit proposals ---
    let proposals = vec![
        RoomProposal {
            ticker: "AAPL".into(),
            gate: TernaryGate::Bullish,
            conviction: 0.85,
            confidence: 0.90,
            narrative_sig: "aapl_narrative".into(),
            matrix_agreement: 0.80,
            veto_override: false,
            timestamp: 1,
        },
        RoomProposal {
            ticker: "TSLA".into(),
            gate: TernaryGate::Bearish,
            conviction: 0.70,
            confidence: 0.65,
            narrative_sig: "tsla_narrative".into(),
            matrix_agreement: 0.30,
            veto_override: false,
            timestamp: 1,
        },
        RoomProposal {
            ticker: "GOOGL".into(),
            gate: TernaryGate::Neutral,
            conviction: 0.50,
            confidence: 0.40,
            narrative_sig: "googl_narrative".into(),
            matrix_agreement: 0.90,
            veto_override: false,
            timestamp: 1,
        },
    ];

    for p in &proposals {
        bridge.submit_proposal(p.clone()).await.unwrap();
    }

    // --- Step 2: Veto resolves ---
    let veto = DefaultVetoEngine::new();
    let portfolio = veto.resolve(&proposals, None).await;

    assert_eq!(portfolio.positions.len(), 3);
    assert!((portfolio.gross_exposure - 1.55).abs() < 1e-6); // 0.85 + 0.70 + 0.00

    // --- Step 3: Execute (broadcast portfolio) ---
    bridge.broadcast_portfolio(portfolio.clone());

    match portfolio_rx.recv().await.unwrap() {
        HybridMessage::PortfolioVectorBroadcast(pf) => {
            // Inspect individual positions
            let aapl = pf.positions.iter().find(|p| p.ticker == "AAPL").unwrap();
            assert!(aapl.weight > 0.0); // Bullish → positive
            assert!(aapl.veto_applied.is_empty());
            assert_eq!(aapl.raw_gate, TernaryGate::Bullish);

            let tsla = pf.positions.iter().find(|p| p.ticker == "TSLA").unwrap();
            assert!(tsla.weight < 0.0); // Bearish → negative

            let googl = pf.positions.iter().find(|p| p.ticker == "GOOGL").unwrap();
            assert!(googl.weight.abs() < 0.01); // Neutral → ~0

            println!("Portfolio: {} positions, net exposure = {:.4}",
                pf.positions.len(), pf.net_exposure);
        }
        _ => panic!("Expected PortfolioVectorBroadcast"),
    }
}
```

---

## Example 5: One Hybrid Cycle (Matrix → Rooms → Veto → Execution)

This example uses the `MockMatrixEngine` and a custom room agent to run a full cycle.

```rust
use std::sync::Arc;
use async_trait::async_trait;
use hybrid_bridge::prelude::*;
use hybrid_bridge::engine::{
    DefaultVetoEngine, HybridConfig, HybridEngine, HybridEngineImpl,
    MatrixEngine, RoomAgent,
};
use hybrid_bridge::mock_matrix::MockMatrixEngine;

// --- Custom Room Agent ---
struct MyRoomAgent {
    ticker: String,
}

#[async_trait]
impl RoomAgent for MyRoomAgent {
    async fn analyze(
        &self,
        snapshot: &MatrixSnapshot,
        _slice: Option<ndarray::Array2<f32>>,
        _cross: Option<ndarray::Array1<f32>>,
    ) -> RoomProposal {
        RoomProposal {
            ticker: self.ticker.clone(),
            gate: TernaryGate::Bullish,
            conviction: 0.75,
            confidence: 0.60,
            narrative_sig: "example_analysis".into(),
            matrix_agreement: snapshot.condition_number.recip(),
            veto_override: false,
            timestamp: snapshot.tick,
        }
    }

    async fn on_symmetry_alert(
        &self, _peer: &str, _score: f64, _topo: &TopologicalSignature,
    ) {}

    async fn suggest_feature(&self, _s: FeatureSuggestion) {}

    async fn set_regime(&mut self, _l: String) {}

    async fn update_narrative(&mut self, _n: String) {}
}

#[tokio::test]
async fn example_hybrid_cycle() {
    // 1. Create matrix engine with 3 tickers
    let matrix = MockMatrixEngine::new(3, 5, 100)
        .with_tickers(&["AAPL", "MSFT", "GOOGL"]);
    matrix.seed_random();

    // 2. Create the bridge
    let bridge = Arc::new(HybridBridge::new());
    let mut pf_rx = bridge.subscribe_portfolio();

    // 3. Create veto engine
    let veto = DefaultVetoEngine::new();

    // 4. Create room agents
    let rooms: Vec<Box<dyn RoomAgent>> = vec![
        Box::new(MyRoomAgent { ticker: "AAPL".into() }),
        Box::new(MyRoomAgent { ticker: "MSFT".into() }),
        Box::new(MyRoomAgent { ticker: "GOOGL".into() }),
    ];

    // 5. Create the engine
    let engine = HybridEngineImpl::new(
        matrix,
        bridge.clone(),
        veto,
        rooms,
        HybridConfig::default(),
    );

    // 6. Run one hybrid cycle
    let start = std::time::Instant::now();
    engine.hybrid_cycle(1).await;
    let elapsed = start.elapsed();

    println!("Hybrid cycle completed in {:?}", elapsed);

    // 7. Verify portfolio broadcast
    match pf_rx.recv().await.unwrap() {
        HybridMessage::PortfolioVectorBroadcast(portfolio) => {
            assert_eq!(portfolio.positions.len(), 3);
            assert!(portfolio.gross_exposure > 0.0);
            println!(
                "Portfolio: {} positions, gross={:.4}, net={:.4}",
                portfolio.positions.len(),
                portfolio.gross_exposure,
                portfolio.net_exposure,
            );
        }
        _ => panic!("Expected PortfolioVectorBroadcast"),
    }

    // 8. Check metrics
    let metrics = bridge.metrics().snapshot();
    println!(
        "Bridge metrics: {} snapshots, {} proposals",
        metrics.snapshots, metrics.proposals,
    );
}
```

---

## Example 6: Feature Suggestion Flow

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_feature_suggestion() {
    let bridge = Arc::new(HybridBridge::new());

    // --- Room suggests a new feature ---
    let suggestion = FeatureSuggestion {
        ticker: "AAPL".into(),
        feature_name: "lithium_correlation".into(),
        source: "earnings_call_analysis".into(),
        urgency: 0.85,
        sample_data: vec![0.1, 0.2, 0.15, 0.3, 0.25, 0.4, 0.35, 0.5,
                          0.45, 0.6, 0.55, 0.7, 0.65, 0.8, 0.75, 0.9,
                          0.85, 1.0, 0.95, 0.9],
    };

    bridge.submit_feature_suggestion(suggestion).await.unwrap();

    // --- Matrix Engine collects ---
    let mut rx = bridge.take_feature_receiver().await.unwrap();
    let received = rx.recv().await.unwrap();

    assert_eq!(received.feature_name, "lithium_correlation");
    assert_eq!(received.source, "earnings_call_analysis");
    assert_eq!(received.sample_data.len(), 20);
    assert!((received.urgency - 0.85).abs() < 1e-10);
}
```

---

## Example 7: SAEP Constraint Registration

```rust
use std::sync::Arc;
use std::collections::HashMap;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_saep_constraint() {
    use hybrid_bridge::engine::DefaultVetoEngine;

    let mut veto = DefaultVetoEngine::new();

    // --- Constraint 1: No single stock > 10% gross exposure ---
    let max_single = SaepConstraint {
        id: "max_single_exposure".into(),
        layer: GovernanceLayer::Room,
        check_fn: Arc::new(|proposal: &RoomProposal, _ctx: &HashMap<String, f64>| {
            let raw_exposure = match proposal.gate {
                TernaryGate::Bullish => proposal.conviction,
                TernaryGate::Bearish => -proposal.conviction,
                TernaryGate::Neutral => 0.0,
            };
            if raw_exposure.abs() > 0.10 {
                Err(Violation {
                    constraint_id: "max_single_exposure".into(),
                    message: format!("Exposure {:.4} exceeds 0.10 max", raw_exposure.abs()),
                    severity: 0.7,
                })
            } else {
                Ok(())
            }
        }),
        action: SaepAction::Limit,
        escalate_to: None,
    };

    // --- Constraint 2: Block bearish on defense tickers ---
    let no_bearish_defense = SaepConstraint {
        id: "no_bearish_defense".into(),
        layer: GovernanceLayer::Sector,
        check_fn: Arc::new(|proposal: &RoomProposal, _ctx: &HashMap<String, f64>| {
            let defense_tickers = ["LMT", "RTX", "NOC", "GD"];
            if defense_tickers.contains(&proposal.ticker.as_str())
                && proposal.gate == TernaryGate::Bearish
            {
                Err(Violation {
                    constraint_id: "no_bearish_defense".into(),
                    message: format!("Cannot be bearish on defense ticker {}", proposal.ticker),
                    severity: 1.0,
                })
            } else {
                Ok(())
            }
        }),
        action: SaepAction::Veto,
        escalate_to: Some(GovernanceLayer::Market),
    };

    // Register constraints
    veto.register_constraint(max_single).await;
    veto.register_constraint(no_bearish_defense).await;

    // --- Test: Proposal with high conviction gets limited ---
    let proposals = vec![
        RoomProposal {
            ticker: "AAPL".into(),
            gate: TernaryGate::Bullish,
            conviction: 0.30, // Above 0.10 max → limited
            confidence: 0.80,
            narrative_sig: "test".into(),
            matrix_agreement: 0.70,
            veto_override: false,
            timestamp: 1,
        },
        RoomProposal {
            ticker: "LMT".into(),
            gate: TernaryGate::Bearish,
            conviction: 0.50,
            confidence: 0.60,
            narrative_sig: "test".into(),
            matrix_agreement: 0.30,
            veto_override: false,
            timestamp: 1,
        },
    ];

    let portfolio = veto.resolve(&proposals, None).await;

    // AAPL: conviction capped to ~50% of original due to Severity=0.7 limit
    let aapl = &portfolio.positions[0];
    assert!(aapl.veto_applied.contains(&"max_single_exposure"));
    // weight = 0.30 * (1 - 0.7) = 0.09 (limited but not vetoed)
    assert!((aapl.weight - 0.09).abs() < 1e-6);

    // LMT: full veto
    let lmt = &portfolio.positions[1];
    assert!(lmt.veto_applied.contains(&"no_bearish_defense"));
    assert!((lmt.weight).abs() < 1e-10);
    assert!((lmt.veto_severity - 1.0).abs() < 1e-6);
}
```

---

## Example 8: Chaos Injection & Recovery

```rust
use ndarray::Array3;
use hybrid_bridge::prelude::*;
use hybrid_bridge::chaos::{inject_nan_random, run_chaos_cycle, ChaosTestResult};

#[test]
fn example_chaos_cycle() {
    let mut tensor = Array3::<f32>::ones((5, 10, 20));

    // --- Initial health check ---
    assert!(detect_non_finite(&tensor).is_empty());

    // --- Inject NaNs ---
    let coords = inject_nan_random(&mut tensor, 3);
    assert_eq!(coords.len(), 3);

    // Verify detection
    let flagged = detect_non_finite(&tensor);
    assert!(!flagged.is_empty());
    assert_eq!(flagged.len(), 3);

    // --- Mask and recover ---
    let masked_count = mask_non_finite(&mut tensor);
    assert_eq!(masked_count, 3);

    // Verify complete recovery
    assert!(detect_non_finite(&tensor).is_empty());

    // Tensor must still be usable
    let sum: f32 = tensor.iter().sum();
    assert!(sum.is_finite());
    assert!(sum > 0.0, "Tensor should still have meaningful data");

    // --- Full chaos cycle (inject → detect → mask → verify) ---
    let mut tensor2 = Array3::<f32>::zeros((10, 20, 30));
    let result: ChaosTestResult = run_chaos_cycle(&mut tensor2, 10);

    assert!(result.failures.is_empty());
    assert_eq!(result.injected, 10);
    assert_eq!(result.detected, result.masked);
    assert!(result.safe_mode_triggered);

    println!(
        "Chaos test: injected={}, detected={}, masked={}, safe_mode={}",
        result.injected, result.detected, result.masked, result.safe_mode_triggered,
    );
}

#[test]
fn example_chaos_mock_matrix() {
    use hybrid_bridge::mock_matrix::MockMatrixEngine;

    let engine = MockMatrixEngine::new(5, 3, 50).with_tickers(&["A", "B", "C", "D", "E"]);
    engine.seed_random();

    // Initially clean
    let rt = tokio::runtime::Runtime::new().unwrap();

    let clean = rt.block_on(async {
        let t = engine.tensor();
        let tensor = t.read().await;
        detect_non_finite(&tensor)
    });
    assert_eq!(clean.len(), 0);

    // Inject chaos
    engine.inject_nan(2, 1, 10);
    engine.inject_inf(4, 0, 5);

    let flagged = rt.block_on(async {
        let t = engine.tensor();
        let tensor = t.read().await;
        detect_non_finite(&tensor)
    });
    assert_eq!(flagged.len(), 2);

    // Full cycle still works after masking
    let snapshot = rt.block_on(engine.full_cycle(1));
    assert!(snapshot.condition_number.is_finite());
}
```

---

## Example 9: Bridge Metrics & Observability

```rust
use std::sync::Arc;
use std::collections::HashMap;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_bridge_metrics() {
    let bridge = Arc::new(HybridBridge::new());

    // Subscribe some receivers
    let _rx1 = bridge.subscribe_matrix();
    let _rx2 = bridge.subscribe_matrix();
    let _rx3 = bridge.subscribe_portfolio();

    // Send various messages
    bridge.broadcast_snapshot(MatrixSnapshot {
        tick: 1,
        n_stocks: 100,
        eigenvalues: vec![],
        eigenvectors: ndarray::Array2::from_shape_vec((0, 0), vec![]).unwrap(),
        topologies: vec![],
        universe_betti: [0, 0, 0],
        regime: "test".into(),
        condition_number: 0.0,
    });

    bridge.submit_proposal(RoomProposal {
        ticker: "TEST".into(),
        gate: TernaryGate::Neutral,
        conviction: 0.5,
        confidence: 0.5,
        narrative_sig: "metrics_test".into(),
        matrix_agreement: 0.5,
        veto_override: false,
        timestamp: 1,
    }).await.unwrap();

    bridge.broadcast_portfolio(PortfolioVector {
        positions: vec![],
        gross_exposure: 0.0,
        net_exposure: 0.0,
        sector_concentrations: HashMap::new(),
        portfolio_var: 0.0,
        timestamp: 1,
    });

    bridge.emit_system_event("info".into(), "metrics test".into());

    // --- Snapshot metrics ---
    let s = bridge.metrics().snapshot();
    assert_eq!(s.snapshots, 1);
    assert_eq!(s.proposals, 1);
    assert_eq!(s.portfolios, 1);
    assert_eq!(s.system_events, 1);
    assert_eq!(s.subscribers, 2); // only matrix subscribers

    println!("Bridge metrics snapshot:");
    println!("  snapshots:   {}", s.snapshots);
    println!("  proposals:   {}", s.proposals);
    println!("  features:    {}", s.features);
    println!("  portfolios:  {}", s.portfolios);
    println!("  system evts: {}", s.system_events);
    println!("  subscribers: {}", s.subscribers);
    println!("  dropped:     {}", s.dropped);

    // Metrics are atomic — they're safe to read concurrently
    assert!(s.subscribers >= 0);
}
```

---

## Example 10: Freeze / Unfreeze Cycle

```rust
use std::sync::Arc;
use std::collections::HashMap;
use hybrid_bridge::prelude::*;
use hybrid_bridge::engine::DefaultVetoEngine;

#[tokio::test]
async fn example_freeze_cycle() {
    let mut veto = DefaultVetoEngine::new();

    let proposal = RoomProposal {
        ticker: "SPY".into(),
        gate: TernaryGate::Bullish,
        conviction: 0.80,
        confidence: 0.90,
        narrative_sig: "freeze_test".into(),
        matrix_agreement: 0.70,
        veto_override: false,
        timestamp: 1,
    };

    // --- Normal operation ---
    let pf = veto.resolve(&[proposal.clone()], None).await;
    assert_eq!(pf.positions.len(), 1);
    assert!((pf.positions[0].weight - 0.80).abs() < 1e-6);

    // --- Freeze ---
    veto.freeze("Market volatility > 3σ").await;

    let pf_frozen = veto.resolve(&[proposal.clone()], None).await;
    assert!(pf_frozen.positions.is_empty(),
        "Frozen veto must return empty positions");

    // --- Unfreeze ---
    veto.unfreeze("Volatility returned to normal").await;

    let pf_unfrozen = veto.resolve(&[proposal.clone()], None).await;
    assert_eq!(pf_unfrozen.positions.len(), 1,
        "Unfrozen veto must process proposals again");
    assert!((pf_unfrozen.positions[0].weight - 0.80).abs() < 1e-6);
}
```

---

## Example 11: System Event Handling

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_system_events() {
    let bridge = Arc::new(HybridBridge::new());
    let mut system_rx = bridge.subscribe_system_events();

    // Emit various system events
    bridge.emit_system_event("freeze".into(), "Market circuit breaker triggered".into());
    bridge.emit_system_event("engine_shutdown".into(), "Scheduled maintenance".into());
    bridge.emit_system_event("error".into(), "Feature tensor NaN detected".into());

    // Receive and classify events
    for i in 0..3 {
        match system_rx.recv().await.unwrap() {
            HybridMessage::SystemEvent { kind, payload } => {
                println!("Event {}: kind={}, payload={}", i, kind, payload);
                match kind.as_str() {
                    "freeze" => assert!(payload.contains("circuit breaker")),
                    "engine_shutdown" => assert!(payload.contains("maintenance")),
                    "error" => assert!(payload.contains("NaN")),
                    other => panic!("Unexpected event kind: {}", other),
                }
            }
            _ => panic!("Expected SystemEvent"),
        }
    }
}
```

---

## Example 12: Shutdown & Graceful Teardown

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

#[tokio::test]
async fn example_shutdown() {
    let bridge = Arc::new(HybridBridge::new());
    let mut system_rx = bridge.subscribe_system_events();

    // Check initial state
    assert!(!bridge.is_shutdown_requested());

    // Request shutdown
    bridge.request_shutdown();
    assert!(bridge.is_shutdown_requested());

    // Verify system event was emitted
    match system_rx.recv().await.unwrap() {
        HybridMessage::SystemEvent { kind, payload } => {
            assert_eq!(kind, "shutdown");
            assert_eq!(payload, "Graceful shutdown requested");
        }
        _ => panic!("Expected shutdown SystemEvent"),
    }

    // After shutdown, broadcasts still work but consumers should stop
    bridge.broadcast_snapshot(MatrixSnapshot {
        tick: 0, n_stocks: 0,
        eigenvalues: vec![], eigenvectors: ndarray::Array2::from_shape_vec((0, 0), vec![]).unwrap(),
        topologies: vec![], universe_betti: [0, 0, 0],
        regime: "shutdown".into(), condition_number: 0.0,
    });

    // Downstream should check `is_shutdown_requested()` and stop processing
    assert!(bridge.is_shutdown_requested());
}
```

---

## Example 13: Custom Veto Engine Implementation

```rust
use std::sync::Arc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::RwLock;
use async_trait::async_trait;
use hybrid_bridge::prelude::*;
use hybrid_bridge::engine::{VetoEngine, HybridEngine, HybridEngineImpl, HybridConfig, MatrixEngine};

/// A simple veto engine that only allows proposals from a whitelist.
struct WhitelistVeto {
    whitelist: Vec<String>,
    frozen: AtomicBool,
    freeze_reason: RwLock<Option<String>>,
    proposals_count: AtomicU64,
}

impl WhitelistVeto {
    fn new(whitelist: &[&str]) -> Self {
        Self {
            whitelist: whitelist.iter().map(|s| s.to_string()).collect(),
            frozen: AtomicBool::new(false),
            freeze_reason: RwLock::new(None),
            proposals_count: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl VetoEngine for WhitelistVeto {
    async fn register_constraint(&mut self, _constraint: SaepConstraint) {
        // Custom veto engines can ignore SAEP constraints or implement
        // their own constraint system
    }

    async fn resolve(
        &self,
        proposals: &[RoomProposal],
        _current: Option<&PortfolioVector>,
    ) -> PortfolioVector {
        if self.frozen.load(Ordering::Acquire) {
            return PortfolioVector {
                positions: vec![],
                gross_exposure: 0.0,
                net_exposure: 0.0,
                sector_concentrations: HashMap::new(),
                portfolio_var: 0.0,
                timestamp: 0,
            };
        }

        let mut positions = Vec::new();
        let mut gross = 0.0_f64;
        let mut net = 0.0_f64;

        for proposal in proposals {
            let is_whitelisted = self.whitelist.contains(&proposal.ticker);
            let veto_applied = if is_whitelisted {
                vec![]
            } else {
                vec!["not_whitelisted".to_string()]
            };
            let weight = if is_whitelisted {
                match proposal.gate {
                    TernaryGate::Bullish => proposal.conviction,
                    TernaryGate::Bearish => -proposal.conviction,
                    TernaryGate::Neutral => 0.0,
                }
            } else {
                0.0
            };

            gross += weight.abs();
            net += weight;

            positions.push(FinalPosition {
                ticker: proposal.ticker.clone(),
                weight,
                raw_gate: proposal.gate.clone(),
                veto_applied,
                veto_severity: if is_whitelisted { 0.0 } else { 1.0 },
            });

            self.proposals_count.fetch_add(1, Ordering::Relaxed);
        }

        PortfolioVector {
            positions,
            gross_exposure: gross,
            net_exposure: net,
            sector_concentrations: HashMap::new(),
            portfolio_var: 0.0,
            timestamp: proposals.first().map(|p| p.timestamp).unwrap_or(0),
        }
    }

    async fn get_portfolio(&self) -> PortfolioVector {
        PortfolioVector {
            positions: vec![],
            gross_exposure: 0.0,
            net_exposure: 0.0,
            sector_concentrations: HashMap::new(),
            portfolio_var: 0.0,
            timestamp: 0,
        }
    }

    async fn freeze(&mut self, reason: &str) {
        self.frozen.store(true, Ordering::Release);
        *self.freeze_reason.write().await = Some(reason.to_string());
    }

    async fn unfreeze(&mut self, reason: &str) {
        self.frozen.store(false, Ordering::Release);
        *self.freeze_reason.write().await = None;
    }
}

#[tokio::test]
async fn example_custom_veto() {
    let veto = WhitelistVeto::new(&["AAPL", "MSFT", "GOOGL"]);

    let proposals = vec![
        RoomProposal {
            ticker: "AAPL".into(), gate: TernaryGate::Bullish,
            conviction: 0.80, confidence: 0.90,
            narrative_sig: "a".into(), matrix_agreement: 0.70,
            veto_override: false, timestamp: 1,
        },
        RoomProposal {
            ticker: "SUSPICIOUS".into(), gate: TernaryGate::Bearish,
            conviction: 0.90, confidence: 0.95,
            narrative_sig: "b".into(), matrix_agreement: 0.10,
            veto_override: false, timestamp: 1,
        },
    ];

    let portfolio = veto.resolve(&proposals, None).await;

    // AAPL goes through
    let aapl = portfolio.positions.iter().find(|p| p.ticker == "AAPL").unwrap();
    assert!((aapl.weight - 0.80).abs() < 1e-6);
    assert!(aapl.veto_applied.is_empty());

    // SUSPICIOUS is vetoed
    let bad = portfolio.positions.iter().find(|p| p.ticker == "SUSPICIOUS").unwrap();
    assert!((bad.weight).abs() < 1e-10);
    assert!(bad.veto_applied.contains(&"not_whitelisted"));
}
```

---

## Running the Examples

All examples are written as `#[tokio::test]` or `#[test]` functions. Add them to your test file or run them with:

```bash
# Run all tests
cd hybrid-bridge
cargo test

# Run a specific example
cargo test example_bridge_creation -- --nocapture

# Run with logging
RUST_LOG=debug cargo test example_hybrid_cycle -- --nocapture
```

**Note:** Examples using `MockMatrixEngine` require the `mock_matrix` module which is a public module of the crate.

---

## Example Dependencies

```toml
[dependencies]
hybrid-bridge = { path = "../hybrid-bridge" }
tokio = { version = "1", features = ["full"] }
ndarray = "0.16"
async-trait = "0.1"
```
