# ADR-0007: Merge Laboratory + Serialized Merge Queue

## Status
ACCEPTED

## Context
Mega Brain V0 must integrate approved candidate commits into target branches without introducing merge conflicts, test regressions, or race conditions. Allowing agents to merge directly creates non-deterministic repository state, silent conflict resolution, and broken canonical branches. The system must guarantee that every merge is pre-validated against the current target state and executed serially per branch.

Key constraints:
- No agent may merge the target branch (Constitutional Principle 13).
- Merge evidence is valid only for a specific (candidate_sha, target_sha, policy_revision, repo_identity) tuple.
- Concurrent merges to the same branch cause non-deterministic outcomes.
- Silent conflict resolution is an anti-pattern (Appendix O).
- Failed merges must be classified and returned to rework, not silently dropped.
- Merge operations must be journaled and recoverable after crash.

## Decision
We implement a two-phase merge system: **Merge Laboratory** for pre-validation and **Serialized Merge Queue** for execution.

### Merge Laboratory
An ephemeral Git worktree created by the Hub (never by an agent) that simulates the merge before authorizing it:
1. Create disposable worktree from current target branch HEAD.
2. Attempt merge of candidate commit.
3. Run full verification suite (tests, lints, security scans).
4. Record result with evidence binding to exact tuple.
5. Clean up disposable worktree regardless of outcome.

Outcomes: CLEAN, CONFLICT, TEST_FAILED, VERIFICATION_FAILED, UNKNOWN.
Only CLEAN results authorize queue entry. All others return Task to appropriate rework state.

### Serialized Merge Queue
Per-target-branch FIFO queue stored in SQLite (`merge_queue` table):
- One item processed at a time per branch.
- Items ordered by priority DESC, created_at ASC.
- Each item carries expected_target_sha; mismatch triggers re-simulation.
- Execution uses atomic Git operations with journaling.
- Success updates canonical branch and marks Task DONE.
- Failure classifies reason and returns Task to REVIEWING or NEEDS_CHANGES.

### Evidence Binding
Merge Laboratory results are valid only for the exact tuple:
```text
(candidate_sha, target_sha, verification_policy_revision, repository_fingerprint)
```
Any change to any member invalidates the result and requires re-simulation. This prevents stale simulations from authorizing merges against evolved targets.

### Crash Recovery
If Hub crashes during MERGING state:
- Startup reconcile detects incomplete merge operation.
- If candidate still applies cleanly to current target, resume.
- If target advanced or candidate stale, re-run laboratory simulation.
- Never assume partial merge succeeded.

## Consequences
### Positive
- Canonical branch integrity guaranteed by pre-validation.
- No silent conflict resolution; all conflicts explicitly classified.
- Serial execution eliminates merge races per branch.
- Evidence binding prevents stale approvals from corrupting state.
- Crash recovery is deterministic via operation journal.
- Agents cannot bypass merge safety by direct Git access.

### Negative
- Merge latency increased by laboratory simulation time.
- Disposable worktrees consume temporary disk space.
- Serial queue creates bottleneck for high-throughput branches.
- Re-simulation on target advancement adds redundant work.
- Complex recovery logic for partial merge states.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Laboratory simulation passes but real merge fails | Use identical Git version and flags; integration test against real repo; preserve lab artifacts for debugging |
| Queue starvation for low-priority items | Priority aging in future versions; monitoring for stuck items; manual escalation path |
| Target advances during simulation | Expected SHA check before execution; automatic re-simulation on mismatch |
| Disk exhaustion from disposable worktrees | Cleanup policy with max age; monitor temp directory size; async cleanup after success |
| Partial merge leaves repo in inconsistent state | Atomic Git operations; journal before each step; reconcile on startup |
| Evidence binding too strict causes excessive rework | Batch compatible candidates; cache intermediate verification results; optimize simulation speed |

## Related
- INV-013: No agent merge
- INV-029: Merge evidence binding
- INV-030: Serialized merge queue
- INV-031: Startup reconcile
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md](../TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md)