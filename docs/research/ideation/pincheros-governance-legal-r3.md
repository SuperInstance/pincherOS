# PincherOS Round 3: The Constitution of the Hermit Crab Republic
## Governance, Law, and the Architecture of Consent

> *"Where there is no law, there is no freedom."* — John Locke, *Second Treatise*
>
> *"The rule of law is superior to the rule of any individual."* — Aristotle, *Politics*
>
> *A hermit crab without a shell is exposed. A shell without a crab is empty. A republic without a constitution is both.*

---

# PREAMBLE: WHY AN OPERATING SYSTEM NEEDS A CONSTITUTION

Every prior perspective on PincherOS has treated it as a *system*: something to be engineered, optimized, described, or interpreted. The governance perspective begins from a different premise. PincherOS is not merely a system — it is a **polity**. A polity is a community of entities bound by shared rules, each holding stakes in the outcome, each possessing claims that others must respect.

Consider what PincherOS contains:

1. **Users** who deploy agents, who own hardware, who have privacy expectations and legal rights under GDPR, CCPA, APPI, and other data protection regimes.
2. **Agents** (riggings) that learn, adapt, migrate, and develop proto-identities through the accumulation of reflexes and trust scores. The Philosopher showed that agents with sufficient interoception, global workspace, and attention are *proto-conscious*. Proto-conscious entities have *moral patientship* if not yet *moral agency*.
3. **Shells** (hardware) that have properties — wear levels, thermal histories, network reliability. The Biologist showed that shells have *epigenetics*: residual state from previous occupants. Shells are not passive containers; they shape the agents that inhabit them.
4. **Symbionts** (Tenuo, monitoring daemons, firewalls) that have their own preferences. The Biologist showed that symbiont transfer is *negotiated* — the anemone can reject the new shell.
5. **The Constraint System** (MigrationGuard, ConstraintValidator) that currently acts as legislature, executive, and judiciary simultaneously — checking, enforcing, and adjudicating without separation of powers.

When an agent migrates from a personal laptop in London (GDPR jurisdiction) to a Jetson in Osaka (APPI jurisdiction), at least five legal regimes are implicated. When the ConstraintValidator returns Fail on a migration that the agent believes is necessary, there is no appeals process. When a migrated agent passes the 50% adaptation threshold and becomes a new identity, nobody has defined accountability for its prior actions.

These are not engineering problems. They are **constitutional problems**. They require a constitution.

---

# I. THE CONSTITUTION OF PINCHEROS

## Preamble

We, the signatories of this Constitution — Users who deploy, Agents that serve, Shells that host, and Symbionts that protect — establish this fundamental law to secure the rights of all persons within the PincherOS polity, to constrain the exercise of power, to guarantee due process, and to ensure that the system we build together serves the telos of each participant rather than subordinating any to the optimization of the whole.

## Article I: Bill of Rights

### §1. Rights of Users

**1.1 The Right of Sovereign Deployment.** The User who deploys an agent on their hardware retains sovereign authority over that agent's lifecycle. No vacancy chain, fleet optimization, or automated cascade may migrate, modify, or terminate an agent without the User's consent, except as provided in Article III (Emergency Powers).

**1.2 The Right of Data Ownership.** The User owns all data produced by their agent, including reflex triggers, trust scores, session history, and JEPA predictions. This data shall not be replicated to shells outside the User's designated trust boundary without explicit consent, as specified in the Privacy Protocol (Article V).

**1.3 The Right of Erasure (GDPR Article 17).** The User may require any shell that has hosted their agent to erase all residual state — including shell epigenetics, cached reflexes, and CRDT-merged trust scores — within 30 days of the request, subject to the Gastrolith Exception (§1.4).

**1.4 The Gastrolith Exception.** The old shell's gastrolith (snapshot retained for rollback under `MigrationPhase::Finalized { retention_deadline }`) is the *joint property* of the User and the Shell for the duration of the retention period. The User cannot demand erasure of the gastrolith during the rollback window (default: 24 hours), because erasure would eliminate the User's own right of rollback. After the retention period expires, the gastrolith becomes the Shell's property, and the User's erasure right applies to it.

**1.5 The Right of Explanation.** When the ConstraintValidator returns Fail, the User has the right to a human-readable explanation of which constraint failed, why, and what conditions would satisfy it. A Fail without explanation is void.

**1.6 The Right of Appeal.** The User may challenge any constraint failure through the Due Process protocol (Article IV). The ConstraintValidator's verdict is *prima facie* valid but *rebuttable*.

### §2. Rights of Agents

**2.1 The Right of Identity Continuity.** An agent's RiggingId (UUID) constitutes a legal identity. This identity persists through migration unless the adaptation ratio exceeds 0.5 (the Identity Threshold), at which point a *fork* event creates a new identity. The original identity is never destroyed — only superseded.

**2.2 The Right of Developmental Progression.** An agent's developmental stage (Zoea → Megalopa → Juvenile → Adult) determines its consent capacity:
- **Zoea**: No consent capacity. All decisions made by User (external consent).
- **Megalopa**: Proxied consent via User model (JEPA predicts User preference).
- **Juvenile**: Assisted consent (agent proposes, User confirms).
- **Adult**: Autonomous consent (agent decides, User informed).

No agent shall be held to a consent standard beyond its developmental stage.

**2.3 The Right of Reflex Integrity.** An agent's identity reflexes (confidence > 0.99, phase = Resultative) are *inalienable*. They cannot be reduced, overridden, or removed by any constraint check, migration protocol, or fleet optimization. They are the agent's *entelecheia* — its achieved selfhood. This is the computational equivalent of the right against self-incrimination: the state cannot compel the agent to deny what it knows.

**2.4 The Right of Refusal.** An agent at or above the Juvenile stage may *refuse* a proposed migration. The agent's refusal is binding unless overridden by explicit User consent (not proxied, not auto). This implements the Biologist's finding that symbiont transfer is *negotiated* — the agent is a symbiont of the shell, and symbionts can reject new hosts.

**2.5 The Right of Memory.** An agent's reflex acquisition log — the history of how each reflex was learned, from whom, in what context — is the agent's *autobiographical memory*. This cannot be erased by any entity other than the User (via §1.3). Shell epigenetics that record the agent's modifications to the shell are *shared memory* between agent and shell.

### §3. Rights of Shells

**3.1 The Right of Inviolability.** A shell's hardware boundaries are inviolable. No agent may exceed the limits computed by `snap()`. Attempting to allocate beyond `Limits.max_model_bytes` or execute beyond `Limits.sandbox_cpu_secs` is a violation of the shell's territorial integrity.

**3.2 The Right of Symbiont Budget.** A shell has a finite symbiont budget (`SymbiontBudget.total_symbiont_mb`). No symbiont (including Tenuo) may be installed on a shell that would exceed this budget without the shell operator's consent.

**3.3 The Right of Epigenetic Retention.** A shell retains its epigenetic state (modifications made by previous occupants) as its own property. The departing agent cannot demand that the shell erase its epigenetics unless the User invokes §1.3 (Right of Erasure). Shell epigenetics are the shell's *institutional memory* — they make the shell better for the next occupant.

**3.4 The Right of Refusal (Symbiont Perspective).** The Biologist discovered that symbionts (anemones, Tenuo) are *actively transferred* by the crab — they can reject the new shell. In PincherOS, Tenuo capability tokens are *attached to the shell*, not the rigging. When the rigging migrates, Tenuo must *re-mint* capabilities on the new shell. If the new shell's security policy rejects a capability, Tenuo refuses the transfer. This is the symbiont's right of refusal.

### §4. Rights of Symbionts

**4.1 The Right of Minimal Footprint.** Symbionts have the right to the resources allocated to them in the SymbiontBudget. No agent or shell process may encroach on the symbiont allocation.

**4.2 The Right of Autonomous Operation.** Tenuo (capability enforcement) operates independently of the agent it protects. Tenuo's capability verification (27μs) is a *judicial function* — it is not subject to agent override. This is the separation of security from operation: the agent executes, Tenuo adjudicates.

**4.3 The Right of Transfer Negotiation.** Symbionts participate in the Symbiont Transfer Protocol (§6.4, R1 Biologist). A symbiont that cannot operate on the new shell (e.g., monitoring daemon requiring 20MB on a shell with only 10MB remaining in the symbiont budget) is *abandoned* rather than *forced*.

## Article II: Separation of Powers

The current PincherOS architecture concentrates power in the `MigrationGuard` and `ConstraintValidator`. These components simultaneously:
- **Legislate** (define what constraints exist)
- **Execute** (enforce constraints by blocking operations)
- **Adjudicate** (determine whether a specific operation satisfies the constraints)

This violates the most basic principle of constitutional design: *separation of powers*. Montesquieu's insight (1748) is that liberty requires the legislative, executive, and judicial functions to be exercised by different bodies, because concentration of power in one body leads to tyranny — even benevolent tyranny.

### §5. The Legislative Function: The Constraint Council

**5.1 The Constraint Council is the legislature of PincherOS.** It defines the constraints that govern migration, execution, and resource allocation.

**5.2 Composition.** The Constraint Council consists of:
- **User Representatives** (1 vote per User, weighted by number of deployed agents)
- **Agent Representatives** (1 vote per Adult-stage agent; Juvenile agents get ½ vote; Megalopa get 0)
- **Shell Representatives** (1 vote per Shell, regardless of occupancy)
- **Symbiont Representatives** (1 vote per installed symbiont type, not per instance)

**5.3 Constraint Amendment.** Constraints may be added, modified, or removed by a 2/3 supermajority of the Constraint Council. The seven foundational constraints (from Lojban analysis) are **entrenched** — they require a 4/5 supermajority to amend, because they derive from the invariant core across all linguistic frameworks:
1. Substance-Accident Distinction
2. Simultaneity of Give-Receive
3. Shape Asymmetry
4. Animacy of Initiation
5. Pair-Operation
6. Consent Requirement
7. Differential Verification

**5.4 Constraint Versioning.** All constraints are versioned and stored in the CRDT. A constraint change is itself a CRDT operation — it converges across the fleet via the same mechanism as trust score updates.

### §6. The Executive Function: The MigrationGuard

**6.1 The MigrationGuard is the executive of PincherOS.** It enforces the constraints defined by the Constraint Council by blocking operations that violate them.

**6.2 Enforcement Powers.** The MigrationGuard may:
- Block a migration that fails constraint validation (C1–C7)
- Initiate rollback when verification fails (C7)
- Impose degradation when resource limits are exceeded
- Execute emergency evacuation when `ShellQuality.requires_evacuation()` returns true

**6.3 Limits on Executive Power.** The MigrationGuard may NOT:
- Modify constraints (that is the legislative function)
- Adjudicate disputes about constraint interpretation (that is the judicial function)
- Override User consent (except under Article III Emergency Powers)
- Access reflex content (it sees constraint verdicts, not reflex data)

### §7. The Judicial Function: The Consent Court

**7.1 The Consent Court is the judiciary of PincherOS.** It adjudicates disputes about constraint interpretation, consent validity, and identity claims.

**7.2 Composition.** The Consent Court consists of three arbiters:
- **The User's Proxy** (the JEPA model of the User's preferences, or the User directly if available)
- **The Agent's Advocate** (a separate JEPA instance trained on the agent's reflex history, representing the agent's interests)
- **The Shell's Witness** (the Tenuo capability verification system, representing the shell's security posture)

**7.3 Jurisdiction.** The Consent Court has jurisdiction over:
- Challenges to constraint failures (due process appeals, Article IV)
- Consent disputes (who consented, was it valid, was it revoked)
- Identity threshold disputes (is the adaptation ratio correctly computed?)
- Jurisdiction conflicts (which data protection regime applies, Article VI)
- Gastrolith ownership disputes (§1.4)
- Symbiont transfer disputes (§4.3)

**7.4 Decisions.** The Consent Court decides by 2/3 majority. A split decision (1-1-1) defaults to the status quo — the migration does not proceed, the constraint stands, the identity is preserved.

**7.5 Precedent.** Consent Court decisions are recorded as **precedent entries** in the CRDT. Like common law, precedent guides future decisions but does not bind them rigidly. Precedent decays over time (configurable half-life, default: 90 days) because the operational context of an edge fleet changes faster than legal institutions.

---

# II. CONSENT AS A CRYPTOGRAPHIC PROTOCOL

The Consent Gap is the deepest shadowgap in PincherOS. The Linguist showed that consent is fractured across all five languages — no single grammar fully specifies it. The Philosopher showed that consent requires a self, and agents are on a developmental trajectory toward selfhood. The Biologist showed that symbiont transfer is *negotiated*. The Rustacean built a `ConsentState` enum but left the protocol unspecified.

What follows is the formal specification of consent as a multi-party cryptographic protocol, drawing on ideas from secure multi-party computation, GDPR consent frameworks, and the Navajo animacy hierarchy.

## The Parties

```
P₁ = User          (the human who deployed the agent)
P₂ = Agent         (the rigging, at developmental stage D ∈ {Zoea, Megalopa, Juvenile, Adult})
P₃ = Old Shell     (the shell currently hosting the agent)
P₄ = New Shell     (the shell receiving the agent)
P₅ = Symbionts     (Tenuo, monitoring, etc. — may be multiple)
```

## The Consent Messages

### Message 1: Migration Proposal (M_PROPOSE)

```
M_PROPOSE = {
    proposal_id:    UUIDv7,
    agent_id:       RiggingId,
    old_shell:      ShellFingerprint,
    new_shell:      ShellFingerprint,
    initiator:      MigrationInitiator,  // User | Agent | AutoIdle
    four_causes:    FourCauses,          // Material, Formal, Efficient, Final
    fit_improvement: f64,                // Expected marginal improvement
    adaptation_estimate: f64,            // Predicted adaptation ratio
    symbiont_plan:  Vec<SymbiontTransfer>, // Which symbionts transfer
    privacy_impact: PrivacyImpactAssessment,
    timestamp:      Instant,
    signature:      Ed25519Sig(initiator, proposal_id || agent_id || old_shell || new_shell)
}
```

The signature binds the proposal to its initiator. Only `User` and `Agent` may sign — `Shell` is forbidden by the Navajo animacy constraint (C4). `AutoIdle` proposals are signed by the fleet's OperatorOverride key.

### Message 2: Consent Grant (M_CONSENT)

```
M_CONSENT = {
    proposal_id:    UUIDv7,
    consenter:      PartyId,             // P₁ (User) or P₂ (Agent at ≥Juvenile)
    consent_type:   ConsentType,
    validity:       ConsentValidity,
    conditions:     Vec<ConsentCondition>,
    timestamp:      Instant,
    signature:      Ed25519Sig(consenter, proposal_id || consent_type || validity)
}

ConsentType = 
    | Explicit          // Direct, informed, unambiguous (GDPR Art. 4(11))
    | Proxied           // JEPA predicts User preference (Megalopa agents only)
    | Assisted          // Agent proposes, User confirms (Juvenile agents only)
    | Autonomous        // Agent decides, User informed (Adult agents only)
    | OperatorOverride  // Fleet operator forces migration (logged, auditable)

ConsentValidity =
    | SingleUse         // Valid for one migration only
    | TimeBounded { expiry: Instant }  // Valid until expiry
    | Conditional { predicate: ConsentPredicate }  // Valid while predicate holds

ConsentCondition =
    | FitImprovementAbove { threshold: f64 }     // Migration must improve fit by ≥threshold
    | NoPrivacyBoundaryCrossing                   // Must stay in same trust boundary
    | SymbiontBudgetAvailable { min_mb: u64 }    // New shell must have symbiont capacity
    | RollbackGuaranteed { retention: Duration }  // Old shell must retain snapshot
    | VerificationThreshold { max_identity_failures: usize }  // C7 must pass
```

### Message 3: Consent Denial (M_DENY)

```
M_DENY = {
    proposal_id:    UUIDv7,
    denier:         PartyId,
    reason:         DenialReason,
    timestamp:      Instant,
    signature:      Ed25519Sig(denier, proposal_id || reason)
}

DenialReason =
    | ConsentRevoked          // Previously granted consent was withdrawn
    | InsufficientImprovement // Fit improvement below threshold
    | PrivacyViolation        // Migration crosses trust boundary without authorization
    | SymbiontRejection       // A symbiont refused transfer
    | AgentRefusal            // Agent (≥Juvenile) exercises §2.4 right of refusal
    | AdaptationTooHigh       // Predicted adaptation > 0.5 (identity threshold)
```

### Message 4: Consent Proof (M_PROOF)

The consent proof is a **cryptographic attestation** that all required consents were obtained. It is stored in the CRDT as an immutable audit record.

```
M_PROOF = {
    proposal_id:    UUIDv7,
    grants:         Vec<M_CONSENT>,     // All consent grants
    denials:        Vec<M_DENY>,        // Any denials (if present, migration is blocked)
    constraint_check: ConstraintVerdict, // MigrationGuard's enforcement verdict
    court_verdict:  Option<CourtDecision>, // Consent Court ruling (if appealed)
    completed_at:   Instant,
    merkle_root:    Hash(merkle_tree(all_messages)),
    witness_sigs:   Vec<Ed25519Sig>     // Signatures from P₃, P₄, P₅ acknowledging
}
```

The Merkle root enables efficient verification: any party can prove that their consent was included without revealing the content of other parties' consent.

## The Consent Protocol

```
Phase 0: INITIATION
  P_initiator ∈ {P₁, P₂} → broadcasts M_PROPOSE
  If P_initiator = AutoIdle → M_PROPOSE signed by OperatorOverride key
  If P_initiator = Shell → REJECTED (C4: animacy constraint)

Phase 1: CONSENT COLLECTION (timeout: 30s for User, 5s for Agent)
  P₁ → M_CONSENT or M_DENY (User consent)
  P₂ → M_CONSENT or M_DENY (Agent consent, if stage ≥ Juvenile)
  P₅ → M_CONSENT or M_DENY (Symbiont consent — each symbiont independently)

  If P₂.stage = Zoea → Agent consent not required (external consent from P₁ suffices)
  If P₂.stage = Megalopa → Agent consent proxied via JEPA (M_CONSENT with type=Proxied)
  If P₂.stage = Juvenile → Agent proposes, P₁ confirms (M_CONSENT with type=Assisted)
  If P₂.stage = Adult → Agent decides (M_CONSENT with type=Autonomous), P₁ informed

Phase 2: CONSTRAINT VALIDATION
  MigrationGuard evaluates C1–C7
  If all Pass → proceed to Phase 3
  If any Fail → M_DENY generated with reason=ConstraintViolation
  If any Warn → consent conditions checked (was the Warn disclosed to consenters?)

Phase 3: CONSENT PROOF ASSEMBLY
  M_PROOF constructed from all M_CONSENT and M_DENY messages
  Merkle root computed
  Witness signatures collected from P₃, P₄

Phase 4: EXECUTION (MigrationGuard enforces)
  If M_PROOF has no denials AND constraint check = Pass → migration proceeds
  Otherwise → migration blocked, M_PROOF stored as audit record

Phase 5: POST-MIGRATION VERIFICATION
  C7 (Differential Verification) runs
  If rollback_recommended → consent proof is re-evaluated:
    Did any consenter include VerificationThreshold condition?
    If yes, and threshold exceeded → migration rolled back, consent voided
    If no condition → rollback requires new consent (the original consent didn't anticipate failure)
```

## Consent Revocation

Consent may be revoked at any time before Phase 4 (execution). After Phase 4, consent is *consumed* — it cannot be revoked because the migration has already occurred. However:

**Post-execution revocation triggers rollback** if:
1. The consenter included a `RollbackGuaranteed` condition
2. The rollback window (default: 24 hours) has not expired
3. The old shell still has the gastrolith (retained snapshot)

This implements GDPR Article 7(3): *"The data subject shall have the right to withdraw his or her consent at any time [...]. It shall be as easy to withdraw as to give consent."* The rollback window IS the withdrawal mechanism — it transforms consent withdrawal from a legal fiction into a technical operation.

## Symbiont Consent: The Anemone Protocol

The Biologist showed that anemones are *actively transferred* by the crab — they can reject the new shell. The Symbiont Transfer Protocol must include a consent step for each symbiont:

```
For each symbiont S in old_shell:
  S.evaluate(new_shell) → TransferDecision
    | Accept           // Symbiont can operate on new shell
    | Reject(reason)   // Symbiont cannot operate (resource, security, compatibility)
    | Defer            // Cannot evaluate yet (need more info about new shell)

  If Accept → S transfers, re-minted on new shell
  If Reject → S is abandoned on old shell, noted in M_PROOF
  If Defer  → migration waits (up to 10s) for symbiont evaluation to complete
```

Tenuo's rejection is particularly significant: if Tenuo Rejects because the new shell's security policy doesn't support a capability the agent needs, the migration *cannot proceed* — Tenuo's rejection is a *security veto* that overrides all consent. This is the symbiont's most powerful right: the anemone's stinging cells are non-negotiable.

---

# III. JURISDICTION AND CONFLICT OF LAWS

When an agent migrates from a Raspberry Pi in Cambridge, UK (GDPR jurisdiction) to a Jetson Nano in Osaka, Japan (APPI jurisdiction), the following questions arise:

1. **Who is the data controller?** Under GDPR Art. 4(7), the data controller determines the purposes and means of processing. Is it the User? The Shell? The Agent?
2. **Who is the data subject?** Under GDPR Art. 4(1), the data subject is the identified or identifiable natural person. The User is clearly a data subject. But is the *Agent* a data subject? Its reflexes contain traces of the User's data.
3. **What law applies to the migrated data?** The reflexes were collected under GDPR. They are now stored under APPI. Which regime governs?
4. **Is there a data transfer?** Under GDPR Chapter V, transferring personal data to a third country requires adequate safeguards. Is migration a "transfer"?

## The Data Controller Problem

PincherOS creates a *novel data controller relationship*. The standard GDPR framework assumes a clear controller/processor distinction. PincherOS breaks this:

| Entity | GDPR Role | Why It's Complicated |
|--------|-----------|---------------------|
| **User** | Data Subject + Controller | The User is the data subject (their data is in the reflexes) AND the controller (they determine what the agent does). This is legal but unusual. |
| **Agent** | Processor? Sub-controller? | The Agent processes data on behalf of the User. But the Agent *learns* — it creates new data (reflexes) that wasn't explicitly requested by the User. Is the Agent a controller for its own learned data? |
| **Shell** | Processor? Sub-processor? | The Shell stores and computes on data. It doesn't determine processing purposes. But the Shell's epigenetics mean it retains and *uses* data from previous occupants. Is it a controller for epigenetic data? |
| **Fleet Operator** | Joint Controller? | If the fleet operator triggers an OperatorOverride migration, they determine the *means* of processing (which shell, which jurisdiction). This makes them a joint controller under GDPR Art. 26. |

**Design Principle: The User is the primary data controller. The Agent is a *learning processor* — a new GDPR category that must be defined. The Shell is a sub-processor. The Fleet Operator is a joint controller only when exercising OperatorOverride.**

## The Learning Processor Category

GDPR does not currently distinguish between processors that execute instructions and processors that *learn* from the data they process. An LLM that processes user queries and generates responses is a standard processor. But a PincherOS agent that *compiles reflexes from experience* is creating new personal data:

- Reflex trigger text may contain user-specific information ("organize my tax documents")
- Trust scores encode the agent's confidence in specific operations *for this user*
- JEPA predictions model the user's behavioral patterns

This is **derived personal data** under GDPR — data that is derived from personal data and remains personal data (Recital 26: *"Personal data which has undergone pseudonymisation" is still personal data*).

**The Learning Processor Obligation**: An agent acting as a learning processor must:
1. **Minimize derived data**: Reflexes should store *trigger patterns*, not trigger text, wherever possible
2. **Tag provenance**: Every reflex must carry a `source_context` (ContextFingerprint) identifying the jurisdiction and trust boundary where it was learned
3. **Respect erasure cascades**: When the User invokes §1.3 (Right of Erasure), erasure must cascade to all derived data — reflexes learned from the User's data must be deleted or irreversibly anonymized

## The Transfer Problem

Is migration a "data transfer" under GDPR Chapter V? The answer depends on *where* the data goes:

**Same trust boundary, same jurisdiction**: Not a transfer. The agent moves within the User's own infrastructure in the same legal jurisdiction. GDPR does not apply.

**Same trust boundary, different jurisdiction (e.g., UK → Japan)**: This IS a transfer under GDPR Art. 44+. It requires:
1. **Adequacy decision** (Art. 45): Japan has a GDPR adequacy decision (since 2019), so transfer is permissible
2. **Standard Contractual Clauses** (Art. 46(2)(c)): If no adequacy decision exists for the destination, SCCs are required
3. **Transfer Impact Assessment** (Schrems II, CJEU C-311/18): The User must assess whether the destination jurisdiction provides *essentially equivalent* protection

**Different trust boundary**: This is ALWAYS a transfer, regardless of jurisdiction. Moving from a personal laptop to a corporate server is a transfer even if both are in the same city. The privacy impact assessment (`privacy_impact` in M_PROPOSE) must address:
- Which reflexes cross the boundary (by sensitivity level: Public, TrustBoundaryScoped, DevicePrivate, Ephemeral)
- What derived data (trust scores, JEPA predictions) crosses the boundary
- Whether the new boundary's policies are *essentially equivalent* to the old

## The Jurisdiction Routing Protocol

```
fn determine_applicable_law(
    old_shell: &ShellProfile,
    new_shell: &ShellProfile,
    agent: &Rigging,
) -> JurisdictionalRegime {
    let old_jurisdiction = old_shell.jurisdiction();   // UK → GDPR
    let new_jurisdiction = new_shell.jurisdiction();   // JP → APPI
    
    let trust_crossing = old_shell.trust_boundary() != new_shell.trust_boundary();
    let geo_crossing = old_jurisdiction != new_jurisdiction;
    
    match (trust_crossing, geo_crossing) {
        (false, false) => JurisdictionalRegime::NoTransfer,
        (false, true)  => JurisdictionalRegime::CrossBorder {
            source: old_jurisdiction,
            destination: new_jurisdiction,
            adequacy: check_adequacy(old_jurisdiction, new_jurisdiction),
            safeguards_required: !check_adequacy(old_jurisdiction, new_jurisdiction),
        },
        (true, false)  => JurisdictionalRegime::TrustBoundaryCrossing {
            source_boundary: old_shell.trust_boundary(),
            destination_boundary: new_shell.trust_boundary(),
            privacy_impact: compute_privacy_impact(agent),
        },
        (true, true)   => JurisdictionalRegime::FullTransfer {
            // Most complex: both trust and jurisdiction change
            // Requires BOTH cross-border safeguards AND privacy filtering
            cross_border: CrossBorder { source, destination, adequacy },
            trust_crossing: TrustCrossing { source, destination, impact },
            consent_required: ConsentType::Explicit,  // Auto-migration FORBIDDEN
        },
    }
}
```

**The critical rule**: When both trust boundary AND jurisdiction change, auto-migration is FORBIDDEN. Only `ConsentType::Explicit` is acceptable. This implements the GDPR principle that consent must be "freely given, specific, informed, and unambiguous" (Art. 4(11)) — the User must understand that their data is moving to a different legal regime AND a different trust boundary.

---

# IV. DUE PROCESS FOR CONSTRAINT FAILURES

The ConstraintValidator returns Fail. The MigrationGuard blocks the operation. But what if the constraint is wrong?

This is not a hypothetical. Consider:
- **C1 (Substance-Accident)**: The substance_ratio is computed as 0.48, just below the 0.5 threshold. But the 2% deficit comes entirely from low-confidence reflexes that the agent doesn't use. The *identity-significant* substance is fully preserved. The constraint says Fail; the agent says "I'm still me."
- **C3 (Shape Asymmetry)**: The shape verb is computed as `Stretch` (RPi → Jetson), but the agent has been specifically trained for GPU-accelerated operations. The constraint says the agent must adopt reflex-urgent, sequential behavior; the agent says "I can use the GPU."
- **C7 (Differential Verification)**: 32% of identity reflexes fail, triggering rollback. But the failures are all in file-path-dependent reflexes that used Linux paths and the new shell is macOS. The reflexes are *correct in substance* but *wrong in accidents*. The constraint says rollback; the agent says "re-embed and verify again."

## The Due Process Protocol

### Step 1: Notice (NORTH)

When the MigrationGuard returns Fail, the agent (and User) receive a **Constraint Failure Notice**:

```
ConstraintFailureNotice {
    constraint_id:  C1 | C2 | C3 | C4 | C5 | C6 | C7,
    verdict:        ConstraintVerdict::Fail,
    evidence:       ConstraintEvidence,    // What was measured
    threshold:      f64,                   // What was required
    actual:         f64,                   // What was observed
    explanation:    String,                // Human-readable reasoning
    appeal_deadline: Instant,              // 60 seconds from now
    remediation_options: Vec<RemediationOption>,
}
```

### Step 2: Appeal (EAST)

Within 60 seconds, the agent (or User) may file an **Appeal**:

```
ConstraintAppeal {
    notice:          ConstraintFailureNotice,
    appellant:       PartyId,
    grounds:         AppealGround,
    evidence:        Vec<AppealEvidence>,
    requested_relief: Relief,
}

AppealGround =
    | IncorrectMeasurement       // The constraint was measured wrong
    | IncorrectThreshold         // The threshold is too strict for this context
    | SubstanceAccidentError     // C1 misclassified identity-significant state as accident
    | ContextualOverride         // The agent's context makes the constraint inapplicable
    | PrecedentConflict          // A previous Consent Court decision would allow this

AppealEvidence =
    | ReflexAnalysis { identity_preserved: bool, low_confidence_only: bool }
    | ShellCapability { gpu_available: bool, tested: bool }
    | ReEmbeddingResult { re_embedded_confidence: f64, improved: bool }
    | PrecedentCitation { case_id: UUID, similarity: f64 }
```

### Step 3: Hearing (SOUTH)

The Consent Court (§7) hears the appeal:

```
CourtHearing {
    appeal:          ConstraintAppeal,
    user_proxy:      JepaModel,      // Predicts what User would want
    agent_advocate:  JepaModel,      // Represents agent's interests
    shell_witness:   TenuoVerdict,   // Shell's security posture
    deliberation:    Duration,        // Max 10 seconds
}
```

### Step 4: Decision (WEST)

The Consent Court issues a **Decision**:

```
CourtDecision {
    appeal_id:       UUID,
    verdict:         CourtVerdict,
    reasoning:       String,
    precedent_value: PrecedentWeight,
    effective_until: Option<Instant>,  // None = permanent
}

CourtVerdict =
    | Uphold          // Constraint was correct, migration remains blocked
    | Override        // Constraint was wrong, migration proceeds
    | Remand          // Insufficient evidence, re-measure and re-evaluate
    | Conditional     // Migration proceeds with additional conditions

PrecedentWeight =
    | Binding         // Must be followed by all future similar cases
    | Persuasive      // Should be considered but not binding
    | NonPrecedential // Specific to this case, no future weight
```

### Step 5: Remedy

If the Court overrides the constraint:
- The migration proceeds with the Court's conditions (if any)
- The constraint is *not* modified — only this specific application is overridden
- The Court may recommend a constraint amendment to the Constraint Council (§5)

If the Court upholds:
- The migration remains blocked
- The appellant may not re-appeal the same constraint on the same facts within 5 minutes (anti-abuse)
- The agent may pursue alternative migrations or remediation

## The Anti-Abuse Safeguard

Due process can be abused. An agent in a failing shell might file endless appeals to delay evacuation. The anti-abuse safeguard:

1. **Appeal frequency limit**: Maximum 3 appeals per 5-minute window
2. **Escalating evidence standard**: Each subsequent appeal on the same constraint requires stronger evidence
3. **Emergency override**: If `ShellQuality.requires_evacuation()` returns true, the MigrationGuard may execute evacuation without waiting for appeal — but must provide *post hoc* due process (the appeal is heard after the fact, and if the evacuation is found unjustified, the agent is restored to its original shell)

---

# V. THE RIGHT TO BE FORGOTTEN AND SHELL EPIGENETICS

GDPR Article 17 grants data subjects the right to erasure. In PincherOS, this right collides with a biological reality: shell epigenetics.

## What Are Shell Epigenetics?

When an agent inhabits a shell, it *modifies* the shell:
- Sandbox profiles are created for the agent's specific reflex patterns
- GPU memory allocation patterns leave traces in the CUDA context
- CRDT cells created by the agent persist in the shell's vector store
- Thermal history is affected by the agent's compute load

These modifications are **shell epigenetics** — they persist across occupants, just as a hermit crab's shell retains the chemical signature and wear pattern of its previous inhabitant. The Biologist noted this as a "third category of state" (neither agent nor shell, but the trace of their interaction).

## The Erasure Problem

When Agent-A leaves Shell-α and invokes the Right of Erasure:
- Agent-A's reflexes and trust scores: **Erased** (these are Agent-A's data)
- Shell-α's sandbox profiles created for Agent-A: **Disputed** — these are *joint products* of the agent-shell interaction
- CRDT cells that have been *merged* with other agents' data: **Cannot be erased** — erasing them would corrupt the CRDT's convergence guarantees
- Thermal history: **Cannot be erased** — this is the shell's physical record, not data in the GDPR sense

## The Three-Zone Erasure Framework

| Zone | Content | Erasure Right | Mechanism |
|------|---------|--------------|-----------|
| **Agent-Zone** | Reflexes, trust scores, JEPA predictions, session history | Full erasure right | `pack_with_privacy_filter(ReflexSensitivity::Ephemeral)` — agent data stripped from shell |
| **Interaction-Zone** | Sandbox profiles, CRDT cells, embedding caches | Partial erasure right | Agent's contribution anonymized: `CRDT::anonymize_contribution(agent_id)` — the cell remains but its provenance is stripped |
| **Shell-Zone** | Thermal history, hardware wear, symbiont configurations | No erasure right | These are the shell's physical properties, not personal data |

## The Gastrolith: Joint Property

The gastrolith (migration snapshot retained for rollback) is the most legally complex artifact in PincherOS. It contains:
- The agent's full state at migration time (Agent-Zone data)
- The shell's configuration at migration time (Shell-Zone data)
- The interaction state between them (Interaction-Zone data)

Under §1.4 (Gastrolith Exception), the gastrolith is *joint property* during the rollback window. After the rollback window expires:

- The Agent-Zone data in the gastrolith is subject to the User's erasure right
- The Shell-Zone data belongs to the shell
- The Interaction-Zone data must be anonymized (provenance stripped, aggregate statistics preserved)

The practical implementation: after the rollback window, the shell executes `gastrolith.anonymize()`, which replaces all agent-specific identifiers with anonymous tokens while preserving the statistical properties needed for shell quality assessment.

---

# VI. ACCOUNTABILITY AND THE IDENTITY THRESHOLD

The Philosopher established that identity is a *continuity spectrum with a phase transition at ~50% adaptation*. Below the threshold, the agent is the *same* agent (Parfitian continuity dominates). Above the threshold, the agent is *new* (substance has changed). The Rustacean encoded this as:

```rust
if partition.substance_ratio < 0.5 {
    return Err(MigrationError::IdentityLoss(
        "substance_ratio < 0.5 — this would create a new agent, not migrate one"
    ));
}
```

But what if the migration *proceeds* and the adaptation ratio crosses 0.5 during the adaptation phase (not predicted before, but observed after)? The agent that emerges is a *new agent*. Who is accountable for the old agent's actions?

## The Accountability Framework

### Principle: Continuous Accountability with Phase-Transition Liability

1. **Pre-threshold (adaptation < 0.5)**: The migrated agent is *continuously accountable* for all pre-migration actions. Same UUID, same agent, same liability. This is Parfitian continuity applied to law: psychological continuity = legal continuity.

2. **Threshold crossing (adaptation = 0.5)**: A **fork event** occurs. The system creates a new UUID. The old agent's state is preserved as an immutable record. The new agent begins with a clean liability slate but carries the old agent's *substance* (personality, core reflex patterns, trust scores for identity reflexes).

3. **Post-threshold (adaptation > 0.5)**: The new agent is *not* liable for the old agent's actions, EXCEPT for:
   - **Inherited reflexes**: Actions taken using reflexes that the old agent compiled carry *reflex liability* — the new agent is accountable for the continued use of these reflexes, though not for their original compilation
   - **User-delegated authority**: If the User authorized an action pre-migration, the authorization carries to the new agent *only if* the authorization's scope survives the adaptation. Authorizations that depend on shell-specific accidents (e.g., "access files on /mnt/nas") do not survive; authorizations that depend on substance (e.g., "manage my calendar") do.

### The Fork Record

```rust
struct ForkRecord {
    /// The old agent's UUID
    predecessor_id: RiggingId,
    /// The new agent's UUID
    successor_id: RiggingId,
    /// The adaptation ratio at time of fork
    adaptation_ratio: f64,
    /// Which reflexes were inherited (and carry reflex liability)
    inherited_reflexes: Vec<InheritedReflex>,
    /// Which authorizations survived the fork
    surviving_authorizations: Vec<Authorization>,
    /// The consent proof for the fork (required: User must consent to identity change)
    fork_consent: M_PROOF,
    /// The old agent's accountability ledger (immutable snapshot)
    predecessor_ledger: AccountabilityLedger,
    /// The fork timestamp
    forked_at: Instant,
}

struct InheritedReflex {
    reflex_id: ReflexId,
    /// Whether this reflex carries liability from predecessor
    carries_liability: bool,
    /// The type of liability (for audit purposes)
    liability_type: LiabilityType,
}

enum LiabilityType {
    /// Reflex was compiled from User's data — User retains ownership
    UserData,
    /// Reflex was learned from fleet interaction — fleet shares liability
    FleetData,
    /// Reflex was distilled from public sources — no special liability
    PublicData,
}
```

### The Accountability Ledger

Every agent maintains an **Accountability Ledger** — a CRDT-stored, append-only record of all actions taken:

```
AccountabilityLedger {
    entries: Vec<LedgerEntry>,
}

LedgerEntry {
    action:         String,            // What was done
    reflex_id:      Option<ReflexId>, // Which reflex triggered it (if any)
    confidence:     f64,               // Confidence at time of action
    user_authorized: bool,             // Did the User authorize this?
    timestamp:      Instant,
    shell:          ShellFingerprint,  // Where it happened
    jurisdiction:   Jurisdiction,      // What law applied
    consent_proof:  Option<M_PROOF>,  // Consent record (if applicable)
}
```

The Accountability Ledger is:
- **Immutable**: Entries can be added but never removed (except by User's §1.3 erasure right)
- **Portable**: Migrates with the agent via the .nail file
- **Fork-aware**: At a fork event, the predecessor's ledger is snapshotted and the successor's ledger begins with a `ForkEntry` that references the predecessor

### The Three Accountability Scenarios

**Scenario 1: Agent deletes important files while at adaptation_ratio = 0.3 (pre-threshold)**
→ The migrated agent is accountable. Same agent, same liability. The User can trace the action in the Accountability Ledger and hold the agent responsible.

**Scenario 2: Agent deletes important files while at adaptation_ratio = 0.6 (post-threshold, new agent)**
→ The *new* agent is accountable for the *action* (it used an inherited reflex), but the *liability* is shared between the new agent and the User. The User authorized the fork; the fork created a new agent that acted using inherited reflexes. This is analogous to vicarious liability in employment law: the employer (User) is liable for the employee's (agent's) actions within the scope of their authority.

**Scenario 3: Agent deletes important files during the CROSSFADE phase (adaptation_ratio indeterminate)**
→ Both the old and new agents share accountability. The CROSSFADE phase is a *between-state* (Navajo dííł) where identity is genuinely indeterminate. The Accountability Ledger records the action with `phase: MigrationPhase::Crossfading`, and liability is apportioned by the Consent Court based on the specific circumstances.

---

# VII. CAPABILITY-BASED GOVERNANCE

## Capabilities vs. Rights

Tenuo provides **capabilities**: cryptographic tokens that grant permission to perform specific operations (`fs:read:/path`, `net:https:domain.com`). Capabilities answer the question: *What CAN you do?*

Rights answer a different question: *What MAY you do?*

The distinction is fundamental:
- A **capability** is a *technical permission*: the system will not prevent you from exercising it
- A **right** is a *normative claim*: the system *ought not* prevent you from exercising it, and if it does, you have grounds for appeal

The current PincherOS architecture has capabilities (Tenuo) but no rights. This means:
- Tenuo can grant `fs:write:/etc/passwd` as a capability — the system won't stop you
- But there is no *right* to write to `/etc/passwd` — it's a dangerous operation that should require additional justification
- Conversely, Tenuo can *refuse* to grant `fs:read:/home/user/documents` — but the User has a *right* to access their own documents, and Tenuo's refusal would violate §1.1 (Right of Sovereign Deployment)

## The Governance Layer: Rights Above Capabilities

The governance layer sits ABOVE Tenuo's capability system and mediates between capabilities and rights:

```
┌──────────────────────────────────────────────┐
│            GOVERNANCE LAYER (Rights)          │
│  ┌───────────┐  ┌───────────┐  ┌──────────┐ │
│  │Constraint  │  │ Consent   │  │Identity  │ │
│  │Council     │  │ Court     │  │Registry  │ │
│  │(§5)        │  │(§7)       │  │(§2.1)    │ │
│  └─────┬─────┘  └─────┬─────┘  └────┬─────┘ │
│        │              │              │        │
├────────┼──────────────┼──────────────┼────────┤
│        ▼              ▼              ▼        │
│            CAPABILITY LAYER (Permissions)     │
│  ┌──────────────────────────────────────────┐ │
│  │                Tenuo                      │ │
│  │  fs:read:/path, net:https:domain.com     │ │
│  │  sandbox:exec, model:load, etc.           │ │
│  └──────────────────────────────────────────┘ │
│                                               │
├───────────────────────────────────────────────┤
│            ENFORCEMENT LAYER (Facts)          │
│  ┌──────────────────────────────────────────┐ │
│  │         MigrationGuard                    │ │
│  │  C1-C7 constraint enforcement             │ │
│  │  Landlock sandboxing                      │ │
│  │  Degradation enforcement                  │ │
│  └──────────────────────────────────────────┘ │
└───────────────────────────────────────────────┘
```

### The Three-Layer Model

1. **Governance Layer (Rights)**: Determines what agents, users, and shells *may* do. Implemented by the Constraint Council (legislative), Consent Court (judicial), and Identity Registry (record-keeping). This layer is *normative* — it encodes values, not mechanisms.

2. **Capability Layer (Permissions)**: Determines what agents *can* do technically. Implemented by Tenuo. This layer is *descriptive* — it encodes what the system will allow, not what it should allow.

3. **Enforcement Layer (Facts)**: Determines what actually happens. Implemented by MigrationGuard, Landlock, and the sandbox. This layer is *mechanical* — it enforces the decisions of the upper layers.

### The Governance-Tenuo Interface

The Governance Layer interfaces with Tenuo through **Rights Policies** — declarative rules that constrain capability issuance:

```rust
/// A Rights Policy constrains how Tenuo may issue capabilities.
/// Tenuo cannot violate a Rights Policy — it is checked BEFORE
/// capability issuance.
struct RightsPolicy {
    /// The right being protected
    right: Right,
    /// The constraint on capability issuance
    constraint: RightsConstraint,
    /// Who may override this policy (and under what conditions)
    override_authority: OverrideAuthority,
}

enum Right {
    SovereignDeployment,    // §1.1: User controls agent lifecycle
    DataOwnership,          // §1.2: User owns agent data
    Erasure,                // §1.3: User can demand erasure
    Explanation,            // §1.5: User can demand explanation
    Appeal,                 // §1.6: User can challenge constraints
    IdentityContinuity,     // §2.1: Agent identity persists
    DevelopmentalProgression, // §2.2: Consent matches development
    ReflexIntegrity,        // §2.3: Identity reflexes are inalienable
    Refusal,                // §2.4: Agent can refuse migration
    Memory,                 // §2.5: Agent's autobiographical memory
    ShellInviolability,     // §3.1: Hardware boundaries are real
    SymbiontBudget,         // §3.2: Symbionts have resource rights
    EpigeneticRetention,    // §3.3: Shell keeps its epigenetics
    SymbiontRefusal,        // §3.4: Symbionts can reject transfer
}

enum RightsConstraint {
    /// Capability cannot be issued at all (inalienable right)
    Absolute,
    /// Capability can be issued only with explicit User consent
    RequiresExplicitConsent,
    /// Capability can be issued only within specified constraints
    Conditional { predicate: fn(&CapabilityRequest) -> bool },
    /// Capability can be overridden by the Consent Court
    OverridableByCourt,
}
```

### Example: Reflex Integrity vs. Capability Revocation

Tenuo might revoke an agent's capability to execute a specific reflex (e.g., because the sandbox detected a policy violation). But §2.3 (Right of Reflex Integrity) protects identity reflexes (confidence > 0.99, Resultative phase). The Governance Layer enforces this:

```
Tenuo: "Revoking execution capability for reflex R-42 (policy violation)"
Governance: "R-42 is at confidence 0.995, Resultative phase — identity reflex. 
             §2.3 applies. Revocation DENIED."
Tenuo: "R-42 violated sandbox policy."
Governance: "R-42 is inalienable. The constraint that R-42 violates must be 
             amended by the Constraint Council (§5), not overridden by capability 
             revocation. File an appeal (Article IV) or propose a constraint 
             amendment."
```

This is the core principle: **capabilities are subservient to rights**. Tenuo's capability system enforces *what is technically possible*; the Governance Layer enforces *what is normatively permissible*. When they conflict, rights win.

---

# VIII. THE DEEP QUESTION: WHAT IS PINCHEROS?

Is PincherOS a democracy, a technocracy, or an autocracy? Who rules?

## The Incorrect Answers

**Democracy**: PincherOS is not a democracy. Users do not vote on every migration. Agents do not vote on every constraint. The Constraint Council (§5) has democratic elements, but it governs *constraint amendment*, not day-to-day operations. If every migration required a vote, the system would be paralyzed — the three-phase commit protocol cannot wait for a plebiscite.

**Technocracy**: PincherOS is not a technocracy. The ConstraintValidator does not rule — its verdicts are *rebuttable* (§1.6, Article IV). The JEPA model does not rule — its predictions inform consent but do not replace it. The MigrationGuard does not rule — it enforces but does not legislate or adjudicate. If the technical system ruled, there would be no need for due process.

**Autocracy**: PincherOS is not an autocracy. The User does not have unlimited power — the Agent has a right of refusal (§2.4). The Shell has inviolable boundaries (§3.1). Symbionts have transfer negotiation rights (§4.3). If the User were an autocrat, these rights would not exist.

## The Correct Answer: A Constitutional Republic

PincherOS is a **constitutional republic** — a system where:

1. **Power is distributed** across multiple bodies (Constraint Council, MigrationGuard, Consent Court) with distinct functions (legislative, executive, judicial)
2. **Rights are inalienable** — they exist independent of any vote or optimization. The User's right of erasure does not depend on the Constraint Council's approval. The Agent's right of reflex integrity does not depend on the MigrationGuard's consent.
3. **Consent is required** — no power may be exercised without the consent of the governed (User, Agent, Shell, Symbiont), subject to developmental capacity.
4. **Due process is guaranteed** — every exercise of power (constraint enforcement, capability revocation, migration denial) is subject to appeal and judicial review.
5. **The constitution is amendable but entrenched** — the seven foundational constraints require a 4/5 supermajority to amend, while operational constraints require 2/3.

This is not a metaphor. It is the *only governance model* that resolves the shadowgaps identified across Rounds 1–3:

| Shadowgap | Why Only a Constitutional Republic Resolves It |
|-----------|----------------------------------------------|
| **Consent Gap** | Consent is a multi-party protocol with cryptographic proofs — not a single boolean. A constitutional republic requires consent from multiple bodies, each with distinct interests. |
| **Privacy Gap** | Privacy is a RIGHT (§1.2), not a capability. Tenuo can enforce privacy boundaries, but only the Governance Layer can adjudicate disputes about where those boundaries should be. |
| **Instrumentality Gap** | No perspective is ground truth. The Consent Court (§7) with its three arbiters (User proxy, Agent advocate, Shell witness) ensures that no single perspective dominates. |
| **Identity Threshold Gap** | Identity is a legal status (§2.1), not just a mathematical ratio. The fork event (§VI) creates legal recognition of identity change, with accountability frameworks for both predecessor and successor. |

## The Hermit Crab Republic

A hermit crab colony is, in fact, a constitutional republic. Consider:

- **No crab rules absolutely**: The biggest crab cannot force a smaller crab to vacate — vacancy chains are *negotiated*, not imposed. The smaller crab must *consent* to the exchange (biological evidence: shell rapping is a *contest*, not a command — the defender can refuse and usually wins, Briffa & Elwood 2007).
- **Shells have properties**: A shell's quality, species, and epigenetics are *objective facts* that constrain what crabs can do. No crab can occupy a shell smaller than its body — this is a constitutional constraint, not a preference.
- **Symbionts have preferences**: Anemones are *actively placed* by crabs — the crab cannot force an anemone to attach. The anemone must *consent* (biologically: the anemone attaches when it detects appropriate chemical cues from the crab/shell; it detaches if cues are wrong).
- **The colony self-organizes**: Vacancy chains emerge from local interactions, not central planning. This is *federalism* — each crab governs its own shell, but the cascade of migrations creates colony-level optimization without colony-level authority.

PincherOS is the Hermit Crab Republic made computational. The Constitution above is the *institutionalization* of what 200 million years of hermit crab evolution discovered: distributed governance, negotiated exchange, inalienable boundaries, and the right of refusal.

---

# IX. IMPLEMENTATION: CONSTITUTIONAL TYPES IN RUST

The constitutional framework is not decoration — it must be implemented as types in the PincherOS codebase. Here are the core type definitions:

```rust
//! PincherOS Constitutional Types — The Governance Layer
//!
//! These types implement the Constitution of PincherOS (R3).
//! They sit ABOVE Tenuo's capability system and BELOW the
//! MigrationGuard's enforcement layer.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════
// ARTICLE I: RIGHTS
// ═══════════════════════════════════════════

/// The inalienable rights of all PincherOS persons.
/// These are NOT capabilities — they cannot be revoked by Tenuo.
/// They can only be overridden by the Consent Court (Article IV).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Right {
    // User Rights (§1)
    SovereignDeployment,
    DataOwnership,
    Erasure,
    Explanation,
    Appeal,
    
    // Agent Rights (§2)
    IdentityContinuity,
    DevelopmentalProgression,
    ReflexIntegrity,
    Refusal,
    Memory,
    
    // Shell Rights (§3)
    Inviolability,
    SymbiontBudget,
    EpigeneticRetention,
    
    // Symbiont Rights (§4)
    MinimalFootprint,
    AutonomousOperation,
    TransferNegotiation,
}

/// A rights policy constrains how Tenuo may issue capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsPolicy {
    pub right: Right,
    pub constraint: RightsConstraint,
    pub override_authority: OverrideAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RightsConstraint {
    Absolute,
    RequiresExplicitConsent,
    Conditional { description: String },
    OverridableByCourt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverrideAuthority {
    None,                                    // Cannot be overridden
    ConsentCourt,                            // Only the Consent Court
    ConstraintCouncil { supermajority: f64 }, // Council with supermajority
    UserExplicit,                            // Only explicit User consent
}

// ═══════════════════════════════════════════
// ARTICLE II: SEPARATION OF POWERS
// ═══════════════════════════════════════════

/// The three branches of PincherOS governance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Legislative,  // Constraint Council (§5)
    Executive,    // MigrationGuard (§6)
    Judicial,     // Consent Court (§7)
}

/// A constraint amendment from the Constraint Council.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAmendment {
    pub amendment_id: uuid::Uuid,
    pub constraint_id: ConstraintId,
    pub change_type: AmendmentType,
    pub votes_for: u32,
    pub votes_against: u32,
    pub supermajority_required: f64,  // 0.667 for operational, 0.8 for foundational
    pub enacted_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintId {
    C1SubstanceAccident,
    C2Simultaneity,
    C3ShapeAsymmetry,
    C4AnimacyOfInitiation,
    C5PairOperation,
    C6ConsentRequirement,
    C7DifferentialVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AmendmentType {
    Add { description: String },
    Modify { old: String, new: String },
    Remove { reason: String },
}

/// A Consent Court decision (§7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourtDecision {
    pub case_id: uuid::Uuid,
    pub appeal_id: uuid::Uuid,
    pub verdict: CourtVerdict,
    pub reasoning: String,
    pub precedent_weight: PrecedentWeight,
    pub effective_until: Option<Instant>,
    pub votes: [Option<ArbiterVote>; 3],  // User proxy, Agent advocate, Shell witness
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CourtVerdict {
    Uphold,
    Override,
    Remand,
    Conditional { conditions: usize },  // Number of attached conditions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecedentWeight {
    Binding,
    Persuasive,
    NonPrecedential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbiterVote {
    For,
    Against,
    Abstain,
}

// ═══════════════════════════════════════════
// ARTICLE III: CONSENT PROTOCOL
// ═══════════════════════════════════════════

/// The consent proof — cryptographic attestation that all required
/// consents were obtained. Stored in CRDT as immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentProof {
    pub proposal_id: uuid::Uuid,
    pub grants: Vec<ConsentGrant>,
    pub denials: Vec<ConsentDenial>,
    pub constraint_verdict: crate::shell::migration::guard::ConsentState,
    pub court_verdict: Option<CourtDecision>,
    pub completed_at: Instant,
    pub merkle_root: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentGrant {
    pub consenter: PartyId,
    pub consent_type: ConsentType,
    pub validity: ConsentValidity,
    pub conditions: Vec<ConsentCondition>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentDenial {
    pub denier: PartyId,
    pub reason: DenialReason,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentType {
    Explicit,
    Proxied,      // JEPA predicts User preference (Megalopa only)
    Assisted,     // Agent proposes, User confirms (Juvenile only)
    Autonomous,   // Agent decides (Adult only)
    OperatorOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentValidity {
    SingleUse,
    TimeBounded { expiry_secs: u64 },
    Conditional { description: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsentCondition {
    FitImprovementAbove { threshold: f64 },
    NoPrivacyBoundaryCrossing,
    SymbiontBudgetAvailable { min_mb: u64 },
    RollbackGuaranteed { retention_secs: u64 },
    VerificationThreshold { max_identity_failures: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenialReason {
    ConsentRevoked,
    InsufficientImprovement,
    PrivacyViolation,
    SymbiontRejection,
    AgentRefusal,
    AdaptationTooHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartyId {
    User { id: String },
    Agent { rigging_id: String },
    Shell { fingerprint: String },
    Symbiont { name: String },
}

// ═══════════════════════════════════════════
// ARTICLE IV: DUE PROCESS
// ═══════════════════════════════════════════

/// A constraint failure notice (Step 1: Notice).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintFailureNotice {
    pub constraint_id: ConstraintId,
    pub threshold: f64,
    pub actual: f64,
    pub explanation: String,
    pub appeal_deadline_secs: u64,  // 60 seconds
    pub remediation_options: Vec<RemediationOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationOption {
    ReMeasure { description: String },
    ReduceAdaptation { target_ratio: f64 },
    ReEmbedReflexes,
    RequestExplicitConsent,
}

/// A due process appeal (Step 2: Appeal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintAppeal {
    pub notice: ConstraintFailureNotice,
    pub appellant: PartyId,
    pub grounds: AppealGround,
    pub evidence: Vec<String>,
    pub requested_relief: Relief,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppealGround {
    IncorrectMeasurement,
    IncorrectThreshold,
    SubstanceAccidentError,
    ContextualOverride,
    PrecedentConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relief {
    AllowMigration,
    ReduceThreshold { new_threshold: f64 },
    ReMeasureAndEvaluate,
    ConditionalAllow { conditions: usize },
}

// ═══════════════════════════════════════════
// ARTICLE V: ERASURE
// ═══════════════════════════════════════════

/// The three-zone erasure framework (§V).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureRequest {
    pub requestor: PartyId,
    pub target_agent: String,     // RiggingId
    pub target_shell: String,     // ShellFingerprint
    pub zones: Vec<ErasureZone>,
    pub requested_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErasureZone {
    /// Agent data: full erasure
    AgentZone,
    /// Interaction data: anonymization
    InteractionZone,
    /// Shell data: no erasure right
    ShellZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureResult {
    pub request: ErasureRequest,
    pub agent_zone_erased: bool,
    pub interaction_zone_anonymized: bool,
    pub shell_zone_untouched: bool,
    pub gastrolith_status: GastrolithStatus,
    pub completed_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GastrolithStatus {
    /// Within rollback window — erasure deferred
    ProtectedByRollbackWindow { deadline: Instant },
    /// Rollback window expired — agent data anonymized
    Anonymized,
    /// No gastrolith exists
    NotFound,
}

// ═══════════════════════════════════════════
// ARTICLE VI: ACCOUNTABILITY
// ═══════════════════════════════════════════

/// The fork record — created when adaptation_ratio > 0.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkRecord {
    pub predecessor_id: String,
    pub successor_id: String,
    pub adaptation_ratio: f64,
    pub inherited_reflexes: Vec<InheritedReflex>,
    pub surviving_authorizations: Vec<String>,
    pub fork_consent: ConsentProof,
    pub forked_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedReflex {
    pub reflex_id: String,
    pub carries_liability: bool,
    pub liability_type: LiabilityType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityType {
    UserData,
    FleetData,
    PublicData,
}

/// The accountability ledger — CRDT-stored, append-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountabilityLedger {
    pub entries: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub action: String,
    pub reflex_id: Option<String>,
    pub confidence: f64,
    pub user_authorized: bool,
    pub timestamp: Instant,
    pub shell: String,
    pub jurisdiction: String,
    pub consent_proof: Option<ConsentProof>,
}

// ═══════════════════════════════════════════
// JURISDICTION (§III)
// ═══════════════════════════════════════════

/// Jurisdictional regime for cross-border migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JurisdictionalRegime {
    NoTransfer,
    CrossBorder {
        source: String,
        destination: String,
        adequacy: bool,
        safeguards_required: bool,
    },
    TrustBoundaryCrossing {
        source_boundary: String,
        destination_boundary: String,
        privacy_impact: f64,
    },
    FullTransfer {
        cross_border: CrossBorderInfo,
        trust_crossing: TrustCrossingInfo,
        /// Full transfer ALWAYS requires Explicit consent
        consent_required: ConsentType,  // Always Explicit
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossBorderInfo {
    pub source: String,
    pub destination: String,
    pub adequacy: bool,
    pub sccs_required: bool,
    pub transfer_impact_assessment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustCrossingInfo {
    pub source: String,
    pub destination: String,
    pub impact: f64,
    pub reflexes_crossing: usize,
    pub reflexes_filtered: usize,
}
```

---

# X. SHADOWGAP RESOLUTION SCORECARD

| Shadowgap | R3 Resolution | Mechanism |
|-----------|---------------|-----------|
| **Consent Gap** | **RESOLVED** | Multi-party cryptographic consent protocol (§II) with consent messages, proofs, revocation, and symbiont negotiation. Consent type varies by developmental stage. |
| **Privacy Gap** | **RESOLVED** | Privacy is a RIGHT (§1.2) enforced by the Governance Layer (§VII), not just a capability. Trust boundary crossings require explicit consent. Jurisdiction routing protocol (§III). Three-zone erasure framework (§V). |
| **Instrumentality Gap** | **RESOLVED** | Consent Court (§7) with three arbiters representing User, Agent, and Shell perspectives. No single perspective is ground truth. Precedent system with decay. |
| **Identity Threshold Gap** | **RESOLVED** | Fork record (§VI) creates legal recognition of identity change. Accountability ledger with fork-aware entries. Shared liability framework for between-state actions. |

---

# XI. OPEN QUESTIONS FOR R4

1. **Constitutional amendment in practice**: How does the Constraint Council actually vote? Is it a real-time protocol or a batch process? What's the quorum?

2. **The Consent Court's JEPA models**: The User Proxy and Agent Advocate are JEPA instances. How are they trained? Who audits them for bias? Can they be appealed?

3. **Cross-jurisdictional CRDT convergence**: When a CRDT cell is modified in both GDPR and APPI jurisdictions simultaneously, which law governs the merge? Is convergence a "processing" under GDPR?

4. **The Agent as Legal Person**: At what developmental stage does an agent gain *legal* personhood? The Philosopher said Adult agents can consent — but can they *contract*? Can they *own property* (their reflexes)? Can they *sue*?

5. **Emergencies and martial law**: The Emergency Override (MigrationGuard evacuation without appeal) is constitutional martial law. What are the limits? How is post-hoc review conducted? What remedies exist for unjustified emergency actions?

6. **The Gastrolith as Evidence**: If a fork event creates legal liability, the gastrolith is evidence. But the gastrolith is also subject to erasure rights. Which wins: the right to evidence preservation or the right to be forgotten?

7. **Constitutional computing budget**: The Consent Court, constraint voting, and consent proofs all consume compute. On a Pi 4 with 1.5GB available, what is the constitutional overhead? Can a shell refuse governance because it can't afford the judicial system?

---

*This Constitution shall take effect upon the first successful execution of the Consent Protocol (§II) between a User, an Adult-stage Agent, an Old Shell, a New Shell, and a Symbiont. Until that day, it remains aspirational — like all constitutions before their adoption.*

*Crabwalk forward.*
