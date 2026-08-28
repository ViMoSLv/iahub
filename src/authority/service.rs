//! Mega Brain V0 — Lease Service
//!
//! High-level authority operations built on the repository layer.
//! All lease mutations go through this service; no handler may bypass it.
//!
//! Key invariants enforced here:
//! - Fencing tokens are monotonically increasing per resource (INV-024)
//! - Only one ACTIVE lease per resource at any time (INV-025)
//! - Heartbeat never extends expires_at (ADR-0004)
//! - Stale authority is rejected before any handler executes
//! - Restart durability: tokens survive DB close/reopen

use super::error::AuthorityError;
use super::model::{
    AcquireRequest, AcquireResult, LeaseRecord, LeaseStatus, ResourceId, StaleReason,
    ValidatedAuthority,
};
use super::repository;
use crate::domain::{FencingToken, LeaseId};
use crate::persistence::transaction::Transaction;

/// LeaseService provides all authority operations. It is stateless;
/// all state lives in SQLite via the provided Transaction.
pub struct LeaseService;

impl LeaseService {
    /// Acquire a new lease for a resource. Atomically allocates the next
    /// monotonic fencing token within the same transaction as the insert.
    ///
    /// Fails with ResourceLocked if another ACTIVE lease exists for this resource.
    /// The fencing token is guaranteed to be > all previous tokens for this resource,
    /// including expired and revoked leases.
    pub fn acquire(
        tx: &Transaction,
        request: &AcquireRequest,
        now: i64,
    ) -> Result<AcquireResult, AuthorityError> {
        // Validate request
        if request.ttl_seconds == 0 {
            return Err(AuthorityError::InvalidRequest {
                message: "ttl_seconds must be > 0".into(),
            });
        }
        if request.resource.resource_type.is_empty() || request.resource.resource_id.is_empty() {
            return Err(AuthorityError::InvalidRequest {
                message: "resource_type and resource_id must be non-empty".into(),
            });
        }
        if request.owner_attempt_id.is_empty() {
            return Err(AuthorityError::InvalidRequest {
                message: "owner_attempt_id must be non-empty".into(),
            });
        }

        // Check for existing active lease (INV-025: one active attempt per resource)
        if let Some(existing) = repository::find_active_lease(tx, &request.resource)? {
            return Err(AuthorityError::ResourceLocked {
                resource_type: request.resource.resource_type.clone(),
                resource_id: request.resource.resource_id.clone(),
                owner_attempt_id: existing.owner_attempt_id,
                current_fencing_token: existing.fencing_token,
            });
        }

        // Allocate next fencing token atomically within this transaction.
        // This counts ALL prior leases (active, expired, revoked) to ensure
        // monotonicity survives restarts and deletions.
        let fencing_token = repository::get_next_fencing_token(tx, &request.resource)?;

        let lease = LeaseRecord {
            id: LeaseId(format!("LEASE-{}", uuid::Uuid::new_v4())),
            resource: request.resource.clone(),
            owner_attempt_id: request.owner_attempt_id.clone(),
            fencing_token,
            status: LeaseStatus::Active,
            issued_at: now,
            heartbeat_at: now,
            expires_at: now + (request.ttl_seconds as i64),
            revoked_at: None,
            version: 1,
            created_at: now,
            updated_at: now,
        };

        repository::insert_lease(tx, &lease)?;

        Ok(AcquireResult { lease })
    }

    /// Validate that a presented authority (lease_id + fencing_token) is current
    /// and valid for the given resource and attempt.
    ///
    /// Returns `Ok(ValidatedAuthority)` on success, or `Err(AuthorityError::StaleAuthority)`
    /// if ANY check fails. There is no success-path stale result — callers cannot
    /// accidentally ignore a stale authority. This MUST be called before executing
    /// any protected command.
    pub fn validate_authority(
        tx: &Transaction,
        lease_id: &LeaseId,
        presented_token: FencingToken,
        expected_resource: &ResourceId,
        expected_attempt: &str,
        now: i64,
    ) -> Result<ValidatedAuthority, AuthorityError> {
        let lease = match repository::find_by_id(tx, lease_id)? {
            Some(l) => l,
            None => {
                return Err(AuthorityError::StaleAuthority {
                    lease_id: lease_id.clone(),
                    reason: StaleReason::LeaseNotFound,
                });
            }
        };

        // Check resource match
        if lease.resource != *expected_resource {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::WrongOwner {
                    expected_attempt: expected_attempt.into(),
                    actual_attempt: lease.owner_attempt_id,
                },
            });
        }

        // Check owner match
        if lease.owner_attempt_id != expected_attempt {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::WrongOwner {
                    expected_attempt: expected_attempt.into(),
                    actual_attempt: lease.owner_attempt_id,
                },
            });
        }

        // Check status
        match lease.status {
            LeaseStatus::Revoked => {
                return Err(AuthorityError::StaleAuthority {
                    lease_id: lease_id.clone(),
                    reason: StaleReason::LeaseRevoked,
                });
            }
            LeaseStatus::Expired => {
                return Err(AuthorityError::StaleAuthority {
                    lease_id: lease_id.clone(),
                    reason: StaleReason::LeaseExpired,
                });
            }
            LeaseStatus::Active => {}
        }

        // Check runtime expiry (lease may still be ACTIVE in DB but past expires_at)
        if now >= lease.expires_at {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::LeaseExpired,
            });
        }

        // Check fencing token: must match exactly AND not be superseded
        if presented_token != lease.fencing_token {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::TokenMismatch {
                    expected: presented_token,
                    actual: lease.fencing_token,
                },
            });
        }

        // Verify no higher token exists for this resource (supersession check)
        let current_max = repository::get_next_fencing_token(tx, &lease.resource)?;
        // get_next_fencing_token returns max+1, so current max is current_max - 1
        let current_max_token = FencingToken(current_max.0.saturating_sub(1));
        if current_max_token > lease.fencing_token {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::SupersededByHigherToken {
                    current: current_max_token,
                },
            });
        }

        Ok(ValidatedAuthority { lease })
    }

    /// Update heartbeat timestamp. Does NOT extend expires_at per ADR-0004.
    /// Requires valid active lease with correct version.
    pub fn heartbeat(
        tx: &Transaction,
        lease_id: &LeaseId,
        now: i64,
        expected_version: u64,
    ) -> Result<u64, AuthorityError> {
        repository::update_heartbeat(tx, lease_id, now, expected_version)
    }

    /// Explicitly renew a lease by extending expires_at. Separate from heartbeat
    /// per ADR-0004. Does NOT change fencing token. Cannot renew expired/revoked leases.
    pub fn renew(
        tx: &Transaction,
        lease_id: &LeaseId,
        additional_ttl_seconds: u64,
        now: i64,
        expected_version: u64,
    ) -> Result<u64, AuthorityError> {
        if additional_ttl_seconds == 0 {
            return Err(AuthorityError::InvalidRequest {
                message: "additional_ttl_seconds must be > 0".into(),
            });
        }

        // Read current lease to compute new expires_at
        let lease =
            repository::find_by_id(tx, lease_id)?.ok_or_else(|| AuthorityError::LeaseNotFound {
                lease_id: lease_id.clone(),
            })?;

        if lease.status != LeaseStatus::Active {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::LeaseNotActive,
            });
        }

        // Cannot renew an already-expired lease (runtime check)
        if now >= lease.expires_at {
            return Err(AuthorityError::StaleAuthority {
                lease_id: lease_id.clone(),
                reason: StaleReason::LeaseExpired,
            });
        }

        let new_expires_at = lease.expires_at + (additional_ttl_seconds as i64);
        repository::renew_lease(tx, lease_id, new_expires_at, now, expected_version)
    }

    /// Revoke an active lease. After revocation, the fencing token is permanently
    /// invalid. New acquisitions for this resource will receive a higher token.
    /// Idempotent: revoking an already-revoked lease succeeds.
    pub fn revoke(
        tx: &Transaction,
        lease_id: &LeaseId,
        now: i64,
        expected_version: u64,
    ) -> Result<u64, AuthorityError> {
        repository::revoke_lease(tx, lease_id, now, expected_version)
    }

    /// Expire all ACTIVE leases whose expires_at <= now.
    /// Returns list of expired lease IDs. Callers should trigger reconcile
    /// for affected attempts after calling this.
    pub fn expire_due(tx: &Transaction, now: i64) -> Result<Vec<LeaseId>, AuthorityError> {
        repository::expire_due_leases(tx, now)
    }

    /// Get the current active lease for a resource, if any.
    pub fn get_current_authority(
        tx: &Transaction,
        resource: &ResourceId,
    ) -> Result<Option<LeaseRecord>, AuthorityError> {
        repository::find_active_lease(tx, resource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    fn make_resource(id: &str) -> ResourceId {
        ResourceId::new("task", id)
    }

    /// Insert prerequisite task and attempt rows so that lease FK constraints
    /// are satisfied. v0004 restored the owner_attempt_id → task_attempts FK.
    fn ensure_attempt_exists(store: &mut SqliteStore, attempt_id: &str, task_id: &str) {
        let tx = store.transaction().unwrap();
        // Insert run (required by tasks FK)
        let _ = tx.conn().execute(
            "INSERT OR IGNORE INTO runs (id, project_id, objective, status, version, created_at, updated_at)
             VALUES ('run-test', 'proj-test', 'test', 'DRAFT', 1, 0, 0)",
            [],
        );
        // Insert task (required by task_attempts FK)
        let _ = tx.conn().execute(
            "INSERT OR IGNORE INTO tasks (id, run_id, title, objective, status, priority, version, created_at, updated_at)
             VALUES (?1, 'run-test', 'test task', 'test', 'CREATED', 0, 1, 0, 0)",
            [task_id],
        );
        // Insert attempt
        let _ = tx.conn().execute(
            "INSERT OR IGNORE INTO task_attempts (id, task_id, attempt_number, status, version, created_at, updated_at)
             VALUES (?1, ?2, 1, 'CREATED', 1, 0, 0)",
            [attempt_id, task_id],
        );
        tx.commit().unwrap();
    }

    fn acquire_lease(
        store: &mut SqliteStore,
        resource: &ResourceId,
        attempt: &str,
        ttl: u64,
        now: i64,
    ) -> LeaseRecord {
        // Ensure the attempt exists for FK integrity (v0004 restored FK)
        ensure_attempt_exists(store, attempt, &resource.resource_id);

        let tx = store.transaction().unwrap();
        let result = LeaseService::acquire(
            &tx,
            &AcquireRequest {
                resource: resource.clone(),
                owner_attempt_id: attempt.into(),
                ttl_seconds: ttl,
            },
            now,
        )
        .unwrap();
        tx.commit().unwrap();
        result.lease
    }

    #[test]
    fn acquire_assigns_monotonic_fencing_token() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");

        let l1 = acquire_lease(&mut store, &resource, "att-1", 300, 1000);
        assert_eq!(l1.fencing_token, FencingToken(1));

        // Revoke first lease
        let tx = store.transaction().unwrap();
        LeaseService::revoke(&tx, &l1.id, 1500, l1.version).unwrap();
        tx.commit().unwrap();

        // Second acquisition gets higher token
        let l2 = acquire_lease(&mut store, &resource, "att-2", 300, 2000);
        assert_eq!(l2.fencing_token, FencingToken(2));
        assert!(l2.fencing_token > l1.fencing_token);
    }

    #[test]
    fn acquire_rejects_when_resource_locked() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");

        let _l1 = acquire_lease(&mut store, &resource, "att-1", 300, 1000);

        // Second attempt on same resource while first is active
        let tx = store.transaction().unwrap();
        let err = LeaseService::acquire(
            &tx,
            &AcquireRequest {
                resource: resource.clone(),
                owner_attempt_id: "att-2".into(),
                ttl_seconds: 300,
            },
            1000,
        )
        .unwrap_err();

        assert!(err.is_locked());
        tx.rollback().unwrap();
    }

    #[test]
    fn validate_authority_accepts_valid_lease() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 300, 1000);

        let tx = store.transaction().unwrap();
        let validated = LeaseService::validate_authority(
            &tx,
            &lease.id,
            lease.fencing_token,
            &resource,
            "att-1",
            1100, // within TTL
        )
        .unwrap();

        assert_eq!(validated.lease.id, lease.id);
        assert_eq!(validated.lease.fencing_token, lease.fencing_token);
        tx.rollback().unwrap();
    }

    #[test]
    fn validate_authority_rejects_stale_token() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 300, 1000);

        let tx = store.transaction().unwrap();
        let err = LeaseService::validate_authority(
            &tx,
            &lease.id,
            FencingToken(999), // wrong token
            &resource,
            "att-1",
            1100,
        )
        .unwrap_err();

        match err {
            AuthorityError::StaleAuthority {
                reason: StaleReason::TokenMismatch { expected, actual },
                ..
            } => {
                assert_eq!(expected, FencingToken(999));
                assert_eq!(actual, FencingToken(1));
            }
            other => panic!("expected TokenMismatch, got {:?}", other),
        }
        tx.rollback().unwrap();
    }

    #[test]
    fn validate_authority_rejects_expired_lease() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 100, 1000);

        let tx = store.transaction().unwrap();
        let err = LeaseService::validate_authority(
            &tx,
            &lease.id,
            lease.fencing_token,
            &resource,
            "att-1",
            1200, // past expires_at (1000 + 100 = 1100)
        )
        .unwrap_err();

        assert!(matches!(
            err,
            AuthorityError::StaleAuthority {
                reason: StaleReason::LeaseExpired,
                ..
            }
        ));
        tx.rollback().unwrap();
    }

    #[test]
    fn validate_authority_rejects_revoked_lease() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 300, 1000);

        // Revoke
        let tx = store.transaction().unwrap();
        LeaseService::revoke(&tx, &lease.id, 1050, lease.version).unwrap();
        tx.commit().unwrap();

        // Validate after revoke
        let tx = store.transaction().unwrap();
        let err = LeaseService::validate_authority(
            &tx,
            &lease.id,
            lease.fencing_token,
            &resource,
            "att-1",
            1060,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            AuthorityError::StaleAuthority {
                reason: StaleReason::LeaseRevoked,
                ..
            }
        ));
        tx.rollback().unwrap();
    }

    #[test]
    fn heartbeat_does_not_extend_expires_at() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 100, 1000);
        assert_eq!(lease.expires_at, 1100);

        let tx = store.transaction().unwrap();
        let new_version = LeaseService::heartbeat(&tx, &lease.id, 1050, lease.version).unwrap();
        tx.commit().unwrap();

        // Verify expires_at unchanged
        let tx = store.transaction().unwrap();
        let updated = repository::find_by_id(&tx, &lease.id).unwrap().unwrap();
        assert_eq!(updated.expires_at, 1100); // unchanged!
        assert_eq!(updated.heartbeat_at, 1050);
        assert_eq!(updated.version, new_version);
        tx.rollback().unwrap();
    }

    #[test]
    fn renew_extends_expires_at_without_changing_token() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 100, 1000);
        assert_eq!(lease.expires_at, 1100);

        let tx = store.transaction().unwrap();
        let new_version = LeaseService::renew(&tx, &lease.id, 200, 1050, lease.version).unwrap();
        tx.commit().unwrap();

        let tx = store.transaction().unwrap();
        let renewed = repository::find_by_id(&tx, &lease.id).unwrap().unwrap();
        assert_eq!(renewed.expires_at, 1300); // 1100 + 200
        assert_eq!(renewed.fencing_token, lease.fencing_token); // unchanged!
        assert_eq!(renewed.version, new_version);
        tx.rollback().unwrap();
    }

    #[test]
    fn renew_rejects_expired_lease() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let lease = acquire_lease(&mut store, &resource, "att-1", 100, 1000);

        let tx = store.transaction().unwrap();
        let err = LeaseService::renew(&tx, &lease.id, 200, 1200, lease.version).unwrap_err();
        assert!(err.is_stale());
        tx.rollback().unwrap();
    }

    #[test]
    fn expire_due_marks_expired_and_allows_reacquisition() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");
        let l1 = acquire_lease(&mut store, &resource, "att-1", 100, 1000);

        // Expire
        let tx = store.transaction().unwrap();
        let expired = LeaseService::expire_due(&tx, 1100).unwrap();
        assert_eq!(expired.len(), 1);
        tx.commit().unwrap();

        // Reacquire should succeed with higher token
        let l2 = acquire_lease(&mut store, &resource, "att-2", 300, 1200);
        assert_eq!(l2.fencing_token, FencingToken(2));
        assert!(l2.fencing_token > l1.fencing_token);

        // Old token must be stale
        let tx = store.transaction().unwrap();
        let err = LeaseService::validate_authority(
            &tx,
            &l1.id,
            l1.fencing_token,
            &resource,
            "att-1",
            1200,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AuthorityError::StaleAuthority {
                reason: StaleReason::LeaseExpired,
                ..
            }
        ));
        tx.rollback().unwrap();
    }

    #[test]
    fn stale_authority_after_revoke_and_reacquire() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("t1");

        // ATT-1 gets token 1
        let l1 = acquire_lease(&mut store, &resource, "att-1", 300, 1000);
        assert_eq!(l1.fencing_token, FencingToken(1));

        // Revoke ATT-1
        let tx = store.transaction().unwrap();
        LeaseService::revoke(&tx, &l1.id, 1050, l1.version).unwrap();
        tx.commit().unwrap();

        // ATT-2 gets token 2
        let l2 = acquire_lease(&mut store, &resource, "att-2", 300, 1100);
        assert_eq!(l2.fencing_token, FencingToken(2));

        // ATT-1 tries to use old token → STALE_AUTHORITY
        let tx = store.transaction().unwrap();
        let err = LeaseService::validate_authority(
            &tx,
            &l1.id,
            FencingToken(1),
            &resource,
            "att-1",
            1100,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AuthorityError::StaleAuthority {
                reason: StaleReason::LeaseRevoked,
                ..
            }
        ));

        // Even if ATT-1 somehow presents token 1 against l2's lease, it fails
        let err = LeaseService::validate_authority(
            &tx,
            &l2.id,
            FencingToken(1),
            &resource,
            "att-2",
            1100,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AuthorityError::StaleAuthority {
                reason: StaleReason::TokenMismatch { .. },
                ..
            }
        ));
        tx.rollback().unwrap();
    }

    #[test]
    fn fencing_token_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("fencing_restart.db");
        let resource = make_resource("t1");

        // Session 1: acquire lease with token 1
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let _l1 = acquire_lease(&mut store, &resource, "att-1", 300, 1000);
            // Store dropped, connection closed
        }

        // Session 2: reopen DB, revoke/expiry simulation, reacquire
        {
            let mut store = SqliteStore::open(&db_path).unwrap();

            // Expire the old lease
            let tx = store.transaction().unwrap();
            let expired = LeaseService::expire_due(&tx, 2000).unwrap();
            assert_eq!(expired.len(), 1);
            tx.commit().unwrap();

            // Reacquire must get token 2, not 1
            let l2 = acquire_lease(&mut store, &resource, "att-2", 300, 2000);
            assert_eq!(l2.fencing_token, FencingToken(2));

            // Old token 1 must be stale
            let tx = store.transaction().unwrap();
            let err = LeaseService::validate_authority(
                &tx,
                &expired[0],
                FencingToken(1),
                &resource,
                "att-1",
                2000,
            )
            .unwrap_err();
            assert!(matches!(
                err,
                AuthorityError::StaleAuthority {
                    reason: StaleReason::LeaseExpired,
                    ..
                }
            ));
            tx.rollback().unwrap();
        }
    }

    #[test]
    fn concurrent_acquire_only_one_wins() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("concurrent_acquire.db");

        // Initialize DB and insert prerequisite attempt rows for FK integrity
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            ensure_attempt_exists(&mut store, "att-1", "t1");
            ensure_attempt_exists(&mut store, "att-2", "t1");
        }

        let resource = make_resource("t1");
        let barrier = Arc::new(Barrier::new(2));
        let path_clone = db_path.clone();
        let resource_clone = resource.clone();

        let b1 = barrier.clone();
        let handle1 = thread::spawn(move || {
            let mut store = SqliteStore::open(&path_clone).unwrap();
            b1.wait();
            let tx = store.transaction().unwrap();
            let result = LeaseService::acquire(
                &tx,
                &AcquireRequest {
                    resource: resource_clone.clone(),
                    owner_attempt_id: "att-1".into(),
                    ttl_seconds: 300,
                },
                1000,
            );
            match result {
                Ok(r) => {
                    tx.commit().unwrap();
                    Some(r.lease.fencing_token)
                }
                Err(_) => {
                    tx.rollback().unwrap();
                    None
                }
            }
        });

        let b2 = barrier.clone();
        let path_clone2 = db_path.clone();
        let resource_clone2 = resource.clone();
        let handle2 = thread::spawn(move || {
            let mut store = SqliteStore::open(&path_clone2).unwrap();
            b2.wait();
            let tx = store.transaction().unwrap();
            let result = LeaseService::acquire(
                &tx,
                &AcquireRequest {
                    resource: resource_clone2.clone(),
                    owner_attempt_id: "att-2".into(),
                    ttl_seconds: 300,
                },
                1000,
            );
            match result {
                Ok(r) => {
                    tx.commit().unwrap();
                    Some(r.lease.fencing_token)
                }
                Err(_) => {
                    tx.rollback().unwrap();
                    None
                }
            }
        });

        let r1 = handle1.join().unwrap();
        let r2 = handle2.join().unwrap();

        // Exactly one must succeed
        let results = [r1, r2];
        let winners: Vec<_> = results.iter().filter(|r| r.is_some()).collect();
        assert_eq!(
            winners.len(),
            1,
            "exactly one concurrent acquire must succeed"
        );
    }

    #[test]
    fn inv_025_one_active_attempt_per_task() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let resource = make_resource("TASK-142");

        // First attempt acquires successfully
        let l1 = acquire_lease(&mut store, &resource, "ATT-3", 300, 1000);
        assert_eq!(l1.fencing_token, FencingToken(1));

        // Second attempt on SAME task must fail
        let tx = store.transaction().unwrap();
        let err = LeaseService::acquire(
            &tx,
            &AcquireRequest {
                resource: resource.clone(),
                owner_attempt_id: "ATT-4".into(),
                ttl_seconds: 300,
            },
            1000,
        )
        .unwrap_err();

        assert!(
            err.is_locked(),
            "INV-025: two active attempts on same task must be rejected"
        );
        tx.rollback().unwrap();

        // After revoke, second attempt can acquire with higher token
        let tx = store.transaction().unwrap();
        LeaseService::revoke(&tx, &l1.id, 1050, l1.version).unwrap();
        tx.commit().unwrap();

        let l2 = acquire_lease(&mut store, &resource, "ATT-4", 300, 1100);
        assert_eq!(l2.fencing_token, FencingToken(2));
        assert_eq!(l2.owner_attempt_id, "ATT-4");
    }
}
