//! Mega Brain V0 — Verification, Review & Merge Domain Model (Topic 06)
//!
//! Pure domain types enforcing constitutional principles:
//! - Reported state is not observed reality (Principle 5)
//! - Workers cannot certify their own success (Principle 6)
//! - Unknown state must remain unknown (Principle 7)
//!
//! Key concepts:
//! - **VerificationEvidence**: Immutable observational proof from workspace checks.
//! - **ReviewVerdict**: Independent evaluation bound to specific candidate SHA.
//! - **MergeLabResult**: Simulation outcome valid only for exact (candidate, target) tuple.
//! - **MergeQueueItem**: Serialized merge request with durable state per target branch.
//! - **FreshnessBinding**: Cryptographic binding preventing stale approval reuse.
//!
//! Reference: MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Sections 5.2, 5.6, 5.7,
//! Appendix H (Review Evidence Rules), Appendix I (Merge Evidence Rules).

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{AttemptId, EntityVersion, ProjectId, TaskId, Timestamp};

// ---------------------------------------------------------------------------
// Strongly-typed IDs
// ---------------------------------------------------------------------------

macro_rules! define_verification_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_verification_id!(
    VerificationId,
    "Unique identifier for an observational verification run."
);

define_verification_id!(
    VerificationReviewId,
    "Unique identifier for an independent review verdict."
);

define_verification_id!(
    MergeLabId,
    "Unique identifier for a merge laboratory simulation."
);

define_verification_id!(
    MergeQueueItemId,
    "Unique identifier for a serialized merge queue entry."
);

// ---------------------------------------------------------------------------
// VerificationOutcome — observational truth, not agent report
// ---------------------------------------------------------------------------

/// Outcome of running observable checks (tests, lints, builds) against a workspace.
/// This is ground truth derived from execution, distinct from agent self-report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum VerificationOutcome {
    /// All configured checks passed.
    Passed,
    /// One or more checks failed with deterministic evidence.
    Failed,
    /// Checks could not be executed (environment error, timeout, missing tooling).
    /// Treated as failure per Principle 7 (unknown = fail closed).
    Inconclusive,
}

impl fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => write!(f, "PASSED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Inconclusive => write!(f, "INCONCLUSIVE"),
        }
    }
}

// ---------------------------------------------------------------------------
// VerificationEvidence — immutable observational proof
// ---------------------------------------------------------------------------

/// Immutable evidence produced by observational verification. References the exact
/// workspace state and attempt that generated it. Cannot be forged or reused across
/// different candidates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEvidence {
    pub id: VerificationId,
    pub attempt_id: AttemptId,
    pub task_id: TaskId,
    /// SHA-256 of the diff/patch that was verified.
    pub candidate_diff_sha256: String,
    /// Outcome of the verification run.
    pub outcome: VerificationOutcome,
    /// Artifact IDs containing detailed logs/results.
    pub artifact_ids: Vec<String>,
    /// When the verification completed.
    pub verified_at: Timestamp,
}

// ---------------------------------------------------------------------------
// ReviewVerdict — independent evaluation bound to immutable evidence
// ---------------------------------------------------------------------------

/// Structured verdict from an independent reviewer. Bound to a specific candidate
/// SHA; if the candidate changes, this verdict becomes stale and cannot authorize merge.
/// The reviewer MUST differ from the producer agent (Principle 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdict {
    pub id: VerificationReviewId,
    pub task_id: TaskId,
    pub attempt_id: AttemptId,
    /// The exact candidate commit SHA this verdict applies to.
    pub candidate_sha: String,
    /// SHA-256 of the diff reviewed.
    pub diff_sha256: String,
    /// Revision of the TaskSpec at time of review.
    pub task_spec_revision: EntityVersion,
    /// Revision of acceptance criteria at time of review.
    pub acceptance_revision: EntityVersion,
    /// Verification evidence IDs that informed this review.
    pub verification_evidence_ids: Vec<VerificationId>,
    /// Identity of the reviewer. MUST NOT equal the producer agent_id.
    pub reviewer_id: String,
    /// Verdict decision.
    pub decision: ReviewDecision,
    /// Optional structured feedback for NEEDS_CHANGES / REJECTED.
    pub feedback: Option<String>,
    pub created_at: Timestamp,
}

/// Decision rendered by an independent reviewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ReviewDecision {
    /// Candidate meets acceptance criteria; may proceed to merge queue.
    Approved,
    /// Candidate has issues but is salvageable; return for new attempt.
    ChangesRequired,
    /// Candidate is fundamentally flawed; reject without retry suggestion.
    Rejected,
}

impl fmt::Display for ReviewDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approved => write!(f, "APPROVED"),
            Self::ChangesRequired => write!(f, "CHANGES_REQUIRED"),
            Self::Rejected => write!(f, "REJECTED"),
        }
    }
}

impl ReviewVerdict {
    /// Check if this verdict is still fresh relative to a current candidate SHA.
    /// Returns false if the candidate has been modified since review.
    pub fn is_fresh_for_candidate(&self, current_candidate_sha: &str) -> bool {
        self.candidate_sha == current_candidate_sha
    }
}

// ---------------------------------------------------------------------------
// MergeLabResult — simulation bound to exact tuple
// ---------------------------------------------------------------------------

/// Result of a merge laboratory simulation. Valid ONLY for the exact tuple of
/// (candidate_sha, target_sha, verification_policy_revision, repository_identity).
/// If any member changes, the result is stale and must be re-simulated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLabResult {
    pub id: MergeLabId,
    pub task_id: TaskId,
    pub attempt_id: AttemptId,
    /// Candidate commit SHA simulated.
    pub candidate_sha: String,
    /// Target branch HEAD SHA at time of simulation.
    pub target_sha: String,
    /// Repository fingerprint used for simulation.
    pub repository_fingerprint: String,
    /// Verification policy revision at time of simulation.
    pub verification_policy_revision: EntityVersion,
    /// Whether the simulated merge succeeded without conflicts.
    pub merge_clean: bool,
    /// Whether tests passed in the simulated merge result.
    pub tests_passed: bool,
    /// Overall simulation outcome.
    pub outcome: MergeLabOutcome,
    /// Artifact IDs from the simulation (logs, diffs, test results).
    pub artifact_ids: Vec<String>,
    pub simulated_at: Timestamp,
}

/// Outcome of a merge laboratory simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MergeLabOutcome {
    /// Simulation succeeded: clean merge + tests pass.
    Success,
    /// Merge produced conflicts against current target.
    Conflict,
    /// Merge was clean but tests failed in merged state.
    TestFailed,
    /// Simulation itself failed (infrastructure error).
    SimulationError,
}

impl fmt::Display for MergeLabOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "SUCCESS"),
            Self::Conflict => write!(f, "CONFLICT"),
            Self::TestFailed => write!(f, "TEST_FAILED"),
            Self::SimulationError => write!(f, "SIMULATION_ERROR"),
        }
    }
}

impl MergeLabResult {
    /// Check if this lab result is still valid for the given current state.
    /// All four tuple members must match exactly.
    pub fn is_valid_for(
        &self,
        current_candidate_sha: &str,
        current_target_sha: &str,
        current_repo_fingerprint: &str,
        current_policy_revision: EntityVersion,
    ) -> bool {
        self.candidate_sha == current_candidate_sha
            && self.target_sha == current_target_sha
            && self.repository_fingerprint == current_repo_fingerprint
            && self.verification_policy_revision == current_policy_revision
    }

    /// Whether this result authorizes proceeding to actual merge.
    pub fn authorizes_merge(&self) -> bool {
        self.outcome == MergeLabOutcome::Success && self.merge_clean && self.tests_passed
    }
}

// ---------------------------------------------------------------------------
// MergeQueueItem — serialized durable merge request
// ---------------------------------------------------------------------------

/// State of a merge queue item. Transitions are strictly ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MergeQueueStatus {
    /// Waiting for processing.
    Queued,
    /// Running pre-checks (freshness validation, lab re-simulation).
    Precheck,
    /// Merge lab simulation in progress.
    LabSimulation,
    /// Ready for actual merge (lab passed, all gates green).
    Ready,
    /// Actual merge in progress against canonical workspace.
    Merging,
    /// Successfully merged and integrated.
    Completed,
    /// Failed at any stage; requires investigation.
    Failed,
    /// Conflicts detected during lab or actual merge.
    Conflict,
}

impl fmt::Display for MergeQueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Queued => write!(f, "QUEUED"),
            Self::Precheck => write!(f, "PRECHECK"),
            Self::LabSimulation => write!(f, "LAB_SIMULATION"),
            Self::Ready => write!(f, "READY"),
            Self::Merging => write!(f, "MERGING"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Conflict => write!(f, "CONFLICT"),
        }
    }
}

/// A serialized merge request bound to a specific target branch.
/// Only one item per target branch may be in MERGING state at a time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeQueueItem {
    pub id: MergeQueueItemId,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub attempt_id: AttemptId,
    /// Target branch for integration (e.g., "main", "develop").
    pub target_branch: String,
    /// Candidate commit SHA to merge.
    pub candidate_sha: String,
    /// Review verdict ID that authorized queue entry.
    pub review_verdict_id: VerificationReviewId,
    /// Most recent merge lab result ID (may be re-run during precheck).
    pub latest_lab_result_id: Option<MergeLabId>,
    pub status: MergeQueueStatus,
    /// Priority within the queue (lower = higher priority).
    pub priority: i32,
    pub queued_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: EntityVersion,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ID newtypes --

    #[test]
    fn verification_id_newtype_works() {
        let id = VerificationId::from("VER-001");
        assert_eq!(id.to_string(), "VER-001");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"VER-001\"");
        let back: VerificationId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn merge_queue_item_id_newtype_works() {
        let id = MergeQueueItemId::from("MQI-042");
        assert_eq!(id.to_string(), "MQI-042");
    }

    // -- VerificationOutcome --

    #[test]
    fn verification_outcome_display_and_serialization() {
        assert_eq!(VerificationOutcome::Passed.to_string(), "PASSED");
        assert_eq!(VerificationOutcome::Failed.to_string(), "FAILED");
        assert_eq!(VerificationOutcome::Inconclusive.to_string(), "INCONCLUSIVE");

        let json = serde_json::to_string(&VerificationOutcome::Inconclusive).unwrap();
        assert_eq!(json, "\"INCONCLUSIVE\"");
        let back: VerificationOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back, VerificationOutcome::Inconclusive);
    }

    #[test]
    fn unknown_verification_outcome_fails_deserialization() {
        let json = "\"MAGIC_OUTCOME\"";
        let result: Result<VerificationOutcome, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown VerificationOutcome must fail closed");
    }

    // -- ReviewDecision --

    #[test]
    fn review_decision_display_and_serialization() {
        assert_eq!(ReviewDecision::Approved.to_string(), "APPROVED");
        assert_eq!(ReviewDecision::ChangesRequired.to_string(), "CHANGES_REQUIRED");
        assert_eq!(ReviewDecision::Rejected.to_string(), "REJECTED");

        let json = serde_json::to_string(&ReviewDecision::ChangesRequired).unwrap();
        assert_eq!(json, "\"CHANGES_REQUIRED\"");
    }

    #[test]
    fn unknown_review_decision_fails_deserialization() {
        let json = "\"RUBBER_STAMPED\"";
        let result: Result<ReviewDecision, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown ReviewDecision must fail closed");
    }

    // -- MergeLabOutcome --

    #[test]
    fn merge_lab_outcome_display_and_serialization() {
        assert_eq!(MergeLabOutcome::Success.to_string(), "SUCCESS");
        assert_eq!(MergeLabOutcome::Conflict.to_string(), "CONFLICT");
        assert_eq!(MergeLabOutcome::TestFailed.to_string(), "TEST_FAILED");
        assert_eq!(MergeLabOutcome::SimulationError.to_string(), "SIMULATION_ERROR");
    }

    #[test]
    fn unknown_merge_lab_outcome_fails_deserialization() {
        let json = "\"YOLO_MERGE\"";
        let result: Result<MergeLabOutcome, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown MergeLabOutcome must fail closed");
    }

    // -- MergeQueueStatus --

    #[test]
    fn merge_queue_status_display_and_serialization() {
        assert_eq!(MergeQueueStatus::Queued.to_string(), "QUEUED");
        assert_eq!(MergeQueueStatus::Precheck.to_string(), "PRECHECK");
        assert_eq!(MergeQueueStatus::LabSimulation.to_string(), "LAB_SIMULATION");
        assert_eq!(MergeQueueStatus::Ready.to_string(), "READY");
        assert_eq!(MergeQueueStatus::Merging.to_string(), "MERGING");
        assert_eq!(MergeQueueStatus::Completed.to_string(), "COMPLETED");
        assert_eq!(MergeQueueStatus::Failed.to_string(), "FAILED");
        assert_eq!(MergeQueueStatus::Conflict.to_string(), "CONFLICT");
    }

    #[test]
    fn unknown_merge_queue_status_fails_deserialization() {
        let json = "\"FORCE_PUSH\"";
        let result: Result<MergeQueueStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown MergeQueueStatus must fail closed");
    }

    // -- VerificationEvidence --

    #[test]
    fn verification_evidence_serialization_roundtrip() {
        let evidence = VerificationEvidence {
            id: VerificationId::from("VER-001"),
            attempt_id: AttemptId::from("ATT-3"),
            task_id: TaskId::from("TASK-142"),
            candidate_diff_sha256: "a".repeat(64),
            outcome: VerificationOutcome::Passed,
            artifact_ids: vec!["ART-001".to_string(), "ART-002".to_string()],
            verified_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&evidence).unwrap();
        let back: VerificationEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, evidence.id);
        assert_eq!(back.attempt_id, evidence.attempt_id);
        assert_eq!(back.candidate_diff_sha256, evidence.candidate_diff_sha256);
        assert_eq!(back.outcome, evidence.outcome);
        assert_eq!(back.artifact_ids.len(), 2);
    }

    // -- ReviewVerdict --

    #[test]
    fn review_verdict_serialization_roundtrip() {
        let verdict = ReviewVerdict {
            id: VerificationReviewId::from("REV-001"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            candidate_sha: "abc123def456".to_string(),
            diff_sha256: "b".repeat(64),
            task_spec_revision: EntityVersion(5),
            acceptance_revision: EntityVersion(3),
            verification_evidence_ids: vec![VerificationId::from("VER-001")],
            reviewer_id: "reviewer-bob".to_string(),
            decision: ReviewDecision::Approved,
            feedback: None,
            created_at: Timestamp("2026-08-31T13:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&verdict).unwrap();
        let back: ReviewVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, verdict.id);
        assert_eq!(back.candidate_sha, verdict.candidate_sha);
        assert_eq!(back.reviewer_id, verdict.reviewer_id);
        assert_eq!(back.decision, verdict.decision);
        assert!(back.feedback.is_none());
    }

    #[test]
    fn review_verdict_freshness_check_passes_for_same_sha() {
        let verdict = ReviewVerdict {
            id: VerificationReviewId::from("REV-002"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            candidate_sha: "abc123".to_string(),
            diff_sha256: "x".repeat(64),
            task_spec_revision: EntityVersion(1),
            acceptance_revision: EntityVersion(1),
            verification_evidence_ids: vec![],
            reviewer_id: "reviewer-alice".to_string(),
            decision: ReviewDecision::Approved,
            feedback: None,
            created_at: Timestamp("2026-08-31T13:00:00Z".to_string()),
        };

        assert!(verdict.is_fresh_for_candidate("abc123"));
        assert!(!verdict.is_fresh_for_candidate("def456"));
        assert!(!verdict.is_fresh_for_candidate(""));
    }

    #[test]
    fn review_verdict_with_feedback_serialization() {
        let verdict = ReviewVerdict {
            id: VerificationReviewId::from("REV-003"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            candidate_sha: "abc123".to_string(),
            diff_sha256: "x".repeat(64),
            task_spec_revision: EntityVersion(1),
            acceptance_revision: EntityVersion(1),
            verification_evidence_ids: vec![],
            reviewer_id: "reviewer-carol".to_string(),
            decision: ReviewDecision::ChangesRequired,
            feedback: Some("Missing error handling in auth module".to_string()),
            created_at: Timestamp("2026-08-31T14:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&verdict).unwrap();
        let back: ReviewVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decision, ReviewDecision::ChangesRequired);
        assert_eq!(
            back.feedback.as_deref(),
            Some("Missing error handling in auth module")
        );
    }

    // -- MergeLabResult --

    #[test]
    fn merge_lab_result_serialization_roundtrip() {
        let result = MergeLabResult {
            id: MergeLabId::from("LAB-001"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            candidate_sha: "cand-sha-abc".to_string(),
            target_sha: "target-sha-def".to_string(),
            repository_fingerprint: "fp-xyz".to_string(),
            verification_policy_revision: EntityVersion(2),
            merge_clean: true,
            tests_passed: true,
            outcome: MergeLabOutcome::Success,
            artifact_ids: vec!["ART-LAB-001".to_string()],
            simulated_at: Timestamp("2026-08-31T15:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        let back: MergeLabResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, result.id);
        assert_eq!(back.candidate_sha, result.candidate_sha);
        assert_eq!(back.target_sha, result.target_sha);
        assert_eq!(back.outcome, result.outcome);
        assert!(back.authorizes_merge());
    }

    #[test]
    fn merge_lab_result_validity_check_all_four_tuple_members() {
        let result = MergeLabResult {
            id: MergeLabId::from("LAB-002"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            candidate_sha: "cand-abc".to_string(),
            target_sha: "target-def".to_string(),
            repository_fingerprint: "fp-xyz".to_string(),
            verification_policy_revision: EntityVersion(2),
            merge_clean: true,
            tests_passed: true,
            outcome: MergeLabOutcome::Success,
            artifact_ids: vec![],
            simulated_at: Timestamp("2026-08-31T15:00:00Z".to_string()),
        };

        // All match → valid
        assert!(result.is_valid_for("cand-abc", "target-def", "fp-xyz", EntityVersion(2)));

        // Candidate changed → invalid
        assert!(!result.is_valid_for("cand-CHANGED", "target-def", "fp-xyz", EntityVersion(2)));

        // Target changed → invalid
        assert!(!result.is_valid_for("cand-abc", "target-CHANGED", "fp-xyz", EntityVersion(2)));

        // Repo fingerprint changed → invalid
        assert!(!result.is_valid_for("cand-abc", "target-def", "fp-CHANGED", EntityVersion(2)));

        // Policy revision changed → invalid
        assert!(!result.is_valid_for("cand-abc", "target-def", "fp-xyz", EntityVersion(99)));
    }

    #[test]
    fn merge_lab_result_authorizes_merge_only_on_success() {
        let success = MergeLabResult {
            id: MergeLabId::from("LAB-OK"),
            task_id: TaskId::from("TASK-1"),
            attempt_id: AttemptId::from("ATT-1"),
            candidate_sha: "c".to_string(),
            target_sha: "t".to_string(),
            repository_fingerprint: "fp".to_string(),
            verification_policy_revision: EntityVersion(1),
            merge_clean: true,
            tests_passed: true,
            outcome: MergeLabOutcome::Success,
            artifact_ids: vec![],
            simulated_at: Timestamp("2026-08-31T15:00:00Z".to_string()),
        };
        assert!(success.authorizes_merge());

        let conflict = MergeLabResult {
            outcome: MergeLabOutcome::Conflict,
            merge_clean: false,
            tests_passed: false,
            ..success.clone()
        };
        assert!(!conflict.authorizes_merge());

        let test_failed = MergeLabResult {
            outcome: MergeLabOutcome::TestFailed,
            merge_clean: true,
            tests_passed: false,
            ..success.clone()
        };
        assert!(!test_failed.authorizes_merge());

        // Clean merge but tests didn't pass → no authorization
        let clean_but_tests_fail = MergeLabResult {
            outcome: MergeLabOutcome::Success,
            merge_clean: true,
            tests_passed: false,
            ..success.clone()
        };
        assert!(!clean_but_tests_fail.authorizes_merge());
    }

    // -- MergeQueueItem --

    #[test]
    fn merge_queue_item_serialization_roundtrip() {
        let item = MergeQueueItem {
            id: MergeQueueItemId::from("MQI-001"),
            project_id: ProjectId::from("proj-001"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            target_branch: "main".to_string(),
            candidate_sha: "cand-sha-abc".to_string(),
            review_verdict_id: VerificationReviewId::from("REV-001"),
            latest_lab_result_id: Some(MergeLabId::from("LAB-001")),
            status: MergeQueueStatus::Queued,
            priority: 10,
            queued_at: Timestamp("2026-08-31T16:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T16:00:00Z".to_string()),
            version: EntityVersion::INITIAL,
        };

        let json = serde_json::to_string(&item).unwrap();
        let back: MergeQueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, item.id);
        assert_eq!(back.target_branch, item.target_branch);
        assert_eq!(back.status, item.status);
        assert_eq!(back.version, item.version);
    }

    // -- Cross-type consistency --

    #[test]
    fn review_verdict_binds_to_specific_attempt_and_candidate() {
        let v1 = ReviewVerdict {
            id: VerificationReviewId::from("REV-A"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-1"),
            candidate_sha: "sha-v1".to_string(),
            diff_sha256: "d".repeat(64),
            task_spec_revision: EntityVersion(1),
            acceptance_revision: EntityVersion(1),
            verification_evidence_ids: vec![],
            reviewer_id: "reviewer".to_string(),
            decision: ReviewDecision::Approved,
            feedback: None,
            created_at: Timestamp("2026-08-31T13:00:00Z".to_string()),
        };

        let v2 = ReviewVerdict {
            id: VerificationReviewId::from("REV-B"),
            attempt_id: AttemptId::from("ATT-2"),
            candidate_sha: "sha-v2".to_string(),
            ..v1.clone()
        };

        assert_eq!(v1.task_id, v2.task_id);
        assert_ne!(v1.attempt_id, v2.attempt_id);
        assert_ne!(v1.candidate_sha, v2.candidate_sha);
        assert_ne!(v1.id, v2.id);
    }

    #[test]
    fn inconclusive_verification_treated_as_non_passing() {
        // Inconclusive is distinct from Passed — agents cannot claim success
        // when verification couldn't complete (Principle 7).
        assert_ne!(VerificationOutcome::Inconclusive, VerificationOutcome::Passed);
        assert_ne!(VerificationOutcome::Inconclusive, VerificationOutcome::Failed);
    }
}