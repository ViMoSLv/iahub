//! Mega Brain V0 — Operation Journal Repository
//!
//! Persistence operations for the `operations` table. All access goes through
//! this module; no raw SQL in service logic.
//!
//! Status and operation_type are stored as canonical unquoted TEXT strings
//! (e.g., "PREPARED", "CREATE_WORKTREE"), NOT JSON-encoded strings.

use rusqlite::params;
use rusqlite::OptionalExtension;

use super::error::OperationError;
use super::model::{OperationId, OperationRecord, OperationStatus, OperationType};
use crate::persistence::transaction::Transaction;

/// Insert a new PREPARED operation entry. MUST be called before any side effect.
pub fn insert_operation(tx: &Transaction, record: &OperationRecord) -> Result<(), OperationError> {
    tx.conn()
        .execute(
            "INSERT INTO operations (
                id, operation_type, status, project_id, task_id, attempt_id,
                command_id, preconditions, input_payload, result_payload,
                external_reference, recovery_hint, prepared_at, started_at,
                committed_at, failed_at, completed_at, last_error, version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                record.id.0,
                record.operation_type.to_db(),
                record.status.to_db(),
                record.project_id,
                record.task_id,
                record.attempt_id,
                record.command_id,
                record.preconditions,
                record.input_payload,
                record.result_payload,
                record.external_reference,
                record.recovery_hint,
                record.prepared_at,
                record.started_at,
                record.committed_at,
                record.failed_at,
                record.completed_at,
                record.last_error,
                record.version,
            ],
        )
        .map_err(|e| OperationError::Persistence {
            message: format!("insert operation failed: {}", e),
        })?;
    Ok(())
}

/// Find an operation by ID.
pub fn find_by_id(
    tx: &Transaction,
    operation_id: &OperationId,
) -> Result<Option<OperationRecord>, OperationError> {
    let row = tx
        .conn()
        .prepare(
            "SELECT id, operation_type, status, project_id, task_id, attempt_id,
                    command_id, preconditions, input_payload, result_payload,
                    external_reference, recovery_hint, prepared_at, started_at,
                    committed_at, failed_at, completed_at, last_error, version
             FROM operations WHERE id = ?1",
        )
        .map_err(|e| OperationError::Persistence {
            message: format!("find_by_id prepare failed: {}", e),
        })?
        .query_row(params![operation_id.0], parse_operation_row)
        .optional()
        .map_err(|e| OperationError::Persistence {
            message: format!("find_by_id query failed: {}", e),
        })?;
    Ok(row)
}

/// Parameters for updating an operation's status.
#[derive(Debug, Clone)]
pub struct StatusUpdate {
    pub operation_id: OperationId,
    pub new_status: OperationStatus,
    pub now: i64,
    pub expected_version: i64,
    pub last_error: Option<String>,
    pub result_payload: Option<String>,
    pub external_reference: Option<String>,
    pub recovery_hint: Option<String>,
}

/// Update an operation's status with optimistic concurrency control.
/// Returns the new version on success.
pub fn update_status(tx: &Transaction, update: &StatusUpdate) -> Result<i64, OperationError> {
    let new_version =
        update
            .expected_version
            .checked_add(1)
            .ok_or_else(|| OperationError::Persistence {
                message: format!(
                    "version overflow for operation {}: version {} cannot be incremented",
                    update.operation_id, update.expected_version
                ),
            })?;

    // Compute timestamp fields based on new status
    let started_at = if update.new_status == OperationStatus::Executing {
        Some(update.now)
    } else {
        None
    };
    let committed_at = if update.new_status == OperationStatus::Committed {
        Some(update.now)
    } else {
        None
    };
    let failed_at = if update.new_status == OperationStatus::Failed {
        Some(update.now)
    } else {
        None
    };
    let completed_at = if update.new_status.is_terminal() {
        Some(update.now)
    } else {
        None
    };

    let affected = tx
        .conn()
        .execute(
            "UPDATE operations SET
                status = COALESCE(?1, status),
                started_at = COALESCE(?2, started_at),
                committed_at = COALESCE(?3, committed_at),
                failed_at = COALESCE(?4, failed_at),
                completed_at = COALESCE(?5, completed_at),
                last_error = COALESCE(?6, last_error),
                result_payload = COALESCE(?7, result_payload),
                external_reference = COALESCE(?8, external_reference),
                recovery_hint = COALESCE(?9, recovery_hint),
                version = ?10
             WHERE id = ?11 AND version = ?12",
            params![
                update.new_status.to_db(),
                started_at,
                committed_at,
                failed_at,
                completed_at,
                update.last_error,
                update.result_payload,
                update.external_reference,
                update.recovery_hint,
                new_version,
                update.operation_id.0,
                update.expected_version,
            ],
        )
        .map_err(|e| OperationError::Persistence {
            message: format!("update_status failed: {}", e),
        })?;

    if affected == 0 {
        let current = find_by_id(tx, &update.operation_id)?;
        match current {
            Some(record) if record.version != update.expected_version => {
                Err(OperationError::VersionConflict {
                    operation_id: update.operation_id.clone(),
                    expected_version: update.expected_version,
                    actual_version: record.version,
                })
            }
            Some(record) => Err(OperationError::InvalidTransition {
                operation_id: update.operation_id.clone(),
                from: record.status,
                to: update.new_status,
            }),
            None => Err(OperationError::NotFound {
                operation_id: update.operation_id.clone(),
            }),
        }
    } else {
        Ok(new_version)
    }
}

/// List all reconcilable operations for startup reconcile.
/// Returns operations in states: PREPARED, EXECUTING, SIDE_EFFECT_OBSERVED, REQUIRES_RECONCILE.
/// Excludes truly terminal states: COMMITTED, ROLLED_BACK, FAILED.
pub fn list_reconcilable(tx: &Transaction) -> Result<Vec<OperationRecord>, OperationError> {
    let mut stmt = tx
        .conn()
        .prepare(
            "SELECT id, operation_type, status, project_id, task_id, attempt_id,
                    command_id, preconditions, input_payload, result_payload,
                    external_reference, recovery_hint, prepared_at, started_at,
                    committed_at, failed_at, completed_at, last_error, version
             FROM operations
             WHERE status IN ('PREPARED', 'EXECUTING', 'SIDE_EFFECT_OBSERVED', 'REQUIRES_RECONCILE')
             ORDER BY prepared_at ASC",
        )
        .map_err(|e| OperationError::Persistence {
            message: format!("list_reconcilable prepare failed: {}", e),
        })?;

    let rows = stmt
        .query_map([], parse_operation_row)
        .map_err(|e| OperationError::Persistence {
            message: format!("list_reconcilable query failed: {}", e),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OperationError::Persistence {
            message: format!("list_reconcilable collect failed: {}", e),
        })?;

    Ok(rows)
}

fn parse_operation_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let op_type_str: String = r.get(1)?;
    let operation_type = OperationType::from_db(&op_type_str).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(1, "operation_type".into(), rusqlite::types::Type::Text)
    })?;

    let status_str: String = r.get(2)?;
    let status = OperationStatus::from_db(&status_str).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(2, "status".into(), rusqlite::types::Type::Text)
    })?;

    Ok(OperationRecord {
        id: OperationId(r.get::<_, String>(0)?),
        operation_type,
        status,
        project_id: r.get(3)?,
        task_id: r.get(4)?,
        attempt_id: r.get(5)?,
        command_id: r.get(6)?,
        preconditions: r.get(7)?,
        input_payload: r.get(8)?,
        result_payload: r.get(9)?,
        external_reference: r.get(10)?,
        recovery_hint: r.get(11)?,
        prepared_at: r.get(12)?,
        started_at: r.get(13)?,
        committed_at: r.get(14)?,
        failed_at: r.get(15)?,
        completed_at: r.get(16)?,
        last_error: r.get(17)?,
        version: r.get(18)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    #[test]
    fn insert_and_find_operation() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationRecord::prepare(
            OperationId("op-001".into()),
            OperationType::CreateWorktree,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let found = find_by_id(&tx, &OperationId("op-001".into()))
            .unwrap()
            .expect("must exist");
        assert_eq!(found.id, OperationId("op-001".into()));
        assert_eq!(found.status, OperationStatus::Prepared);
        assert_eq!(found.operation_type, OperationType::CreateWorktree);
        assert_eq!(found.version, 1);

        tx.commit().unwrap();
    }

    #[test]
    fn find_missing_returns_none() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let result = find_by_id(&tx, &OperationId("nonexistent".into())).unwrap();
        assert!(result.is_none());

        tx.rollback().unwrap();
    }

    #[test]
    fn update_status_advances_version() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationRecord::prepare(
            OperationId("op-002".into()),
            OperationType::SpawnAgent,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let new_version = update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-002".into()),
                new_status: OperationStatus::Executing,
                now: 1700000100,
                expected_version: 1,
                last_error: None,
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap();
        assert_eq!(new_version, 2);

        let updated = find_by_id(&tx, &OperationId("op-002".into()))
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, OperationStatus::Executing);
        assert_eq!(updated.version, 2);
        assert_eq!(updated.started_at, Some(1700000100));

        tx.commit().unwrap();
    }

    #[test]
    fn update_status_with_stale_version_fails() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationRecord::prepare(
            OperationId("op-003".into()),
            OperationType::CanonicalMerge,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let err = update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-003".into()),
                new_status: OperationStatus::Executing,
                now: 1700000100,
                expected_version: 999, // wrong version
                last_error: None,
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::VersionConflict { .. }));

        tx.rollback().unwrap();
    }

    #[test]
    fn version_overflow_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationRecord::prepare(
            OperationId("op-overflow".into()),
            OperationType::CreateGitRef,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let err = update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-overflow".into()),
                new_status: OperationStatus::Executing,
                now: 1700000100,
                expected_version: i64::MAX, // overflow on +1
                last_error: None,
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, OperationError::Persistence { ref message } if message.contains("overflow")),
            "expected overflow error, got {:?}",
            err
        );

        tx.rollback().unwrap();
    }

    #[test]
    fn list_reconcilable_includes_requires_reconcile() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        // PREPARED
        let r1 = OperationRecord::prepare(
            OperationId("op-a".into()),
            OperationType::CreateWorktree,
            100,
        );
        insert_operation(&tx, &r1).unwrap();

        // EXECUTING
        let r2 =
            OperationRecord::prepare(OperationId("op-b".into()), OperationType::SpawnAgent, 200);
        insert_operation(&tx, &r2).unwrap();
        update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-b".into()),
                new_status: OperationStatus::Executing,
                now: 201,
                expected_version: 1,
                last_error: None,
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap();

        // REQUIRES_RECONCILE
        let r3 = OperationRecord::prepare(
            OperationId("op-c".into()),
            OperationType::RemoveWorktree,
            300,
        );
        insert_operation(&tx, &r3).unwrap();
        update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-c".into()),
                new_status: OperationStatus::Executing,
                now: 301,
                expected_version: 1,
                last_error: None,
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap();
        update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-c".into()),
                new_status: OperationStatus::RequiresReconcile,
                now: 302,
                expected_version: 2,
                last_error: Some("process died mid-op".into()),
                result_payload: None,
                external_reference: None,
                recovery_hint: Some("check if worktree dir exists".into()),
            },
        )
        .unwrap();

        // FAILED (terminal — should NOT appear)
        let r4 =
            OperationRecord::prepare(OperationId("op-d".into()), OperationType::CreateGitRef, 400);
        insert_operation(&tx, &r4).unwrap();
        update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-d".into()),
                new_status: OperationStatus::Failed,
                now: 401,
                expected_version: 1,
                last_error: Some("ref exists".into()),
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap();

        let reconcilable = list_reconcilable(&tx).unwrap();
        assert_eq!(
            reconcilable.len(),
            3,
            "must include PREPARED, EXECUTING, and REQUIRES_RECONCILE"
        );
        assert_eq!(reconcilable[0].id, OperationId("op-a".into()));
        assert_eq!(reconcilable[1].id, OperationId("op-b".into()));
        assert_eq!(reconcilable[2].id, OperationId("op-c".into()));
        assert_eq!(reconcilable[2].status, OperationStatus::RequiresReconcile);
        assert_eq!(
            reconcilable[2].recovery_hint.as_deref(),
            Some("check if worktree dir exists")
        );

        tx.commit().unwrap();
    }

    #[test]
    fn canonical_status_stored_unquoted() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let record = OperationRecord::prepare(
            OperationId("op-canonical".into()),
            OperationType::CreateWorktree,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let raw_status: String = tx
            .conn()
            .query_row(
                "SELECT status FROM operations WHERE id = ?1",
                params!["op-canonical"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            raw_status, "PREPARED",
            "status must be unquoted canonical TEXT"
        );

        let raw_type: String = tx
            .conn()
            .query_row(
                "SELECT operation_type FROM operations WHERE id = ?1",
                params!["op-canonical"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            raw_type, "CREATE_WORKTREE",
            "operation_type must be unquoted canonical TEXT"
        );

        tx.commit().unwrap();
    }

    #[test]
    fn prepared_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("op_restart.db");

        // Session 1: create PREPARED operation
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();
            let record = OperationRecord::prepare(
                OperationId("op-persist".into()),
                OperationType::CreateWorktree,
                1700000000,
            );
            insert_operation(&tx, &record).unwrap();
            tx.commit().unwrap();
        }

        // Session 2: reopen and verify
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();

            let found = find_by_id(&tx, &OperationId("op-persist".into()))
                .unwrap()
                .expect("PREPARED operation must survive restart");
            assert_eq!(found.status, OperationStatus::Prepared);
            assert_eq!(found.operation_type, OperationType::CreateWorktree);

            let reconcilable = list_reconcilable(&tx).unwrap();
            assert_eq!(reconcilable.len(), 1);
            assert_eq!(reconcilable[0].id, OperationId("op-persist".into()));

            tx.rollback().unwrap();
        }
    }

    #[test]
    fn requires_reconcile_survives_restart_with_hint() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("op_reconcile_restart.db");

        // Session 1: create operation that ends up REQUIRES_RECONCILE
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();
            let record = OperationRecord::prepare(
                OperationId("op-recon".into()),
                OperationType::RemoveWorktree,
                100,
            );
            insert_operation(&tx, &record).unwrap();
            update_status(
                &tx,
                &StatusUpdate {
                    operation_id: OperationId("op-recon".into()),
                    new_status: OperationStatus::Executing,
                    now: 200,
                    expected_version: 1,
                    last_error: None,
                    result_payload: None,
                    external_reference: None,
                    recovery_hint: None,
                },
            )
            .unwrap();
            update_status(
                &tx,
                &StatusUpdate {
                    operation_id: OperationId("op-recon".into()),
                    new_status: OperationStatus::RequiresReconcile,
                    now: 300,
                    expected_version: 2,
                    last_error: Some("process killed".into()),
                    result_payload: None,
                    external_reference: None,
                    recovery_hint: Some("verify worktree cleanup".into()),
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        // Session 2: reopen, verify, and resolve
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();

            let reconcilable = list_reconcilable(&tx).unwrap();
            assert_eq!(reconcilable.len(), 1);
            assert_eq!(reconcilable[0].id, OperationId("op-recon".into()));
            assert_eq!(reconcilable[0].status, OperationStatus::RequiresReconcile);
            assert_eq!(
                reconcilable[0].recovery_hint.as_deref(),
                Some("verify worktree cleanup")
            );
            assert_eq!(
                reconcilable[0].last_error.as_deref(),
                Some("process killed")
            );

            // Resolve: evidence confirms side effect was observed
            let v4 = update_status(
                &tx,
                &StatusUpdate {
                    operation_id: OperationId("op-recon".into()),
                    new_status: OperationStatus::SideEffectObserved,
                    now: 400,
                    expected_version: 3,
                    last_error: None,
                    result_payload: Some("{\"cleaned\":true}".into()),
                    external_reference: None,
                    recovery_hint: None,
                },
            )
            .unwrap();
            assert_eq!(v4, 4);

            // Commit the resolved operation
            let v5 = update_status(
                &tx,
                &StatusUpdate {
                    operation_id: OperationId("op-recon".into()),
                    new_status: OperationStatus::Committed,
                    now: 500,
                    expected_version: 4,
                    last_error: None,
                    result_payload: None,
                    external_reference: None,
                    recovery_hint: None,
                },
            )
            .unwrap();
            assert_eq!(v5, 5);

            let final_record = find_by_id(&tx, &OperationId("op-recon".into()))
                .unwrap()
                .unwrap();
            assert_eq!(final_record.status, OperationStatus::Committed);
            assert!(final_record.status.is_terminal());

            // No longer reconcilable
            let reconcilable_after = list_reconcilable(&tx).unwrap();
            assert!(reconcilable_after.is_empty());

            tx.commit().unwrap();
        }
    }
}
