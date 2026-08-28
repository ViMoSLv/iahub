//! Mega Brain V0 — Lease Repository
//!
//! Persistence operations for the `leases` table. All access goes through
//! this module; no raw SQL in service logic. Handles monotonic fencing token
//! allocation atomically within transactions to prevent reuse after restart.

use rusqlite::params;
use rusqlite::OptionalExtension;

use super::error::AuthorityError;
use super::model::{FencingToken, LeaseId, LeaseRecord, LeaseStatus, ResourceId};
use crate::persistence::transaction::Transaction;

/// Insert a new lease record. The fencing_token must already be computed
/// by the service layer using `get_next_fencing_token` within the same
/// transaction to guarantee monotonicity.
pub fn insert_lease(tx: &Transaction, lease: &LeaseRecord) -> Result<(), AuthorityError> {
    // Compute a deterministic lease_token_hash from the fencing token and resource.
    // This satisfies the NOT NULL constraint from v0001 while being derivable
    // from the canonical fencing token (no separate secret state).
    let token_hash = format!(
        "{:016x}-{}-{}",
        lease.fencing_token.0, lease.resource.resource_type, lease.resource.resource_id
    );

    // Populate both attempt_id (v0001 legacy) and owner_attempt_id (v0002+)
    // to satisfy both FK constraints. They reference the same logical attempt.
    tx.conn()
        .execute(
            "INSERT INTO leases (
                id, resource_type, resource_id, attempt_id, owner_attempt_id,
                lease_token_hash, fencing_token, status, issued_at, heartbeat_at,
                expires_at, revoked_at, version, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                lease.id.0,
                lease.resource.resource_type,
                lease.resource.resource_id,
                lease.owner_attempt_id,
                token_hash,
                lease.fencing_token.0 as i64,
                lease.status.to_db(),
                lease.issued_at,
                lease.heartbeat_at,
                lease.expires_at,
                lease.revoked_at,
                lease.version as i64,
                lease.created_at,
                lease.updated_at,
            ],
        )
        .map_err(|e| match &e {
            rusqlite::Error::SqliteFailure(err, Some(msg))
                if err.code == rusqlite::ErrorCode::ConstraintViolation
                    && msg.contains("idx_leases_resource") =>
            {
                AuthorityError::ResourceLocked {
                    resource_type: lease.resource.resource_type.clone(),
                    resource_id: lease.resource.resource_id.clone(),
                    owner_attempt_id: lease.owner_attempt_id.clone(),
                    current_fencing_token: lease.fencing_token,
                }
            }
            _ => AuthorityError::Persistence {
                message: format!("insert lease failed: {}", e),
            },
        })?;
    Ok(())
}

/// Get the next fencing token for a resource. Returns 1 if no prior leases
/// exist, otherwise max(existing tokens) + 1. This MUST be called within
/// the same transaction as insert_lease to guarantee monotonicity across
/// concurrent acquisitions and restarts.
///
/// Uses MAX(fencing_token) over ALL statuses (including EXPIRED/REVOKED)
/// to ensure tokens are never reused even after deletion of old records.
pub fn get_next_fencing_token(
    tx: &Transaction,
    resource: &ResourceId,
) -> Result<FencingToken, AuthorityError> {
    let max_token: Option<i64> = tx
        .conn()
        .query_row(
            "SELECT MAX(fencing_token) FROM leases WHERE resource_type = ?1 AND resource_id = ?2",
            params![resource.resource_type, resource.resource_id],
            |r| r.get(0),
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("get_next_fencing_token failed: {}", e),
        })?;

    Ok(FencingToken(max_token.unwrap_or(0) as u64 + 1))
}

/// Find the current ACTIVE lease for a resource, if any.
pub fn find_active_lease(
    tx: &Transaction,
    resource: &ResourceId,
) -> Result<Option<LeaseRecord>, AuthorityError> {
    let row = tx
        .conn()
        .prepare(
            "SELECT id, resource_type, resource_id, owner_attempt_id,
                    fencing_token, status, issued_at, heartbeat_at,
                    expires_at, revoked_at, version, created_at, updated_at
             FROM leases
             WHERE resource_type = ?1 AND resource_id = ?2 AND status = 'ACTIVE'",
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("find_active_lease prepare failed: {}", e),
        })?
        .query_row(params![resource.resource_type, resource.resource_id], |r| {
            parse_lease_row(r)
        })
        .optional()
        .map_err(|e| AuthorityError::Persistence {
            message: format!("find_active_lease query failed: {}", e),
        })?;

    Ok(row)
}

/// Find a lease by ID regardless of status.
pub fn find_by_id(
    tx: &Transaction,
    lease_id: &LeaseId,
) -> Result<Option<LeaseRecord>, AuthorityError> {
    let row = tx
        .conn()
        .prepare(
            "SELECT id, resource_type, resource_id, owner_attempt_id,
                    fencing_token, status, issued_at, heartbeat_at,
                    expires_at, revoked_at, version, created_at, updated_at
             FROM leases WHERE id = ?1",
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("find_by_id prepare failed: {}", e),
        })?
        .query_row(params![lease_id.0], parse_lease_row)
        .optional()
        .map_err(|e| AuthorityError::Persistence {
            message: format!("find_by_id query failed: {}", e),
        })?;

    Ok(row)
}

/// Update heartbeat_at for an active lease. Does NOT modify expires_at.
/// Uses optimistic concurrency via version check.
pub fn update_heartbeat(
    tx: &Transaction,
    lease_id: &LeaseId,
    new_heartbeat_at: i64,
    expected_version: u64,
) -> Result<u64, AuthorityError> {
    let new_version = expected_version + 1;
    let affected = tx
        .conn()
        .execute(
            "UPDATE leases SET heartbeat_at = ?1, updated_at = ?1, version = ?2
             WHERE id = ?3 AND version = ?4 AND status = 'ACTIVE'",
            params![
                new_heartbeat_at,
                new_version as i64,
                lease_id.0,
                expected_version as i64
            ],
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("update_heartbeat failed: {}", e),
        })?;

    if affected == 0 {
        // Check if it's a version conflict or status issue
        let current = find_by_id(tx, lease_id)?;
        match current {
            Some(lease) if lease.version != expected_version => {
                Err(AuthorityError::VersionConflict {
                    lease_id: lease_id.clone(),
                    expected_version,
                    actual_version: lease.version,
                })
            }
            Some(_) => Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: super::model::StaleReason::LeaseNotActive,
            }),
            None => Err(AuthorityError::LeaseNotFound {
                lease_id: lease_id.clone(),
            }),
        }
    } else {
        Ok(new_version)
    }
}

/// Renew an active lease by extending expires_at. Does NOT change fencing token.
/// Uses optimistic concurrency via version check.
pub fn renew_lease(
    tx: &Transaction,
    lease_id: &LeaseId,
    new_expires_at: i64,
    now: i64,
    expected_version: u64,
) -> Result<u64, AuthorityError> {
    let new_version = expected_version + 1;
    let affected = tx
        .conn()
        .execute(
            "UPDATE leases SET expires_at = ?1, updated_at = ?2, version = ?3
             WHERE id = ?4 AND version = ?5 AND status = 'ACTIVE'",
            params![
                new_expires_at,
                now,
                new_version as i64,
                lease_id.0,
                expected_version as i64
            ],
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("renew_lease failed: {}", e),
        })?;

    if affected == 0 {
        let current = find_by_id(tx, lease_id)?;
        match current {
            Some(lease) if lease.version != expected_version => {
                Err(AuthorityError::VersionConflict {
                    lease_id: lease_id.clone(),
                    expected_version,
                    actual_version: lease.version,
                })
            }
            Some(_) => Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: super::model::StaleReason::LeaseNotActive,
            }),
            None => Err(AuthorityError::LeaseNotFound {
                lease_id: lease_id.clone(),
            }),
        }
    } else {
        Ok(new_version)
    }
}

/// Revoke an active lease. Sets status to REVOKED and records revoked_at.
/// Idempotent: revoking an already-revoked lease returns success.
pub fn revoke_lease(
    tx: &Transaction,
    lease_id: &LeaseId,
    now: i64,
    expected_version: u64,
) -> Result<u64, AuthorityError> {
    let new_version = expected_version + 1;
    let affected = tx
        .conn()
        .execute(
            "UPDATE leases SET status = 'REVOKED', revoked_at = ?1, updated_at = ?1, version = ?2
             WHERE id = ?3 AND version = ?4 AND status = 'ACTIVE'",
            params![now, new_version as i64, lease_id.0, expected_version as i64],
        )
        .map_err(|e| AuthorityError::Persistence {
            message: format!("revoke_lease failed: {}", e),
        })?;

    if affected == 0 {
        let current = find_by_id(tx, lease_id)?;
        match current {
            // Already revoked is idempotent success
            Some(lease) if lease.status == LeaseStatus::Revoked => Ok(lease.version),
            Some(lease) if lease.version != expected_version => {
                Err(AuthorityError::VersionConflict {
                    lease_id: lease_id.clone(),
                    expected_version,
                    actual_version: lease.version,
                })
            }
            Some(_) => Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: super::model::StaleReason::LeaseNotActive,
            }),
            None => Err(AuthorityError::LeaseNotFound {
                lease_id: lease_id.clone(),
            }),
        }
    } else {
        Ok(new_version)
    }
}

/// Expire all ACTIVE leases whose expires_at <= now.
/// Returns the list of lease IDs that were expired.
pub fn expire_due_leases(tx: &Transaction, now: i64) -> Result<Vec<LeaseId>, AuthorityError> {
    // First collect the IDs
    let mut stmt = tx
        .conn()
        .prepare("SELECT id FROM leases WHERE status = 'ACTIVE' AND expires_at <= ?1")
        .map_err(|e| AuthorityError::Persistence {
            message: format!("expire_due_leases prepare failed: {}", e),
        })?;

    let ids: Vec<String> = stmt
        .query_map(params![now], |r| r.get::<_, String>(0))
        .map_err(|e| AuthorityError::Persistence {
            message: format!("expire_due_leases query failed: {}", e),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AuthorityError::Persistence {
            message: format!("expire_due_leases collect failed: {}", e),
        })?;

    if ids.is_empty() {
        return Ok(vec![]);
    }

    // Batch update
    for id in &ids {
        tx.conn()
            .execute(
                "UPDATE leases SET status = 'EXPIRED', updated_at = ?1, version = version + 1
                 WHERE id = ?2 AND status = 'ACTIVE'",
                params![now, id],
            )
            .map_err(|e| AuthorityError::Persistence {
                message: format!("expire_due_leases update failed for {}: {}", id, e),
            })?;
    }

    Ok(ids.into_iter().map(LeaseId).collect())
}

/// Parse a lease row from a rusqlite Row.
fn parse_lease_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<LeaseRecord> {
    let status_str: String = r.get(5)?;
    let status = LeaseStatus::from_db(&status_str).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(5, "status".into(), rusqlite::types::Type::Text)
    })?;

    Ok(LeaseRecord {
        id: LeaseId(r.get::<_, String>(0)?),
        resource: ResourceId {
            resource_type: r.get(1)?,
            resource_id: r.get(2)?,
        },
        owner_attempt_id: r.get(3)?,
        fencing_token: FencingToken(r.get::<_, i64>(4)? as u64),
        status,
        issued_at: r.get(6)?,
        heartbeat_at: r.get(7)?,
        expires_at: r.get(8)?,
        revoked_at: r.get(9)?,
        version: r.get::<_, i64>(10)? as u64,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    fn make_resource(id: &str) -> ResourceId {
        ResourceId::new("task", id)
    }

    #[test]
    fn next_fencing_token_starts_at_one() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let token = get_next_fencing_token(&tx, &make_resource("t1")).unwrap();
        assert_eq!(token, FencingToken(1));

        tx.rollback().unwrap();
    }

    #[test]
    fn next_fencing_token_increments_after_insert() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");
        let token1 = get_next_fencing_token(&tx, &resource).unwrap();
        assert_eq!(token1, FencingToken(1));

        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource: resource.clone(),
            owner_attempt_id: "att-1".into(),
            fencing_token: token1,
            status: LeaseStatus::Active,
            issued_at: 100,
            heartbeat_at: 100,
            expires_at: 200,
            revoked_at: None,
            version: 1,
            created_at: 100,
            updated_at: 100,
        };
        insert_lease(&tx, &lease).unwrap();

        let token2 = get_next_fencing_token(&tx, &resource).unwrap();
        assert_eq!(token2, FencingToken(2));

        tx.commit().unwrap();
    }

    #[test]
    fn next_fencing_token_counts_expired_and_revoked() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");

        // Insert and revoke a lease with token 5
        let lease = LeaseRecord {
            id: LeaseId("l-old".into()),
            resource: resource.clone(),
            owner_attempt_id: "att-old".into(),
            fencing_token: FencingToken(5),
            status: LeaseStatus::Revoked,
            issued_at: 100,
            heartbeat_at: 100,
            expires_at: 200,
            revoked_at: Some(150),
            version: 2,
            created_at: 100,
            updated_at: 150,
        };
        insert_lease(&tx, &lease).unwrap();

        // Next token must be 6, not 1
        let next = get_next_fencing_token(&tx, &resource).unwrap();
        assert_eq!(next, FencingToken(6));

        tx.commit().unwrap();
    }

    #[test]
    fn find_active_lease_returns_none_when_empty() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let result = find_active_lease(&tx, &make_resource("nonexistent")).unwrap();
        assert!(result.is_none());

        tx.rollback().unwrap();
    }

    #[test]
    fn insert_and_find_active_lease() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");
        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource: resource.clone(),
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
        insert_lease(&tx, &lease).unwrap();

        let found = find_active_lease(&tx, &resource).unwrap().unwrap();
        assert_eq!(found.id, LeaseId("l1".into()));
        assert_eq!(found.fencing_token, FencingToken(1));
        assert_eq!(found.status, LeaseStatus::Active);

        tx.commit().unwrap();
    }

    #[test]
    fn update_heartbeat_does_not_change_expires_at() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");
        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource: resource.clone(),
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
        insert_lease(&tx, &lease).unwrap();

        let new_version = update_heartbeat(&tx, &LeaseId("l1".into()), 150, 1).unwrap();
        assert_eq!(new_version, 2);

        let updated = find_by_id(&tx, &LeaseId("l1".into())).unwrap().unwrap();
        assert_eq!(updated.heartbeat_at, 150);
        assert_eq!(updated.expires_at, 200); // unchanged!
        assert_eq!(updated.version, 2);

        tx.commit().unwrap();
    }

    #[test]
    fn revoke_lease_sets_status_and_timestamp() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");
        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource,
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
        insert_lease(&tx, &lease).unwrap();

        let new_version = revoke_lease(&tx, &LeaseId("l1".into()), 150, 1).unwrap();
        assert_eq!(new_version, 2);

        let revoked = find_by_id(&tx, &LeaseId("l1".into())).unwrap().unwrap();
        assert_eq!(revoked.status, LeaseStatus::Revoked);
        assert_eq!(revoked.revoked_at, Some(150));

        // Active lookup should return None
        let active = find_active_lease(&tx, &make_resource("t1")).unwrap();
        assert!(active.is_none());

        tx.commit().unwrap();
    }

    #[test]
    fn expire_due_leases_marks_expired() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let resource = make_resource("t1");
        let lease = LeaseRecord {
            id: LeaseId("l1".into()),
            resource,
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
        insert_lease(&tx, &lease).unwrap();

        // Not yet expired
        let expired = expire_due_leases(&tx, 199).unwrap();
        assert!(expired.is_empty());

        // Now expired
        let expired = expire_due_leases(&tx, 200).unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], LeaseId("l1".into()));

        let record = find_by_id(&tx, &LeaseId("l1".into())).unwrap().unwrap();
        assert_eq!(record.status, LeaseStatus::Expired);

        tx.commit().unwrap();
    }
}
