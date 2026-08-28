# ADR-0011: Orchestrator Delegation Model

## Status
ACCEPTED

## Context
Mega Brain V0 must coordinate multiple coding agents without treating prompts as coordination state or allowing workers to self-certify completion. Early architectural analysis revealed that conflating task semantics with prompt text, or trusting worker self-reports, leads to non-deterministic failures, audit gaps, and inability to replay or re-delegate work when models, templates, or strategies change.

Key constraints:
- The Hub is the source of truth for coordination state (INV-009).
- Reported state is not observed reality (INV-005).
- Workers cannot certify their own success (INV-006).
- Task identity survives retries, reviews, and agent replacement (INV-015).
- Architectural decisions belong to the Orchestrator/Hub, not execution workers.
- Future versions must support model swapping, prompt template evolution, and capability-based agent selection without altering task semantics.

## Decision
We separate the orchestration pipeline into five distinct, durably-stored concepts:

### 1. Task ≠ Prompt
Tasks carry structured semantic information (objective, scope, constraints, invariants, dependencies, acceptance criteria, required evidence). Prompts are **compiled artifacts** produced from tasks + context snapshots by a dedicated PromptCompiler component. Changing prompt templates or switching models never alters task semantics.

### 2. Delegation Entity
A Task may be delegated multiple times (retries, reviews, recovery). Each delegation records:
- `task_id` — which task
- `agent_id` — which agent (selected by capability, not hardcoded name)
- `authority_scope` — explicit CAN/CANNOT boundaries
- `stop_conditions` — conditions requiring escalation
- `context_snapshot_id` — immutable reference to versioned context
- `compiled_prompt_id` — reference to the prompt artifact sent to the agent
- `worker_report` — what the agent claimed (never trusted as truth)
- `verification_evidence` — independent observational evidence

### 3. Context Snapshot (Versioned)
Every delegation references an immutable snapshot capturing project revision, architecture revision, invariant revision, ADR revision, and schema version at creation time. This enables future audit: "with what context did this agent make this decision?"

### 4. Worker Report vs Verification Evidence
Worker reports (`reported_status`, `summary`, `claims`) are stored separately from verification evidence (`observed_status`, `checks_passed`, `violations`). Only verified evidence can authorize transitions to DONE.

### 5. Capability-Based Agent Selection
Agents declare capabilities (code-generation, code-review, architecture-review, research, testing, security-review, etc.). The Orchestrator matches task requirements against capabilities. Agent model names are implementation details, not selection criteria.

### 6. Stop Conditions
Every delegation carries explicit stop conditions. When encountered, the worker MUST stop and escalate (`STOPPED_NEEDS_DECISION`) rather than improvise architectural decisions.

### 7. Authority Boundaries
Each delegation has an explicit `AuthorityScope` defining permitted paths, denied paths, and boolean flags for file creation/deletion, command execution, architecture modification, and merging. Violations are failures.

## Consequences

### Positive
- Prompts are regenerable artifacts; changing templates or models does not corrupt task semantics.
- Every delegation is fully auditable via context snapshot reference.
- Worker self-certification is structurally impossible (separate report vs evidence types).
- Agent selection is decoupled from specific model names.
- Out-of-authority actions are caught by stop conditions before damage occurs.
- Task retry/review/recovery creates new delegations without mutating original task spec.
- Future PromptCompiler implementations can evolve independently of domain model.

### Negative
- Additional entity types increase schema complexity (future migration v0002+).
- Context snapshot creation adds overhead per delegation.
- Capability registry requires maintenance as agents evolve.
- PromptCompiler becomes a critical component requiring its own testing strategy.
- Developers must think in five concepts instead of "send prompt to agent".

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Context snapshot bloat | Snapshots are content-addressed; identical contexts share storage |
| Capability list becomes stale | Periodic health checks validate declared capabilities against observed behavior |
| PromptCompiler produces bad prompts | Compiler version tracked; rollback to previous version possible |
| Stop conditions too restrictive cause excessive escalation | Configurable per-task; human override path for edge cases |
| Authority scope too coarse for complex tasks | Hierarchical path patterns; future refinement with glob/regex support |
| Delegation proliferation makes audit difficult | Index by task_id + created_at; retention policy for old delegations |

## New Invariants
This ADR introduces five new invariants (INV-037 through INV-041):
- INV-037: Prompts are compiled artifacts, not coordination state
- INV-038: Every delegation must reference a versioned context snapshot
- INV-039: Agent selection is capability-based, not hardcoded
- INV-040: Workers encountering out-of-authority decisions must stop and escalate
- INV-041: Only verified evidence may transition work to certified DONE

## Implementation Phasing
Domain types are created immediately (Topic 04 hardening). Schema tables, runtime logic, and PromptCompiler implementation are deferred to appropriate future topics. No existing behavior is altered.

## Related
- INV-005: Reported not observed
- INV-006: No self-certification
- INV-013: No agent merge
- INV-015: Task survives lifecycle
- INV-016: Done requires evidence
- ADR-0005: Observed vs Reported State
- ADR-0006: Independent Verification and Review
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md](../TOPICOS/07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md)
- [TOPICOS/09-PLANNER-E-SCHEDULER.md](../TOPICOS/09-PLANNER-E-SCHEDULER.md)