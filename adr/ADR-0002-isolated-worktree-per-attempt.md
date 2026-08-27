# ADR-0002: Isolated Worktree Per Writing Attempt

## Status
ACCEPTED

## Context
Multiple coding agents must operate on the same repository in parallel without interfering with each other or with the canonical integration workspace. Shared mutable state between agents is a primary source of non-deterministic failures, merge conflicts, and security violations in multi-agent systems.

Key constraints:
- Agents are untrusted workers; they may attempt writes outside their authorized scope.
- The canonical workspace must never be mutated directly by any agent.
- Workspace isolation must survive agent crashes, restarts, and lease expiry.
- Cleanup must not destroy unintegrated work without explicit evidence.
- Windows, Linux, and macOS must all support the isolation mechanism.

## Decision
Every writing Task Attempt receives its own **Git worktree** as the default workspace mode. When Git worktrees are unavailable or an execution backend requires it, a **LOCAL_COPY** fallback is used. The canonical integration workspace is never assigned to any agent.

### Default Mode: GIT_WORKTREE
```text
git worktree add <path> -b <branch> <base-commit>
```
- Each Attempt gets a unique branch name derived from task and attempt IDs.
- Base commit is recorded at creation time for scope drift detection.
- Worktree path is stored in SQLite and validated against symlinks/junctions before use.

### Fallback Mode: LOCAL_COPY
- Full `git clone --no-checkout` + `git checkout <base-commit>` into a temp directory.
- Used only when worktree creation fails or provider requires a standalone repo.
- Same write scope enforcement applies.

### Canonical Workspace Protection
- The original user project directory is registered as the integration workspace.
- No Write Scope capability ever includes the canonical path.
- Merge operations are performed by the Hub in a separate ephemeral worktree (Merge Laboratory), never by agents.

## Consequences
### Positive
- True filesystem isolation between concurrent agents.
- Git-native branching model; no custom overlay filesystem needed.
- Scope drift detectable via diff against base commit at submission time.
- Cleanup is safe: orphaned worktrees can be quarantined rather than deleted.
- Compatible with all major Git hosting providers and CI systems.

### Negative
- Git worktree creation has overhead (~100–500ms per worktree).
- Disk usage scales linearly with active Attempts.
- Windows junction/symlink handling requires extra validation.
- LOCAL_COPY fallback doubles disk usage for affected Attempts.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Worktree leak after crash | Startup reconcile scans workspaces table vs filesystem; quarantines orphans |
| Symlink escape from worktree | Path canonicalization + junction detection before granting write scope |
| Disk exhaustion from many worktrees | Concurrency cap per project; cleanup policy with max idle time |
| Branch name collision | Deterministic naming: `mb/{task_id}/{attempt_no}`; unique constraint in DB |
| Stale worktree after lease expiry | Lease revocation triggers workspace release; fencing token prevents reuse |

## Related
- INV-002: No shared workspace
- INV-012: No canonical mutation
- INV-026: Path safety
- INV-034: Cleanup safety
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/05-WORKSPACE-ISOLATION-E-WRITE-SCOPE.md](../TOPICOS/05-WORKSPACE-ISOLATION-E-WRITE-SCOPE.md)