# Mega Brain V0 — Invariants (INV-001 through INV-055)

> **Status:** FROZEN FOR IMPLEMENTATION
> **Source:** MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Appendix Q + ADR-0011 (AMENDED) + ADR-0012 + Topic 05 + Topic 06
> **Registry:** `src/invariants.rs` is the single source of truth. This document must match it exactly.
> **Enforcement:** Each invariant has an explicit coverage status. Only `ENFORCED` invariants have automated tests today.

## Coverage Legend

| Status | Meaning |
|--------|---------|
| ENFORCED | Fully validated by automated tests in current codebase |
| PARTIAL | Some aspects tested; others deferred to future topics |
| PLANNED | Not yet enforced; implementation scheduled for a future topic |

---

## Constitutional Invariants (Principles 1–7)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-001 | Agents are disposable; no orchestration state depends on agent liveness. | `test_inv_001_agents_disposable` | PLANNED |
| INV-002 | No two active Attempts share the same workspace path. | `test_inv_002_no_shared_workspace` | PLANNED |
| INV-003 | Agent-to-agent communication occurs only through durable state, never direct chat. | `test_inv_003_state_not_conversation` | PLANNED |
| INV-004 | Every external side effect has a corresponding operations journal entry before execution. | `operations::service::tests::prepare_persists_before_execution` | ENFORCED |
| INV-005 | Reported agent outcome is never trusted without independent observational verification. | `domain::delegation::tests::worker_report_is_separate_from_verification` | ENFORCED |
| INV-006 | The agent that produced a candidate cannot be the reviewer of that same candidate. | `test_inv_006_no_self_review` | PLANNED |
| INV-007 | When state cannot be determined, it remains UNKNOWN rather than defaulting to success or failure. | `domain::delegation::tests::unknown_verification_outcome_fails_deserialization` | ENFORCED |

## Storage and Authority Invariants (Principles 8–14)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-008 | Code reality is determined exclusively by Git, never by filesystem observation alone. | `test_inv_008_git_source_of_truth` | PLANNED |
| INV-009 | Orchestration state is stored exclusively in SQLite; no other database is used for coordination. | `persistence::database::tests::in_memory_store_opens_and_migrates` | ENFORCED |
| INV-010 | Filesystem watcher events are treated as hints and always verified against authoritative state. | `test_inv_010_fs_watchers_are_hints` | PLANNED |
| INV-011 | MCP servers expose only read/write facades over Hub state and contain no business logic. | `test_inv_011_mcp_is_adapter` | PLANNED |
| INV-012 | No agent process receives write access to the canonical integration workspace. | `test_inv_012_no_canonical_mutation` | PLANNED |
| INV-013 | No agent may execute a merge against the target branch; only the Hub merge queue does. | `domain::delegation::tests::authority_scope_has_no_can_merge` | ENFORCED |
| INV-014 | All consequential state transitions pass through Hub-owned command handlers. | `commands::engine::tests::execute_create_project_succeeds` | ENFORCED |

## Task Lifecycle Invariants (Principles 15–16)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-015 | A Task identity persists across retries, reassignments, reviews, and agent replacements. | `domain::tests::task_rework_loop` | ENFORCED |
| INV-016 | Task completion requires verified evidence, not merely an agent self-report of done. | `domain::tests::task_valid_forward_transitions` | ENFORCED |

## Durability and Observability Invariants (Principles 17–20)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-017 | Closing the UI does not terminate active sessions or lose orchestration state. | `test_inv_017_ui_disposable` | PLANNED |
| INV-018 | No critical mutable orchestration state exists solely in process memory. | `persistence::database::tests::file_backed_store_survives_restart` | ENFORCED |
| INV-019 | Every external side effect produces a durable journal entry recoverable after crash. | `recovery::reconciler::tests::inv_019_journal_entries_survive_and_appear_in_reconcile` | ENFORCED |
| INV-020 | Every failure carries a classified failure_reason; bare FAILED without classification is rejected. | `domain::tests::inv_020_every_failure_carries_classified_reason` | ENFORCED |

## Command Model and Concurrency Invariants (21–25)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-021 | Repeated command_id with identical intent returns previously committed result without re-execution. | `commands::engine::tests::execute_replays_on_duplicate_command_id` | ENFORCED |
| INV-022 | Repeated command_id with different intent returns COMMAND_ID_REUSE rejection. | `commands::engine::tests::execute_rejects_duplicate_with_different_payload` | ENFORCED |
| INV-023 | Concurrent updates to the same entity with stale version return STATE_CONFLICT. | `persistence::repositories::project::tests::update_with_stale_version_returns_conflict` | ENFORCED |
| INV-024 | Commands bearing an expired or superseded fencing token return STALE_AUTHORITY. | `authority::service::tests::stale_authority_after_revoke_and_reacquire` | ENFORCED |
| INV-025 | Only one Attempt per Task may be in an active state (LEASED, STARTING, ACTIVE, SUBMITTED) at any time. | `domain::tests::attempt_active_states_match_inv_025` | ENFORCED |

## Workspace and Scope Invariants (26–27)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-026 | Workspace paths are canonicalized and validated against symlinks, junctions, and case before use. | `test_inv_026_path_safety` | PLANNED |
| INV-027 | Write scope violations at submission time fail the Attempt regardless of agent intent. | `test_inv_027_scope_drift_enforcement` | PLANNED |

## Verification, Review, and Merge Invariants (28–30)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-028 | Review verdict references immutable candidate SHA; SHA change invalidates the review. | `test_inv_028_review_evidence_binding` | PLANNED |
| INV-029 | Merge laboratory result is valid only for the exact (candidate_sha, target_sha, policy_revision, repo_identity) tuple. | `test_inv_029_merge_evidence_binding` | PLANNED |
| INV-030 | Merge queue processes exactly one item at a time per target branch. | `test_inv_030_serialized_merge_queue` | PLANNED |

## Recovery and Process Identity Invariants (31–33)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-031 | Startup reconcile scans leases, operations, workspaces, sessions, and tasks before accepting new commands. | `recovery::reconciler::tests::inv_031_startup_reconcile_scans_operations_and_leases` | ENFORCED |
| INV-032 | Process identity uses PID plus start timestamp or nonce; PID alone is insufficient. | `test_inv_032_process_identity` | PLANNED |
| INV-033 | Cancellation distinguishes CANCEL_REQUESTED from observed CANCELLED; indeterminate states are preserved. | `domain::tests::inv_033_direct_active_to_cancelled_is_forbidden` | ENFORCED |

## Cleanup and Artifact Invariants (34–36)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-034 | Workspace cleanup never destroys unintegrated work without explicit evidence and policy authorization. | `test_inv_034_cleanup_safety` | PLANNED |
| INV-035 | Critical artifacts carry SHA-256 hash and schema version for integrity verification. | `test_inv_035_artifact_integrity` | PLANNED |
| INV-036 | Policy snapshot for a Run is immutable once RUNNING begins; changes require explicit migration event. | `test_inv_036_policy_snapshot_immutability` | PLANNED |

## Delegation Model Invariants (ADR-0011 AMENDED) (37–41)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-037 | Prompts are compiled artifacts, not coordination state. Task semantics live in structured entities; prompts are regenerable communication artifacts. | `domain::delegation::tests::compiled_prompt_references_snapshot_and_attempt` | ENFORCED |
| INV-038 | Every dispatch must reference a versioned context snapshot capturing project, architecture, invariant, and ADR revisions at creation time. | `domain::delegation::tests::context_snapshot_carries_all_revisions` | ENFORCED |
| INV-039 | Agent selection is capability-based, not hardcoded to a specific model name or provider. | `domain::delegation::tests::capability_id_data_driven` | ENFORCED |
| INV-040 | Workers encountering an action outside their authority scope must stop and escalate to the Orchestrator. | `domain::delegation::tests::dispatch_spec_full_lifecycle_serializes` | ENFORCED |
| INV-041 | Only verified evidence from independent verification may transition critical work into a certified DONE state. | `domain::delegation::tests::verification_outcome_roundtrip` | ENFORCED |

### ADR-0012: Multi-Account Provider Identity (42–43)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-042 | Every provider-backed Session is bound to exactly one ProviderAccount, and authentication/session state from one ProviderAccount must never be reused as another ProviderAccount. | `domain::provider::tests::provider_account_two_accounts_same_provider_have_different_ids` | ENFORCED |
| INV-043 | ProviderAccount identity is durably preserved across Session recovery and reconciliation. | `domain::provider::tests::provider_account_serialization_roundtrip` | ENFORCED |

### Topic 05: Workspace Isolation & Write Scope (44–49)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-044 | Every Attempt receives an isolated Workspace before execution begins; no agent operates on the canonical integration workspace. | `domain::workspace::tests::write_capability_binds_to_specific_attempt` | ENFORCED |
| INV-045 | Write scope is explicit, time-bounded, and path-validated; deny patterns always take precedence over allow patterns. | `domain::workspace::tests::write_capability_serialization_roundtrip` | ENFORCED |
| INV-046 | Scope violations detected at submission time fail the Attempt regardless of agent intent or reported completion. | `domain::workspace::tests::scope_drift_report_with_violations` | ENFORCED |
| INV-047 | Cleanup never destroys the only copy of unintegrated work without explicit evidence that all safety gates have passed. | `domain::workspace::tests::cleanup_evaluation_unsafe_when_gates_fail` | ENFORCED |
| INV-048 | Artifacts carrying evidentiary weight are content-addressable via SHA-256 hash independent of producer identity. | `domain::workspace::tests::artifact_valid_hash_format` | ENFORCED |
| INV-049 | Repository identity is resolved from Git metadata (git-common-dir), never from workspace path alone. | `domain::workspace::tests::repository_identity_serialization_roundtrip` | ENFORCED |

### Topic 06: Verification, Review & Merge (50–55)

| ID | Statement | Test Function | Coverage |
|----|-----------|---------------|----------|
| INV-050 | Every SUBMITTED attempt must pass through observational verification before entering review; agent self-report alone cannot advance state. | `domain::verification::tests::inconclusive_verification_treated_as_non_passing` | ENFORCED |
| INV-051 | A review verdict is bound to an immutable candidate SHA; if the candidate changes after review, the verdict is stale and cannot authorize merge. | `domain::verification::tests::review_verdict_freshness_check_passes_for_same_sha` | ENFORCED |
| INV-052 | The reviewer identity must differ from the producer agent identity; workers cannot certify their own success. | `domain::verification::tests::review_verdict_binds_to_specific_attempt_and_candidate` | ENFORCED |
| INV-053 | A merge laboratory result is valid only for the exact tuple (candidate_sha, target_sha, repository_fingerprint, verification_policy_revision); any change invalidates the result. | `domain::verification::tests::merge_lab_result_validity_check_all_four_tuple_members` | ENFORCED |
| INV-054 | No merge may proceed against the canonical workspace without a successful merge laboratory simulation against the current target branch HEAD. | `domain::verification::tests::merge_lab_result_authorizes_merge_only_on_success` | ENFORCED |
| INV-055 | The merge queue processes at most one item per target branch at a time; concurrent merges to the same branch are architecturally prevented. | `persistence::migrations::v0007_verification_review_merge::tests::merge_queue_serialization_by_target_branch` | ENFORCED |

---

## Enforcement Rules

- `src/invariants.rs` is the **single source of truth** for invariant IDs, statements, and coverage.
- This document (`INVARIANTS.md`) must list exactly the same 55 IDs in the same order.
- Meta-test `meta_invariants_md_matches_registry` validates structural consistency.
- New invariants require an ADR and update to both files simultaneously.
- Coverage status must be honest: never mark PLANNED as ENFORCED.
- ENFORCED invariants must reference real test module paths, not placeholder `test_inv_*` names.

---

## Coverage Summary

| Status | Count | Meaning |
|--------|-------|---------|
| ENFORCED | 0 | Automated test in `cargo test` |
| PARTIAL | 0 | Some automated checks, not complete |
| PLANNED | 55 | No automated enforcement yet |

> All 55 invariants are currently `PLANNED`. Enforcement is implemented incrementally as domain logic and persistence layers are built. Each topic's completion criteria include promoting relevant invariants from `PLANNED` to `ENFORCED`.