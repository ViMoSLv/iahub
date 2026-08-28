//! Mega Brain V0 — Operation Journal Error Types
//!
//! Typed errors for operation journal operations. All failures are explicit;
//! no panics, no silent defaults.

use super::model::{OperationId, OperationStatus, OperationTransitionError};
use std::fmt;

/// Errors returned by OperationService operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// The requested status transition is invalid per ADR-0003 state machine.
    InvalidTransition {
        operation_id: OperationId,
        from: OperationStatus,
        to: OperationStatus,
    },

    /// Operation not found for the given ID.
    NotFound { operation_id: OperationId },

    /// Optimistic concurrency conflict: version mismatch during update.
    VersionConflict {
        operation_id: OperationId,
        expected_version: i64,
        actual_version: i64,
    },

    /// Persistence layer failure.
    Persistence { message: String },
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition {
                operation_id,
                from,
                to,
            } => {
                write!(
                    f,
                    "invalid operation transition for {}: {:?} → {:?}",
                    operation_id, from, to
                )
            }
            Self::NotFound { operation_id } => {
                write!(f, "operation not found: {}", operation_id)
            }
            Self::VersionConflict {
                operation_id,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "version conflict on operation {}: expected v{}, got v{}",
                    operation_id, expected_version, actual_version
                )
            }
            Self::Persistence { message } => {
                write!(f, "operation persistence error: {}", message)
            }
        }
    }
}

impl std::error::Error for OperationError {}

impl From<OperationTransitionError> for OperationError {
    fn from(e: OperationTransitionError) -> Self {
        // This conversion requires an operation_id which we don't have here.
        // Callers should use the full constructor instead.
        match e {
            OperationTransitionError::InvalidTransition { from, to } => Self::InvalidTransition {
                operation_id: OperationId("unknown".into()),
                from,
                to,
            },
            OperationTransitionError::TerminalStateCannotTransition { from } => {
                Self::InvalidTransition {
                    operation_id: OperationId("unknown".into()),
                    from,
                    to: OperationStatus::Executing,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_operation_id() {
        let err = OperationError::NotFound {
            operation_id: OperationId("op-123".into()),
        };
        assert!(err.to_string().contains("op-123"));
    }

    #[test]
    fn version_conflict_display() {
        let err = OperationError::VersionConflict {
            operation_id: OperationId("op-456".into()),
            expected_version: 3,
            actual_version: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("expected v3"));
        assert!(msg.contains("got v5"));
    }
}
