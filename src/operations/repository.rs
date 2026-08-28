//! Mega Brain V0 — Operation Journal Repository
//!
//! Persistence operations for the `operations` table. All access goes through
//! this module; no raw SQL in service logic.

use rusqlite::params;
use rusqlite::OptionalExtension;

use super::error::OperationError;
use super::model::{OperationId, OperationRecord, OperationStatus, OperationType};
use crate::persistence::transaction::Transaction;

/// Insert a new PREPARED operation entry. MUST be called before any side effect.
pub fn insert_operation(tx: &Transaction, record: &OperationRecord) -> Result<(), OperationError> {
    let op_type =
        serde_json::to_string(&record.operation_type).map_err(|e| OperationError::Persistence {
            message: format!("failed to serialize operation_type: {}", e),
        })?;
    let status =
        serde_json::to_string(&record.status).map_err(|e| OperationError::Persistence {
            message: format!("failed to serialize status: {}", e),
        })?;

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
                op_type,
                status,
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
    let new_version = update.expected_version + 1;
    let status_json =
        serde_json::to_string(&update.new_status).map_err(|e| OperationError::Persistence {
            message: format!("failed to serialize status: {}", e),
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
                status_json,
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
            Some(_) => Err(OperationError::InvalidTransition {
                operation_id: update.operation_id.clone(),
                from: OperationStatus::Prepared, // placeholder
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

/// List all non-terminal operations (for startup reconcile).
pub fn list_non_terminal(tx: &Transaction) -> Result<Vec<OperationRecord>, OperationError> {
    let mut stmt = tx
        .conn()
        .prepare(
            "SELECT id, operation_type, status, project_id, task_id, attempt_id,
                    command_id, preconditions, input_payload, result_payload,
                    external_reference, recovery_hint, prepared_at, started_at,
                    committed_at, failed_at, completed_at, last_error, version
             FROM operations
             WHERE status NOT IN ('\"COMMITTED\"', '\"ROLLED_BACK\"', '\"REQUIRES_RECONCILE\"', '\"FAILED\"')
             ORDER BY prepared_at ASC",
        )
        .map_err(|e| OperationError::Persistence {
            message: format!("list_non_terminal prepare failed: {}", e),
        })?;

    let rows = stmt
        .query_map([], parse_operation_row)
        .map_err(|e| OperationError::Persistence {
            message: format!("list_non_terminal query failed: {}", e),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| OperationError::Persistence {
            message: format!("list_non_terminal collect failed: {}", e),
        })?;

    Ok(rows)
}

fn parse_operation_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let op_type_str: String = r.get(1)?;
    let operation_type: OperationType =
        serde_json::from_str(&op_type_str).unwrap_or(OperationType::Other {
            detail: op_type_str,
        });

    let status_str: String = r.get(2)?;
    let status: OperationStatus = serde_json::from_str(&status_str).map_err(|_| {
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
            OperationType::WorktreeCreate,
            1700000000,
        );
        insert_operation(&tx, &record).unwrap();

        let found = find_by_id(&tx, &OperationId("op-001".into()))
            .unwrap()
            .expect("must exist");
        assert_eq!(found.id, OperationId("op-001".into()));
        assert_eq!(found.status, OperationStatus::Prepared);
        assert_eq!(found.operation_type, OperationType::WorktreeCreate);
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
            OperationType::AgentSpawn,
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
            OperationType::MergeExecute,
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
    fn list_non_terminal_excludes_terminal() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        // Insert PREPARED
        let r1 = OperationRecord::prepare(
            OperationId("op-a".into()),
            OperationType::WorktreeCreate,
            100,
        );
        insert_operation(&tx, &r1).unwrap();

        // Insert EXECUTING
        let r2 =
            OperationRecord::prepare(OperationId("op-b".into()), OperationType::AgentSpawn, 200);
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

        // Insert FAILED (terminal)
        let r3 =
            OperationRecord::prepare(OperationId("op-c".into()), OperationType::GitRefCreate, 300);
        insert_operation(&tx, &r3).unwrap();
        update_status(
            &tx,
            &StatusUpdate {
                operation_id: OperationId("op-c".into()),
                new_status: OperationStatus::Failed,
                now: 301,
                expected_version: 1,
                last_error: Some("connection refused".into()),
                result_payload: None,
                external_reference: None,
                recovery_hint: None,
            },
        )
        .unwrap();

        let non_terminal = list_non_terminal(&tx).unwrap();
        assert_eq!(non_terminal.len(), 2);
        assert_eq!(non_terminal[0].id, OperationId("op-a".into()));
        assert_eq!(non_terminal[1].id, OperationId("op-b".into()));

        tx.commit().unwrap();
    }
}
