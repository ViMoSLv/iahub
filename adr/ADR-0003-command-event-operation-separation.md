# ADR-0003: Command / Event / Operation Separation

## Status
ACCEPTED

## Context
Mega Brain V0 must handle mutations that span multiple subsystems (SQLite, Git, filesystem, agent processes) while guaranteeing idempotency, crash recovery, and auditability. Mixing intent, state change, and side effect into a single code path makes recovery impossible and testing intractable.

Key constraints:
- Every mutation must be replay-safe after crash or restart.
- Duplicate delivery of the same command must not produce duplicate effects.
- Side effects (Git ops, process spawns, file writes) must be reversible or compensatable.
- The system must distinguish "what was requested" from "what happened" from "what external action was taken".
- All three concepts must be durably stored in SQLite.

## Decision
We separate all mutations into three distinct, durably-stored concepts:

### Command
An **intent** to mutate state. Identified by `command_id` + `payload_hash`. Stored in the `commands` table before execution begins. Idempotent: re-delivery with same hash returns cached result; different hash returns 409.

### Event
A **fact** that something happened to an aggregate. Stored in the `events` table. Immutable once written. Drives state transitions and outbox publishing. Never contains side-effect details.

### Operation
A **journal entry** for an external side effect. Stored in the `operations` table. Has explicit states: PREPARED → EXECUTING → SIDE_EFFECT_OBSERVED → COMMITTED | ROLLED_BACK | REQUIRES_RECONCILE | FAILED. Each operation type has defined recovery semantics.

### Flow
```text
Command received → validate idempotency → write Command(PENDING)
  → write Event(intent_received)
  → write Operation(PREPARED) for each side effect
  → execute side effect → update Operation(SIDE_EFFECT_OBSERVED)
  → apply state transition → update Command(COMPLETED)
  → write Event(state_changed)
  → publish via outbox
```

## Consequences
### Positive
- Crash at any point leaves sufficient journal entries for deterministic recovery.
- Idempotency is enforced at the Command layer, not scattered across handlers.
- Events provide a complete audit trail independent of side-effect outcomes.
- Operations enable targeted reconciliation without re-executing entire commands.
- Testing can mock operations without losing command/event integrity.

### Negative
- Three-table write per mutation increases transaction complexity.
- Developers must think in three concepts instead of one "do thing" function.
- Recovery logic must handle partial operation states explicitly.
- Outbox adds eventual consistency delay for external consumers.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Operation stuck in EXECUTING after crash | Startup reconcile scans non-terminal operations; classifies as recoverable or failed |
| Command completed but event not written | Transactional write: Command + Event in same SQLite transaction |
| Outbox publication fails indefinitely | Exponential backoff + max attempts + alerting on stuck entries |
| Developer bypasses Command layer | Lint/test gate: no direct state mutation outside command handlers |
| Operation recovery semantics incomplete | Each new operation type requires documented recovery procedure + test |

## Related
- INV-004: Journal before side effect
- INV-019: Journal all side effects
- INV-021: Command idempotency
- INV-031: Startup reconcile
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md](../TOPICOS/04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md)