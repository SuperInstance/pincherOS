# hybrid-bridge API Reference

> **The Hybrid Manifold communication backbone** — connects the Matrix Engine, Room Agents, and Veto Engine through high-performance async channels on ARM64.

**Version:** 0.1.0 · **Crate:** `hybrid_bridge`

---

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture Overview](#architecture-overview)
- [Module Reference](#module-reference)
- [Key Types](#key-types)
- [Usage Examples](#usage-examples)
- [Performance Targets](#performance-targets)
- [ARM64 Tuning Notes](#arm64-tuning-notes)

---

## Quick Start

Add the following to your `Cargo.toml`:

```toml
[dependencies]
hybrid-bridge = { path = "../hybrid-bridge" }
tokio = { version = "1", features = ["full"] }
ndarray = "0.16"
```

The crate depends on:

| Dependency | Version | Purpose |
|---|---|---|
| `pincher-core` | (workspace) | Core trait definitions & shared types |
| `ternary-types` | (workspace) | Ternary gate definitions |
| `ndarray` | 0.16 (serde) | N-dimensional array for feature tensors |
| `tokio` | (workspace) | Async runtime & channels |
| `arrow` | 53 (ipc) | Columnar data interchange |
| `serde` / `serde_json` | (workspace) | Serialization |
| `tracing` | (workspace) | Instrumentation & observability |
| `uuid` | 1 (v4, serde) | Unique identifiers |
| `chrono` | (workspace) | Timestamps |
| `thiserror` | (workspace) | Error derivation |

Import via the prelude:

```rust
use hybrid_bridge::prelude::*;
```

---

## Architecture Overview

The Hybrid Manifold processes data in a **four-phase pipeline**:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                            HybridEngine                                  │
│                                                                          │
│   MATRIX PHASE          ROOMS PHASE          VETO PHASE       EXECUTION │
│   ┌─────────────┐      ┌──────────────┐    ┌────────────┐   ┌─────────┐ │
│   │             │      │              │    │            │   │         │ │
│   │   Feature   │ ────►│  Room Agent  │───►│   Veto     │──►│Portfolio│ │
│   │   Tensor    │ snap │     #1       │  p  │  Engine    │ b │Broadcast│ │
│   │   X[n,m,h]  │ shot │              │ r   │  (SAEP     │ r │         │ │
│   │             │      ├──────────────┤ o   │  Constraint│ o │         │ │
│   │ Fast Cycle  │      │  Room Agent  │ p   │  Checker)  │ a │         │ │
│   │ (< 3ms)     │      │     #2       │ o   │            │ d │         │ │
│   │             │      ├──────────────┤ s   │            │ c │         │ │
│   │ Medium Cycle│      │  Room Agent  │ a   │  Resolves  │ a │         │ │
│   │ (~200-500ms)│      │     #N       │ l   │  N props   │ s │         │ │
│   │             │      │              │ s   │  into 1    │ t │         │ │
│   │  Full Cycle │      └──────────────┘    │  Portfolio │   │         │ │
│   │  (~2-5s)    │              ▲           └─────┬──────┘   └─────────┘ │
│   │             │              │                 │                      │
│   └─────────────┘              │                 │                      │
│         │                      │ feature         │                      │
│         │                      │ suggestions     │                      │
│         ▼                      ▼                 ▼                      │
│   ┌─────────────────────────────────────────────────────────┐           │
│   │                   HybridBridge                           │           │
│   │  ┌──────────┐  ┌──────────────┐  ┌──────┐  ┌─────────┐  │           │
│   │  │ matrix_tx│  │ proposal_tx  │  │ veto │  │system_tx│  │           │
│   │  │broadcast │  │ mpsc (8K)    │  │ tx   │  │ mpsc    │  │           │
│   │  │ (256)    │  │              │  │ bcast│  │ (64)    │  │           │
│   │  └──────────┘  └──────────────┘  └──────┘  └─────────┘  │           │
│   └─────────────────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────────────────────┘
```

### Channel Topology

| Channel | Type | Capacity | Direction |
|---|---|---|---|
| `matrix_tx` | `broadcast` | 256 | Matrix → All Rooms |
| `proposal_tx` | `mpsc` | 8,192 | All Rooms → Veto |
| `feature_tx` | `mpsc` | 1,024 | Rooms → Matrix |
| `veto_tx` | `broadcast` | 256 | Veto → All Subscribers |
| `system_tx` | `broadcast` | 64 | Any → All Subscribers |

- **broadcast** = fan-out to N subscribers (slow consumers miss messages)
- **mpsc** = multi-producer, single-consumer (backpressure via bounded channel)

### Data Flow

1. **Matrix Engine** ingests ticker data into the feature tensor `X[n_stocks, n_features, n_history]`
2. **Fast cycle** (every tick) produces `MatrixMetadata`; **medium cycle** (every 5 ticks) produces `PartialSnapshot`; **full cycle** (every 20 ticks) produces `MatrixSnapshot`
3. Matrix broadcasts the snapshot to all Room Agents via `HybridBridge`
4. Each **Room Agent** analyzes its ticker's slice and submits a `RoomProposal` (with `TernaryGate`)
5. **Veto Engine** collects all proposals, checks SAEP constraints, and outputs a `PortfolioVector`
6. Portfolio is broadcast to execution subscribers

---

## Module Reference

### `types` — Core data structures

**File:** `src/types.rs`

Defines all shared types flowing through the bridge:

- **Topology** — `TopologicalSignature`, `MatrixMetadata`, `PartialSnapshot`, `MatrixSnapshot`
- **Decisions** — `TernaryGate`, `RoomProposal`, `FeatureSuggestion`
- **Veto** — `PortfolioVector`, `FinalPosition`, `SaepConstraint`, `Violation`, `SaepAction`, `GovernanceLayer`
- **Messages** — `HybridMessage` (enum with 7 variants)
- **Utilities** — `detect_non_finite()`, `mask_non_finite()`

### `bridge` — Communication backbone

**File:** `src/bridge.rs`

`HybridBridge` owns all channels connecting the three layers. Thread-safe via `Arc<HybridBridge>`.

- `HybridBridge::new()` — default channel capacities
- `HybridBridge::with_capacities(...)` — custom channel sizes
- `subscribe_matrix()` — subscribe to snapshot broadcasts
- `subscribe_portfolio()` — subscribe to portfolio broadcasts
- `subscribe_system_events()` — subscribe to system events
- `broadcast_snapshot()` — Matrix sends snapshot to all rooms
- `submit_proposal()` — Room submits proposal to Veto (async)
- `try_submit_proposal()` — non-blocking proposal submission
- `submit_feature_suggestion()` — Room suggests feature to Matrix
- `broadcast_portfolio()` — Veto broadcasts portfolio to subscribers
- `emit_system_event()` — emit freeze/error/shutdown events
- `request_shutdown()` — request graceful shutdown
- `take_proposal_receiver()` — one-shot: get proposal mpsc receiver
- `take_feature_receiver()` — one-shot: get feature mpsc receiver
- `BridgeMetrics` / `BridgeMetricSnapshot` — atomic counters for observability

### `engine` — Hybrid Engine orchestrator

**File:** `src/engine.rs`

Defines the core traits and the concrete `HybridEngineImpl`:

- **Traits:**
  - `MatrixEngine` — feature tensor lifecycle (`ingest`, `fast_cycle`, `medium_cycle`, `full_cycle`, `get_slice`, `get_cross_section`, `add_ticker`, `remove_ticker`)
  - `RoomAgent` — analysis callback (`analyze`, `on_symmetry_alert`, `suggest_feature`, `set_regime`, `update_narrative`)
  - `VetoEngine` — SAEP constraint resolution (`register_constraint`, `resolve`, `get_portfolio`, `freeze`, `unfreeze`)
  - `HybridEngine` — top-level orchestrator (`hybrid_cycle`, `run`, `shutdown`)
- **Implementations:**
  - `HybridEngineImpl<M, V>` — concrete 4-phase cycle engine
  - `DefaultVetoEngine` — built-in SAEP constraint checker
- **Config:** `HybridConfig` (cycle intervals, concurrency, tracing)
- **Constants:** `FAST_CYCLE_INTERVAL` (1), `MEDIUM_CYCLE_INTERVAL` (5), `FULL_CYCLE_INTERVAL` (20), `MAX_CONCURRENT_ROOMS` (128)

### `chaos` — Chaos testing utilities

**File:** `src/chaos.rs`

Deliberately injects pathological data to verify robustness:

- `inject_nan_random()` — inject NaN at random tensor positions
- `inject_inf_random()` — inject Inf at random tensor positions
- `run_chaos_cycle()` — full inject → detect → mask → verify cycle
- `ChaosTestResult` — structured result with injected/detected/masked counts
- `SAFE_MODE_THRESHOLD` — 1 non-finite cell triggers safe mode

### `mock_matrix` — Mock Matrix Engine

**File:** `src/mock_matrix.rs`

In-memory `Array3<f32>`-backed `MatrixEngine` for testing:

- `MockMatrixEngine::new(n_stocks, n_features, n_history)`
- `.with_tickers(&[...])` — pre-register ticker names
- `.seed_random()` — populate tensor with random data in [-1, 1]
- `.inject_nan(stock, feature, time)` — inject NaN at specific coordinates
- `.inject_inf(stock, feature, time)` — inject Inf
- `.tensor()` — return `Arc<RwLock<Array3<f32>>>` for direct inspection
- Implements all `MatrixEngine` trait methods

**Note:** `mock_room` and `mock_veto` modules exist in `src/tests/` integration tests but are not currently compiled as public modules. Refer to `tests/integration_tests.rs` for mock room/veto patterns using the trait directly.

### `error` — Error types

**File:** `src/error.rs`

```rust
pub enum HybridError {
    TickerNotFound(String),
    DimensionMismatch { expected: usize, actual: usize },
    TdaError(String),
    StaleSnapshot { elapsed: u64 },
    Frozen { reason: String },
    ChannelClosed,
    Io(std::io::Error),
    Join(tokio::task::JoinError),
    Internal(String),
}
```

`HybridResult<T>` is a convenience alias for `Result<T, HybridError>`.

---

## Key Types

### `HybridBridge`

The central communication hub. `Send + Sync`. Share via `Arc<HybridBridge>`.

```rust
pub struct HybridBridge {
    matrix_tx: broadcast::Sender<HybridMessage>,
    proposal_tx: mpsc::Sender<RoomProposal>,
    feature_tx: mpsc::Sender<FeatureSuggestion>,
    veto_tx: broadcast::Sender<HybridMessage>,
    system_tx: broadcast::Sender<HybridMessage>,
    proposal_rx: AsyncMutex<Option<mpsc::Receiver<RoomProposal>>>,
    feature_rx: AsyncMutex<Option<mpsc::Receiver<FeatureSuggestion>>>,
    metrics: BridgeMetrics,
    shutdown_flag: Arc<AtomicBool>,
}
```

### `MatrixSnapshot`

The richest output from a full matrix cycle. Contains eigenvalues, eigenvectors, topological signatures, and regime label.

```rust
pub struct MatrixSnapshot {
    pub tick: u64,
    pub n_stocks: usize,
    pub eigenvalues: Vec<f64>,
    pub eigenvectors: Array2<f64>,
    pub topologies: Vec<TopologicalSignature>,
    pub universe_betti: [usize; 3],
    pub regime: String,
    pub condition_number: f64,
}
```

### `RoomProposal`

A room's decision — gate, conviction, and metadata — submitted to the Veto Engine.

```rust
pub struct RoomProposal {
    pub ticker: String,
    pub gate: TernaryGate,
    pub conviction: f64,        // [0.0, 1.0]
    pub confidence: f64,        // [0.0, 1.0]
    pub narrative_sig: String,  // hash of narrative (for audit)
    pub matrix_agreement: f64,  // [0, 1]
    pub veto_override: bool,    // agent flags skepticism
    pub timestamp: u64,
}
```

### `PortfolioVector`

The final portfolio after veto resolution. One `FinalPosition` per ticker.

```rust
pub struct PortfolioVector {
    pub positions: Vec<FinalPosition>,
    pub gross_exposure: f64,
    pub net_exposure: f64,
    pub sector_concentrations: HashMap<String, f64>,
    pub portfolio_var: f64,
    pub timestamp: u64,
}
```

### `TernaryGate`

The decision direction for a ticker.

```rust
pub enum TernaryGate {
    Bullish,  // +1
    Neutral,  //  0 (leminal zone)
    Bearish,  // -1
}
```

Methods: `to_i8()` returns `{-1, 0, 1}`.

---

## Usage Examples

### Creating a HybridBridge

```rust
use std::sync::Arc;
use hybrid_bridge::prelude::*;

let bridge = Arc::new(HybridBridge::new());

// With custom channel capacities:
let bridge = Arc::new(HybridBridge::with_capacities(
    512,    // matrix broadcast capacity
    512,    // veto broadcast capacity
    16384,  // proposal channel capacity
    2048,   // feature suggestion capacity
));
```

### Subscribing to Snapshots

```rust
let bridge = Arc::new(HybridBridge::new());

// Matrix engine side: broadcast
let snapshot = MatrixSnapshot {
    tick: 1,
    n_stocks: 3,
    eigenvalues: vec![0.95, 0.03, 0.02],
    eigenvectors: array![[0.7071, 0.7071], [0.7071, -0.7071]],
    topologies: vec![],
    universe_betti: [3, 1, 0],
    regime: "stable".into(),
    condition_number: 2.5,
};
bridge.broadcast_snapshot(snapshot);

// Room agent side: subscribe
let mut rx = bridge.subscribe_matrix();
tokio::spawn(async move {
    while let Ok(msg) = rx.recv().await {
        if let HybridMessage::SnapshotBroadcast(snap) = msg {
            println!("Tick {}: regime = {}", snap.tick, snap.regime);
        }
    }
});
```

### Submitting a Proposal

```rust
let bridge = Arc::new(HybridBridge::new());
let proposal = RoomProposal {
    ticker: "AAPL".into(),
    gate: TernaryGate::Bullish,
    conviction: 0.85,
    confidence: 0.72,
    narrative_sig: "abc123".into(),
    matrix_agreement: 0.91,
    veto_override: false,
    timestamp: 1000,
};

// Async submission (preferred)
bridge.submit_proposal(proposal).await.unwrap();

// Or non-blocking:
bridge.try_submit_proposal(proposal).unwrap();
```

### Running a Hybrid Cycle

```rust
use hybrid_bridge::prelude::*;
use hybrid_bridge::engine::DefaultVetoEngine;

// Create components
let matrix = MockMatrixEngine::new(100, 10, 250)
    .with_tickers(&tickers);
let bridge = Arc::new(HybridBridge::new());
let veto = DefaultVetoEngine::new();
let rooms: Vec<Box<dyn RoomAgent>> = /* create room agents */;

let config = HybridConfig::default();
let engine = HybridEngineImpl::new(
    matrix, bridge, veto, rooms, config,
);

// Run one cycle
engine.hybrid_cycle(1).await;

// Run the event loop (runs forever until shutdown)
engine.run().await;

// Graceful shutdown
engine.shutdown().await;
```

### Injecting Chaos for Testing

```rust
use ndarray::Array3;
use hybrid_bridge::chaos::{run_chaos_cycle, inject_nan_random, inject_inf_random};

let mut tensor = Array3::<f32>::zeros((10, 5, 100));

// Inject NaNs
inject_nan_random(&mut tensor, 5);

// Inject Infs
inject_inf_random(&mut tensor, 3);

// Full chaos cycle: inject → detect → mask → verify
let result = run_chaos_cycle(&mut tensor, 10);
assert!(result.failures.is_empty());
assert_eq!(result.detected, result.masked);  // cleanup worked
```

---

## Performance Targets

All targets measured on ARM64 (Apple M-series or equivalent) with default channel capacities and 5,000 stocks.

| Operation | Target | Measurement |
|---|---|---|
| Matrix fast cycle | **< 3 ms** | Pure tensor ingest + simple statistics |
| Room analysis (per agent) | **< 100 ms** | Slice extraction + ternary gate computation |
| Veto resolution (5,000 rooms) | **< 10 ms** | SAEP constraint iteration + aggregation |
| Medium matrix cycle | **200–500 ms** | Correlation matrix + streaming PCA |
| Full matrix cycle (slow path) | **2–5 s** | Eigendecomposition + TDA (topological data analysis) |
| End-to-end hybrid cycle | **< 1 s** | Fast path: Matrix → Rooms → Veto → Execution |
| Channel send (broadcast) | **< 1 µs** | `broadcast::Sender::send()` hot path |
| Channel send (mpsc) | **< 500 ns** | `mpsc::Sender::try_send()` contention-free |
| Bridge creation | **< 50 µs** | Channel allocation + startup |

### Scaling Estimates

| Stocks | Fast Cycle | Room Phase (128 concurrent) | Veto | E2E |
|---|---|---|---|---|
| 100 | < 100 µs | < 10 ms | < 1 ms | < 50 ms |
| 1,000 | < 500 µs | < 50 ms | < 3 ms | < 200 ms |
| 5,000 | < 3 ms | < 200 ms | < 10 ms | < 1 s |
| 10,000 | < 6 ms | < 500 ms | < 20 ms | < 2 s |

### Channel Backpressure Limits

The mpsc channels provide backpressure. At 8,192 proposal capacity, a fully blocked Veto Engine can handle ~82 seconds of proposals at 100 proposals/tick before `try_submit_proposal` returns `Full`. For production, ensure the Veto Engine keeps up with the room analysis cadence.

---

## ARM64 Tuning Notes

### Platform Specifics

The crate is designed and benchmarked for **ARM64** (Apple M1/M2/M3/M4, AWS Graviton, Ampere Altra).

### Cache-Line Aware Channel Sizing

Channel capacities are chosen to fit within L2 cache (typically 16–32 MB on Apple M-series):

| Channel | Capacity | Est. Memory |
|---|---|---|
| `matrix_tx` | 256 | ~64 KB per message size |
| `proposal_tx` | 8,192 | ~2 MB peak |
| `feature_tx` | 1,024 | ~256 KB |
| `veto_tx` | 256 | ~64 KB |
| `system_tx` | 64 | ~16 KB |

### SIMD Considerations

- `ndarray` uses the `sleef-sys` backend on ARM64 for vectorized array operations
- The `mask_non_finite` function is a scalar loop — SIMD acceleration is a future optimization
- `detect_non_finite` iterates 3D tensors with contiguous inner dimension for cache locality (`(stock, feature, time)` where time is fastest-varying)

### CPU Core Allocation Recommendation

| Role | Cores | Priority |
|---|---|---|
| Matrix Engine | 2–4 | High (compute-bound: TDA, PCA) |
| Veto Engine | 1–2 | High (sequential aggregation) |
| Room Agents | 4–8 (total) | Medium (N agents share via semaphore) |
| Bridge I/O | 1 | Low (channel dispatching) |

### NUMA / Topology

For socketed ARM64 systems (e.g., Ampere Altra with multiple dies):

- Keep `HybridBridge` channels shared — they use lock-free primitives internally
- Room analysis can be pinned to separate cores via `std::thread::available_parallelism()` for work-stealing
- The `room_semaphore` in `HybridEngineImpl` prevents oversubscription on any topology

### Tokio Runtime Configuration

Recommended runtime setup:

```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    // ...
}
```

- Fast-path cycles (< 1ms) can run on single-threaded runtime
- Full cycles (TDA, 2–5s) benefit from dedicated worker threads via `tokio::task::spawn_blocking`

---

## Error Handling

All fallible bridge operations return `HybridResult<T>`:

- **Channel operations** — `HybridError::ChannelClosed` when a sender's receiver is dropped
- **Receiver conflicts** — `HybridError::Internal("Proposal receiver already taken")` if `take_proposal_receiver()` is called twice
- **Backpressure** — `HybridError::Internal("Proposal channel full")` from `try_submit_proposal` when mpsc buffer is exhausted
- **Frozen veto** — `HybridError::Frozen { reason }` if the veto engine is in freeze state

---

## Safety

- **No `unsafe` code** — all channels are safe Rust
- **No locking** in the hot path — `HybridBridge` uses lock-free `broadcast` and `mpsc` channels
- **NaN/Inf guard** — `detect_non_finite` / `mask_non_finite` utilities check tensor health; safe mode triggers at ≥1 non-finite cell
- **All types** implement `Send + Sync`
