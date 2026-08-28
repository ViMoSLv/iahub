# ADR-0011: Orchestrator Delegation Model

## Status

**AMENDED / ACCEPTED AS AMENDED** (2026-08-28)

Originally accepted as part of Topic 04 scaffold. Amended after architectural
audit revealed dual-lifecycle conflict between `Delegation` aggregate and
existing `Attempt` entity.

## Context

The Mega Brain orchestrator must dispatch work to external agents. This requires:

1. Selecting an agent based on capabilities, not hardcoded provider/model names
2. Providing immutable, versioned context so decisions are auditable
3. Compiling prompts as artifacts, not inline coordination state
4. Defining explicit authority boundaries workers cannot exceed
5. Separating worker self-reports from independent verification evidence
6. Supporting stop conditions that force escalation over improvisation

The initial implementation introduced a `Delegation` aggregate with its own
identity (`DelegationId`) and lifecycle (`DelegationStatus`). However, the
architecture already possesses `Attempt` as the identity of a concrete execution
trial. Having both created two entities competing for the same semantic ground:
which agent executes, retry semantics, report/verification, and terminal state.

## Decision

### Amended Architecture

**`Attempt` remains the sole execution identity.** There is no independent
`Delegation` aggregate. Instead, dispatch-related data is modeled as immutable
value objects attached to an Attempt:

```
Task
  └── Attempt (execution identity)
        ├── DispatchSpec (immutable value object)
        │     ├── attempt_id
        │     ├── task_id
        │     ├── agent_id
        │     ├── authority_scope
        │     ├── stop_conditions
        │     ├── context_snapshot_id
        │     └── compiled_prompt_id
        ├── ContextSnapshot (versioned, immutable)
        ├── CompiledPrompt (regenerable artifact)
        ├── WorkerReport (claimed state — NOT trusted)
        └── VerificationEvidence (independent observation)
```

### Key Type Changes

| Before | After | Rationale |
|--------|-------|-----------|
| `DelegationId` (newtype) | Removed | No independent identity needed |
| `DelegationStatus` (state machine) | Removed | Duplicate lifecycle; Attempt owns state |
| `AgentCapability` (enum) | `CapabilityId(String)` | Data-driven; new capabilities without recompilation |
| `AuthorityScope.can_merge` | Removed | Workers NEVER merge (INV-013) |
| `VerificationEvidence.observed_status: String` | `VerificationOutcome` enum | Typed; Unknown is first-class |

### Why Not a Second Lifecycle?

`Task → Attempt` already represents a concrete execution trial with full
lifecycle management (creation, transitions, retry, terminal states). Adding
`Delegation` as a parallel aggregate would introduce:

- **Dual authority**: Which entity owns the "current state" of execution?
- **State drift**: Delegation says EXECUTING but Attempt says FAILED
- **Identity confusion**: Retry creates Attempt-2 + Delegation-9 — which is canonical?
- **Invariant complexity**: Every invariant touching execution must now reason about two entities

A value object (`DispatchSpec`) captures all dispatch intent without these costs.
It is created once per Attempt, never mutated, and carries no lifecycle of its own.

### Preserved Design Principles

These principles from the original ADR remain valid and are now enforced through
the amended types:

- **Prompts are artifacts** (INV-037): `CompiledPrompt` is a regenerable artifact
  with content hash, compiler version, and snapshot reference
- **Versioned context** (INV-038): `ContextSnapshot` captures project/architecture/
  invariant/ADR revisions at dispatch time
- **Capability-based selection** (INV-039): `CapabilityId` is data-driven
- **Authority boundaries** (INV-040): `AuthorityScope` explicitly lists what
  workers can/cannot do; out-of-authority triggers stop condition
- **Verified completion** (INV-041): Only `VerificationEvidence` with typed
  `VerificationOutcome` can authorize certified completion
- **Worker ≠ Verifier**: `WorkerReport` and `VerificationEvidence` are distinct
  types that cannot be accidentally interchanged

## Consequences

### Positive

- Single source of truth for execution state (`Attempt`)
- No dual-lifecycle synchronization bugs possible
- New agent capabilities deployable via manifests, not Core recompilation
- Authority scope cannot grant impossible permissions (no `can_merge`)
- Verification outcomes are typed, not free-text strings
- All dispatch data is immutable and auditable

### Negative

- Code referencing `DelegationId` or `DelegationStatus` must be updated
- Callers that expected delegation lifecycle events must use Attempt events instead
- Migration of any persisted delegation records to Attempt-attached value objects

### Neutral

- `DispatchSpec` is always created alongside an Attempt; they share fate
- Context snapshots and compiled prompts remain domain-only until Scheduler topic

## Risks

- **Risk**: Callers may still think in terms of "delegations" rather than "dispatch specs"
  - **Mitigation**: Documentation, type system enforcement, removed old types entirely
- **Risk**: Data-driven `CapabilityId` loses compile-time exhaustiveness checking
  - **Mitigation**: Known capabilities exposed as constants; runtime validation at boundaries
- **Risk**: Removing `DelegationStatus` means losing granular dispatch-phase tracking
  - **Mitigation**: Attempt state machine covers execution phases; dispatch is instantaneous

## References

- INV-005: Reported state ≠ observed reality
- INV-006: Workers cannot certify their own success
- INV-007: Unknown remains unknown
- INV-013: Workers never merge target branch
- INV-037: Prompts are compiled artifacts
- INV-038: Dispatch requires versioned context snapshot
- INV-039: Capability-based agent selection
- INV-040: Out-of-authority must stop and escalate
- INV-041: Verified evidence for certified completion