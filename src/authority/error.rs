//! Mega Brain V0 — Authority Error Types
//!
//! Typed errors for lease operations. All authority failures are explicit;
//! no panics, no silent defaults. STALE_AUTHORITY is the canonical error
//! for any attempt to use a superseded fencing token.

use crate::authority::model::StaleReason;
use crate::domain::{FencingToken, LeaseId};
use std::fmt;

/// Errors returned by LeaseService operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    /// The provided fencing token is stale: lease expired, revoked, or
    /// superseded by a higher token. Command handlers must NOT execute.
    StaleAuthority {
        lease_id: LeaseId,
        reason: StaleReason,
    },

    /// Resource already has an ACTIVE lease owned by a different attempt.
    ResourceLocked {
        resource_type: String,
        resource_id: String,
        owner_attempt_id: String,
        current_fencing_token: FencingToken,
    },

    /// Lease not found for the given ID.
    LeaseNotFound { lease_id: LeaseId },

    /// Optimistic concurrency conflict: version mismatch during update.
    VersionConflict {
        lease_id: LeaseId,
        expected_version: u64,
        actual_version: u64,
    },

    /// Invalid request parameters (e.g., zero TTL, empty resource ID).
    InvalidRequest { message: String },

    /// Persistence layer failure.
    Persistence { message: String },
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorityError::StaleAuthority { lease_id, reason } => {
                write!(
                    f,
                    "STALE_AUTHORITY: lease {} is stale ({})",
                    lease_id, reason
                )
            }
            AuthorityError::ResourceLocked {
                resource_type,
                resource_id,
                owner_attempt_id,
                current_fencing_token,
            } => {
                write!(
                    f,
                    "RESOURCE_LOCKED: {}/{} held by attempt {} (token {})",
                    resource_type, resource_id, owner_attempt_id, current_fencing_token
                )
            }
            AuthorityError::LeaseNotFound { lease_id } => {
                write!(f, "LEASE_NOT_FOUND: {}", lease_id)
            }
            AuthorityError::VersionConflict {
                lease_id,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "VERSION_CONFLICT: lease {} expected v{}, got v{}",
                    lease_id, expected_version, actual_version
                )
            }
            AuthorityError::InvalidRequest { message } => {
                write!(f, "INVALID_REQUEST: {}", message)
            }
            AuthorityError::Persistence { message } => {
                write!(f, "PERSISTENCE_ERROR: {}", message)
            }
        }
    }
}

impl std::error::Error for AuthorityError {}

impl AuthorityError {
    pub fn is_stale(&self) -> bool {
        matches!(self, AuthorityError::StaleAuthority { .. })
    }

    pub fn is_locked(&self) -> bool {
        matches!(self, AuthorityError::ResourceLocked { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_authority_is_stale() {
        let err = AuthorityError::StaleAuthority {
            lease_id: LeaseId("l1".into()),
            reason: StaleReason::LeaseExpired,
        };
        assert!(err.is_stale());
        assert!(!err.is_locked());
    }

    #[test]
    fn resource_locked_is_locked() {
        let err = AuthorityError::ResourceLocked {
            resource_type: "task".into(),
            resource_id: "t1".into(),
            owner_attempt_id: "att-1".into(),
            current_fencing_token: FencingToken(1),
        };
        assert!(err.is_locked());
        assert!(!err.is_stale());
    }

    #[test]
    fn display_includes_stale_reason() {
        let err = AuthorityError::StaleAuthority {
            lease_id: LeaseId("l1".into()),
            reason: StaleReason::TokenMismatch {
                expected: FencingToken(41),
                actual: FencingToken(42),
            },
        };
        let msg = format!("{}", err);
        assert!(msg.contains("STALE_AUTHORITY"));
        assert!(msg.contains("41"));
        assert!(msg.contains("42"));
    }
}
