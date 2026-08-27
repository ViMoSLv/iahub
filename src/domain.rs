/// Mega Brain V0 — Core Domain Model
///
/// All entities defined here are durable, versioned, and governed by explicit
/// state machines. No entity may be mutated outside of a Command handler.
/// Reference: MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Section 4.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Project (Section 4.1)
// ---------------------------------------------------------------------------

/// A Git repository registered in Mega Brain.
/// Identified by repository fingerprint, not merely by current folder path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub canonical_path: String,
    pub git_common_dir: String,
    pub repository_fingerprint: String,
    pub default_target_branch: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

// ---------------------------------------------------------------------------
// Run (Section 4.2)
// ---------------------------------------------------------------------------

/// Top-level execution of a user objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub project_id: String,
    pub objective: String,
    pub status: RunStatus,
    pub plan_revision: i64,
    pub policy_snapshot_json: String,
    pub reported_outcome: Option<String>,
    pub verified_outcome: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
    pub version: i64,
}

/// Exhaustive Run states per STATE-MACHINES.md §5.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunStatus {
    Draft,
    Planning,
    PlanValidating,
    Ready,
    Running,
    Parked,
    Blocked,
    Escalated,
    Incomplete,
    Failed,
    Cancelled,
    OutcomeUnknown,
    Succeeded,
}

impl RunStatus {
    /// Returns true if this is a terminal state from which no transitions originate.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::Incomplete
                | Self::Escalated
                | Self::Cancelled
                | Self::OutcomeUnknown
        )
    }
}

// ---------------------------------------------------------------------------
// Task (Section 4.3)
// ---------------------------------------------------------------------------

/// A stable logical unit of work that survives retry, reassignment, review, and rework.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub run_id: String,
    pub title: String,
    pub objective: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub acceptance_json: String,
    pub verification_json: String,
    pub write_scope_json: String,
    pub budget_json: String,
    pub current_attempt_id: Option<String>,
    pub candidate_commit_sha: Option<String>,
    pub base_commit_sha: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
    pub version: i64,
}

/// Exhaustive Task states per STATE-MACHINES.md §5.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Created,
    Ready,
    Claimed,
    Running,
    Submitted,
    Verifying,
    Reviewing,
    MergeReady,
    Merging,
    Done,
    NeedsChanges,
    Blocked,
    Parked,
    Cancelled,
    Escalated,
    Incomplete,
    Failed,
    Conflict,
}

impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
}

// ---------------------------------------------------------------------------
// Attempt (Section 4.4)
// ---------------------------------------------------------------------------

/// One execution attempt of one Task. Owns execution authority via lease + fencing token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAttempt {
    pub id: String,
    pub task_id: String,
    pub attempt_no: i64,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub workspace_id: Option<String>,
    pub status: AttemptStatus,
    pub lease_id: Option<String>,
    pub fencing_token: Option<i64>,
    pub started_at: Option<String>,
    pub submitted_at: Option<String>,
    pub ended_at: Option<String>,
    pub reported_outcome: Option<String>,
    pub failure_reason: Option<String>,
}

/// Exhaustive Attempt states per STATE-MACHINES.md §5.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttemptStatus {
    Created,
    Leased,
    Starting,
    Active,
    Submitted,
    Blocked,
    Stale,
    Failed,
    Cancelled,
    Lost,
}

impl AttemptStatus {
    /// States considered "active" for the one-active-attempt-per-task invariant (INV-025).
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Leased | Self::Starting | Self::Active | Self::Submitted
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Cancelled | Self::Lost | Self::Stale
        )
    }
}

// ---------------------------------------------------------------------------
// Session (Section 4.5)
// ---------------------------------------------------------------------------

/// One live agent process/provider session attached to an Attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub task_attempt_id: Option<String>,
    pub provider_session_id: Option<String>,
    pub holder_process_id: Option<String>,
    pub state: SessionState,
    pub started_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub ended_at: Option<String>,
    pub exit_code: Option<i32>,
    pub observed_json: String,
}

/// Only observable session states per STATE-MACHINES.md §5.4.
/// THINKING/REASONING are intentionally excluded unless provider emits trustworthy telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionState {
    Created,
    Starting,
    Connected,
    Active,
    Idle,
    Unresponsive,
    Exited,
    Lost,
    Terminated,
}

// ---------------------------------------------------------------------------
// Workspace (Section 4.6)
// ---------------------------------------------------------------------------

/// Physical filesystem allocated to an Attempt. Default: GIT_WORKTREE, fallback: LOCAL_COPY.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub project_id: String,
    pub task_attempt_id: Option<String>,
    pub mode: WorkspaceMode,
    pub path: String,
    pub branch_name: Option<String>,
    pub base_commit_sha: String,
    pub status: WorkspaceStatus,
    pub created_at: String,
    pub sealed_at: Option<String>,
    pub removed_at: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceMode {
    GitWorktree,
    LocalCopy,
}

/// Exhaustive Workspace states per STATE-MACHINES.md §5.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceStatus {
    Planned,
    Creating,
    Ready,
    InUse,
    Dirty,
    Sealed,
    Releasing,
    Removed,
    Broken,
    Orphaned,
}

// ---------------------------------------------------------------------------
// Lease & Fencing Token (Sections 4.7–4.8)
// ---------------------------------------------------------------------------

/// Time-bounded authority over a Task Attempt or resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub owner_attempt_id: Option<String>,
    pub fencing_token: i64,
    pub expires_at: String,
    pub heartbeat_at: String,
    pub status: LeaseStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeaseStatus {
    Active,
    Expired,
    Revoked,
}

// ---------------------------------------------------------------------------
// Artifact (Section 4.9)
// ---------------------------------------------------------------------------

/// Immutable or append-only evidence produced during execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    pub artifact_type: ArtifactType,
    pub project_id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    pub content_path: Option<String>,
    pub inline_payload: Option<String>,
    pub sha256: String,
    pub size: u64,
    pub schema_version: String,
    pub created_at: String,
    pub producer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactType {
    ContextPack,
    Handoff,
    Diff,
    CandidateCommit,
    TestReport,
    Review,
    SecurityReport,
    TerminalOutput,
    Plan,
    Decision,
    MergeAnalysis,
}

// ---------------------------------------------------------------------------
// Review (Section 4.10)
// ---------------------------------------------------------------------------

/// Independent evaluation of a candidate produced by an Attempt.
/// Reviewer agent_id must differ from producing agent_id (INV-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub task_id: String,
    pub candidate_commit_sha: String,
    pub reviewer_agent_id: Option<String>,
    pub status: ReviewStatus,
    pub verdict_json: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Exhaustive Review states per STATE-MACHINES.md §5.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewStatus {
    Pending,
    Assigned,
    InReview,
    ChangesRequired,
    Rejected,
    Approved,
}

// ---------------------------------------------------------------------------
// Merge Item (Section 4.11)
// ---------------------------------------------------------------------------

/// Durable request to integrate one approved candidate into a target branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeItem {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub candidate_commit_sha: String,
    pub target_branch: String,
    pub expected_target_sha: String,
    pub priority: i32,
    pub status: MergeStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub failure_reason: Option<String>,
}

/// Exhaustive Merge states per STATE-MACHINES.md §5.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MergeStatus {
    Queued,
    Precheck,
    LabSimulation,
    Ready,
    Merging,
    Completed,
    Conflict,
    TestFailed,
    Failed,
}

// ---------------------------------------------------------------------------
// Task Dependency
// ---------------------------------------------------------------------------

/// Explicit dependency edge between two tasks with a reason (FlowCrew synthesis).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDependency {
    pub task_id: String,
    pub depends_on_task_id: String,
    pub reason: String,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Hard,
    Soft,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_status_terminal_states_are_correct() {
        let terminals = [
            RunStatus::Succeeded,
            RunStatus::Failed,
            RunStatus::Incomplete,
            RunStatus::Escalated,
            RunStatus::Cancelled,
            RunStatus::OutcomeUnknown,
        ];
        for s in &terminals {
            assert!(s.is_terminal(), "{:?} should be terminal", s);
        }

        let non_terminals = [
            RunStatus::Draft,
            RunStatus::Planning,
            RunStatus::PlanValidating,
            RunStatus::Ready,
            RunStatus::Running,
            RunStatus::Parked,
            RunStatus::Blocked,
        ];
        for s in &non_terminals {
            assert!(!s.is_terminal(), "{:?} should NOT be terminal", s);
        }
    }

    #[test]
    fn attempt_active_states_match_inv_025() {
        // INV-025: Only LEASED, STARTING, ACTIVE, SUBMITTED are "active"
        assert!(AttemptStatus::Leased.is_active());
        assert!(AttemptStatus::Starting.is_active());
        assert!(AttemptStatus::Active.is_active());
        assert!(AttemptStatus::Submitted.is_active());

        assert!(!AttemptStatus::Created.is_active());
        assert!(!AttemptStatus::Blocked.is_active());
        assert!(!AttemptStatus::Stale.is_active());
        assert!(!AttemptStatus::Failed.is_active());
        assert!(!AttemptStatus::Cancelled.is_active());
        assert!(!AttemptStatus::Lost.is_active());
    }

    #[test]
    fn task_done_and_cancelled_are_terminal() {
        assert!(TaskStatus::Done.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Verifying.is_terminal());
        assert!(!TaskStatus::MergeReady.is_terminal());
    }

    #[test]
    fn all_run_statuses_serialize_roundtrip() {
        let statuses = [
            RunStatus::Draft,
            RunStatus::Planning,
            RunStatus::PlanValidating,
            RunStatus::Ready,
            RunStatus::Running,
            RunStatus::Parked,
            RunStatus::Blocked,
            RunStatus::Escalated,
            RunStatus::Incomplete,
            RunStatus::Failed,
            RunStatus::Cancelled,
            RunStatus::OutcomeUnknown,
            RunStatus::Succeeded,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: RunStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_task_statuses_serialize_roundtrip() {
        let statuses = [
            TaskStatus::Created,
            TaskStatus::Ready,
            TaskStatus::Claimed,
            TaskStatus::Running,
            TaskStatus::Submitted,
            TaskStatus::Verifying,
            TaskStatus::Reviewing,
            TaskStatus::MergeReady,
            TaskStatus::Merging,
            TaskStatus::Done,
            TaskStatus::NeedsChanges,
            TaskStatus::Blocked,
            TaskStatus::Parked,
            TaskStatus::Cancelled,
            TaskStatus::Escalated,
            TaskStatus::Incomplete,
            TaskStatus::Failed,
            TaskStatus::Conflict,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: TaskStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_attempt_statuses_serialize_roundtrip() {
        let statuses = [
            AttemptStatus::Created,
            AttemptStatus::Leased,
            AttemptStatus::Starting,
            AttemptStatus::Active,
            AttemptStatus::Submitted,
            AttemptStatus::Blocked,
            AttemptStatus::Stale,
            AttemptStatus::Failed,
            AttemptStatus::Cancelled,
            AttemptStatus::Lost,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: AttemptStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_session_states_serialize_roundtrip() {
        let states = [
            SessionState::Created,
            SessionState::Starting,
            SessionState::Connected,
            SessionState::Active,
            SessionState::Idle,
            SessionState::Unresponsive,
            SessionState::Exited,
            SessionState::Lost,
            SessionState::Terminated,
        ];
        for s in &states {
            let json = serde_json::to_string(s).expect("serialize");
            let back: SessionState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_workspace_statuses_serialize_roundtrip() {
        let statuses = [
            WorkspaceStatus::Planned,
            WorkspaceStatus::Creating,
            WorkspaceStatus::Ready,
            WorkspaceStatus::InUse,
            WorkspaceStatus::Dirty,
            WorkspaceStatus::Sealed,
            WorkspaceStatus::Releasing,
            WorkspaceStatus::Removed,
            WorkspaceStatus::Broken,
            WorkspaceStatus::Orphaned,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: WorkspaceStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_review_statuses_serialize_roundtrip() {
        let statuses = [
            ReviewStatus::Pending,
            ReviewStatus::Assigned,
            ReviewStatus::InReview,
            ReviewStatus::ChangesRequired,
            ReviewStatus::Rejected,
            ReviewStatus::Approved,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: ReviewStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn all_merge_statuses_serialize_roundtrip() {
        let statuses = [
            MergeStatus::Queued,
            MergeStatus::Precheck,
            MergeStatus::LabSimulation,
            MergeStatus::Ready,
            MergeStatus::Merging,
            MergeStatus::Completed,
            MergeStatus::Conflict,
            MergeStatus::TestFailed,
            MergeStatus::Failed,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: MergeStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn artifact_types_serialize_kebab_case() {
        let t = ArtifactType::CandidateCommit;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"candidate-commit\"");

        let t2 = ArtifactType::ContextPack;
        let json2 = serde_json::to_string(&t2).unwrap();
        assert_eq!(json2, "\"context-pack\"");
    }

    #[test]
    fn workspace_modes_serialize_roundtrip() {
        let modes = [WorkspaceMode::GitWorktree, WorkspaceMode::LocalCopy];
        for m in &modes {
            let json = serde_json::to_string(m).expect("serialize");
            let back: WorkspaceMode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*m, back);
        }
    }

    #[test]
    fn lease_statuses_serialize_roundtrip() {
        let statuses = [LeaseStatus::Active, LeaseStatus::Expired, LeaseStatus::Revoked];
        for s in &statuses {
            let json = serde_json::to_string(s).expect("serialize");
            let back: LeaseStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn dependency_types_serialize_roundtrip() {
        let types = [DependencyType::Hard, DependencyType::Soft];
        for t in &types {
            let json = serde_json::to_string(t).expect("serialize");
            let back: DependencyType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*t, back);
        }
    }
}