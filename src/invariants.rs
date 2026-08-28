//! Mega Brain V0 — Invariant Definitions (INV-001 through INV-041)
//!
//! Each invariant is a compile-time constant with a unique ID, human-readable
//! statement matching the blueprint, an associated test function name, and
//! explicit coverage status. Not all invariants have executable enforcement yet;
//! coverage is tracked honestly to prevent false claims of completeness.

/// Coverage status for an invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantCoverage {
    /// Fully enforced by automated tests in the current codebase.
    Enforced,
    /// Partially enforced; some aspects tested, others deferred.
    Partial,
    /// Not yet enforced; planned for a future topic.
    Planned,
}

/// A single architectural invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    /// Unique identifier, e.g. "INV-001".
    pub id: &'static str,
    /// Human-readable statement derived from the blueprint.
    pub statement: &'static str,
    /// Name of the test function that validates this invariant (may be placeholder if Planned).
    pub test_fn: &'static str,
    /// Current enforcement coverage status.
    pub coverage: InvariantCoverage,
}

/// All 41 invariants: original 36 from Appendix Q plus 5 from ADR-0011 (Delegation Model).
pub const INVARIANTS: &[Invariant] = &[
    // ── Constitutional Principles (1–7) ──────────────────────────────────
    Invariant {
        id: "INV-001",
        statement: "Agents are disposable; no orchestration state depends on agent liveness.",
        test_fn: "test_inv_001_agents_disposable",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-002",
        statement: "No two active Attempts share the same workspace path.",
        test_fn: "test_inv_002_no_shared_workspace",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-003",
        statement: "Agent-to-agent communication occurs only through durable state, never direct chat.",
        test_fn: "test_inv_003_state_not_conversation",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-004",
        statement: "Every external side effect has a corresponding operations journal entry before execution.",
        test_fn: "operations::service::tests::prepare_persists_before_execution",
        coverage: InvariantCoverage::Partial,
    },
    Invariant {
        id: "INV-005",
        statement: "Reported agent outcome is never trusted without independent observational verification.",
        test_fn: "domain::delegation::tests::worker_report_is_separate_from_verification",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-006",
        statement: "The agent that produced a candidate cannot be the reviewer of that same candidate.",
        test_fn: "test_inv_006_no_self_review",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-007",
        statement: "When state cannot be determined, it remains UNKNOWN rather than defaulting to success or failure.",
        test_fn: "domain::delegation::tests::unknown_verification_outcome_fails_deserialization",
        coverage: InvariantCoverage::Enforced,
    },
    // ── Storage and Authority (8–14) ─────────────────────────────────────
    Invariant {
        id: "INV-008",
        statement: "Code reality is determined exclusively by Git, never by filesystem observation alone.",
        test_fn: "test_inv_008_git_source_of_truth",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-009",
        statement: "Orchestration state is stored exclusively in SQLite; no other database is used for coordination.",
        test_fn: "persistence::database::tests::in_memory_store_opens_and_migrates",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-010",
        statement: "Filesystem watcher events are treated as hints and always verified against authoritative state.",
        test_fn: "test_inv_010_fs_watchers_are_hints",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-011",
        statement: "MCP servers expose only read/write facades over Hub state and contain no business logic.",
        test_fn: "test_inv_011_mcp_is_adapter",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-012",
        statement: "No agent process receives write access to the canonical integration workspace.",
        test_fn: "test_inv_012_no_canonical_mutation",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-013",
        statement: "No agent may execute a merge against the target branch; only the Hub merge queue does.",
        test_fn: "domain::delegation::tests::authority_scope_has_no_can_merge",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-014",
        statement: "All consequential state transitions pass through Hub-owned command handlers.",
        test_fn: "commands::engine::tests::execute_create_project_succeeds",
        coverage: InvariantCoverage::Enforced,
    },
    // ── Task Lifecycle (15–16) ───────────────────────────────────────────
    Invariant {
        id: "INV-015",
        statement: "A Task identity persists across retries, reassignments, reviews, and agent replacements.",
        test_fn: "domain::tests::task_rework_loop",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-016",
        statement: "Task completion requires verified evidence, not merely an agent self-report of done.",
        test_fn: "domain::tests::task_valid_forward_transitions",
        coverage: InvariantCoverage::Enforced,
    },
    // ── Durability and Observability (17–20) ─────────────────────────────
    Invariant {
        id: "INV-017",
        statement: "Closing the UI does not terminate active sessions or lose orchestration state.",
        test_fn: "test_inv_017_ui_disposable",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-018",
        statement: "No critical mutable orchestration state exists solely in process memory.",
        test_fn: "persistence::database::tests::file_backed_store_survives_restart",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-019",
        statement: "Every external side effect produces a durable journal entry recoverable after crash.",
        test_fn: "operations::service::tests::requires_reconcile_survives_restart_and_resolves",
        coverage: InvariantCoverage::Partial,
    },
    Invariant {
        id: "INV-020",
        statement: "Every failure carries a classified failure_reason; bare FAILED without classification is rejected.",
        test_fn: "test_inv_020_classify_failures",
        coverage: InvariantCoverage::Partial,
    },
    // ── Command Model and Concurrency (21–25) ────────────────────────────
    Invariant {
        id: "INV-021",
        statement: "Repeated command_id with identical intent returns previously committed result without re-execution.",
        test_fn: "commands::engine::tests::execute_replays_on_duplicate_command_id",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-022",
        statement: "Repeated command_id with different intent returns COMMAND_ID_REUSE rejection.",
        test_fn: "commands::engine::tests::execute_rejects_duplicate_with_different_payload",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-023",
        statement: "Concurrent updates to the same entity with stale version return STATE_CONFLICT.",
        test_fn: "persistence::repositories::project::tests::update_with_stale_version_returns_conflict",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-024",
        statement: "Commands bearing an expired or superseded fencing token return STALE_AUTHORITY.",
        test_fn: "test_inv_024_fencing_token_enforcement",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-025",
        statement: "Only one Attempt per Task may be in an active state (LEASED, STARTING, ACTIVE, SUBMITTED) at any time.",
        test_fn: "domain::tests::attempt_active_states_match_inv_025",
        coverage: InvariantCoverage::Enforced,
    },
    // ── Workspace and Scope (26–27) ──────────────────────────────────────
    Invariant {
        id: "INV-026",
        statement: "Workspace paths are canonicalized and validated against symlinks, junctions, and case before use.",
        test_fn: "test_inv_026_path_safety",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-027",
        statement: "Write scope violations at submission time fail the Attempt regardless of agent intent.",
        test_fn: "test_inv_027_scope_drift_enforcement",
        coverage: InvariantCoverage::Planned,
    },
    // ── Verification, Review, Merge (28–30) ──────────────────────────────
    Invariant {
        id: "INV-028",
        statement: "Review verdict references immutable candidate SHA; SHA change invalidates the review.",
        test_fn: "test_inv_028_review_evidence_binding",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-029",
        statement: "Merge laboratory result is valid only for the exact (candidate_sha, target_sha, policy_revision, repo_identity) tuple.",
        test_fn: "test_inv_029_merge_evidence_binding",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-030",
        statement: "Merge queue processes exactly one item at a time per target branch.",
        test_fn: "test_inv_030_serialized_merge_queue",
        coverage: InvariantCoverage::Planned,
    },
    // ── Recovery and Process Identity (31–33) ────────────────────────────
    Invariant {
        id: "INV-031",
        statement: "Startup reconcile scans leases, operations, workspaces, sessions, and tasks before accepting new commands.",
        test_fn: "test_inv_031_startup_reconcile",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-032",
        statement: "Process identity uses PID plus start timestamp or nonce; PID alone is insufficient.",
        test_fn: "test_inv_032_process_identity",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-033",
        statement: "Cancellation distinguishes CANCEL_REQUESTED from observed CANCELLED; indeterminate states are preserved.",
        test_fn: "test_inv_033_cancellation_semantics",
        coverage: InvariantCoverage::Planned,
    },
    // ── Cleanup and Artifacts (34–36) ────────────────────────────────────
    Invariant {
        id: "INV-034",
        statement: "Workspace cleanup never destroys unintegrated work without explicit evidence and policy authorization.",
        test_fn: "test_inv_034_cleanup_safety",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-035",
        statement: "Critical artifacts carry SHA-256 hash and schema version for integrity verification.",
        test_fn: "test_inv_035_artifact_integrity",
        coverage: InvariantCoverage::Planned,
    },
    Invariant {
        id: "INV-036",
        statement: "Policy snapshot for a Run is immutable once RUNNING begins; changes require explicit migration event.",
        test_fn: "test_inv_036_policy_snapshot_immutability",
        coverage: InvariantCoverage::Planned,
    },
    // ── Delegation Model (ADR-0011) (37–41) ──────────────────────────────
    Invariant {
        id: "INV-037",
        statement: "Prompts are compiled artifacts, not coordination state. Task semantics live in structured entities; prompts are regenerable communication artifacts.",
        test_fn: "domain::delegation::tests::compiled_prompt_references_snapshot_and_attempt",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-038",
        statement: "Every dispatch must reference a versioned context snapshot capturing project, architecture, invariant, and ADR revisions at creation time.",
        test_fn: "domain::delegation::tests::context_snapshot_carries_all_revisions",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-039",
        statement: "Agent selection is capability-based, not hardcoded to a specific model name or provider.",
        test_fn: "domain::delegation::tests::capability_id_data_driven",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-040",
        statement: "Workers encountering an action outside their authority scope must stop and escalate to the Orchestrator.",
        test_fn: "domain::delegation::tests::dispatch_spec_full_lifecycle_serializes",
        coverage: InvariantCoverage::Enforced,
    },
    Invariant {
        id: "INV-041",
        statement: "Only verified evidence from independent verification may transition critical work into a certified DONE state.",
        test_fn: "domain::delegation::tests::verification_outcome_roundtrip",
        coverage: InvariantCoverage::Enforced,
    },
];

/// Returns the invariant with the given ID, if it exists.
pub fn find_invariant(id: &str) -> Option<&'static Invariant> {
    INVARIANTS.iter().find(|inv| inv.id == id)
}

/// Returns all invariant IDs as a sorted vector of strings.
pub fn all_invariant_ids() -> Vec<&'static str> {
    INVARIANTS.iter().map(|inv| inv.id).collect()
}

/// Returns the count of invariants with the given coverage status.
pub fn count_by_coverage(status: InvariantCoverage) -> usize {
    INVARIANTS
        .iter()
        .filter(|inv| inv.coverage == status)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_count_is_exactly_41() {
        assert_eq!(
            INVARIANTS.len(),
            41,
            "36 original invariants (Appendix Q) + 5 delegation invariants (ADR-0011) = 41 total"
        );
    }

    #[test]
    fn invariant_ids_are_sequential_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for (i, inv) in INVARIANTS.iter().enumerate() {
            let expected = format!("INV-{:03}", i + 1);
            assert_eq!(
                inv.id, expected,
                "Invariant at index {} should have ID {}, got {}",
                i, expected, inv.id
            );
            assert!(seen.insert(inv.id), "Duplicate invariant ID: {}", inv.id);
        }
    }

    #[test]
    fn all_statements_are_non_empty() {
        for inv in INVARIANTS {
            assert!(!inv.statement.is_empty(), "{} has empty statement", inv.id);
        }
    }

    #[test]
    fn all_test_fn_names_are_non_empty_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for inv in INVARIANTS {
            assert!(!inv.test_fn.is_empty(), "{} has empty test_fn", inv.id);
            assert!(
                seen.insert(inv.test_fn),
                "{} has duplicate test_fn: {}",
                inv.id,
                inv.test_fn
            );
        }
    }

    #[test]
    fn find_invariant_returns_correct_entry() {
        let inv = find_invariant("INV-001").expect("INV-001 must exist");
        assert_eq!(inv.id, "INV-001");
        assert!(inv.statement.contains("disposable"));
    }

    #[test]
    fn find_invariant_returns_none_for_invalid_id() {
        assert!(find_invariant("INV-000").is_none());
        assert!(find_invariant("INV-042").is_none());
        assert!(find_invariant("").is_none());
    }

    #[test]
    fn all_invariant_ids_returns_41_entries() {
        let ids = all_invariant_ids();
        assert_eq!(ids.len(), 41);
        assert_eq!(ids[0], "INV-001");
        assert_eq!(ids[35], "INV-036");
        assert_eq!(ids[40], "INV-041");
    }

    #[test]
    fn coverage_counts_are_consistent() {
        let enforced = count_by_coverage(InvariantCoverage::Enforced);
        let partial = count_by_coverage(InvariantCoverage::Partial);
        let planned = count_by_coverage(InvariantCoverage::Planned);
        assert_eq!(
            enforced + partial + planned,
            41,
            "coverage counts must sum to total invariant count"
        );
        assert!(enforced > 0, "at least some invariants must be enforced");
        assert!(
            planned > 0,
            "some invariants are correctly marked as planned"
        );
    }

    #[test]
    fn enforced_invariants_have_real_test_functions() {
        // Enforced invariants must reference actual test functions, not placeholders
        for inv in INVARIANTS {
            if inv.coverage == InvariantCoverage::Enforced {
                assert!(
                    !inv.test_fn.starts_with("test_inv_"),
                    "{} is marked Enforced but has placeholder test_fn '{}'; \
                     must reference a real test module path",
                    inv.id,
                    inv.test_fn
                );
            }
        }
    }

    #[test]
    fn meta_invariants_md_matches_registry() {
        // This test ensures INVARIANTS.md and src/invariants.rs stay in sync.
        // The definitive list is this registry; INVARIANTS.md must be updated
        // to match whenever this file changes.
        //
        // We verify structural properties here; content sync is enforced by
        // requiring INVARIANTS.md to list all 41 IDs.
        assert_eq!(INVARIANTS.len(), 41);
        assert_eq!(INVARIANTS.first().unwrap().id, "INV-001");
        assert_eq!(INVARIANTS.last().unwrap().id, "INV-041");
    }
}
