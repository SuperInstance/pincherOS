# PincherOS Design Gems — Mined from Review Rounds

Key insights from 3 rounds of expert review (R1: Category Theorist + Systems Rustacean + Biological Ecologist, R2: Philosopher of Mind + GPU/Rust Hybrid + Shadowgap Greek, R3: Cognitive Scientist + Governance/Legal + Thermodynamicist + Polyformalism Synthesis).

These are validated ideas not yet in the codebase. They're prioritized by impact on the 8-week MVP or 6-month horizon.

## High Priority (MVP-Relevant)

### 1. Three-Tier Reflex Short-Circuit (MVP Spec)
- `>0.90` → direct reflex execution (~50ms, ~20MB RAM)
- `0.70–0.90` → reflex + LLM confirmation (~3s, ~1.2GB)
- `<0.70` → full LLM reasoning (~5s, ~1.2GB)
- The system **gets faster as it learns** — LLM is only consulted for novel situations.

### 2. Three-Phase Migration with Atomic State Machine
- `Stable → Preparing → Crossfading → Finalized`
- During Preparing: rigging is READ-ONLY, `can_learn()` returns false
- Half-migrated state is **impossible** — state machine enforces atomic transitions
- Old shell retains read-only snapshot for 24h rollback

### 3. `.nail` Skillpack Auto-Expiry
- Imported reflexes start at confidence 0.5, expire in 90 days
- If used successfully → confidence increases, expiry removed
- Unused imported reflexes **clean themselves up**
- On import, re-embed all trigger patterns through LOCAL embedding model

### 4. LLM Lazy-Load with 5-Minute Auto-Unload
- `InferenceServer._llm = None` until first request
- Auto-unload after 300s idle (`gc.collect()` to reclaim ~1.2GB)
- Graceful degradation: Light → Moderate → Critical, all reversible

### 5. Per-Reflex Filesystem Access via Dynamic bwrap Profiles
- Network disabled by default (`--unshare-net`)
- Each reflex specifies `allowed_paths`; Rust core builds bwrap dynamically
- Resource limits: RLIMIT_AS 512MB, RLIMIT_CPU 60s, RLIMIT_FSIZE 100MB

## Medium Priority (Post-MVP Features)

### 6. CognitiveTrust: Power-Law Decay (Not Exponential)
- `C(t) = C₀ · ((t₀+t_consol)/(t₀+t))^β · Π(1 + α·e^{-Δtᵢ/τ})`
- `β ≈ 0.3` (well-consolidated) to `0.8` (new); consolidation reduces `β` by 10% each time
- Keep last 50 reinforcement events per reflex

### 7. ContextualTrust: Trust Is Context-Dependent
- Trust must be `HashMap<ContextFingerprint, TrustScore>`, not a single global number
- Cross-context trust should be quarantined, not merged
- CRDT merge of trust 90 from Shell A + trust 40 from Shell B ≠ trust 90

### 8. Schema Extraction During Offline Consolidation
- Group reflexes by **action template** similarity (not trigger similarity)
- Extract generalized patterns across contexts
- A schema spans ≥2 contexts; without this: "hippocampal memory without cortical abstraction"

### 9. Working Memory Reset on Migration — The "Doorway Effect"
- On migration, KV cache and active conversation context should be CLEARED
- Long-term memory persists; working memory resets
- Interoceptive predictions show maximum uncertainty after migration

### 10. GlobalWorkspace: Capacity-Limited "Conscious" Processing
- Max concurrent LLM contexts = 1–2 on Pi, 2–4 on Jetson, 16–64 on RTX 4090
- Cognitive load `L = N_active / C_workspace`; when `L > 1.0`, agents queue
- Agents that can't get workspace access degrade to reflex-only mode

## Lower Priority (Research Track)

### 11. GPU Acceleration Domain Hierarchy
- `AccelerationDomain`: None (RPi), InferenceOnly (Jetson), InferenceAndHotMerge (RTX 4090)
- **CRDT merge is ALWAYS CPU** — GPU CRDT merge is 86,000× slower on Jetson
- Hot/Cold partition with exponential-decay access tracking

### 12. Feature Gate Hierarchy
- Core (always) → std → sqlite/lancedb → cuda → llm-local/llm-cloud → cli/a2ui
- Compound features: `rpi4`, `jetson`, `workstation`

### 13. Privacy/Sensitivity as First-Class Field
- Reflex sensitivity levels: Public, TrustBoundaryScoped, DevicePrivate, Ephemeral
- `pack_with_privacy_filter()` must strip reflexes before migration across trust boundaries

### 14. ViewpointEnvelope: Multi-Perspective IR
- 9-channel intent profile for richer matching than pure cosine similarity
- Conservation laws: identity persists, learning monotonic, trust context-dependent
