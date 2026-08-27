# Mega Brain V0 — Architecture

> **Status:** FROZEN FOR IMPLEMENTATION
> **Source:** MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md
> **Scope:** Local-first multi-agent software engineering control plane

## Executive Summary

Mega Brain is a durable software-engineering control plane that treats coding agents as replaceable workers operating on isolated Git worktrees. The system owns authoritative state of work, permissions, verification, review, integration, and recovery.

```text
OBJECTIVE → RUN → PLAN/TASK DAG → SCHEDULER → TASK ATTEMPT → AGENT SESSION
→ ISOLATED WORKSPACE → SUBMISSION → OBSERVED REALITY → VERIFICATION
→ INDEPENDENT REVIEW → MERGE LABORATORY → SERIALIZED MERGE QUEUE → CANONICAL TARGET
```

V0 target: two independent coding agents working on one real repository in parallel through isolated worktrees, with zero manual Git operations, deterministic recovery, independent verification, review, and safe integration.

## Seven Constitutional Principles

1. **Agents are disposable. State is durable.**
2. **Agents do not share working trees.**
3. **Agents communicate through state, not conversation.**
4. **Every side effect must be replay-safe.**
5. **Reported state is not observed reality.**
6. **Workers cannot certify their own success.**
7. **Unknown state must remain unknown.**

## Additional Non-Negotiable Principles (8–20)

8. Git is the source of truth for code reality.
9. SQLite is the source of truth for orchestration state.
10. Filesystem watchers are hints, never authority.
11. MCP is an adapter, not the core architecture.
12. No agent may directly mutate the canonical integration workspace.
13. No agent may merge the target branch.
14. The Hub owns all consequential state transitions.
15. The same logical task survives retries, reviews, rework, and agent replacement.
16. A task is not complete because an agent says "done".
17. The UI must be disposable.
18. No critical mutable orchestration state may exist only in RAM.
19. All external side effects must be journaled.
20. Failures must be classified, not flattened into generic FAILED.

## Anti-Patterns (Rejected at PR Review)

- Agent-to-agent chat as authoritative workflow state
- Shared mutable working tree between active coding agents
- Task status controlled by terminal parsing
- Provider-specific fields in core Task state
- UI-owned scheduler state
- Filesystem watcher as final truth
- Silent merge conflict resolution
- Unbounded autonomous retry loop
- Mutable frozen PlanSpec
- PID-only process ownership
- Implicit global write scope
- Critical JSON state rewritten non-atomically
- New side effect without recovery semantics
- New status without exhaustive consequence mapping

## Core Domain Entities

| Entity | Purpose |
|--------|---------|
| Project | Registered Git repository identified by fingerprint |
| Run | Top-level execution of a user objective |
| Task | Stable logical unit of work surviving lifecycle events |
| Attempt | One execution attempt of one Task with lease authority |
| Session | Live agent process/provider session attached to an Attempt |
| Workspace | Isolated filesystem (Git worktree or local copy) per Attempt |
| Lease | Time-bounded authority over a resource |
| Fencing Token | Monotonically increasing authority number preventing stale reuse |
| Artifact | Immutable or append-only evidence (diff, test report, review, etc.) |
| Review | Independent evaluation of a candidate commit |
| Merge Item | Durable request to integrate approved candidate into target branch |

## Storage

- **SQLite WAL** — sole source of truth for orchestration state
- **Git** — sole source of truth for code reality
- **Hub daemon** — only logical writer to coordination database
- No Redis, Kafka, NATS, Postgres, vector DB, or cloud services in V0

## Command Model

Every consequential mutation is a Command with idempotent delivery:
- Repeated `command_id` + identical payload → return cached result
- Repeated `command_id` + different payload → 409 COMMAND_ID_PAYLOAD_MISMATCH
- Stale version → 409 STATE_CONFLICT
- Expired fencing token → 409 STALE_AUTHORITY

## Recovery

On startup, the Hub reconciles: leases, operations, workspaces, sessions, and tasks. No trust in graceful shutdown. Unknown state remains unknown until verified.

## Related Documents

- [INVARIANTS.md](./INVARIANTS.md) — INV-001 through INV-036
- [STATE-MACHINES.md](./STATE-MACHINES.md) — Exhaustive transition tables
- [adr/](./adr/) — Architectural Decision Records ADR-0001 through ADR-0010
- [TOPICOS/](./TOPICOS/) — Implementation topic breakdown