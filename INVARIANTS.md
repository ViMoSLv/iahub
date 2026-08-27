# Mega Brain V0 — Invariants (INV-001 through INV-036)

> **Status:** FROZEN FOR IMPLEMENTATION
> **Source:** MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Appendix Q
> **Enforcement:** Each invariant has a corresponding automated test in `src/invariants.rs`. Violations block PR merge.

## Constitutional Invariants (Principles 1–7)

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-001 | Agents are disposable; no orchestration state depends on agent liveness. | `test_inv_001_agents_disposable` |
| INV-002 | No two active Attempts share the same workspace path. | `test_inv_002_no_shared_workspace` |
| INV-003 | Agent-to-agent communication occurs only through durable state, never direct chat. | `test_inv_003_state_not_conversation` |
| INV-004 | Every external side effect has a corresponding operations journal entry before execution. | `test_inv_004_journal_before_side_effect` |
| INV-005 | Reported agent outcome is never trusted without independent observational verification. | `test_inv_005_reported_not_observed` |
| INV-006 | The agent that produced a candidate cannot be the reviewer of that same candidate. | `test_inv_006_no_self_review` |
| INV-007 | When state cannot be determined, it remains UNKNOWN rather than defaulting to success or failure. | `test_inv_007_unknown_remains_unknown` |

## Storage and Authority Invariants (Principles 8–14)

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-008 | Code reality is determined exclusively by Git, never by filesystem observation alone. | `test_inv_008_git_source_of_truth` |
| INV-009 | Orchestration state is stored exclusively in SQLite; no other database is used for coordination. | `test_inv_009_sqlite_source_of_truth` |
| INV-010 | Filesystem watcher events are treated as hints and always verified against authoritative state. | `test_inv_010_fs_watchers_are_hints` |
| INV-011 | MCP servers expose only read/write facades over Hub state and contain no business logic. | `test_inv_011_mcp_is_adapter` |
| INV-012 | No agent process receives write access to the canonical integration workspace. | `test_inv_012_no_canonical_mutation` |
| INV-013 | No agent may execute a merge against the target branch; only the Hub merge queue does. | `test_inv_013_no_agent_merge` |
| INV-014 | All consequential state transitions pass through Hub-owned command handlers. | `test_inv_014_hub_owns_transitions` |

## Task Lifecycle Invariants (Principles 15–16)

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-015 | A Task identity persists across retries, reassignments, reviews, and agent replacements. | `test_inv_015_task_survives_lifecycle` |
| INV-016 | Task completion requires verified evidence, not merely an agent self-report of done. | `test_inv_016_done_requires_evidence` |

## Durability and Observability Invariants (Principles 17–20)

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-017 | Closing the UI does not terminate active sessions or lose orchestration state. | `test_inv_017_ui_disposable` |
| INV-018 | No critical mutable orchestration state exists solely in process memory. | `test_inv_018_no_ram_only_state` |
| INV-019 | Every external side effect produces a durable journal entry recoverable after crash. | `test_inv_019_journal_all_side_effects` |
| INV-020 | Every failure carries a classified failure_reason; bare FAILED without classification is rejected. | `test_inv_020_classify_failures` |

## Command Model and Concurrency Invariants

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-021 | Repeated command_id with identical payload returns previously committed result without re-execution. | `test_inv_021_command_idempotency` |
| INV-022 | Repeated command_id with different payload returns 409 COMMAND_ID_PAYLOAD_MISMATCH. | `test_inv_022_payload_mismatch_rejected` |
| INV-023 | Concurrent updates to the same entity with stale version return 409 STATE_CONFLICT. | `test_inv_023_optimistic_concurrency` |
| INV-024 | Commands bearing an expired or superseded fencing token return 409 STALE_AUTHORITY. | `test_inv_024_fencing_token_enforcement` |
| INV-025 | Only one Attempt per Task may be in an active state (LEASED, STARTING, ACTIVE, SUBMITTED) at any time. | `test_inv_025_one_active_attempt_per_task` |

## Workspace and Scope Invariants

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-026 | Workspace paths are canonicalized and validated against symlinks, junctions, and case before use. | `test_inv_026_path_safety` |
| INV-027 | Write scope violations at submission time fail the Attempt regardless of agent intent. | `test_inv_027_scope_drift_enforcement` |

## Verification, Review, and Merge Invariants

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-028 | Review verdict references immutable candidate SHA; SHA change invalidates the review. | `test_inv_028_review_evidence_binding` |
| INV-029 | Merge laboratory result is valid only for the exact (candidate_sha, target_sha, policy_revision, repo_identity) tuple. | `test_inv_029_merge_evidence_binding` |
| INV-030 | Merge queue processes exactly one item at a time per target branch. | `test_inv_030_serialized_merge_queue` |

## Recovery and Process Identity Invariants

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-031 | Startup reconcile scans leases, operations, workspaces, sessions, and tasks before accepting new commands. | `test_inv_031_startup_reconcile` |
| INV-032 | Process identity uses PID plus start timestamp or nonce; PID alone is insufficient. | `test_inv_032_process_identity` |
| INV-033 | Cancellation distinguishes CANCEL_REQUESTED from observed CANCELLED; indeterminate states are preserved. | `test_inv_033_cancellation_semantics` |

## Cleanup and Artifact Invariants

| ID | Statement | Test Function |
|----|-----------|---------------|
| INV-034 | Workspace cleanup never destroys unintegrated work without explicit evidence and policy authorization. | `test_inv_034_cleanup_safety` |
| INV-035 | Critical artifacts carry SHA-256 hash and schema version for integrity verification. | `test_inv_035_artifact_integrity` |
| INV-036 | Policy snapshot for a Run is immutable once RUNNING begins; changes require explicit migration event. | `test_inv_036_policy_snapshot_immutability` |

## Enforcement

- All 36 invariants are defined as constants in `src/invariants.rs`.
- Each invariant has a dedicated test function that must pass in CI.
- PRs that introduce code violating any invariant are rejected regardless of functional correctness.
- New invariants require an ADR and update to this document.