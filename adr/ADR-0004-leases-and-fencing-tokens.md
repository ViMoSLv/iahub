# ADR-0004: Leases + Fencing Tokens

## Status
ACCEPTED

## Context
Mega Brain V0 must guarantee that only one agent has write authority over a Task at any time, and that stale agents cannot mutate state after their authority has been revoked. Relying solely on process liveness or timestamps is insufficient because processes can hang, networks can partition, and clocks can drift.

Key constraints:
- Authority must be time-bounded and revocable.
- Stale authority must be cryptographically or monotonically invalidated.
- Lease expiry must trigger deterministic recovery, not silent failure.
- Heartbeats must come from the Session Holder/sidecar, never from the LLM.
- Fencing tokens must survive Hub restarts (persisted in SQLite).

## Decision
We use **time-bounded leases** paired with **monotonically increasing fencing tokens** for all mutable resource authority.

### Lease
A lease grants exclusive authority over a resource (Task Attempt, Workspace) for a bounded duration. Stored in the `leases` table with:
- `resource_type` + `resource_id` — what is leased
- `owner_attempt_id` — who holds it
- `fencing_token` — monotonic authority number
- `expires_at` — absolute expiry timestamp
- `heartbeat_at` — last renewal timestamp
- `status` — ACTIVE | EXPIRED | REVOKED

### Fencing Token
A strictly increasing integer per resource. Every mutating Command must include the current fencing token. If the token is less than the stored value, the command is rejected with 409 STALE_AUTHORITY. Tokens are persisted in SQLite and incremented atomically within the same transaction as lease revocation.

### Heartbeat Protocol
The Session Holder process emits heartbeats at ≤50% of lease duration. The Hub updates `heartbeat_at` but does NOT extend `expires_at` automatically — extension requires explicit Command with valid fencing token. This prevents zombie sessions from holding authority indefinitely.

### Expiry Handling
On lease expiry:
1. Mark lease EXPIRED
2. Increment fencing token for resource
3. Transition owning Attempt to STALE or LOST
4. Emit event for scheduler reassignment
5. Quarantine workspace if uncertain

## Consequences
### Positive
- Stale agents are mathematically prevented from mutating state.
- Lease expiry is deterministic and auditable via events table.
- Heartbeat separation from LLM prevents prompt-injection-based authority extension.
- Fencing tokens survive crashes (SQLite persistence).
- Multiple resources can share the same fencing mechanism.

### Negative
- Every mutating command requires fencing token validation overhead.
- Heartbeat traffic scales with active Attempts.
- Clock skew between Hub and Session Holder can cause premature expiry.
- Token increment must be atomic with lease state change (transaction complexity).

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Clock skew causes false expiry | Use Hub-local monotonic clock for expiry; heartbeat tolerance window |
| Fencing token overflow | Use i64; monitor high-water mark; alert at 2^50 |
| Heartbeat lost due to network | Retry with exponential backoff; grace period before expiry |
| Zombie session holds lease past expiry | Startup reconcile scans expired leases; revokes unconditionally |
| Concurrent lease acquisition race | UNIQUE constraint on (resource_type, resource_id, fencing_token); optimistic concurrency on version |

## Related
- INV-024: Fencing token enforcement
- INV-025: One active attempt per task
- INV-031: Startup reconcile
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md](../TOPICOS/04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md)