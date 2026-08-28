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

impl CommandError {
    /// Returns a stable, machine-readable error code for this error variant.
    ///
    /// These codes are part of the public API contract and must not change
    /// across versions. They are consumed by CLI, MCP adapters, and future
    /// HTTP/REST boundaries for programmatic error handling.
    ///
    /// | Code                          | Variant                  |
    /// |-------------------------------|--------------------------|
    /// | `COMMAND_ID_PAYLOAD_MISMATCH` | DuplicateCommandMismatch |
    /// | `STATE_CONFLICT`              | StateConflict            |
    /// | `NOT_FOUND`                   | NotFound                 |
    /// | `INVALID_COMMAND`             | InvalidCommand           |
    /// | `INVALID_TRANSITION`          | InvalidTransition        |
    /// | `PRECONDITION_FAILED`         | PreconditionFailed       |
    /// | `STALE_AUTHORITY`             | StaleAuthority           |
    /// | `PERSISTENCE_ERROR`           | Persistence              |
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateCommandMismatch { .. } => "COMMAND_ID_PAYLOAD_MISMATCH",
            Self::StateConflict { .. } => "STATE_CONFLICT",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::InvalidCommand { .. } => "INVALID_COMMAND",
            Self::InvalidTransition { .. } => "INVALID_TRANSITION",
            Self::PreconditionFailed { .. } => "PRECONDITION_FAILED",
            Self::StaleAuthority { .. } => "STALE_AUTHORITY",
            Self::Persistence(_) => "PERSISTENCE_ERROR",
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
    fn error_codes_are_stable_and_exhaustive() {
        // Every variant must have a non-empty, uppercase-with-underscores code.
        // This test ensures no new variant is added without a corresponding code.
        let errors: Vec<CommandError> = vec![
            CommandError::DuplicateCommandMismatch {
                command_id: CommandId::from("c"),
            },
            CommandError::StateConflict {
                entity: "E",
                id: "i".into(),
                expected_version: EntityVersion(1),
            },
            CommandError::NotFound {
                entity: "E",
                id: "i".into(),
            },
            CommandError::InvalidCommand { detail: "d".into() },
            CommandError::InvalidTransition {
                detail: "E: A -> B".into(),
            },
            CommandError::PreconditionFailed { detail: "d".into() },
            CommandError::StaleAuthority {
                resource_type: "task".to_string(),
                resource_id: "t1".into(),
                presented_token: crate::domain::FencingToken(1),
                current_token: crate::domain::FencingToken(2),
                reason: crate::authority::model::StaleReason::LeaseExpired,
            },
            CommandError::Persistence(PersistenceError::Busy { detail: "d".into() }),
        ];

        for err in &errors {
            let code = err.code();
            assert!(!code.is_empty(), "code must not be empty for {:?}", err);
            assert!(
                code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
                "code '{}' must be UPPER_SNAKE_CASE",
                code
            );
        }

        // Verify specific codes match the documented contract
        assert_eq!(errors[0].code(), "COMMAND_ID_PAYLOAD_MISMATCH");
        assert_eq!(errors[1].code(), "STATE_CONFLICT");
        assert_eq!(errors[2].code(), "NOT_FOUND");
        assert_eq!(errors[3].code(), "INVALID_COMMAND");
        assert_eq!(errors[4].code(), "INVALID_TRANSITION");
        assert_eq!(errors[5].code(), "PRECONDITION_FAILED");
        assert_eq!(errors[6].code(), "STALE_AUTHORITY");
        assert_eq!(errors[7].code(), "PERSISTENCE_ERROR");
    }

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
