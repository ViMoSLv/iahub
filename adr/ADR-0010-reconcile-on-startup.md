# ADR-0010: Reconcile on Startup

## Status
ACCEPTED

## Context
Mega Brain V0 must survive crashes, power loss, and unclean shutdowns without leaving the system in an inconsistent or unsafe state. Relying on graceful shutdown is insufficient because processes can be killed by OOM, SIGKILL, task manager, or OS updates. The system must assume the previous process died mid-write and recover deterministically before accepting new work.

Key constraints:
- No critical mutable orchestration state may exist only in RAM (Principle 18).
- Every side effect must be replay-safe (Principle 4).
- Unknown state must remain unknown (Principle 7).
- Filesystem watchers are hints, never authority (Principle 10).
- Failures must be classified, not flattened into generic FAILED (Principle 20).
- Recovery must complete within bounded time for V0 scale (< 30 seconds for 100 tasks).

## Decision
On every startup, the Hub executes a **deterministic reconcile sequence** before accepting any commands. This sequence scans all authoritative state sources and corrects inconsistencies without trusting prior runtime state.

### Reconcile Sequence (Ordered)
1. **Lease Scan**: Find all leases with `expires_at < now()`. Mark EXPIRED, increment fencing tokens, transition owning Attempts to STALE/LOST, emit revoke events.
2. **Operation Scan**: Find all operations in non-terminal states (PREPARED, EXECUTING, SIDE_EFFECT_OBSERVED). Classify each as recoverable (resume), compensatable (rollback), or failed (escalate). Never assume partial completion succeeded.
3. **Workspace Scan**: Compare `workspaces` table against filesystem. Detect missing worktrees (BROKEN), orphaned directories (ORPHANED), and path mismatches. Quarantine uncertain cases rather than delete.
4. **Session Scan**: Verify process identity for all ACTIVE/CONNECTED sessions using PID + start timestamp + nonce (Appendix C). Mark UNRESPONSIVE/LOST when identity cannot be proven. Do not claim death from lookup failure alone.
5. **Task/Attempt Scan**: Detect orphaned attempts (no valid lease, no live session), stuck states (SUBMITTED > threshold without verification), and invariant violations. Emit classify events for each correction.
6. **Event Emission**: Write reconcile audit events to `events` table for every correction made. These are queryable by UI/CLI/MCP for post-mortem analysis.

### Design Principles
- **No trust in graceful shutdown**: Assume previous process died at any point.
- **Idempotent corrections**: Running reconcile twice produces same result.
- **Fail closed**: Uncertain state remains UNKNOWN/QUARANTINED, not assumed safe.
- **Bounded duration**: Each scan has timeout; total reconcile < 30s for 100 tasks.
- **Audit trail**: Every correction logged as event with before/after state.

### What Reconcile Does NOT Do
- Does not re-execute completed commands.
- Does not retry failed agent work automatically.
- Does not modify canonical Git branches.
- Does not extend expired leases without explicit renewal command.
- Does not trust filesystem watcher state accumulated during downtime.

## Consequences
### Positive
- System recovers deterministically from any crash scenario.
- No silent corruption from partial writes or zombie processes.
- Audit trail enables post-mortem debugging of recovery actions.
- Bounded startup time prevents indefinite unavailability.
- Idempotent reconcile allows safe restart loops.
- Unknown states preserved rather than masked as success/failure.

### Negative
- Startup latency added (up to 30 seconds at V0 scale).
- Complex recovery logic per operation type.
- Quarantined resources require manual intervention or policy-driven cleanup.
- False positives in orphan detection may quarantine valid work.
- Testing requires chaos scenarios covering every crash point.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Reconcile itself crashes mid-correction | Transactional writes per correction; idempotent design allows safe restart |
| False orphan detection quarantines valid work | Conservative thresholds; preserve evidence in quarantine; manual review path |
| Reconcile takes too long under load | Per-scan timeouts; parallel independent scans; progress logging; alert on duration |
| Operation recovery semantics incomplete | Each new operation type requires documented recovery procedure + chaos test |
| Lease revocation races with heartbeat renewal | Atomic transaction: check expiry + revoke + increment token in single write |
| Session identity check fails due to OS PID recycling | Require start timestamp + nonce match; treat mismatch as UNRESPONSIVE, not TERMINATED |
| Workspace quarantine fills disk | Retention policy; max quarantine size; alert on threshold; manual cleanup tool |

## Related
- INV-007: Unknown remains unknown
- INV-018: No RAM-only state
- INV-031: Startup reconcile
- INV-032: Process identity
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/08-RECOVERY-RECONCILE-E-OBSERVABILITY.md](../TOPICOS/08-RECOVERY-RECONCILE-E-OBSERVABILITY.md)