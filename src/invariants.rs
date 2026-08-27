/// Mega Brain V0 — Invariant Definitions (INV-001 through INV-036)
///
/// Each invariant is a compile-time constant with a unique ID, human-readable
/// statement matching the blueprint, and an associated test function name.
/// Violations are detected by automated tests and PR gates.

/// A single architectural invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    /// Unique identifier, e.g. "INV-001".
    pub id: &'static str,
    /// Human-readable statement derived from the blueprint.
    pub statement: &'static str,
    /// Name of the test function that validates this invariant.
    pub test_fn: &'static str,
}

/// All 36 invariants required by Appendix Q before MB-BOOTSTRAP-001.
pub const INVARIANTS: &[Invariant] = &[
    Invariant {
        id: "INV-001",
        statement: "Agents are disposable; no orchestration state depends on agent liveness.",
        test_fn: "test_inv_001_agents_disposable",
    },
    Invariant {
        id: "INV-002",
        statement: "No two active Attempts share the same workspace path.",
        test_fn: "test_inv_002_no_shared_workspace",
    },
    Invariant {
        id: "INV-003",
        statement: "Agent-to-agent communication occurs only through durable state, never direct chat.",
        test_fn: "test_inv_003_state_not_conversation",
    },
    Invariant {
        id: "INV-004",
        statement: "Every external side effect has a corresponding operations journal entry before execution.",
        test_fn: "test_inv_004_journal_before_side_effect",
    },
    Invariant {
        id: "INV-005",
        statement: "Reported agent outcome is never trusted without independent observational verification.",
        test_fn: "test_inv_005_reported_not_observed",
    },
    Invariant {
        id: "INV-006",
        statement: "The agent that produced a candidate cannot be the reviewer of that same candidate.",
        test_fn: "test_inv_006_no_self_review",
    },
    Invariant {
        id: "INV-007",
        statement: "When state cannot be determined, it remains UNKNOWN rather than defaulting to success or failure.",
        test_fn: "test_inv_007_unknown_remains_unknown",
    },
    Invariant {
        id: "INV-008",
        statement: "Code reality is determined exclusively by Git, never by filesystem observation alone.",
        test_fn: "test_inv_008_git_source_of_truth",
    },
    Invariant {
        id: "INV-009",
        statement: "Orchestration state is stored exclusively in SQLite; no other database is used for coordination.",
        test_fn: "test_inv_009_sqlite_source_of_truth",
    },
    Invariant {
        id: "INV-010",
        statement: "Filesystem watcher events are treated as hints and always verified against authoritative state.",
        test_fn: "test_inv_010_fs_watchers_are_hints",
    },
    Invariant {
        id: "INV-011",
        statement: "MCP servers expose only read/write facades over Hub state and contain no business logic.",
        test_fn: "test_inv_011_mcp_is_adapter",
    },
    Invariant {
        id: "INV-012",
        statement: "No agent process receives write access to the canonical integration workspace.",
        test_fn: "test_inv_012_no_canonical_mutation",
    },
    Invariant {
        id: "INV-013",
        statement: "No agent may execute a merge against the target branch; only the Hub merge queue does.",
        test_fn: "test_inv_013_no_agent_merge",
    },
    Invariant {
        id: "INV-014",
        statement: "All consequential state transitions pass through Hub-owned command handlers.",
        test_fn: "test_inv_014_hub_owns_transitions",
    },
    Invariant {
        id: "INV-015",
        statement: "A Task identity persists across retries, reassignments, reviews, and agent replacements.",
        test_fn: "test_inv_015_task_survives_lifecycle",
    },
    Invariant {
        id: "INV-016",
        statement: "Task completion requires verified evidence, not merely an agent self-report of done.",
        test_fn: "test_inv_016_done_requires_evidence",
    },
    Invariant {
        id: "INV-017",
        statement: "Closing the UI does not terminate active sessions or lose orchestration state.",
        test_fn: "test_inv_017_ui_disposable",
    },
    Invariant {
        id: "INV-018",
        statement: "No critical mutable orchestration state exists solely in process memory.",
        test_fn: "test_inv_018_no_ram_only_state",
    },
    Invariant {
        id: "INV-019",
        statement: "Every external side effect produces a durable journal entry recoverable after crash.",
        test_fn: "test_inv_019_journal_all_side_effects",
    },
    Invariant {
        id: "INV-020",
        statement: "Every failure carries a classified failure_reason; bare FAILED without classification is rejected.",
        test_fn: "test_inv_020_classify_failures",
    },
    Invariant {
        id: "INV-021",
        statement: "Repeated command_id with identical payload returns previously committed result without re-execution.",
        test_fn: "test_inv_021_command_idempotency",
    },
    Invariant {
        id: "INV-022",
        statement: "Repeated command_id with different payload returns 409 COMMAND_ID_PAYLOAD_MISMATCH.",
        test_fn: "test_inv_022_payload_mismatch_rejected",
    },
    Invariant {
        id: "INV-023",
        statement: "Concurrent updates to the same entity with stale version return 409 STATE_CONFLICT.",
        test_fn: "test_inv_023_optimistic_concurrency",
    },
    Invariant {
        id: "INV-024",
        statement: "Commands bearing an expired or superseded fencing token return 409 STALE_AUTHORITY.",
        test_fn: "test_inv_024_fencing_token_enforcement",
    },
    Invariant {
        id: "INV-025",
        statement: "Only one Attempt per Task may be in an active state (LEASED, STARTING, ACTIVE, SUBMITTED) at any time.",
        test_fn: "test_inv_025_one_active_attempt_per_task",
    },
    Invariant {
        id: "INV-026",
        statement: "Workspace paths are canonicalized and validated against symlinks, junctions, and case before use.",
        test_fn: "test_inv_026_path_safety",
    },
    Invariant {
        id: "INV-027",
        statement: "Write scope violations at submission time fail the Attempt regardless of agent intent.",
        test_fn: "test_inv_027_scope_drift_enforcement",
    },
    Invariant {
        id: "INV-028",
        statement: "Review verdict references immutable candidate SHA; SHA change invalidates the review.",
        test_fn: "test_inv_028_review_evidence_binding",
    },
    Invariant {
        id: "INV-029",
        statement: "Merge laboratory result is valid only for the exact (candidate_sha, target_sha, policy_revision, repo_identity) tuple.",
        test_fn: "test_inv_029_merge_evidence_binding",
    },
    Invariant {
        id: "INV-030",
        statement: "Merge queue processes exactly one item at a time per target branch.",
        test_fn: "test_inv_030_serialized_merge_queue",
    },
    Invariant {
        id: "INV-031",
        statement: "Startup reconcile scans leases, operations, workspaces, sessions, and tasks before accepting new commands.",
        test_fn: "test_inv_031_startup_reconcile",
    },
    Invariant {
        id: "INV-032",
        statement: "Process identity uses PID plus start timestamp or nonce; PID alone is insufficient.",
        test_fn: "test_inv_032_process_identity",
    },
    Invariant {
        id: "INV-033",
        statement: "Cancellation distinguishes CANCEL_REQUESTED from observed CANCELLED; indeterminate states are preserved.",
        test_fn: "test_inv_033_cancellation_semantics",
    },
    Invariant {
        id: "INV-034",
        statement: "Workspace cleanup never destroys unintegrated work without explicit evidence and policy authorization.",
        test_fn: "test_inv_034_cleanup_safety",
    },
    Invariant {
        id: "INV-035",
        statement: "Critical artifacts carry SHA-256 hash and schema version for integrity verification.",
        test_fn: "test_inv_035_artifact_integrity",
    },
    Invariant {
        id: "INV-036",
        statement: "Policy snapshot for a Run is immutable once RUNNING begins; changes require explicit migration event.",
        test_fn: "test_inv_036_policy_snapshot_immutability",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_count_is_exactly_36() {
        assert_eq!(
            INVARIANTS.len(),
            36,
            "Appendix Q requires exactly 36 invariants (INV-001 through INV-036)"
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
            assert!(
                !inv.statement.is_empty(),
                "{} has empty statement",
                inv.id
            );
        }
    }

    #[test]
    fn all_test_fn_names_are_non_empty_and_unique() {
        let mut seen = std::collections::HashSet::new();
        for inv in INVARIANTS {
            assert!(
                !inv.test_fn.is_empty(),
                "{} has empty test_fn",
                inv.id
            );
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
        assert!(find_invariant("INV-037").is_none());
        assert!(find_invariant("").is_none());
    }

    #[test]
    fn all_invariant_ids_returns_36_entries() {
        let ids = all_invariant_ids();
        assert_eq!(ids.len(), 36);
        assert_eq!(ids[0], "INV-001");
        assert_eq!(ids[35], "INV-036");
    }
}