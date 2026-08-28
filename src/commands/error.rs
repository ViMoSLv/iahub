//! Mega Brain V0 — Command Engine Error Types
//!
//! Typed errors for command execution. No `anyhow` or `rusqlite::Error` in
//! public API; persistence errors are translated at the boundary.

use std::fmt;

use crate::domain::{CommandId, EntityVersion};
use crate::persistence::PersistenceError;

/// All errors originating from command execution.
#[derive(Debug)]
pub enum CommandError {
    /// Same command_id already exists with a different payload hash.
    DuplicateCommandMismatch { command_id: CommandId },
    /// Optimistic concurrency conflict: expected version does not match stored.
    StateConflict {
        entity: &'static str,
        id: String,
        expected_version: EntityVersion,
    },
    /// Referenced entity does not exist.
    NotFound { entity: &'static str, id: String },
    /// Command payload failed validation before execution.
    InvalidCommand { detail: String },
    /// Domain state machine rejected the transition.
    InvalidTransition { detail: String },
    /// A precondition for the command was not satisfied.
    PreconditionFailed { detail: String },
    /// Stale fencing token: authority has been superseded by a newer lease.
    StaleAuthority {
        resource_type: String,
        resource_id: String,
        presented_token: crate::domain::FencingToken,
        current_token: crate::domain::FencingToken,
        reason: crate::authority::model::StaleReason,
    },
    /// Persistence layer error translated to command context.
    Persistence(PersistenceError),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCommandMismatch { command_id } => {
                write!(
                    f,
                    "command {} already exists with different payload",
                    command_id
                )
            }
            Self::StateConflict {
                entity,
                id,
                expected_version,
            } => {
                write!(
                    f,
                    "state conflict on {} {}: expected version {}",
                    entity, id, expected_version.0
                )
            }
            Self::NotFound { entity, id } => {
                write!(f, "{} not found: {}", entity, id)
            }
            Self::InvalidCommand { detail } => {
                write!(f, "invalid command: {}", detail)
            }
            Self::InvalidTransition { detail } => {
                write!(f, "invalid transition: {}", detail)
            }
            Self::PreconditionFailed { detail } => {
                write!(f, "precondition failed: {}", detail)
            }
            Self::StaleAuthority {
                resource_type,
                resource_id,
                presented_token,
                current_token,
                reason,
            } => {
                write!(
                    f,
                    "stale authority on {} {}: presented fencing token {} but current is {} ({})",
                    resource_type, resource_id, presented_token.0, current_token.0, reason
                )
            }
            Self::Persistence(e) => {
                write!(f, "persistence error: {}", e)
            }
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Persistence(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PersistenceError> for CommandError {
    fn from(e: PersistenceError) -> Self {
        // Translate specific persistence errors into typed command errors
        match &e {
            PersistenceError::VersionConflict {
                entity,
                id,
                expected_version,
            } => Self::StateConflict {
                entity,
                id: id.clone(),
                expected_version: EntityVersion(*expected_version),
            },
            PersistenceError::NotFound { entity, id } => Self::NotFound {
                entity,
                id: id.clone(),
            },
            _ => Self::Persistence(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_mismatch_display() {
        let err = CommandError::DuplicateCommandMismatch {
            command_id: CommandId::from("cmd-1"),
        };
        assert!(err.to_string().contains("cmd-1"));
        assert!(err.to_string().contains("different payload"));
    }

    #[test]
    fn state_conflict_display() {
        let err = CommandError::StateConflict {
            entity: "Project",
            id: "proj-1".to_string(),
            expected_version: EntityVersion(5),
        };
        let msg = err.to_string();
        assert!(msg.contains("Project"));
        assert!(msg.contains("proj-1"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn persistence_version_conflict_translates() {
        let pe = PersistenceError::VersionConflict {
            entity: "Task",
            id: "t-1".to_string(),
            expected_version: 3,
        };
        let ce: CommandError = pe.into();
        match ce {
            CommandError::StateConflict {
                entity,
                id,
                expected_version,
            } => {
                assert_eq!(entity, "Task");
                assert_eq!(id, "t-1");
                assert_eq!(expected_version, EntityVersion(3));
            }
            other => panic!("expected StateConflict, got {:?}", other),
        }
    }

    #[test]
    fn persistence_not_found_translates() {
        let pe = PersistenceError::NotFound {
            entity: "Run",
            id: "r-99".to_string(),
        };
        let ce: CommandError = pe.into();
        match ce {
            CommandError::NotFound { entity, id } => {
                assert_eq!(entity, "Run");
                assert_eq!(id, "r-99");
            }
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn other_persistence_errors_wrap() {
        let pe = PersistenceError::Busy {
            detail: "timeout".to_string(),
        };
        let ce: CommandError = pe.into();
        assert!(matches!(ce, CommandError::Persistence(_)));
    }
}
