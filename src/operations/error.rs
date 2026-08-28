//! Mega Brain V0 — Operation Journal Error Types
//!
//! Typed errors for operation journal operations. All failures are explicit;
//! no panics, no silent defaults, no fabricated identities.

use super::model::{OperationId, OperationStatus};
use std::fmt;

/// Errors returned by OperationService operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// Operation not found by ID.
    NotFound { operation_id: OperationId },

    /// Optimistic concurrency conflict: version mismatch during update.
    VersionConflict {
        operation_id: OperationId,
        expected_version: i64,
        actual_version: i64,
    },

    /// Invalid state transition attempted.
    InvalidTransition {
        operation_id: OperationId,
        from: OperationStatus,
        to: OperationStatus,
    },

    /// Persistence layer failure.
    Persistence { message: String },
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { operation_id } => {
                write!(f, "OPERATION_NOT_FOUND: {}", operation_id)
            }
            Self::VersionConflict {
                operation_id,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "VERSION_CONFLICT: operation {} expected v{}, got v{}",
                    operation_id, expected_version, actual_version
                )
            }
            Self::InvalidTransition {
                operation_id,
                from,
                to,
            } => {
                write!(
                    f,
                    "INVALID_TRANSITION: operation {} cannot transition {:?} → {:?}",
                    operation_id, from, to
                )
            }
            Self::Persistence { message } => {
                write!(f, "PERSISTENCE_ERROR: {}", message)
            }
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_operation_id() {
        let err = OperationError::NotFound {
            operation_id: OperationId("op-123".into()),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("op-123"));
        assert!(msg.contains("OPERATION_NOT_FOUND"));
    }

    #[test]
    fn version_conflict_display() {
        let err = OperationError::VersionConflict {
            operation_id: OperationId("op-456".into()),
            expected_version: 3,
            actual_version: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("VERSION_CONFLICT"));
        assert!(msg.contains("3"));
        assert!(msg.contains("5"));
    }
}
