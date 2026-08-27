# ADR-0001: Rust Hub + SQLite + Git CLI

## Status
ACCEPTED

## Context
Mega Brain V0 requires a local-first, durable orchestration engine that survives crashes, restarts, and agent failures without external infrastructure. The system must manage isolated Git worktrees, enforce write scopes, serialize merges, and reconcile state on startup — all on Windows, Linux, and eventually macOS.

Key constraints:
- No cloud services, no external databases, no message brokers in V0.
- Must run as a single daemon process with deterministic recovery.
- Must interoperate with Git natively (worktrees, refs, diffs).
- Must support high-frequency small transactions (leases, heartbeats, events).
- Must be auditable and debuggable by a single developer.

## Decision
We use **Rust** for the Hub daemon, **SQLite in WAL mode** for all orchestration state, and the **Git CLI** (`git` binary) for all repository operations.

### Why Rust
- Memory safety without GC pauses during lease expiry or reconcile.
- Strong type system encodes domain invariants at compile time.
- Excellent FFI for calling Git CLI and system APIs.
- Single-binary deployment with no runtime dependencies.
- Mature async runtime (tokio) for concurrent session holding.

### Why SQLite (WAL)
- Zero operational overhead; embedded in the same process.
- ACID transactions with foreign keys and partial unique indexes.
- WAL mode allows concurrent readers (UI, CLI, MCP) while Hub writes.
- Proven durability with `PRAGMA synchronous = FULL`.
- Schema versioning via migration files.

### Why Git CLI (not libgit2/git2)
- Worktree management is fully supported and battle-tested.
- Avoids subtle behavioral differences between library and canonical Git.
- Easier to audit: every operation is a visible shell command.
- Provider adapters also need Git CLI; one dependency serves both.
- Acceptable performance for V0 scale (< 100 concurrent worktrees).

## Consequences
### Positive
- Single deployable artifact; no Docker/K8s/cloud required.
- Crash recovery is deterministic via startup reconcile.
- Type system prevents entire classes of state machine violations.
- SQLite file can be inspected directly with standard tools.
- Git operations are identical to what developers use manually.

### Negative
- Rust has steeper learning curve than Python/TypeScript.
- Git CLI calls require careful argument sanitization and error parsing.
- SQLite limits horizontal scaling (acceptable for V0; revisit in V1+).
- No built-in distributed coordination (by design for V0).

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Git CLI output format changes | Pin Git version; parse structured output where possible; integration tests against pinned version |
| SQLite lock contention under load | WAL mode + busy_timeout; batch writes; monitor slow queries |
| Rust compilation time slows CI | Use sccache; prebuilt toolchain containers; incremental builds |
| Developer unfamiliarity with Rust | Pair programming; architecture decision records; invariant-driven tests as documentation |

## Related
- INV-009: SQLite source of truth
- INV-008: Git source of truth
- INV-014: Hub owns transitions
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/03-STORAGE-E-SQLITE-SCHEMA.md](../TOPICOS/03-STORAGE-E-SQLITE-SCHEMA.md)