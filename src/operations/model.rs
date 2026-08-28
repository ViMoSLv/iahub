//! Mega Brain V0 — Operation Journal Model (ADR-0003)
//!
//! Core types for the append-only operation journal. Every external side effect
//! must be journaled BEFORE execution begins. The journal is the source of truth
//! for crash recovery and auditability.

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum OperationType {
    WorktreeCreate,
    WorktreeRemove,
    GitRefCreate,
    GitRefDelete,
    AgentSpawn,
    AgentTerminate,
    MergeExecute,
    MergeAbort,
    LeaseAcquire,
    LeaseRevoke,
    ArtifactStore,
    NotificationSend,
    /// Catch-all for operations not yet classified.
    Other {
        detail: String,
    },
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorktreeCreate => write!(f, "WORKTREE_CREATE"),
            Self::WorktreeRemove => write!(f, "WORKTREE_REMOVE"),
            Self::GitRefCreate => write!(f, "GIT_REF_CREATE"),
            Self::GitRefDelete => write!(f, "GIT_REF_DELETE"),
            Self::AgentSpawn => write!(f, "AGENT_SPAWN"),
            Self::AgentTerminate => write!(f, "AGENT_TERMINATE"),
            Self::MergeExecute => write!(f, "MERGE_EXECUTE"),
            Self::MergeAbort => write!(f, "MERGE_ABORT"),
            Self::LeaseAcquire => write!(f, "LEASE_ACQUIRE"),
            Self::LeaseRevoke => write!(f, "LEASE_REVOKE"),
            Self::ArtifactStore => write!(f, "ARTIFACT_STORE"),
            Self::NotificationSend => write!(f, "NOTIFICATION_SEND"),
            Self::Other { detail } => write!(f, "OTHER({})", detail),
        }
    }
}

/// Lifecycle states for an operation journal entry.
/// Unknown variants fail closed on deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
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
    RequiresReconcile,
    /// Operation failed definitively with no side effects.
    Failed,
}

impl OperationStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Committed | Self::RolledBack | Self::RequiresReconcile | Self::Failed
        )
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
pub fn validate_operation_transition(
    from: OperationStatus,
    to: OperationStatus,
) -> Result<(), OperationTransitionError> {
    let valid = matches!(
        (from, to),
        (OperationStatus::Prepared, OperationStatus::Executing)
            | (OperationStatus::Prepared, OperationStatus::Failed)
            | (
                OperationStatus::Executing,
                OperationStatus::SideEffectObserved
            )
            | (OperationStatus::Executing, OperationStatus::Failed)
            | (
                OperationStatus::Executing,
                OperationStatus::RequiresReconcile
            )
            | (
                OperationStatus::SideEffectObserved,
                OperationStatus::Committed
            )
            | (
                OperationStatus::SideEffectObserved,
                OperationStatus::RolledBack
            )
            | (
                OperationStatus::SideEffectObserved,
                OperationStatus::RequiresReconcile
            )
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
            OperationStatus::RequiresReconcile,
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
    fn operation_type_display_matches_serde_variant() {
        assert_eq!(OperationType::WorktreeCreate.to_string(), "WORKTREE_CREATE");
        assert_eq!(OperationType::AgentSpawn.to_string(), "AGENT_SPAWN");
        assert_eq!(
            OperationType::Other {
                detail: "custom".into()
            }
            .to_string(),
            "OTHER(custom)"
        );
    }

    #[test]
    fn operation_status_roundtrip() {
        for status in [
            OperationStatus::Prepared,
            OperationStatus::Executing,
            OperationStatus::SideEffectObserved,
            OperationStatus::Committed,
            OperationStatus::RolledBack,
            OperationStatus::RequiresReconcile,
            OperationStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: OperationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn unknown_operation_status_fails_deserialization() {
        let result: Result<OperationStatus, _> = serde_json::from_str("\"NONEXISTENT\"");
        assert!(result.is_err(), "unknown status must fail closed");
    }

    #[test]
    fn prepare_creates_version_one() {
        let record = OperationRecord::prepare(
            OperationId("op-001".into()),
            OperationType::WorktreeCreate,
            1700000000,
        );
        assert_eq!(record.version, 1);
        assert_eq!(record.status, OperationStatus::Prepared);
        assert_eq!(record.prepared_at, 1700000000);
        assert!(record.started_at.is_none());
        assert!(record.committed_at.is_none());
    }
}
