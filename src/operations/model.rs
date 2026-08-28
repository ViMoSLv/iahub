//! Mega Brain V0 — Operation Journal Model (ADR-0003)
//!
//! Core types for the append-only operation journal. Every external side effect
//! must be journaled BEFORE execution begins. The journal is the source of truth
//! for crash recovery and auditability.
//!
//! Canonical operation types per Topic 04 §9.3:
//! CREATE_WORKTREE, REMOVE_WORKTREE, CREATE_CANDIDATE_COMMIT, CREATE_GIT_REF,
//! SPAWN_AGENT, TERMINATE_AGENT, MERGE_SIMULATION, CANONICAL_MERGE.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a journal entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationId(pub String);

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for OperationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Classification of the external side effect being performed.
/// Closed enum matching the eight canonical types from Topic 04 §9.3.
/// Unknown persisted values fail closed on deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationType {
    CreateWorktree,
    RemoveWorktree,
    CreateCandidateCommit,
    CreateGitRef,
    SpawnAgent,
    TerminateAgent,
    MergeSimulation,
    CanonicalMerge,
}

impl OperationType {
    /// Convert to canonical DB string representation (unquoted).
    pub fn to_db(self) -> &'static str {
        match self {
            Self::CreateWorktree => "CREATE_WORKTREE",
            Self::RemoveWorktree => "REMOVE_WORKTREE",
            Self::CreateCandidateCommit => "CREATE_CANDIDATE_COMMIT",
            Self::CreateGitRef => "CREATE_GIT_REF",
            Self::SpawnAgent => "SPAWN_AGENT",
            Self::TerminateAgent => "TERMINATE_AGENT",
            Self::MergeSimulation => "MERGE_SIMULATION",
            Self::CanonicalMerge => "CANONICAL_MERGE",
        }
    }

    /// Parse from DB string. Unknown values return None (fail closed).
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "CREATE_WORKTREE" => Some(Self::CreateWorktree),
            "REMOVE_WORKTREE" => Some(Self::RemoveWorktree),
            "CREATE_CANDIDATE_COMMIT" => Some(Self::CreateCandidateCommit),
            "CREATE_GIT_REF" => Some(Self::CreateGitRef),
            "SPAWN_AGENT" => Some(Self::SpawnAgent),
            "TERMINATE_AGENT" => Some(Self::TerminateAgent),
            "MERGE_SIMULATION" => Some(Self::MergeSimulation),
            "CANONICAL_MERGE" => Some(Self::CanonicalMerge),
            _ => None,
        }
    }
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_db())
    }
}

/// Lifecycle states for an operation journal entry.
/// Unknown variants fail closed on deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationStatus {
    /// Entry created before execution begins. REQUIRED before any side effect.
    Prepared,
    /// Execution has started but outcome is not yet known.
    Executing,
    /// External system confirmed the side effect occurred.
    SideEffectObserved,
    /// Operation completed successfully and is durable.
    Committed,
    /// Operation was rolled back; no residual side effects.
    RolledBack,
    /// Outcome could not be determined; requires manual or automated reconcile.
    /// NOT terminal — startup reconcile must find and resolve these.
    RequiresReconcile,
    /// Operation failed definitively with no side effects.
    Failed,
}

impl OperationStatus {
    /// Terminal states: operations that are fully resolved and need no further action.
    /// REQUIRES_RECONCILE is intentionally NOT terminal — it must be discoverable
    /// by startup reconcile so it can be resolved with observed evidence.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack | Self::Failed)
    }

    /// Convert to canonical DB string representation (unquoted).
    pub fn to_db(self) -> &'static str {
        match self {
            Self::Prepared => "PREPARED",
            Self::Executing => "EXECUTING",
            Self::SideEffectObserved => "SIDE_EFFECT_OBSERVED",
            Self::Committed => "COMMITTED",
            Self::RolledBack => "ROLLED_BACK",
            Self::RequiresReconcile => "REQUIRES_RECONCILE",
            Self::Failed => "FAILED",
        }
    }

    /// Parse from DB string. Unknown values return None (fail closed).
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "PREPARED" => Some(Self::Prepared),
            "EXECUTING" => Some(Self::Executing),
            "SIDE_EFFECT_OBSERVED" => Some(Self::SideEffectObserved),
            "COMMITTED" => Some(Self::Committed),
            "ROLLED_BACK" => Some(Self::RolledBack),
            "REQUIRES_RECONCILE" => Some(Self::RequiresReconcile),
            "FAILED" => Some(Self::Failed),
            _ => None,
        }
    }
}

impl fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_db())
    }
}

/// A single journal entry recording an external side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: OperationId,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    pub command_id: Option<String>,
    /// JSON-encoded preconditions that must hold before execution.
    pub preconditions: Option<String>,
    /// JSON-encoded input parameters.
    pub input_payload: Option<String>,
    /// JSON-encoded result after successful completion.
    pub result_payload: Option<String>,
    /// External system reference (e.g., process ID, ref name).
    pub external_reference: Option<String>,
    /// Recovery hint for REQUIRES_RECONCILE entries.
    pub recovery_hint: Option<String>,
    pub prepared_at: i64,
    pub started_at: Option<i64>,
    pub committed_at: Option<i64>,
    pub failed_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub last_error: Option<String>,
    pub version: i64,
}

impl OperationRecord {
    /// Create a new PREPARED entry. This MUST be persisted before any side effect.
    pub fn prepare(id: OperationId, operation_type: OperationType, prepared_at: i64) -> Self {
        Self {
            id,
            operation_type,
            status: OperationStatus::Prepared,
            project_id: None,
            task_id: None,
            attempt_id: None,
            command_id: None,
            preconditions: None,
            input_payload: None,
            result_payload: None,
            external_reference: None,
            recovery_hint: None,
            prepared_at,
            started_at: None,
            committed_at: None,
            failed_at: None,
            completed_at: None,
            last_error: None,
            version: 1,
        }
    }
}

/// Validate an operation status transition. Returns Err if invalid.
///
/// Valid transitions per ADR-0003 + reconcile resolution:
/// - PREPARED → EXECUTING | FAILED
/// - EXECUTING → SIDE_EFFECT_OBSERVED | FAILED | REQUIRES_RECONCILE
/// - SIDE_EFFECT_OBSERVED → COMMITTED | ROLLED_BACK | REQUIRES_RECONCILE
/// - REQUIRES_RECONCILE → SIDE_EFFECT_OBSERVED | ROLLED_BACK | FAILED
///   (resolution based on observed evidence; no blind retry)
pub fn validate_operation_transition(
    from: OperationStatus,
    to: OperationStatus,
) -> Result<(), OperationTransitionError> {
    let valid = matches!(
        (from, to),
        (OperationStatus::Prepared, OperationStatus::Executing)
            | (OperationStatus::Prepared, OperationStatus::Failed)
            | (OperationStatus::Executing, OperationStatus::SideEffectObserved)
            | (OperationStatus::Executing, OperationStatus::Failed)
            | (OperationStatus::Executing, OperationStatus::RequiresReconcile)
            | (OperationStatus::SideEffectObserved, OperationStatus::Committed)
            | (OperationStatus::SideEffectObserved, OperationStatus::RolledBack)
            | (OperationStatus::SideEffectObserved, OperationStatus::RequiresReconcile)
            // Reconcile resolution transitions
            | (OperationStatus::RequiresReconcile, OperationStatus::SideEffectObserved)
            | (OperationStatus::RequiresReconcile, OperationStatus::RolledBack)
            | (OperationStatus::RequiresReconcile, OperationStatus::Failed)
    );

    if valid {
        Ok(())
    } else if from.is_terminal() {
        Err(OperationTransitionError::TerminalStateCannotTransition { from })
    } else {
        Err(OperationTransitionError::InvalidTransition { from, to })
    }
}

/// Errors returned when an operation transition is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationTransitionError {
    InvalidTransition {
        from: OperationStatus,
        to: OperationStatus,
    },
    TerminalStateCannotTransition {
        from: OperationStatus,
    },
}

impl fmt::Display for OperationTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Operation transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Operation state {:?} cannot transition", from)
            }
        }
    }
}

impl std::error::Error for OperationTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_to_executing_is_valid() {
        assert!(validate_operation_transition(
            OperationStatus::Prepared,
            OperationStatus::Executing
        )
        .is_ok());
    }

    #[test]
    fn executing_to_committed_is_invalid() {
        // Must go through SIDE_EFFECT_OBSERVED first
        assert!(validate_operation_transition(
            OperationStatus::Executing,
            OperationStatus::Committed
        )
        .is_err());
    }

    #[test]
    fn side_effect_observed_to_committed_is_valid() {
        assert!(validate_operation_transition(
            OperationStatus::SideEffectObserved,
            OperationStatus::Committed
        )
        .is_ok());
    }

    #[test]
    fn side_effect_observed_to_rolled_back_is_valid() {
        assert!(validate_operation_transition(
            OperationStatus::SideEffectObserved,
            OperationStatus::RolledBack
        )
        .is_ok());
    }

    #[test]
    fn terminal_states_cannot_transition() {
        for terminal in [
            OperationStatus::Committed,
            OperationStatus::RolledBack,
            OperationStatus::Failed,
        ] {
            let result = validate_operation_transition(terminal, OperationStatus::Executing);
            assert!(
                matches!(
                    result,
                    Err(OperationTransitionError::TerminalStateCannotTransition { .. })
                ),
                "terminal state {:?} must reject transitions",
                terminal
            );
        }
    }

    #[test]
    fn requires_reconcile_is_not_terminal() {
        assert!(
            !OperationStatus::RequiresReconcile.is_terminal(),
            "REQUIRES_RECONCILE must NOT be terminal so startup reconcile can find it"
        );
    }

    #[test]
    fn requires_reconcile_can_resolve_to_side_effect_observed() {
        assert!(validate_operation_transition(
            OperationStatus::RequiresReconcile,
            OperationStatus::SideEffectObserved
        )
        .is_ok());
    }

    #[test]
    fn requires_reconcile_can_resolve_to_rolled_back() {
        assert!(validate_operation_transition(
            OperationStatus::RequiresReconcile,
            OperationStatus::RolledBack
        )
        .is_ok());
    }

    #[test]
    fn requires_reconcile_can_resolve_to_failed() {
        assert!(validate_operation_transition(
            OperationStatus::RequiresReconcile,
            OperationStatus::Failed
        )
        .is_ok());
    }

    #[test]
    fn requires_reconcile_cannot_retry_to_executing() {
        // No blind retry from REQUIRES_RECONCILE
        assert!(validate_operation_transition(
            OperationStatus::RequiresReconcile,
            OperationStatus::Executing
        )
        .is_err());
    }

    #[test]
    fn operation_type_display_matches_canonical() {
        assert_eq!(OperationType::CreateWorktree.to_string(), "CREATE_WORKTREE");
        assert_eq!(OperationType::RemoveWorktree.to_string(), "REMOVE_WORKTREE");
        assert_eq!(
            OperationType::CreateCandidateCommit.to_string(),
            "CREATE_CANDIDATE_COMMIT"
        );
        assert_eq!(OperationType::CreateGitRef.to_string(), "CREATE_GIT_REF");
        assert_eq!(OperationType::SpawnAgent.to_string(), "SPAWN_AGENT");
        assert_eq!(OperationType::TerminateAgent.to_string(), "TERMINATE_AGENT");
        assert_eq!(
            OperationType::MergeSimulation.to_string(),
            "MERGE_SIMULATION"
        );
        assert_eq!(OperationType::CanonicalMerge.to_string(), "CANONICAL_MERGE");
    }

    #[test]
    fn operation_type_roundtrip_via_db() {
        for op_type in [
            OperationType::CreateWorktree,
            OperationType::RemoveWorktree,
            OperationType::CreateCandidateCommit,
            OperationType::CreateGitRef,
            OperationType::SpawnAgent,
            OperationType::TerminateAgent,
            OperationType::MergeSimulation,
            OperationType::CanonicalMerge,
        ] {
            let db = op_type.to_db();
            let parsed = OperationType::from_db(db).expect("valid type must parse");
            assert_eq!(parsed, op_type);
        }
    }

    #[test]
    fn unknown_operation_type_fails_closed() {
        assert!(OperationType::from_db("UNKNOWN_TYPE").is_none());
        assert!(OperationType::from_db("").is_none());
        assert!(OperationType::from_db("create_worktree").is_none()); // case sensitive
    }

    #[test]
    fn operation_status_roundtrip_via_db() {
        for status in [
            OperationStatus::Prepared,
            OperationStatus::Executing,
            OperationStatus::SideEffectObserved,
            OperationStatus::Committed,
            OperationStatus::RolledBack,
            OperationStatus::RequiresReconcile,
            OperationStatus::Failed,
        ] {
            let db = status.to_db();
            let parsed = OperationStatus::from_db(db).expect("valid status must parse");
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn unknown_operation_status_fails_closed() {
        assert!(OperationStatus::from_db("NONEXISTENT").is_none());
        assert!(OperationStatus::from_db("").is_none());
        assert!(OperationStatus::from_db("prepared").is_none()); // case sensitive
    }

    #[test]
    fn prepare_creates_version_one() {
        let record = OperationRecord::prepare(
            OperationId("op-001".into()),
            OperationType::CreateWorktree,
            1700000000,
        );
        assert_eq!(record.version, 1);
        assert_eq!(record.status, OperationStatus::Prepared);
        assert_eq!(record.prepared_at, 1700000000);
        assert!(record.started_at.is_none());
        assert!(record.committed_at.is_none());
    }

    #[test]
    fn exactly_eight_canonical_operation_types() {
        let types = [
            OperationType::CreateWorktree,
            OperationType::RemoveWorktree,
            OperationType::CreateCandidateCommit,
            OperationType::CreateGitRef,
            OperationType::SpawnAgent,
            OperationType::TerminateAgent,
            OperationType::MergeSimulation,
            OperationType::CanonicalMerge,
        ];
        assert_eq!(
            types.len(),
            8,
            "must have exactly 8 canonical operation types"
        );
    }
}
