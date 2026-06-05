# Your First Ternary Application

*A developer guide to building a confidence-aware access control system using Pincher's reflex engine and ternary voting.*

---

## The Idea

Most applications treat authentication and authorization as binary gates: you're either "in" or "out."

But real access control is rarely binary. A session token might be valid but expired. A user might have partial permissions. An admin might need to approve a one-off escalation.

What if your authorization system could *express degrees of confidence*?

- ✅ **Allow**: The primary rule matches. Full confidence.
- ⏳ **Pending**: Multiple rules conflict. Escalate for confirmation.
- ❌ **Deny**: No rules match. Reject.

This is a ternary state machine. And Pincher's reflex engine is built for exactly this.

---

## Step 1: Define Your Ternary Decision

```rust
use pincher_core::route::RouteState;

/// An access request with some context
struct AccessRequest {
    user_role: String,
    resource: String,
    action: String,
    time_of_day: u8,
}

/// The result of an authorization check
#[derive(Debug)]
struct AccessDecision {
    route: RouteState,  // Positive = Allow, Neutral = Pending, Negative = Deny
    reason: String,
    confidence: f64,
}
```

---

## Step 2: Build a Ternary-weighted Policy Graph

Pincher's route module lets you build a directed graph where edges have ternary weights:

```rust
use pincher_core::route::TernaryRouteGraph;
use ternary_types::Ternary;

let mut graph = TernaryRouteGraph::new();

// A strong match: Positive edge
graph.add_edge(
    "admin_can_delete",
    Role::Admin,
    Ternary::Positive,
);

// A weak match: check further
graph.add_edge(
    "editor_can_delete",
    Role::Editor,
    Ternary::Neutral,
);

// A definite mismatch
graph.add_edge(
    "viewer_can_delete",
    Role::Viewer,
    Ternary::Negative,
);
```

The route module explores this graph using the same conservation law we discussed — the sum of all active edges must remain balanced.

---

## Step 3: The Reflex Engine Evaluates

When a request comes in, the reflex engine:

1. **Embeds** the request into a feature vector
2. **Matches** against known policy reflexes using cosine similarity
3. **Returns ternary-weighted candidates**

```rust
fn check_access(engine: &ReflexEngine, request: &AccessRequest) -> AccessDecision {
    // The engine returns a ternary-weighted result
    let result = engine.evaluate(&[
        ("role", request.user_role.as_str()),
        ("resource", request.resource.as_str()),
        ("action", request.action.as_str()),
    ]);
    
    match result {
        RouteState::Allow => AccessDecision {
            route: RouteState::Positive,
            reason: "Explicit rule matched".into(),
            confidence: 0.92,
        },
        RouteState::Pending => AccessDecision {
            route: RouteState::Neutral,
            reason: "Partial match — confirm with admin".into(),
            confidence: 0.55,
        },
        RouteState::Deny => AccessDecision {
            route: RouteState::Negative,
            reason: "No matching policy".into(),
            confidence: 0.98,
        },
    }
}
```

---

## Step 4: The "a-ha" Moment — Composing Decisions

Here's where ternary logic shines. You can *compose* multiple access decisions:

```rust
use ternary_types::{Ternary, TernaryOps};

fn compose_access(decisions: &[AccessDecision]) -> AccessDecision {
    // Multiple auth checks combine naturally
    let ternary_sum: Ternary = decisions
        .iter()
        .map(|d| d.route)
        .fold(Ternary::Neutral, |acc, r| acc + r);
    
    // Average confidence
    let avg_confidence: f64 = decisions
        .iter()
        .map(|d| d.confidence)
        .sum::<f64>() / decisions.len() as f64;
    
    AccessDecision {
        route: ternary_sum,
        reason: format!("Composed from {} checks", decisions.len()),
        confidence: avg_confidence,
    }
}
```

Because `Ternary` supports `+`, `-`, and `*` operations (through the `TernaryOps` trait), you can combine decisions algebraically — something impossible with booleans or plain enums.

---

## Step 5: Run It in a Sandbox

Pincher wraps every reflex execution in a **bubblewrap sandbox**, governed by the **Veto Engine**:

```rust
use pincher_core::security::VetoDecision;

fn run_with_veto(engine: &ReflexEngine, request: &AccessRequest) -> Result<(), String> {
    let decision = check_access(engine, request);
    
    // The veto engine checks ternary-weighted rules
    match engine.veto(decision.route) {
        VetoDecision::Allow => {
            println!("Granted: {} — {}", decision.reason, decision.confidence);
            Ok(())
        }
        VetoDecision::RequireConfirmation => {
            println!("Pending: {}. Admin approval needed.", decision.reason);
            Err("Requires escalation".into())
        }
        VetoDecision::Deny => {
            println!("Denied: {}", decision.reason);
            Err("Access denied".into())
        }
    }
}
```

---

## The Big Picture

You just built an access control system that:
- **Expresses three states** (not two)
- **Composes decisions algebraically** (not with if-else spaghetti)
- **Has a conservation invariant** (total system state is trackable)
- **Runs in a sandbox with veto power** (safe by default)

This pattern generalizes beyond auth. The same ternary-weighted reflex engine handles routing, budgeting, scheduling, and any system where "yes/no" is too coarse.

**Where will you apply ternary next?**
