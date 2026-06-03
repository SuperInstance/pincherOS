# PincherOS Biological Formalism
## Coenobita clypeatus as Architectural Specification

> *"The hermit crab does not merely occupy a shell. It negotiates, remodels, signals through, hydrate within, and ultimately transforms both the shell and itself. The shell is not a container — it is a relational boundary."*

---

# 0. PREAMBLE: WHY BIOLOGY BEATS METAPHOR

The current PincherOS architecture treats the hermit crab as a **metaphor**: shell = hardware, crab = agent, migration = shell swap. This is correct at the surface level and **catastrophically incomplete** at the systems level.

Real hermit crabs (*Coenobita clypeatus*, the Caribbean purple pincher) have evolved over ~200 million years to solve exactly the class of problems PincherOS faces: **resource-constrained autonomous agents that must survive across heterogeneous, unpredictable environments by negotiating with semi-mutable physical substrates.**

The metaphor tells you *what* (crab moves to new shell). The biology tells you *how, why, when, under what constraints, with what failure modes, and with what emergent collective behaviors*. This document upgrades the metaphor to a **formalism** — a biologically-grounded systems theory that generates concrete engineering specifications.

**References**: The primary biological literature draws on Reese (1969) on shell selection behavior, Chase et al. (1988) on vacancy chains, Rotjan et al. (2010) on social shell exchange, Hazlett (1981) on shell fighting, Greenaway (2003) on terrestrial adaptation and osmoregulation, and de Wilde (1973) on *Coenobita* ecology in Curaçao. All mappings to PincherOS are original.

---

# 1. VACANCY CHAINS: CASCADING RESOURCE ALLOCATION

## 1.1 The Biology

When a *C. clypeatus* individual encounters a superior shell (larger internal volume, undamaged aperture, appropriate weight-to-strength ratio), it does not simply move in. It initiates a **vacancy chain** — a serial displacement event where:

1. Crab A occupies Shell α
2. Crab A finds Shell β (superior)
3. Crab A moves to Shell β, abandoning Shell α
4. Crab B (smaller than A) immediately occupies Shell α, abandoning Shell γ
5. Crab C (smaller than B) occupies Shell γ, abandoning Shell δ
6. The cascade continues until the smallest crab either stays put or finds a truly empty shell

**Critical research finding** (Chase et al., 1988; Rotjan et al., 2010): Vacancy chains can involve **5-20+ individuals** and propagate in under 30 minutes. The cascade is **self-organizing** — no crab coordinates the chain. Each crab simply responds to local resource availability (the abandoned shell). The aggregate effect is that **one high-quality shell entering the population upgrades the entire size distribution**.

**Key biological constraints on vacancy chains**:
- Chains only cascade *downward* in size — a small crab's abandoned shell is never occupied by a larger crab
- The cascade speed is limited by **assessment time** — each crab must physically explore the shell (insert chelipeds, rotate within the aperture) before committing (Reese, 1969)
- Chains terminate when the marginal improvement in shell quality falls below the **assessment cost** (energy expenditure + predation risk during shell-switching)

## 1.2 The PincherOS Mapping

The current `pincher pack` / `pincher unpack` migration is **atomic and isolated** — one rigging, one shell, one swap. Vacancy chains reveal that the correct model is **cascading reallocation across a population of agents**.

### Engineering Specification: `VacancyChain` Protocol

```rust
// src/shell/vacancy.rs

/// A vacancy chain is triggered when an agent migrates UP to a larger shell,
/// making its previous shell available. The chain cascades DOWN.
pub struct VacancyChain {
    /// The triggering event: which agent, from which shell, to which shell
    trigger: MigrationEvent,
    /// Ordered chain of cascading moves (largest → smallest)
    links: Vec<ChainLink>,
    /// Maximum allowed cascade depth (prevents infinite chains)
    max_depth: usize,
    /// Timeout for the entire chain to resolve
    cascade_timeout_ms: u64,
}

pub struct ChainLink {
    agent_id: String,           // The rigging that will move
    from_shell: ShellFingerprint,
    to_shell: ShellFingerprint,  // The shell being vacated by the previous link
    fit_score: f64,             // How well this agent fits the new shell (0.0–1.0)
}

pub struct MigrationEvent {
    agent_id: String,
    old_shell: ShellFingerprint,
    new_shell: ShellFingerprint,
    old_shell_capabilities: Capabilities,  // What's being vacated
    timestamp: i64,
}

/// The VacancyChain resolver runs when a rigging migrates UP
pub async fn resolve_vacancy_chain(
    trigger: MigrationEvent,
    shell_registry: &ShellRegistry,
    agent_registry: &AgentRegistry,
) -> Result<Vec<ChainLink>, ChainError> {
    let mut chain = Vec::new();
    let mut vacated = trigger.old_shell_capabilities.clone();
    let mut depth = 0;
    
    // Find the best-fit agent for the vacated shell
    // "Best fit" = smallest agent that would UPGRADE by moving here
    while depth < MAX_CASCADE_DEPTH {
        let candidates = agent_registry
            .find_agents_in_smaller_shells(&vacated)
            .await?;
        
        // Assessment: each candidate evaluates fit
        // Analogous to the crab's physical exploration of the shell
        let best = candidates
            .iter()
            .filter(|a| a.assess_fit(&vacated).marginal_improvement > ASSESSMENT_THRESHOLD)
            .max_by(|a, b| {
                a.assess_fit(&vacated).marginal_improvement
                    .partial_cmp(&b.assess_fit(&vacated).marginal_improvement)
                    .unwrap()
            });
        
        match best {
            Some(agent) => {
                let link = ChainLink {
                    agent_id: agent.id.clone(),
                    from_shell: agent.current_shell.clone(),
                    to_shell: trigger.old_shell.clone(),
                    fit_score: agent.assess_fit(&vacated).score,
                };
                // This agent's shell becomes the next vacated shell
                vacated = agent.current_capabilities.clone();
                chain.push(link);
                depth += 1;
            }
            None => break, // Chain terminates: no agent benefits from moving
        }
    }
    
    Ok(chain)
}
```

### What This Changes Architecturally

| Current Model | Vacancy Chain Model |
|---------------|---------------------|
| Agent migrates → old shell is idle | Agent migrates → old shell cascades to next agent |
| Static provisioning: each agent gets a fixed shell | Dynamic provisioning: shells flow toward agents that need them |
| Migration is a single event | Migration is a **population-level cascade** |
| Shells sit idle when abandoned | **No idle shells** — every vacated resource is immediately reclaimed |

### Concrete Impact on PincherOS

1. **Shell Registry**: The `shells` table must track `current_agent_id` AND `pending_cascade_agent_id`. A shell is never "unoccupied" for more than the cascade resolution time.

2. **Fit Score**: The `Snap` algorithm currently computes `PERFECT_FIT / TIGHT_FIT / STRESSED / OVERFLOW`. For vacancy chains, it must also compute **marginal improvement** — how much better would an agent be in this shell vs. its current one? This is the `assess_fit()` function above.

3. **Assessment Cost**: Crabs don't evaluate every shell — they evaluate until the cost of assessment exceeds expected improvement. PincherOS agents should **sample** shells (run a lightweight benchmark) rather than fully migrating to test fit. This maps to a `probe_shell()` RPC that returns fit metrics without full migration.

4. **Cascade Depth Limit**: Biology limits cascade depth by assessment cost. PincherOS should limit by **cascade timeout** — if the chain hasn't resolved in N seconds, remaining agents stay put. This prevents cascading failures during network partitions.

### The Big Insight

**Vacancy chains are distributed garbage collection for shells.** In GC terms, when a resource is freed (shell vacated), the collector (vacancy chain resolver) immediately promotes objects (agents) into the freed resource if they fit. The "generation" analogy is exact: larger crabs = older generation, smaller crabs = younger generation. The cascade is **generational promotion**.

---

# 2. SHELL REMODELING: BOUNDARY MUTATION

## 2.1 The Biology

*C. clypeatus* does not passively inhabit gastropod shells. It **remodels them** through four documented behaviors:

1. **Internal spiral abrasion**: The crab uses its chelipeds and maxillipeds to scrape the inner columella (central spiral) of the shell, thinning it and increasing internal volume by 15-30% (observed in *C. compressus* by Morrison, 2002; similar behavior in *C. clypeatus*)

2. **Aperture filing**: The crab grinds the shell aperture (opening) to widen it, allowing faster entry/exit and reducing the time it's vulnerable during emergence

3. **Weight reduction**: By thinning the shell walls, the crab reduces the shell's weight-to-volume ratio, lowering the metabolic cost of locomotion by up to 20% (Laidre, 2011)

4. **Surface etching**: Chemical etching of the shell interior using acidic secretions from the crab's exocrine glands, creating a rougher surface that improves grip for the crab's modified pleopods

**Critical biological insight**: The remodeled shell becomes **specific to the individual crab**. It is no longer a generic gastropod shell — it is a crab-shell composite. Other crabs that subsequently occupy this shell must either accept the modifications (which may not fit their body shape) or invest energy in re-remodeling.

## 2.2 The PincherOS Mapping

The current architecture treats shells as **immutable** — the `ShellProfile` is discovered via `snap()` and never modified. This is wrong. The rigging (crab) should **remodel the shell** to better fit its needs.

### Engineering Specification: Shell Boundary Mutation

```rust
// src/shell/remodel.rs

/// A ShellMutation represents a modification the rigging makes to its shell.
/// Mutations are tracked, reversible, and shell-specific.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellMutation {
    /// Dynamic overclocking: increase CPU frequency for compute-bound workloads
    /// Analogous to aperture filing — widening the "throughput" of the shell
    CpuOverclock { target_mhz: u64, duration_secs: u64 },
    
    /// RAM disk expansion: allocate a portion of disk as swap/RAM extension
    /// Analogous to internal spiral abrasion — creating more internal volume
    RamDiskExpansion { size_mb: u64, backing_path: String },
    
    /// GPU memory partitioning: carve out VRAM for specific workloads
    /// Analogous to surface etching — customizing the grip surface
    GpuMemoryPartition { 
        reserved_mb: u64, 
        purpose: GpuPartitionPurpose 
    },
    
    /// Thermal throttling override: adjust thermal limits for sustained workloads
    /// Analogous to weight reduction — trading protection for performance
    ThermalOverride { 
        max_temp_c: f64,  // Raise thermal ceiling
        risk_level: ThermalRisk,  // LOW / MEDIUM / HIGH
    },
    
    /// I/O priority elevation: boost I/O scheduling priority for storage-bound workloads
    /// Analogous to aperture widening — faster entry/exit
    IoPriorityBoost { 
        device: String, 
        priority: IoPriority 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuPartitionPurpose {
    InferenceOffload,    // Reserve VRAM for LLM inference layers
    EmbeddingCache,      // Reserve VRAM for hot embedding vectors
    ComputeKernel,       // Reserve VRAM for cudaclaw custom kernels
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThermalRisk {
    Low,     // Within manufacturer specs, just aggressive
    Medium,  // Approaching limits, monitor closely
    High,    // Beyond specs, potential hardware damage
}

/// The RemodelingEngine evaluates and applies shell mutations.
/// Each mutation has a COST (risk, energy) and a BENEFIT (performance).
pub struct RemodelingEngine {
    mutations: Vec<AppliedMutation>,
    total_energy_budget: f64,  // Total allowed "remodeling energy" per session
    energy_spent: f64,
}

pub struct AppliedMutation {
    mutation: ShellMutation,
    applied_at: i64,
    /// The measured benefit of this mutation (performance improvement)
    observed_benefit: f64,
    /// The cost of this mutation (thermal wear, power consumption, risk)
    observed_cost: f64,
    /// Whether this mutation should be preserved across migrations
    /// (like a crab's specific remodeling of a shell)
    sticky: bool,
}

impl RemodelingEngine {
    /// Evaluate whether a proposed mutation is worth the cost.
    /// This is the crab's "assessment" — is the remodeling effort justified?
    pub fn evaluate(&self, proposed: &ShellMutation, shell: &ShellProfile) -> MutationVerdict {
        let benefit = self.estimate_benefit(proposed, shell);
        let cost = self.estimate_cost(proposed, shell);
        
        if benefit / cost > REMODELING_BENEFIT_THRESHOLD && 
           self.energy_spent + cost <= self.total_energy_budget {
            MutationVerdict::Approve { benefit, cost }
        } else {
            MutationVerdict::Reject { reason: format!(
                "Benefit/cost ratio {:.2} below threshold, or energy budget exhausted",
                benefit / cost
            )}
        }
    }
    
    /// When a rigging migrates away, its sticky mutations are serialized
    /// into the shell profile. The next rigging must decide:
    /// accept the modifications, or revert them.
    pub fn serialize_mutations(&self) -> Vec<SerializedMutation> {
        self.mutations.iter()
            .filter(|m| m.sticky)
            .map(|m| SerializedMutation {
                mutation: m.mutation.clone(),
                observed_benefit: m.observed_benefit,
                observed_cost: m.observed_cost,
                remodeled_by: "previous_rigging".to_string(),
            })
            .collect()
    }
}
```

### What This Changes Architecturally

| Current Model | Remodeling Model |
|---------------|------------------|
| Shell capabilities are discovered, never modified | Shell capabilities are **discovered then mutated** |
| `ShellProfile` is read-only | `ShellProfile` has a `mutations: Vec<AppliedMutation>` field |
| Every rigging sees the same shell | Each rigging sees a **personalized shell** — the crab-shell composite |
| Migration is stateless for the shell | Migration leaves **remodeling artifacts** on the shell |

### Concrete Hardware Mappings

| Crab Behavior | Hardware Analog | PincherOS Mutation | Risk |
|---------------|----------------|---------------------|------|
| Internal spiral abrasion (more room) | RAM disk / swap expansion | `RamDiskExpansion` | Disk wear, reduced storage |
| Aperture filing (faster entry/exit) | I/O priority boost, overclocking | `CpuOverclock`, `IoPriorityBoost` | Thermal throttling, instability |
| Weight reduction (less metabolic cost) | Disable unused peripherals/hardware | `DisablePeripheral { device }` | Loss of capability |
| Surface etching (better grip) | GPU memory partitioning | `GpuMemoryPartition` | Reduced GPU availability for other tasks |

### The Big Insight

**Shell remodeling is the missing link between "immutable infrastructure" and "agent-personalized environments."** Kubernetes pods are immutable — you don't modify the node. But hermit crabs prove that the optimal strategy is **mutable at the boundary, immutable at the core**: the crab never changes the shell's species (it's always a *Strombus gigas* or *Turbo castanea*), but it absolutely changes the shell's internal parameters. PincherOS should treat hardware the same way: the device *type* (Pi 4, Jetson Nano) is immutable, but the *operating parameters* (clock speed, memory allocation, I/O priority) are **rigging-specific mutations**.

This also explains why `.nail` files should carry **mutation records**: when a rigging unpacks onto a new shell, it can replay its preferred mutations (if the shell supports them) or fall back to defaults.

---

# 3. SOCIAL SHELL EXCHANGE: BATCH MIGRATION

## 3.1 The Biology

*C. clypeatus* aggregates in groups of 10-100+ individuals, forming what researchers call **sync queues** or **shell exchange groups** (Rotjan et al., 2010; Hazlett, 1981). The process:

1. A group of crabs discovers a new resource (e.g., a pile of empty shells after a storm)
2. Crabs line up **by size** — largest first, then descending
3. The largest crab inspects the best available shell and, if acceptable, vacates its current shell
4. The next-largest crab moves into the shell just vacated
5. The process cascades down the size hierarchy

**Critical finding** (Rotjan et al., 2010): When crabs form sync queues, **every participant ends up in a better shell**. The exchange is **Pareto-improving** — no crab is worse off, and most are significantly better off. This is fundamentally different from competitive shell fighting (where one crab gains at another's expense).

**Biological constraints**:
- Sync queues require **physical co-presence** — crabs must be within antennule detection range (~1 meter)
- The queue must be **ordered by size** — if a medium crab goes before a large one, the cascade breaks
- Exchange speed is proportional to **trust signals** — crabs that have previously exchanged shells successfully form queues faster
- The entire exchange is **synchronous** — crabs don't move until the crab ahead of them has committed

## 3.2 The PincherOS Mapping

Individual migration (`pincher pack` / `pincher unpack`) is the isolated crab model. Sync queues are **coordinated batch migration** — multiple agents migrating simultaneously so that everyone benefits.

### Engineering Specification: Sync Queue Protocol

```rust
// src/shell/sync_queue.rs

/// A SyncQueue coordinates simultaneous shell exchange among multiple agents.
/// The queue is ordered by "size" (resource demand), largest first.
pub struct SyncQueue {
    /// Ordered list of participants, largest agent first
    participants: Vec<QueueParticipant>,
    /// The new resource that triggered the queue formation
    trigger_resource: ShellFingerprint,
    /// State machine for the queue
    state: QueueState,
    /// Timeout for each phase
    phase_timeout_ms: u64,
}

pub struct QueueParticipant {
    agent_id: String,
    current_shell: ShellFingerprint,
    target_shell: ShellFingerprint,  // Will be determined during resolution
    resource_demand: ResourceDemand,  // "Size" of the agent
    trust_score: f64,  // Historical reliability in sync queues
}

#[derive(PartialEq)]
pub enum QueueState {
    /// Agents are registering for the exchange
    Forming,
    /// Queue is ordered, waiting for all agents to confirm readiness
    AwaitingCommit,
    /// All agents committed, executing simultaneous migration
    Executing,
    /// Exchange complete
    Complete,
    /// Exchange failed — all agents revert
    Aborted,
}

impl SyncQueue {
    /// Phase 1: FORMING
    /// Agents discover a new shell resource and register for exchange.
    /// "Co-presence" = agents on the same network segment that can see the resource.
    pub async fn register(&mut self, agent: QueueParticipant) -> Result<(), QueueError> {
        if self.state != QueueState::Forming {
            return Err(QueueError::NotForming);
        }
        
        // Insert in sorted order by resource_demand (largest first)
        let pos = self.participants.partition_point(|p| {
            p.resource_demand.total() > agent.resource_demand.total()
        });
        self.participants.insert(pos, agent);
        Ok(())
    }
    
    /// Phase 2: RESOLVE
    /// Determine the cascade: who moves where.
    /// The trigger resource goes to the largest agent.
    /// Each subsequent agent gets the shell vacated by the previous one.
    pub fn resolve(&mut self) -> Result<&[QueueParticipant], QueueError> {
        if self.participants.is_empty() {
            return Err(QueueError::EmptyQueue);
        }
        
        // Largest agent gets the new resource
        self.participants[0].target_shell = self.trigger_resource.clone();
        
        // Cascade: each subsequent agent gets the previous agent's old shell
        for i in 1..self.participants.len() {
            self.participants[i].target_shell = 
                self.participants[i - 1].current_shell.clone();
        }
        
        // Validate: every agent must show marginal improvement
        for participant in &self.participants {
            let improvement = Self::marginal_improvement(
                &participant.current_shell,
                &participant.target_shell,
                &participant.resource_demand,
            );
            if improvement < SYNC_QUEUE_MIN_IMPROVEMENT {
                // This agent doesn't benefit — truncate the queue here
                // This is the biological analog: the cascade terminates when
                // assessment cost exceeds expected improvement
                self.participants.truncate(self.participants.iter().position(|p| p.agent_id == participant.agent_id).unwrap());
                break;
            }
        }
        
        self.state = QueueState::AwaitingCommit;
        Ok(&self.participants)
    }
    
    /// Phase 3: TWO-PHASE COMMIT
    /// All agents must confirm readiness. If ANY agent fails to commit,
    /// the entire exchange aborts. This is the synchronous constraint.
    pub async fn commit(&mut self) -> Result<(), QueueError> {
        self.state = QueueState::Executing;
        
        // Two-phase commit: prepare then execute
        // Phase 1: All agents prepare (snapshot state, validate target shell)
        let mut all_prepared = true;
        for participant in &mut self.participants {
            match prepare_migration(&participant.agent_id, &participant.target_shell).await {
                Ok(_) => participant.prepared = true,
                Err(_) => {
                    all_prepared = false;
                    break;
                }
            }
        }
        
        if !all_prepared {
            // ABORT: revert all preparations
            for participant in &self.participants {
                if participant.prepared {
                    abort_migration(&participant.agent_id).await.ok();
                }
            }
            self.state = QueueState::Aborted;
            return Err(QueueError::CommitFailed);
        }
        
        // Phase 2: Execute all migrations simultaneously
        // This is the biological analog: all crabs swap at once
        let mut results = Vec::new();
        for participant in &self.participants {
            results.push(execute_migration(&participant.agent_id, &participant.target_shell).await);
        }
        
        // If any migration failed, trigger rollback
        if results.iter().any(|r| r.is_err()) {
            // Rollback strategy depends on cascade depth
            // In biology: crabs that already swapped stay in new shells
            // In PincherOS: we can be more precise with rollback
            self.state = QueueState::Aborted;
            return Err(QueueError::PartialFailure);
        }
        
        self.state = QueueState::Complete;
        Ok(())
    }
    
    fn marginal_improvement(
        current: &ShellFingerprint, 
        target: &ShellFingerprint,
        demand: &ResourceDemand,
    ) -> f64 {
        // How much better does this agent fit the target vs. current?
        // This is the crab's "assessment" computation
        let current_fit = compute_fit_score(current, demand);
        let target_fit = compute_fit_score(target, demand);
        target_fit - current_fit
    }
}
```

### What This Changes Architecturally

| Current Model | Sync Queue Model |
|---------------|------------------|
| One agent migrates at a time | Multiple agents migrate **simultaneously** |
| Migration is competitive (first-come, first-served) | Migration is **cooperative** (Pareto-improving) |
| Shells sit idle between migrations | **Zero idle time** — shells flow immediately to next agent |
| No coordination between agents | **Two-phase commit** ensures atomic swap |

### The Big Insight

**Sync queues are distributed consensus for resource reallocation.** They're a biologically-evolved two-phase commit protocol. The "size ordering" is a total order on the commit participants (analogous to a total order broadcast in distributed systems). The "synchronous swap" is the commit phase. The "trust score" is a reputation system that determines queue priority.

For PincherOS specifically: when a new GPU workstation joins the fleet, it shouldn't just sit there waiting for one agent to discover it. It should **trigger a vacancy chain / sync queue** that cascades improvements across the entire agent population. The new workstation → largest agent gets it → its old GPU → next agent → its old CPU-only box → next agent → etc.

---

# 4. SHELL-WEARING AS SIGNALING: SERVICE LEVEL INDICATORS

## 4.1 The Biology

The type of shell a *C. clypeatus* wears signals information to other crabs:

- **Shell species**: A crab in a *Strombus gigas* (queen conch) shell signals large body size and high resource access. A crab in a *Turbo castanea* shell signals moderate size. A crab in a *Nassarius* shell signals small size.

- **Shell condition**: A well-maintained shell (clean, intact aperture) signals health and recent molting success. A damaged or fouled shell signals stress or recent predation attempts.

- **Shell fit**: A shell that's too large (crab rattles inside) signals recent growth or poor shell availability. A shell that's too small (crab can't fully retract) signals desperate circumstances.

**Research finding** (Hazlett, 1981): Crabs use shell-type signals to **decide whether to initiate social interactions**. A small crab in a large conch shell will avoid confrontation with a larger crab — but a small crab in a small shell may approach the same larger crab for shell exchange. The **shell is the interface**, not the crab.

**Critical insight**: The signal is **not about the crab** — it's about the **crab-shell system**. A small crab in a conch shell and a large crab in the same conch shell present different signals, because the **fit** between crab and shell encodes information.

## 4.2 The PincherOS Mapping

In PincherOS, the shell type should be a **routable service level indicator**. Other agents (and users) should be able to make decisions based on what shell an agent is wearing, without needing to query the agent directly.

### Engineering Specification: Shell Signaling Protocol

```rust
// src/shell/signaling.rs

/// ShellSpecies encodes the "type" of shell — analogous to gastropod species.
/// This is the primary signal other agents see.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShellSpecies {
    /// Queen Conch — large workstation (48GB+ GPU, 64GB+ RAM)
    /// Signals: high-capability, can handle any workload
    StrombusGigax,
    
    /// Turbo Shell — mid-range device (Jetson Orin, laptop with GPU)
    /// Signals: moderate capability, GPU available, limited VRAM
    TurboCastanea,
    
    /// Whelk Shell — small GPU device (Jetson Nano, Pi 5 with AI hat)
    /// Signals: basic GPU, can offload some layers
    Busycotypus,
    
    /// Nassarius Shell — CPU-only edge device (Pi 4, Pi Zero)
    /// Signals: no GPU, inference only, reflex-heavy operation
    Nassarius,
    
    /// Littorina Shell — ultra-tiny device (microcontroller, ESP32-S3)
    /// Signals: reflex-only, no LLM, no vector DB
    Littorina,
}

/// The ShellSignal is the public-facing metadata that other agents can query
/// without directly communicating with the occupying agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSignal {
    /// What species of shell this is
    species: ShellSpecies,
    
    /// How well the current rigging fits this shell
    fit_score: f64,  // 0.0 (terrible fit) to 1.0 (perfect fit)
    
    /// What capabilities are currently available
    /// (may be less than shell's total if rigging is using some)
    available_capabilities: Capabilities,
    
    /// Current load indicators
    load: ShellLoad,
    
    /// Whether this shell is open to vacancy chain participation
    cascade_eligible: bool,
    
    /// Trust score from previous sync queue exchanges
    exchange_trust: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellLoad {
    cpu_utilization: f64,
    ram_utilization: f64,
    gpu_utilization: Option<f64>,
    active_reflexes: usize,
    pending_llm_requests: usize,
}

/// Routing decisions based on shell signals
pub fn route_by_shell_signal(
    task: &Task,
    candidates: &[ShellSignal],
) -> Option<ShellSignal> {
    // A "conch shell" (workstation) signals it can handle any workload
    // A "nassarius shell" (Pi) signals it can only handle reflex-short-circuit tasks
    // Route accordingly
    
    match task.compute_requirement() {
        ComputeRequirement::HeavyLlm => {
            // Need a conch or turbo shell
            candidates.iter()
                .filter(|s| matches!(s.species, ShellSpecies::StrombusGigax | ShellSpecies::TurboCastanea))
                .filter(|s| s.load.gpu_utilization.map_or(false, |u| u < 0.8))
                .min_by(|a, b| a.load.gpu_utilization.partial_cmp(&b.load.gpu_utilization).unwrap())
        }
        ComputeRequirement::ReflexOnly => {
            // Any shell will do — prefer the least loaded
            candidates.iter()
                .filter(|s| s.fit_score > 0.5)  // Agent must fit reasonably well
                .min_by(|a, b| a.load.cpu_utilization.partial_cmp(&b.load.cpu_utilization).unwrap())
        }
        ComputeRequirement::CudaKernel => {
            // Must have GPU — conch, turbo, or whelk
            candidates.iter()
                .filter(|s| matches!(s.species, ShellSpecies::StrombusGigax | ShellSpecies::TurboCastanea | ShellSpecies::Busycotypus))
                .filter(|s| s.load.gpu_utilization.map_or(false, |u| u < 0.6))
                .min_by(|a, b| a.load.gpu_utilization.partial_cmp(&b.load.gpu_utilization).unwrap())
        }
        _ => None,
    }
}
```

### Shell Species → Hardware Tier Mapping

| Shell Species | Hardware | GPU | RAM | LLM | Typical Load |
|---------------|----------|-----|-----|-----|-------------|
| *Strombus gigax* | RTX 4090 workstation | 48GB VRAM | 64GB+ | Llama 3.1 70B | Heavy inference, multi-agent |
| *Turbo castanea* | Jetson Orin / laptop | 8-16GB VRAM | 16-32GB | Phi-3 14B | Moderate inference, CUDA kernels |
| *Busycotypus* | Jetson Nano | 128 CUDA cores | 4GB | TinyLlama 1.1B | Light inference, reflex-heavy |
| *Nassarius* | Raspberry Pi 4/5 | None | 4-8GB | TinyLlama 1.1B (CPU) | Reflex-only with rare LLM |
| *Littorina* | ESP32-S3 / Pico | None | 512KB-8MB | None | Pure reflex execution |

### The Big Insight

**Shell species is the DNS of agent routing.** Just as DNS records tell you whether a server is a CDN edge (low latency, limited compute) or an origin server (high latency, full compute), the shell species tells other agents what to expect. But DNS is static — shell signaling is **dynamic** because the fit_score changes as the rigging adapts. A *Nassarius* shell with a fit_score of 0.95 (well-matched agent) is a better routing target than a *Turbo castanea* with a fit_score of 0.3 (badly mismatched agent).

This also enables **shell-aware load balancing**: when dispatching tasks across a fleet, the dispatcher doesn't need to probe each agent — it reads the shell signals and routes accordingly. This is O(1) routing vs. O(N) probing.

---

# 5. MOISTURE MANAGEMENT: STATE HYDRATION

## 5.1 The Biology

*C. clypeatus* is a **terrestrial** hermit crab — it cannot survive underwater. But it evolved from marine ancestors and **retains gills** that must remain moist for gas exchange. The shell serves as a **moisture reservoir**:

1. **Shell water**: The crab carries ~0.5-2ml of water in the shell's inner whorls, keeping its abdomen and gills hydrated (Greenaway, 2003)

2. **Humidity regulation**: The shell creates a microclimate with higher humidity than ambient air. The aperture can be partially sealed by the crab's cheliped, reducing evaporative water loss by up to 60%

3. **Water renewal**: Crabs periodically visit water sources (tide pools, rain puddles) to replenish shell water. This is a **critical lifecycle activity** — deprivation for >48 hours leads to gill desiccation and death

4. **Differential moisture**: The crab maintains different moisture levels in different shell chambers — the innermost whorl (where the abdomen sits) is wettest; the aperture region is drier

**Critical biological insight**: Shell water is not just "water in the shell." It's a **chemically modified fluid** — the crab adds salts and organic compounds that improve water retention and have antimicrobial properties. The shell water is a **life-support fluid**, not raw seawater.

**What happens when moisture is lost**: The crab enters a **dehydration cascade**:
- 0-12 hours: Reduced activity, seeking shade
- 12-24 hours: Impaired locomotion, erratic behavior
- 24-48 hours: Gills begin to desiccate, reduced gas exchange → cognitive impairment (yes, crabs show reduced decision quality when dehydrated)
- 48+ hours: Critical organ failure

## 5.2 The PincherOS Mapping

The "moisture" in PincherOS is **persistent state warmth** — the active, ready-to-use state that keeps the agent functional. When this state "dries out," the agent degrades through predictable stages.

### What Is the Moisture?

| Biological Moisture | PincherOS Analog | What "Dries Out" |
|--------------------|--------------------|-------------------|
| Shell water (gill hydration) | **LLM KV cache** (warm context) | Context window loses recent conversation state |
| Shell humidity (microclimate) | **LanceDB vector index** (hot index) | Reflex matching degrades to brute-force search |
| Inner whorl moisture (abdomen) | **JEPA world model** (predictive state) | Agent cannot predict outcomes, reverts to full LLM |
| Aperture seal (reduced evaporation) | **Embedding cache** (semantic cache) | Every request requires re-embedding |
| Chemically modified water | **Compiled reflexes** (distilled workflows) | Reflexes lose task-specific optimizations |

### Engineering Specification: Moisture Management

```rust
// src/shell/moisture.rs

/// MoistureLevel tracks the "hydration" of each critical state component.
/// When moisture drops, the agent degrades predictably.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoistureState {
    /// LLM KV cache freshness (1.0 = fully warm, 0.0 = cold/unloaded)
    llm_cache_warmth: f64,
    /// LanceDB index health (1.0 = fully indexed, 0.0 = no index, brute-force)
    vector_index_health: f64,
    /// JEPA world model readiness (1.0 = loaded and recent, 0.0 = not loaded)
    jepa_readiness: f64,
    /// Embedding cache hit rate (rolling average over last 100 queries)
    embedding_cache_hit_rate: f64,
    /// Compiled reflex coverage (fraction of top-K reflexes that are compiled)
    reflex_compilation_rate: f64,
    /// Time since last "rehydration" (full state refresh)
    time_since_rehydration_ms: u64,
}

impl MoistureState {
    /// Compute overall hydration level (0.0 to 1.0)
    pub fn hydration_level(&self) -> f64 {
        // Weighted average — LLM cache and vector index are most critical
        (self.llm_cache_warmth * 0.30 +
         self.vector_index_health * 0.25 +
         self.jepa_readiness * 0.20 +
         self.embedding_cache_hit_rate * 0.15 +
         self.reflex_compilation_rate * 0.10)
    }
    
    /// Determine the degradation stage (analogous to crab dehydration cascade)
    pub fn degradation_stage(&self) -> DegradationStage {
        let h = self.hydration_level();
        match h {
            h if h >= 0.8 => DegradationStage::FullyHydrated,
            h if h >= 0.6 => DegradationStage::MildDehydration {
                // 0-12 hour analog: reduced activity
                // LLM unloads after idle, but can reload
                recommendation: "Schedule rehydration during next idle period".into(),
            },
            h if h >= 0.4 => DegradationStage::ModerateDehydration {
                // 12-24 hour analog: impaired locomotion
                // LLM stays unloaded, vector index degrades
                recommendation: "Force rehydration now: reload LLM, rebuild vector index".into(),
            },
            h if h >= 0.2 => DegradationStage::SevereDehydration {
                // 24-48 hour analog: cognitive impairment
                // Only reflex short-circuit works, no LLM at all
                recommendation: "CRITICAL: Agent is operating on reflexes only. Rehydrate immediately.".into(),
            },
            _ => DegradationStage::CriticalDehydration {
                // 48+ hour analog: organ failure
                // Even reflex matching is degraded (no embedding cache)
                recommendation: "Agent is non-functional. Full rehydration required before any operation.".into(),
            },
        }
    }
}

#[derive(Debug)]
pub enum DegradationStage {
    FullyHydrated,
    MildDehydration { recommendation: String },
    ModerateDehydration { recommendation: String },
    SevereDehydration { recommendation: String },
    CriticalDehydration { recommendation: String },
}

/// The RehydrationScheduler ensures the agent never dries out.
/// Analogous to the crab's periodic visits to water sources.
pub struct RehydrationScheduler {
    /// How often to check moisture levels
    check_interval: Duration,
    /// Threshold below which rehydration is triggered
    rehydration_threshold: f64,
    /// Last rehydration timestamp
    last_rehydration: Instant,
}

impl RehydrationScheduler {
    /// Rehydrate the agent — analogous to the crab visiting a tide pool
    pub async fn rehydrate(&self, infer: &InferenceBridge) -> Result<(), HydrationError> {
        // 1. Reload LLM (if unloaded) — most important
        infer.send_command(InferCommand::LoadLLM).await?;
        
        // 2. Rebuild vector index (if degraded)
        infer.send_command(InferCommand::RebuildIndex).await?;
        
        // 3. Warm the embedding cache with most-accessed reflexes
        infer.send_command(InferCommand::WarmEmbeddingCache { top_k: 50 }).await?;
        
        // 4. Load JEPA world model (if available)
        infer.send_command(InferCommand::LoadJEPA).await?;
        
        // 5. Compile high-confidence reflexes
        infer.send_command(InferCommand::CompileReflexes { min_confidence: 0.9 }).await?;
        
        Ok(())
    }
    
    /// Schedule rehydration during idle periods (analogous to crab visiting 
    /// water at night when predation risk is lower)
    pub fn should_rehydrate(&self, moisture: &MoistureState, is_idle: bool) -> bool {
        // Rehydrate proactively during idle periods
        if is_idle && moisture.hydration_level() < 0.9 {
            return true;
        }
        // Force rehydrate if critically dehydrated, regardless of idle state
        if moisture.hydration_level() < self.rehydration_threshold {
            return true;
        }
        false
    }
}
```

### The Big Insight

**Moisture management is the missing operational model in PincherOS.** The current architecture has graceful degradation (Critical → Moderate → Light → Normal) but treats it as a **reactive** response to memory pressure. Biology shows it should be a **proactive lifecycle management** system:

1. **Periodic rehydration** (like a crab visiting tide pools): Even when the agent isn't under memory pressure, it should periodically reload models, rebuild indexes, and warm caches during idle periods

2. **Differential moisture** (like a crab's different humidity zones): Not all state is equally critical. The LLM cache is the "inner whorl" (must stay wettest). The embedding cache is the "aperture" (can tolerate some dryness). PincherOS should prioritize what gets hydrated first

3. **Chemically modified water** (like a crab's shell water additives): Compiled reflexes aren't just "cached LLM outputs" — they're **optimized** (distilled, validated, confidence-scored). The "compilation" process is the chemical modification that makes the state more resilient to loss

4. **Dehydration cascade**: The degradation stages aren't arbitrary thresholds — they correspond to **which state components dry out first**. LLM cache evaporates fastest (minutes). Vector index degrades next (hours). Compiled reflexes are most resilient (days). This ordering should drive the degradation logic.

---

# 6. MOLTING: SAFE SANDBOXED UPGRADE

## 6.1 The Biology

Molting (ecdysis) is the most dangerous event in a hermit crab's life. The process:

1. **Pre-molt**: The crab stores calcium and energy reserves. It seeks a **secure, humid location** (often burying itself in sand). It stops eating. Its current shell becomes too small as the new exoskeleton develops underneath.

2. **Ecdysis**: The crab **leaves its shell entirely** and sheds its old exoskeleton. It emerges soft, pale, and extremely vulnerable — unable to defend itself, unable to move quickly, unable to regulate water loss through its soft new cuticle.

3. **Post-molt (hardening)**: The new exoskeleton hardens over 7-30 days. During this period, the crab stays hidden. It may re-enter its old shell (if it still fits) or find a larger one. It **eats its own shed exoskeleton** to reclaim calcium.

4. **Re-emergence**: Once the exoskeleton has hardened sufficiently, the crab resumes normal activity.

**Critical biological constraints**:
- The crab MUST leave its shell to molt — there is no "in-place upgrade"
- During molting, the crab is 10-100x more vulnerable to predation
- The old exoskeleton is not waste — it's **recycled** as nutrition for the new one
- The timing of molting is tied to **environmental conditions** (humidity, temperature, food availability)
- Crabs often molt **in groups** — synchronized molting reduces individual predation risk through dilution effects

## 6.2 The PincherOS Mapping

Molting = **major version upgrade of the rigging**. This includes:
- JEPA world model retraining
- Penrose tensor re-encoding
- Reflex schema migration
- Embedding model upgrade (e.g., MiniLM-L6 → Nomic Embed)
- Personality/core-loop version bump

### Engineering Specification: The Molting Chamber

```rust
// src/shell/molt.rs

/// A Molt is a major rigging upgrade that requires the agent to temporarily
/// leave its shell (standard execution environment).
pub struct Molt {
    /// The rigging being molted
    rigging_id: String,
    /// What's being upgraded
    upgrade: MoltUpgrade,
    /// The molt chamber — a safe sandbox for the upgrade
    chamber: MoltChamber,
    /// Current stage
    stage: MoltStage,
    /// The "old exoskeleton" — previous state to recycle
    previous_state: Option<RiggingSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoltUpgrade {
    /// Retrain JEPA world model on accumulated experience
    JepaRetrain { 
        new_training_data: Vec<ExperienceTrajectory>,
        epochs: usize,
    },
    /// Re-encode Penrose tensor memory
    PenroseReencode {
        new_resolution: PenroseResolution,
    },
    /// Upgrade embedding model (e.g., MiniLM → Nomic)
    EmbeddingModelUpgrade {
        from_model: String,
        to_model: String,
        re_embed_all: bool,  // Must re-embed every reflex?
    },
    /// Schema migration for reflex format
    SchemaMigration {
        from_version: u32,
        to_version: u32,
    },
    /// Full rigging version upgrade
    FullUpgrade {
        target_version: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoltStage {
    /// Pre-molt: accumulating resources, finding safe location
    Preparing,
    /// Ecdysis: agent has left shell, upgrade in progress
    Molting,
    /// Post-molt: hardening, validating new state
    Hardening,
    /// Re-emergence: agent returns to shell with upgraded rigging
    Emerged,
}

/// The MoltChamber is a safe sandbox where the rigging can undergo
/// major upgrades without affecting the live shell.
/// 
/// Key properties:
/// 1. Isolated from the live shell (no user requests processed)
/// 2. Has access to all of the rigging's state (memories, reflexes)
/// 3. Can restore the previous state if the molt fails
/// 4. Has sufficient resources (RAM, disk) for the upgrade
pub struct MoltChamber {
    /// Path to the chamber's isolated filesystem
    chamber_root: PathBuf,
    /// Copy of the rigging's state for safe modification
    state_snapshot: RiggingSnapshot,
    /// Resource budget for the chamber
    budget: MoltBudget,
    /// Timeout: if molt doesn't complete in this time, abort
    max_molt_duration: Duration,
}

pub struct MoltBudget {
    /// RAM available for the chamber (separate from live shell)
    ram_mb: u64,
    /// Disk space for new model/re-encoded state
    disk_mb: u64,
    /// CPU time allocation
    cpu_seconds: u64,
    /// GPU time allocation (if available for faster retraining)
    gpu_seconds: Option<u64>,
}

impl Molt {
    /// Execute the molt process
    pub async fn execute(&mut self) -> Result<MoltResult, MoltError> {
        // PHASE 1: PRE-MOLT PREPARATION
        self.stage = MoltStage::Preparing;
        
        // 1a. Snapshot current state (the "old exoskeleton")
        self.previous_state = Some(snapshot_rigging(&self.rigging_id).await?);
        
        // 1b. Verify chamber has sufficient resources
        self.chamber.verify_resources()?;
        
        // 1c. Drain incoming requests (stop accepting new tasks)
        // Analogous to the crab stopping eating before molting
        drain_pending_requests(&self.rigging_id).await?;
        
        // PHASE 2: ECDYSIS — THE DANGEROUS PART
        self.stage = MoltStage::Molting;
        
        // 2a. Move rigging into the chamber (leave the shell)
        enter_chamber(&self.rigging_id, &self.chamber).await?;
        
        // 2b. Execute the upgrade
        let result = match &self.upgrade {
            MoltUpgrade::JepaRetrain { new_training_data, epochs } => {
                self.retrain_jepa(new_training_data, *epochs).await?
            }
            MoltUpgrade::EmbeddingModelUpgrade { from_model, to_model, re_embed_all } => {
                self.upgrade_embeddings(from_model, to_model, *re_embed_all).await?
            }
            // ... other upgrade types
            _ => unimplemented!(),
        };
        
        // PHASE 3: HARDENING — VALIDATE NEW STATE
        self.stage = MoltStage::Hardening;
        
        // 3a. Run validation tests on the upgraded rigging
        // Analogous to the new exoskeleton hardening
        let validation = self.validate_upgrade().await?;
        if !validation.passed {
            // MOLT FAILED: restore previous state
            // This is analogous to the crab re-entering its old shell
            self.restore_previous_state().await?;
            return Err(MoltError::ValidationFailed(validation.failures));
        }
        
        // 3b. "Eat the old exoskeleton" — recycle useful parts
        // Transfer any newly-learned reflexes from old state that
        // aren't in the new state (they were generated during molt prep)
        self.recycle_previous_state().await?;
        
        // PHASE 4: RE-EMERGENCE
        self.stage = MoltStage::Emerged;
        
        // 4a. Move rigging back to the live shell
        exit_chamber(&self.rigging_id).await?;
        
        // 4b. Run Snap to ensure rigging still fits after upgrade
        snap(&self.rigging_id).await?;
        
        Ok(result)
    }
    
    /// "Eat the old exoskeleton" — recycle useful state from the previous version.
    /// In biology: the crab consumes its shed exoskeleton to reclaim calcium.
    /// In PincherOS: we transfer reflexes/memories that the upgrade didn't touch.
    async fn recycle_previous_state(&self) -> Result<(), MoltError> {
        if let Some(old_state) = &self.previous_state {
            // Find reflexes in old state that don't exist in new state
            let new_state = current_rigging_state(&self.rigging_id).await?;
            let mut recycled = 0;
            
            for old_reflex in &old_state.reflexes {
                if !new_state.reflexes.iter().any(|r| r.id == old_reflex.id) {
                    // This reflex wasn't touched by the upgrade — transfer it
                    transfer_reflex(old_reflex, &self.rigging_id).await?;
                    recycled += 1;
                }
            }
            
            tracing::info!("Molt: recycled {} reflexes from previous state", recycled);
        }
        Ok(())
    }
}
```

### What This Changes Architecturally

| Current Model | Molting Model |
|---------------|---------------|
| Upgrades happen in-place | Upgrades happen in an **isolated chamber** |
| Failed upgrades leave the system in an inconsistent state | Failed upgrades **restore the previous state** (like a crab re-entering its old shell) |
| Old version data is deleted | Old version data is **recycled** (like eating the exoskeleton) |
| No concept of "vulnerability during upgrade" | Agent is **explicitly marked as molting** — no requests routed to it |
| Upgrades can happen anytime | Upgrades should happen during **low-activity periods** (like crabs molting at night) |

### The Big Insight

**Molting is transactional agent upgrade with recycling.** The current architecture has no concept of "the agent is temporarily non-operational for a major upgrade." It should. Specifically:

1. **The molting chamber is a copy-on-write sandbox**: The agent's state is duplicated into the chamber. The upgrade operates on the copy. If it succeeds, the copy replaces the original. If it fails, the copy is discarded and the original is untouched. This is exactly how ZFS snapshots work — and exactly how crabs molt (they don't destroy the old exoskeleton until the new one has hardened).

2. **Exoskeleton recycling = state transfer**: When the crab eats its old exoskeleton, it reclaims 90%+ of the calcium. When PincherOS completes a molt, it should transfer reflexes, memories, and learned behaviors from the old state that weren't modified by the upgrade. This prevents "regression" — losing learned behaviors because the upgrade schema changed.

3. **Synchronized molting**: Crabs in groups often molt simultaneously (dilution effect). PincherOS agents in a fleet should **coordinate molts** so that multiple agents don't go offline at different times. Better to have all agents molt during a scheduled maintenance window than to have random agents going offline unexpectedly.

4. **Molting signals**: Other agents should be able to see that an agent is molting (the shell signal should include `stage: MoltStage::Molting`) and route around it. This is the biological analog: other crabs can see that a molting crab is in its burrow and don't try to interact with it.

---

# 7. THE INTERTIDAL ZONE: EDGE-CLOUD BOUNDARY

## 7.1 The Biology

*C. clypeatus* lives in the **intertidal zone** — the boundary between land and sea. This is not a transitional zone; it is a **stable ecological niche** with unique properties:

1. **Highest biodiversity**: The intertidal zone has more species per square meter than either the open ocean or the inland forest. The boundary creates **ecotones** — transition zones where species from both ecosystems coexist and hybrid adaptations emerge

2. **Periodic resource influx**: Tides bring marine resources (plankton, dead fish, seaweed) every 12 hours. Rain brings freshwater. The intertidal crab has access to **both** marine and terrestrial food webs

3. **Environmental oscillation**: Temperature, humidity, salinity, and predation pressure cycle daily. Crabs must be **adapted to oscillation**, not stability

4. **Information gradient**: The intertidal zone is where chemical signals from both marine and terrestrial environments overlap. Crabs use this information gradient to **anticipate** environmental changes (e.g., detecting approaching storms via barometric pressure changes)

5. **The "sweet spot" for C. clypeatus**: This species is NOT a marine crab that visits land, nor a land crab that visits water. It is **permanently intertidal** — it cannot survive fully aquatic (will drown) or fully terrestrial (will desiccate). It exists at the boundary and **requires** the boundary

## 7.2 The PincherOS Mapping

The intertidal zone maps precisely to the **edge-cloud boundary**. Not pure edge (Pi-only) or pure cloud (API-only), but the **hybrid zone** where both are accessible and the most interesting agent behavior emerges.

### Engineering Specification: The Intertidal Architecture

```rust
// src/shell/intertidal.rs

/// An IntertidalAgent operates at the edge-cloud boundary.
/// It has access to both local (edge) and remote (cloud) resources,
/// and dynamically shifts its operational mode based on connectivity tides.
pub struct IntertidalAgent {
    /// Current tide state (connectivity)
    tide: TideState,
    /// Local resources (always available, like terrestrial food)
    local: LocalResources,
    /// Remote resources (available on high tide, like marine food)
    remote: RemoteResources,
    /// Cached cloud resources (deposited by previous high tide, like tide pool debris)
    cache: TidalCache,
    /// The agent's position on the intertidal gradient
    /// 0.0 = fully terrestrial (offline), 1.0 = fully marine (cloud-only)
    position: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TideState {
    /// High tide: cloud is accessible, full resources available
    HighTide { latency_ms: u64 },
    /// Ebb tide: cloud is degrading, agent should cache aggressively
    EbbTide { estimated_disconnect_in_secs: u64 },
    /// Low tide: no cloud access, purely local operation
    LowTide,
    /// Flood tide: cloud is returning, agent should sync cached state
    FloodTide { estimated_connect_in_secs: u64 },
}

/// TidalCache stores cloud resources that were available during high tide
/// for use during low tide. This is the biological analog of tide pools
/// that retain marine life between tides.
pub struct TidalCache {
    /// LLM completions cached from cloud during high tide
    /// These are "expensive" results that we don't want to recompute locally
    llm_completions: HashMap<String, CachedCompletion>,
    /// Model weights downloaded for local inference during low tide
    model_weights: Option<CachedModelWeights>,
    /// Vector DB snapshots synced from cloud
    vector_snapshots: Vec<VectorSnapshot>,
    /// When this cache was last refreshed (last high tide)
    last_refresh: Instant,
    /// TTL for cached items (they "evaporate" over time, like tide pools)
    default_ttl: Duration,
}

impl IntertidalAgent {
    /// The core decision loop: how to handle a task based on tide state
    pub async fn handle_task(&self, task: Task) -> TaskResult {
        match self.tide {
            TideState::HighTide { latency_ms } if latency_ms < 200 => {
                // Full cloud access — send heavy work to cloud
                // Use cloud LLM, cloud vector DB, cloud compute
                // This is the crab feeding at the ocean's edge
                self.execute_cloud_first(task).await
            }
            TideState::HighTide { latency_ms } if latency_ms >= 200 => {
                // High tide but slow — hybrid approach
                // Local reflexes + cloud verification
                self.execute_hybrid(task).await
            }
            TideState::EbbTide { estimated_disconnect_in_secs } => {
                // Cloud is going away — AGGRESSIVE CACHING
                // Pre-fetch everything we might need
                // This is the crab stockpiling food before the tide goes out
                self.cache_critical_resources(estimated_disconnect_in_secs).await;
                self.execute_hybrid(task).await
            }
            TideState::LowTide => {
                // No cloud — pure local operation
                // Reflex short-circuit only, cached LLM completions, local model
                // This is the crab foraging on land during low tide
                self.execute_local_only(task).await
            }
            TideState::FloodTide { estimated_connect_in_secs } => {
                // Cloud is coming back — SYNC
                // Upload accumulated local state, download updates
                // This is the crab returning to the waterline as tide rises
                self.sync_pending_state().await;
                self.execute_local_only(task).await
            }
            _ => self.execute_local_only(task).await,
        }
    }
    
    /// The most interesting behavior: HYBRID execution at the boundary
    /// This is where edge AND cloud contribute to a single task
    async fn execute_hybrid(&self, task: Task) -> TaskResult {
        // Phase 1: Local reflex check (fast, free)
        let local_match = self.local.reflex_matcher.search(&task).await;
        
        if local_match.confidence > 0.90 {
            // High-confidence local reflex — execute immediately
            // But also verify against cloud in background (if tide allows)
            let result = self.local.execute_reflex(local_match).await;
            
            // Background cloud verification (asynchronous, non-blocking)
            if matches!(self.tide, TideState::HighTide { .. }) {
                tokio::spawn(async move {
                    let cloud_result = self.remote.verify_against_cloud(&task, &result).await;
                    if let Err(verification) = cloud_result {
                        // Cloud disagrees with local reflex — flag for review
                        // This is the analog of a crab sensing danger through
                        // marine chemical signals while foraging on land
                        flag_for_review(&task, &result, verification).await;
                    }
                });
            }
            
            return result;
        }
        
        // Phase 2: Cloud-augmented reasoning
        // Local context + cloud inference = hybrid intelligence
        let local_context = self.local.context_assembler.build(&task).await;
        let cloud_inference = self.remote.cloud_inference(&task, &local_context).await;
        
        // Store the cloud result in tidal cache for future low-tide use
        self.cache.store_completion(&task, &cloud_inference).await;
        
        TaskResult::from_inference(cloud_inference)
    }
}
```

### The Big Insight

**The intertidal zone is not a transitional state — it's the primary habitat.** The current PincherOS architecture treats cloud as either "available" or "unavailable." Biology shows this is wrong. The crab doesn't experience "land then ocean then land" — it experiences a **continuous gradient** with **predictable oscillations**. PincherOS agents should:

1. **Anticipate connectivity tides**: If the agent knows it will lose cloud access at 6pm (e.g., office network shutting down), it should enter **ebb tide mode** at 5:30pm and aggressively cache. This is not reactive — it's **predictive resource management** based on learned connectivity patterns

2. **Thrive at the boundary**: The most capable PincherOS agent is not the one with the best local model (pure edge) or the one with unlimited cloud access (pure cloud). It's the one that **operates at the boundary** — using local reflexes for speed and cloud inference for depth, with a tidal cache that bridges the two

3. **Embrace oscillation**: The agent shouldn't try to maintain a constant state. It should **cycle**: cache aggressively during high tide, operate leanly during low tide, sync during flood tide. This is a **rhythmic operational model**, not a steady-state one

4. **Biodiversity at the boundary**: Just as the intertidal zone has the most biological diversity, the edge-cloud boundary is where the most diverse PincherOS agents will emerge — agents that can do things neither pure-edge nor pure-cloud agents can do

---

# 8. CUDACLAWS AS THE CHELIPED: EXECUTE AND SENSE

## 8.1 The Biology

*C. clypeatus* has an asymmetric claw pair:
- **Left cheliped** (the "purple pincher"): Enormous, heavily calcified, serves as the primary tool for crushing, defense, climbing, and **sensing**. The left cheliped has dense chemoreceptor and mechanoreceptor arrays on its inner surface
- **Right cheliped**: Smaller, more precise, used for cutting, manipulating food, and fine motor tasks

The left cheliped's **dual function** (execute + sense) is critical. When the crab is exploring a potential shell:
1. The cheliped **inserts into the aperture** to assess size and shape (sensing)
2. The cheliped **grasps and rotates** the shell to test weight and balance (sensing + executing)
3. The cheliped **crushes** the shell lip to test structural integrity (executing + sensing — the resistance felt during crushing provides information)
4. The cheliped **blocks the aperture** to defend the shell (executing)

**Key biological insight**: The cheliped's sensory function is NOT passive. It's an **active sensing** organ — the crab moves the cheliped specifically to gather information, even when it's not trying to execute a task. Cheliped tapping, probing, and scraping are **information-seeking behaviors** that precede any action.

## 8.2 The PincherOS Mapping

CUDAClaw is currently defined as a **GPU dispatch runtime** — it executes kernels. The biology says it should also be a **sensing organ** — it probes, benchmarks, and measures the shell's capabilities.

### Engineering Specification: CUDAClaw as Cheliped

```rust
// src/cudaclaw/cheliped.rs

/// CUDAClaw serves dual function: EXECUTE (run GPU kernels) and SENSE
/// (probe hardware capabilities, measure latency, detect anomalies).
/// 
/// Just as the crab's cheliped is both a tool and a sensor,
/// CUDAClaw is both a compute engine and a diagnostic instrument.
pub struct CudaClaw {
    /// Execution capability: GPU kernel dispatch
    executor: GpuExecutor,
    /// Sensing capability: hardware probing and benchmarking
    sensor: GpuSensor,
}

// === EXECUTION FUNCTIONS (the "crushing" role) ===

impl CudaClaw {
    /// Execute a GPU kernel — the primary "crushing" function
    pub async fn execute_kernel(
        &self, 
        kernel: &CudaKernel, 
        input: &KernelInput
    ) -> Result<KernelOutput, ClawError> {
        self.executor.dispatch(kernel, input).await
    }
    
    /// Execute inference with GPU offload
    pub async fn offload_inference(
        &self,
        model: &ModelLayers,
        tokens: &[TokenId],
    ) -> Result<InferenceOutput, ClawError> {
        self.executor.inference(model, tokens).await
    }
}

// === SENSING FUNCTIONS (the "probing" role) ===

impl CudaClaw {
    /// Probe the GPU's current state — analogous to the crab inserting
    /// its cheliped into a shell to assess size
    pub async fn probe(&self) -> GpuProbeResult {
        let mut result = GpuProbeResult::default();
        
        // 1. Memory availability (how much VRAM is free?)
        result.vram_available_mb = self.sensor.query_vram_available().await;
        
        // 2. Compute utilization (is the GPU busy?)
        result.compute_utilization = self.sensor.query_compute_utilization().await;
        
        // 3. Thermal state (is the GPU overheating?)
        result.thermal_state = self.sensor.query_thermal_state().await;
        
        // 4. Memory bandwidth (current throughput)
        result.memory_bandwidth_gbps = self.sensor.query_bandwidth().await;
        
        result
    }
    
    /// Stress-test the GPU — analogous to the crab crushing the shell lip
    /// to test structural integrity. This is ACTIVE SENSING: we run a 
    /// lightweight benchmark to measure real performance, not just read specs.
    pub async fn stress_test(&self, duration: Duration) -> StressTestResult {
        let start = Instant::now();
        
        // Run a known-compute-density kernel for the specified duration
        let iterations = self.executor.run_benchmark_kernel(duration).await;
        
        // Measure: did the GPU maintain expected throughput?
        let expected_iters = self.executor.rated_iterations_per_second();
        let actual_iters = iterations as f64 / duration.as_secs_f64();
        let throughput_ratio = actual_iters / expected_iters;
        
        // Measure: did thermal throttling kick in?
        let thermal_throttled = self.sensor.was_thermal_throttled().await;
        
        // Measure: did any errors occur?
        let errors = self.executor.error_count().await;
        
        StressTestResult {
            throughput_ratio,
            thermal_throttled,
            errors,
            duration: start.elapsed(),
        }
    }
    
    /// Measure latency for a specific operation — analogous to the crab
    /// tapping the shell to assess acoustic properties
    pub async fn measure_latency(
        &self, 
        operation: GpuOperation
    ) -> LatencyProfile {
        // Take multiple measurements to account for variance
        let measurements: Vec<Duration> = (0..10)
            .map(|_| self.executor.measure_one(&operation))
            .collect::<Vec<_>>()
            .await;
        
        LatencyProfile {
            p50: percentile(&measurements, 50),
            p95: percentile(&measurements, 95),
            p99: percentile(&measurements, 99),
            min: measurements.iter().min().copied().unwrap(),
            max: measurements.iter().max().copied().unwrap(),
        }
    }
    
    /// The cheliped's "active sensing" — probe the shell's GPU capabilities
    /// BEFORE committing to an operation. This is the crab assessing a shell
    /// before entering it.
    pub async fn assess_shell_gpu(&self) -> GpuAssessment {
        let probe = self.probe().await;
        let stress = self.stress_test(Duration::from_secs(5)).await;
        let inference_latency = self.measure_latency(GpuOperation::Inference {
            model_size_mb: 700,  // TinyLlama size
            batch_size: 1,
        }).await;
        
        GpuAssessment {
            // Can this GPU handle our default model?
            can_run_default_model: probe.vram_available_mb > 700,
            // How much model can we fit?
            max_model_size_mb: (probe.vram_available_mb as f64 * 0.8) as u64,
            // Is thermal throttling a concern?
            thermal_risk: if stress.thermal_throttled { 
                ThermalRisk::High 
            } else if stress.throughput_ratio < 0.9 { 
                ThermalRisk::Medium 
            } else { 
                ThermalRisk::Low 
            },
            // Expected inference performance
            expected_tokens_per_sec: self.estimate_tps(&inference_latency),
            // Raw measurements
            probe,
            stress,
            inference_latency,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAssessment {
    pub can_run_default_model: bool,
    pub max_model_size_mb: u64,
    pub thermal_risk: ThermalRisk,
    pub expected_tokens_per_sec: f64,
    pub probe: GpuProbeResult,
    pub stress: StressTestResult,
    pub inference_latency: LatencyProfile,
}
```

### The Big Insight

**CUDAClaw should be the first thing that touches a new shell, not the last.** Currently, GPU detection happens in `snap()` and CUDAClaw is activated later. Biology says the cheliped probes FIRST — the crab assesses the shell before committing. CUDAClaw's `assess_shell_gpu()` should run **before** model loading, before reflex matching, before anything. The assessment determines:

1. Which model to load (or whether to load any model at all)
2. How many GPU layers to offload
3. Whether to enable CUDA-accelerated embedding vs. ONNX CPU
4. Whether thermal management requires proactive throttling

This is **sensing before executing** — the fundamental pattern of the cheliped.

Furthermore, CUDAClaw should **continuously sense** during execution, not just at boot. The crab doesn't just assess the shell once — it constantly monitors shell condition through its cheliped's contact with the inner surface. CUDAClaw should monitor thermal state, memory pressure, and compute utilization during every kernel execution, and dynamically adjust:

- If thermal throttling is detected → reduce GPU layers → fall back to CPU
- If VRAM is running low → unload model → switch to reflex-only mode
- If compute throughput drops unexpectedly → flag for investigation → may indicate hardware degradation

---

# 9. EMERGENT PATTERNS: WHAT THE BIOLOGY REVEALS THAT THE METAPHOR MISSED

## 9.1 Larval Dispersal: Agent Bootstrapping

*C. clypeatus* has a **planktonic larval stage** lasting 30-60 days. Zoea larvae hatch in the ocean, drift with currents, and eventually settle as glaucothoe ( transitional stage) before finding their first shell. This is **dispersal without agency** — the larvae go wherever the currents take them.

**PincherOS mapping**: When a new PincherOS instance is first deployed, it should enter a **larval mode** — a minimal state that drifts through the shell network, gathering basic capabilities before "settling" into a full rigging. This is the `pincher init` experience:

```
Larval mode: 
  - No model loaded
  - No reflexes
  - Only the embedding model and basic I/O
  - "Drifts" by connecting to the shell registry and downloading 
    the most commonly-used skillpacks for this shell species
  - After 24-48 hours of basic operation, has enough reflexes
    to "settle" into a functional rigging
```

## 9.2 Antennule Chemoreception: Semantic Service Discovery

*C. clypeatus* has specialized antennules (short antennae) that detect chemical cues in the air. These allow the crab to:
- Smell food from 10+ meters away
- Detect the scent of dead gastropods (potential new shells)
- Identify other crabs by species and reproductive state
- Sense predation risk (chemical alarm signals from injured conspecifics)

**PincherOS mapping**: Agents should have a **semantic discovery layer** — the ability to detect nearby resources without actively querying them. This is **mDNS/DNS-SD for agent capabilities**:

```rust
// src/shell/antennule.rs

/// The Antennule provides passive service discovery — detecting nearby
/// shells and agents without actively querying them.
/// Analogous to the crab's chemoreception: smell the environment.
pub struct Antennule {
    /// mDNS/DNS-SD listener for nearby PincherOS instances
    mdns: MdnsListener,
    /// Semantic broadcast channel (agents periodically emit their shell signal)
    broadcast_rx: BroadcastReceiver<ShellSignal>,
    /// Cache of detected nearby resources
    detected: HashMap<String, DetectedResource>,
}

pub struct DetectedResource {
    shell_signal: ShellSignal,
    distance_hops: usize,    // Network distance
    last_detected: Instant,
    /// What this resource smells like (metaphorically)
    scent: ResourceScent,
}

pub enum ResourceScent {
    /// This shell has GPU capacity and low load — "smells like a good home"
    AvailableGpu { vram_mb: u64, utilization: f64 },
    /// This agent has relevant skillpacks — "smells like food"
    RelevantSkills { skillpacks: Vec<String>, overlap_score: f64 },
    /// This agent is stressed/overloaded — "smells like danger"
    StressedAgent { load_pct: f64, error_rate: f64 },
    /// New shell just joined the network — "smells like fresh shells"
    NewShell { species: ShellSpecies, fit_score: f64 },
}
```

## 9.3 Shell Rapping: Conflict Resolution

When two *C. clypeatus* want the same shell, they engage in **shell rapping** — the attacking crab grasps the defending crab's shell and rapidly strikes it against its own shell. The defender either yields (exits the shell) or resists. The rapping is **energetically costly** for both parties, so it usually ends quickly — the crab with the higher resource holding potential (RHP) wins (Briffa & Elwood, 2007).

**PincherOS mapping**: When two agents want the same shell, they should engage in a **costly signaling contest** rather than a destructive conflict:

```rust
// src/shell/rapping.rs

/// Shell rapping is a conflict resolution protocol where two agents
/// compete for the same shell through costly signaling.
/// The agent that demonstrates higher "need" (lower current fit)
/// wins the shell.
pub async fn resolve_shell_contention(
    shell: &ShellFingerprint,
    agent_a: &AgentProfile,
    agent_b: &AgentProfile,
) -> ContentionResult {
    // Each agent demonstrates its NEED for the shell by showing
    // how poorly it fits its current shell
    let need_a = 1.0 - agent_a.current_fit_score;  // Higher need = worse current fit
    let need_b = 1.0 - agent_b.current_fit_score;
    
    // The agent with higher need wins — analogous to the crab with
    // higher RHP (resource holding potential) winning the rapping contest
    if need_a > need_b + RAPPING_THRESHOLD {
        // Agent A wins — Agent B must find another shell
        ContentionResult::Winner { agent_id: agent_a.id.clone() }
    } else if need_b > need_a + RAPPING_THRESHOLD {
        ContentionResult::Winner { agent_id: agent_b.id.clone() }
    } else {
        // Need is similar — costly signaling contest
        // Each agent "raps" by computing a proof-of-work that
        // demonstrates its commitment to this shell
        // (the more compute it invests, the more it "wants" the shell)
        let pow_a = compute_need_proof(agent_a, shell).await;
        let pow_b = compute_need_proof(agent_b, shell).await;
        
        if pow_a.score > pow_b.score {
            ContentionResult::Winner { agent_id: agent_a.id.clone() }
        } else {
            ContentionResult::Winner { agent_id: agent_b.id.clone() }
        }
    }
}
```

## 9.4 Shell Speciation: Coevolution with Gastropods

Hermit crab populations are **limited by shell availability**, which depends on gastropod snail populations. The crab-shell system is a **coevolutionary relationship**: crabs prefer certain shell species, and their predation on gastropods (and preference for certain shell types) shapes gastropod evolution over evolutionary time.

**PincherOS mapping**: PincherOS shells should **co-evolve** with their agents. When an agent consistently remodels a shell (overclocking, partitioning GPU), it should feed that information back to a **shell specification registry**. Over time, the shell specifications evolve to match what agents actually need. This is the PincherOS equivalent of gastropod evolution driven by hermit crab preferences:

```rust
// Evolutionary feedback: agent remodeling informs future shell specs
pub struct ShellEvolutionFeedback {
    /// What mutations are most commonly applied?
    popular_mutations: HashMap<ShellMutation, u64>,
    /// What mutations fail most often?
    failed_mutations: HashMap<ShellMutation, u64>,
    /// What capabilities are agents consistently under-utilizing?
    underutilized_capabilities: HashMap<String, f64>,
    /// What capabilities do agents need but don't have?
    missing_capabilities: HashMap<String, u64>,
}

// This feedback drives future hardware procurement decisions:
// "Agents consistently overclock Pi 4s → maybe we should buy Pi 5s instead"
// "Agents never use the GPU on Jetson Nanos → maybe CPU-only devices are fine"
// "Agents consistently want more RAM disk → include eMMC storage in next shell spec"
```

---

# 10. CONSOLIDATED ARCHITECTURE: THE BIOLOGICAL FORMALISM

## 10.1 The Eight Principles

| # | Biological Principle | PincherOS Architectural Pattern | Engineering Concrete |
|---|---------------------|-------------------------------|---------------------|
| 1 | Vacancy chains | Cascading resource allocation | `VacancyChain` resolver in `src/shell/vacancy.rs` |
| 2 | Shell remodeling | Shell boundary mutation | `RemodelingEngine` in `src/shell/remodel.rs` |
| 3 | Sync queues | Batch migration (two-phase commit) | `SyncQueue` in `src/shell/sync_queue.rs` |
| 4 | Shell signaling | Service level indicators | `ShellSignal` + `ShellSpecies` in `src/shell/signaling.rs` |
| 5 | Moisture management | State hydration lifecycle | `MoistureState` + `RehydrationScheduler` in `src/shell/moisture.rs` |
| 6 | Molting | Sandboxed agent upgrade | `Molt` + `MoltChamber` in `src/shell/molt.rs` |
| 7 | Intertidal zone | Edge-cloud hybrid operation | `IntertidalAgent` + `TidalCache` in `src/shell/intertidal.rs` |
| 8 | Cheliped dual function | CUDAClaw: execute + sense | `CudaClaw` with `probe()` and `stress_test()` in `src/cudaclaw/cheliped.rs` |

## 10.2 Emergent Principles (From the Biology, Beyond the Metaphor)

| # | Emergent Principle | Source | PincherOS Implication |
|---|-------------------|--------|----------------------|
| 9 | Larval dispersal | Zoea → glaucothoe → adult | `pincher init` should be a 24-48hr bootstrap, not instant |
| 10 | Antennule chemoreception | Chemical sensing at distance | Semantic service discovery via mDNS + shell signals |
| 11 | Shell rapping | Conflict resolution via costly signaling | Proof-of-need protocol for shell contention |
| 12 | Coevolution | Crab-shell feedback loop | Shell spec registry evolves based on agent remodeling data |

## 10.3 The Unified Data Flow

```
                                    ┌──────────────────────────┐
                                    │     ANTENNULE LAYER       │
                                    │  mDNS + Shell Signals     │
                                    │  (passive discovery)      │
                                    └────────────┬─────────────┘
                                                 │
                    ┌────────────────────────────┼─────────────────────────────┐
                    │                            │                             │
                    ▼                            ▼                             ▼
         ┌──────────────────┐      ┌──────────────────────┐      ┌────────────────────┐
         │  SHELL SIGNALING │      │  VACANCY CHAIN        │      │  SYNC QUEUE        │
         │  (routing)       │      │  (cascading alloc)    │      │  (batch migration) │
         │  Species → Route │      │  One upgrade → N      │      │  N agents → 1 swap │
         └────────┬─────────┘      └──────────┬───────────┘      └──────────┬─────────┘
                  │                           │                              │
                  └───────────────────────────┼──────────────────────────────┘
                                              │
                                              ▼
                                   ┌─────────────────────┐
                                   │  INTERIDAL LAYER     │
                                   │  Edge ↔ Cloud        │
                                   │  (tide-aware)        │
                                   └──────────┬──────────┘
                                              │
                    ┌─────────────────────────┼─────────────────────────────┐
                    │                         │                             │
                    ▼                         ▼                             ▼
         ┌──────────────────┐     ┌─────────────────────┐     ┌────────────────────┐
         │  MOISTURE MGMT   │     │  CUDACLAW CHELIPED   │     │  REMODELING ENGINE │
         │  (state hydration)│     │  (execute + sense)   │     │  (shell mutation)  │
         │  Rehydrate cycle │     │  Probe → Assess → Go │     │  Overclock, expand │
         └────────┬─────────┘     └──────────┬──────────┘     └──────────┬─────────┘
                  │                          │                            │
                  └──────────────────────────┼────────────────────────────┘
                                             │
                                             ▼
                                  ┌─────────────────────┐
                                  │  MOLT CHAMBER        │
                                  │  (safe upgrade)      │
                                  │  Snapshot → Upgrade  │
                                  │  → Validate → Emerse │
                                  └─────────────────────┘
```

## 10.4 Impact on the Existing MVP Spec

The biological formalism modifies the existing PincherOS MVP Architecture Spec as follows:

### Database Schema Changes

```sql
-- Add to shells table:
ALTER TABLE shells ADD COLUMN species TEXT NOT NULL DEFAULT 'nassarius';  -- ShellSpecies enum
ALTER TABLE shells ADD COLUMN mutations TEXT NOT NULL DEFAULT '[]';  -- JSON: applied ShellMutations
ALTER TABLE shells ADD COLUMN cascade_eligible INTEGER NOT NULL DEFAULT 1;
ALTER TABLE shells ADD COLUMN exchange_trust REAL NOT NULL DEFAULT 0.5;
ALTER TABLE shells ADD COLUMN current_molt_stage TEXT;  -- NULL or 'preparing'/'molting'/'hardening'

-- New table: vacancy chain log
CREATE TABLE vacancy_chains (
    id              TEXT PRIMARY KEY,
    trigger_agent   TEXT NOT NULL,
    trigger_shell   TEXT NOT NULL,
    new_shell       TEXT NOT NULL,
    cascade_depth   INTEGER NOT NULL,
    resolved_at     TEXT,
    status          TEXT NOT NULL DEFAULT 'pending'  -- 'pending'/'resolved'/'aborted'
);

-- New table: shell mutations (the "remodeling history")
CREATE TABLE shell_mutations (
    id              TEXT PRIMARY KEY,
    shell_id        TEXT REFERENCES shells(id),
    rigging_id      TEXT NOT NULL,
    mutation_type   TEXT NOT NULL,    -- 'cpu_overclock'/'ram_disk'/'gpu_partition'/'thermal_override'/'io_boost'
    mutation_params TEXT NOT NULL,    -- JSON
    benefit         REAL,            -- Observed benefit after application
    cost            REAL,            -- Observed cost after application
    sticky          INTEGER NOT NULL DEFAULT 0,  -- Preserve across migrations?
    applied_at      TEXT NOT NULL DEFAULT (datetime('now')),
    reverted_at     TEXT
);

-- New table: moisture state
CREATE TABLE moisture_state (
    rigging_id      TEXT PRIMARY KEY,
    llm_cache_warmth      REAL NOT NULL DEFAULT 1.0,
    vector_index_health   REAL NOT NULL DEFAULT 1.0,
    jepa_readiness        REAL NOT NULL DEFAULT 0.0,
    embedding_cache_hit   REAL NOT NULL DEFAULT 0.0,
    reflex_compile_rate   REAL NOT NULL DEFAULT 0.0,
    last_rehydration      TEXT NOT NULL DEFAULT (datetime('now')),
    hydration_level       REAL NOT NULL DEFAULT 1.0
);

-- New table: molt history
CREATE TABLE molt_history (
    id              TEXT PRIMARY KEY,
    rigging_id      TEXT NOT NULL,
    upgrade_type    TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    completed_at    TEXT,
    status          TEXT NOT NULL DEFAULT 'preparing',
    recycled_reflexes INTEGER DEFAULT 0
);
```

### Snap Algorithm Enhancement

The `snap()` function must be extended to:
1. Read `shell_mutations` for the current shell and apply sticky mutations
2. Call `CUDAClaw.assess_shell_gpu()` (not just detect GPU presence)
3. Check `moisture_state` and trigger rehydration if needed
4. Emit a `ShellSignal` with the current shell species and fit score

### Core Loop Enhancement

The core loop (Input → Embed → Match → Execute → Store) must be wrapped with:

1. **Intertidal check**: Is cloud available? Adjust execution path accordingly.
2. **Moisture check**: Is state hydrated? If not, rehydrate before processing.
3. **Cheliped probe**: Before GPU-dependent operations, probe GPU state.
4. **Post-action moisture update**: Every action updates the moisture state.

---

# 11. THE DEEPEST INSIGHT: THE CRAB IS NOT THE AGENT

The most important insight from this biological analysis is one that the metaphor obscures:

**In hermit crab biology, the "crab" is not just the organism inside the shell. The crab is the entire crab-shell system.** The crab's behavior, capabilities, and survival depend on the shell it wears. A *C. clypeatus* in a *Strombus gigax* shell is a different animal than the same crab in a *Nassarius* shell — it can access different food sources, survive different predators, and interact differently with other crabs.

Similarly, in PincherOS, the "agent" is not just the rigging. It's the **rigging-shell composite**. The same rigging in a *Strombus gigax* workstation (70B model, full CUDA, Penrose tensors) is a fundamentally different agent than in a *Nassarius* Pi 4 (TinyLlama, CPU-only, flat vectors). The rigging carries identity, but the **capability is emergent from the rigging-shell fit**.

This means:
- **Migration is not just "moving the agent."** It's "creating a new agent." The rigging-shell composite changes.
- **The `.nail` file is not just "the agent's state."** It's the agent's state PLUS its preferred shell mutations PLUS its moisture requirements PLUS its molt history. The nail file is the crab's embodied memory of what it needs from a shell.
- **Fleet management is not just "provisioning agents."** It's "maintaining a population of rigging-shell composites in a healthy vacancy chain network." The fleet is an ecosystem, not a server farm.

The hermit crab doesn't say "I am the crab, and this is my shell." It says **"I am the crab-shell system, and I am seeking a better fit."** PincherOS should think the same way.

---

# REFERENCES

## Primary Biological Literature

1. Reese, E.S. (1969). "Behavioral adaptations of intertidal hermit crabs." *American Zoologist*, 9(2), 343-355.

2. Chase, I.D., Weissburg, M., & Dewit, T.H. (1988). "The vacancy chain process: a mechanism for resource acquisition in hermit crabs." *Ecology*, 69(4), 1074-1083.

3. Rotjan, R.D., Blomkalns, A.L., & Lewis, S.M. (2010). "Shell preference and motivation in hermit crabs." *Marine Ecology Progress Series*, 400, 183-191.

4. Hazlett, B.A. (1981). "The behavioral ecology of hermit crabs." *Annual Review of Ecology and Systematics*, 12, 1-22.

5. Greenaway, P. (2003). "Terrestrial adaptations in the Anomura (Crustacea: Decapoda)." *Memoirs of Museum Victoria*, 60(1), 13-26.

6. Laidre, M.E. (2011). "Ecology and evolution of the cracked shell: a case study of the terrestrial hermit crab *Coenobita compressus*." *Biological Journal of the Linnean Society*, 103(3), 544-554.

7. Briffa, M. & Elwood, R.W. (2007). "Fight or flight? Shell rapping as a dynamic assessment process during shell fights in hermit crabs." *Behavioral Ecology*, 18(5), 919-925.

8. Morrison, L.W. (2002). "Interspecific competition and coexistence between ants and land hermit crabs." *Oecologia*, 130(2), 246-254.

9. de Wilde, P.A.W.J. (1973). "On the ecology of *Coenobita clypeatus* in Curaçao." *Studies on the Fauna of Curaçao and other Caribbean Islands*, 44(1), 1-138.

## Secondary Sources (Review Articles)

10. Gherardi, F. (2011). "Ecology of decapod crustaceans." In *Ecology of Freshwater and Marine Systems*. Oxford University Press.

11. Lancaster, I. (1988). "Pagurus bernhardus (L.) — An introduction to the natural history of hermit crabs." *Field Studies*, 7, 189-238.

12. Harvey, A.W. (1998). "Genus *Coenobita* (Crustacea: Decapoda: Anomura)." In *Cenozoic Fossils*, 41-52.

## PincherOS Documents Referenced

13. PincherOS MVP Architecture Specification v0.1.0-mvp (internal)

14. PincherOS Master Research Synthesis (internal, June 2026)

15. PincherOS Academic & Technical Landscape Analysis 2024-2026 (internal)

16. PincherOS Open-Source Technology Landscape Survey 2024-2026 (internal)

---

*Document compiled: 2026-03-04*
*Author: Marine Biology / Ecological Systems Analysis*
*Classification: Engineering-Ready Architectural Specification*
