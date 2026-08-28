//! Mega Brain V0 — Lease Domain Model (ADR-0004)
//!
//! Core types for durable lease management with monotonic fencing tokens.
//! All status transitions are validated at the type level; unknown statuses
//! fail closed during deserialization.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a lease instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeaseId(pub String);

impl fmt::Display for LeaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for LeaseId {
    fn from(s: String) -> Self {
        LeaseId(s)
    }
}

/// Composite resource identifier: (resource_type, resource_id).
/// This is the granularity at which fencing tokens are monotonically allocated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId {
    pub resource_type: String,
    pub resource_id: String,
}

impl ResourceId {
    pub fn new(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
        }
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.resource_type, self.resource_id)
    }
}

/// Monotonic fencing token. Source of truth is SQLite; never generated from
/// timestamps, UUIDs, or in-memory counters. Each new lease for a given
/// ResourceId receives a token strictly greater than all previous tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FencingToken(pub u64);

impl FencingToken {
    pub fn next(self) -> Self {
        FencingToken(self.0 + 1)
    }
}

impl fmt::Display for FencingToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lease lifecycle states per ADR-0004.
/// Unknown variants fail closed on deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeaseStatus {
    Active,
    Expired,
    Revoked,
}

impl fmt::Display for LeaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeaseStatus::Active => write!(f, "ACTIVE"),
            LeaseStatus::Expired => write!(f, "EXPIRED"),
            LeaseStatus::Revoked => write!(f, "REVOKED"),
        }
    }
}

impl LeaseStatus {
    /// Parse from DB string. Unknown values return None (fail closed).
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "ACTIVE" => Some(LeaseStatus::Active),
            "EXPIRED" => Some(LeaseStatus::Expired),
            "REVOKED" => Some(LeaseStatus::Revoked),
            _ => None,
        }
    }

    pub fn to_db(self) -> &'static str {
        match self {
            LeaseStatus::Active => "ACTIVE",
            LeaseStatus::Expired => "EXPIRED",
            LeaseStatus::Revoked => "REVOKED",
        }
    }
}

/// Complete persisted lease record. Maps directly to the `leases` table schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecord {
    pub id: LeaseId,
    pub resource: ResourceId,
    pub owner_attempt_id: String,
    pub fencing_token: FencingToken,
    pub status: LeaseStatus,
    pub issued_at: i64,
    pub heartbeat_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
    pub version: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl LeaseRecord {
    /// Check if this lease is currently valid for authority operations.
    pub fn is_active(&self) -> bool {
        self.status == LeaseStatus::Active
    }

    /// Check if this lease has expired based on the given timestamp.
    /// Does NOT mutate state; caller must use expire_due() for state transition.
    pub fn is_expired_at(&self, now: i64) -> bool {
        self.status == LeaseStatus::Active && now >= self.expires_at
    }
}

/// Parameters for acquiring a new lease.
#[derive(Debug, Clone)]
pub struct AcquireRequest {
    pub resource: ResourceId,
    pub owner_attempt_id: String,
    pub ttl_seconds: u64,
}

/// Parameters for renewing an existing lease.
#[derive(Debug, Clone)]
pub struct RenewRequest {
    pub lease_id: LeaseId,
    pub expected_fencing_token: FencingToken,
    pub additional_ttl_seconds: u64,
}

/// Result of a successful acquire operation.
#[derive(Debug, Clone)]
pub struct AcquireResult {
    pub lease: LeaseRecord,
}

/// Result of authority validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityValidation {
    /// Authority is valid and current.
    Valid,
    /// Authority is stale: lease expired, revoked, or superseded by higher token.
    Stale { reason: StaleReason },
}

/// Specific reason why authority is stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    LeaseNotFound,
    LeaseNotActive,
    LeaseRevoked,
    LeaseExpired,
    TokenMismatch {
        expected: FencingToken,
        actual: FencingToken,
    },
    SupersededByHigherToken {
        current: FencingToken,
    },
    WrongOwner {
        expected_attempt: String,
        actual_attempt: String,
    },
}

impl fmt::Display for StaleReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StaleReason::LeaseNotFound => write!(f, "lease not found"),
            StaleReason::LeaseNotActive => write!(f, "lease is not ACTIVE"),
            StaleReason::LeaseRevoked => write!(f, "lease has been revoked"),
            StaleReason::LeaseExpired => write!(f, "lease has expired"),
            StaleReason::TokenMismatch { expected, actual } => {
                write!(
                    f,
                    "fencing token mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            StaleReason::SupersededByHigherToken { current } => {
                write!(f, "superseded by higher fencing token {}", current)
            }
            StaleReason::WrongOwner {
                expected_attempt,
                actual_attempt,
            } => {
                write!(
                    f,
                    "wrong owner: expected attempt {}, got {}",
                    expected_attempt, actual_attempt
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fencing_token_next_is_monotonic() {
        let t = FencingToken(1);
        assert_eq!(t.next(), FencingToken(2));
        assert_eq!(t.next().next(), FencingToken(3));
    }

    #[test]
    fn fencing_token_ordering() {
        assert!(FencingToken(2) > FencingToken(1));
        assert!(FencingToken(1) < FencingToken(42));
    }

    #[test]
    fn lease_status_roundtrip() {
        for status in [
            LeaseStatus::Active,
            LeaseStatus::Expired,
            LeaseStatus::Revoked,
        ] {
            let db = status.to_db();
            let parsed = LeaseStatus::from_db(db).expect("valid status must parse");
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn lease_status_unknown_fails_closed() {
        assert!(LeaseStatus::from_db("UNKNOWN").is_none());
        assert!(LeaseStatus::from_db("").is_none());
        assert!(LeaseStatus::from_db("active").is_none()); // case sensitive
    }

    #[test]
    fn lease_record_is_expired_at_boundary() {
        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource: ResourceId::new("task", "t1"),
            owner_attempt_id: "att-1".into(),
            fencing_token: FencingToken(1),
            status: LeaseStatus::Active,
            issued_at: 100,
            heartbeat_at: 100,
            expires_at: 200,
            revoked_at: None,
            version: 1,
            created_at: 100,
            updated_at: 100,
        };

        assert!(!lease.is_expired_at(199));
        assert!(lease.is_expired_at(200));
        assert!(lease.is_expired_at(201));
    }

    #[test]
    fn non_active_lease_never_reports_expired_at() {
        let mut lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource: ResourceId::new("task", "t1"),
            owner_attempt_id: "att-1".into(),
            fencing_token: FencingToken(1),
            status: LeaseStatus::Revoked,
            issued_at: 100,
            heartbeat_at: 100,
            expires_at: 200,
            revoked_at: Some(150),
            version: 2,
            created_at: 100,
            updated_at: 150,
        };

        // Even though now > expires_at, revoked lease doesn't report "expired"
        // because it's already in a terminal state.
        assert!(!lease.is_expired_at(300));

        lease.status = LeaseStatus::Expired;
        assert!(!lease.is_expired_at(300)); // already expired, not "newly" expired
    }
}
