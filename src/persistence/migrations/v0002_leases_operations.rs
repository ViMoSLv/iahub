//! Mega Brain V0 — Migration v0002: Leases + Operations Schema Evolution
//!
//! Evolves the v0001 `leases` and `operations` tables to match ADR-0003/ADR-0004
//! requirements for Topic 04. Since v0001 is frozen, this migration adds missing
//! columns and adapts the schema forward without destructive changes.
//!
//! Key changes:
//! - leases: add `status`, `owner_attempt_id` alias, `created_at`, `updated_at`
//! - operations: add `recovery_hint`, `completed_at` for ADR-0003 compliance
//! - task_attempts: add cancellation evidence columns per Appendix E
//!
//! Uniqueness of ACTIVE leases is enforced by Lease Service + fencing tokens.

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // ── leases: evolve v0001 schema to ADR-0004 ─────────────────────────────
    // v0001 has: id, resource_type, resource_id, attempt_id, lease_token_hash,
    //            fencing_token, issued_at, heartbeat_at, expires_at, revoked_at, version
    // ADR-0004 needs: owner_attempt_id, status, created_at, updated_at

    // Add status column (ACTIVE/EXPIRED/REVOKED). Default existing rows to ACTIVE
    // since v0001 had no explicit status; revoked_at presence indicates REVOKED.
    let has_status: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('leases') WHERE name = 'status'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_status {
        conn.execute_batch(
            "ALTER TABLE leases ADD COLUMN status TEXT NOT NULL DEFAULT 'ACTIVE';",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add leases.status: {}", e),
            source: Some(e),
        })?;
    }

    // Add owner_attempt_id as alias for attempt_id (v0001 used attempt_id)
    let has_owner_attempt: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('leases') WHERE name = 'owner_attempt_id'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_owner_attempt {
        conn.execute_batch(
            "ALTER TABLE leases ADD COLUMN owner_attempt_id TEXT REFERENCES task_attempts(id);",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add leases.owner_attempt_id: {}", e),
            source: Some(e),
        })?;
        // Copy existing attempt_id values to owner_attempt_id
        conn.execute_batch(
            "UPDATE leases SET owner_attempt_id = attempt_id WHERE owner_attempt_id IS NULL;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to backfill leases.owner_attempt_id: {}", e),
            source: Some(e),
        })?;
    }

    // Add created_at / updated_at timestamps
    let has_created_at: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('leases') WHERE name = 'created_at'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_created_at {
        conn.execute_batch(
            "ALTER TABLE leases ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add leases.created_at: {}", e),
            source: Some(e),
        })?;
        // Backfill from issued_at as reasonable default
        conn.execute_batch(
            "UPDATE leases SET created_at = issued_at WHERE created_at = 0;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to backfill leases.created_at: {}", e),
            source: Some(e),
        })?;
    }

    let has_updated_at: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('leases') WHERE name = 'updated_at'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_updated_at {
        conn.execute_batch(
            "ALTER TABLE leases ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add leases.updated_at: {}", e),
            source: Some(e),
        })?;
        conn.execute_batch(
            "UPDATE leases SET updated_at = issued_at WHERE updated_at = 0;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to backfill leases.updated_at: {}", e),
            source: Some(e),
        })?;
    }

    // Add index on owner_attempt_id if not present
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_leases_owner_v2 ON leases(owner_attempt_id);",
    ).map_err(|e| PersistenceError::MigrationFailed {
        version: 2,
        message: format!("failed to create idx_leases_owner_v2: {}", e),
        source: Some(e),
    })?;

    // Add composite index for expiry scanning
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_leases_expiry_v2 ON leases(status, expires_at);",
    ).map_err(|e| PersistenceError::MigrationFailed {
        version: 2,
        message: format!("failed to create idx_leases_expiry_v2: {}", e),
        source: Some(e),
    })?;

    // ── operations: evolve v0001 schema to ADR-0003 ─────────────────────────
    // v0001 has: id, operation_type, status, project_id, task_id, attempt_id,
    //            command_id, preconditions, input_payload, result_payload,
    //            external_reference, prepared_at, started_at, committed_at,
    //            failed_at, last_error, version
    // ADR-0003 needs: recovery_hint, completed_at, payload (rename input_payload?)

    let has_recovery_hint: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('operations') WHERE name = 'recovery_hint'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_recovery_hint {
        conn.execute_batch(
            "ALTER TABLE operations ADD COLUMN recovery_hint TEXT;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add operations.recovery_hint: {}", e),
            source: Some(e),
        })?;
    }

    let has_completed_at: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('operations') WHERE name = 'completed_at'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_completed_at {
        conn.execute_batch(
            "ALTER TABLE operations ADD COLUMN completed_at INTEGER;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add operations.completed_at: {}", e),
            source: Some(e),
        })?;
    }

    // Ensure command index exists (v0001 only had status index)
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_operations_command_v2 ON operations(command_id);",
    ).map_err(|e| PersistenceError::MigrationFailed {
        version: 2,
        message: format!("failed to create idx_operations_command_v2: {}", e),
        source: Some(e),
    })?;

    // ── task_attempts: cancellation evidence columns (Appendix E) ────────────
    let has_cancel_requested: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('task_attempts') WHERE name = 'cancel_requested_at'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_cancel_requested {
        conn.execute_batch(
            "ALTER TABLE task_attempts ADD COLUMN cancel_requested_at INTEGER;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add task_attempts.cancel_requested_at: {}", e),
            source: Some(e),
        })?;
    }

    let has_term_evidence: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('task_attempts') WHERE name = 'termination_evidence'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_term_evidence {
        conn.execute_batch(
            "ALTER TABLE task_attempts ADD COLUMN termination_evidence TEXT;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add task_attempts.termination_evidence: {}", e),
            source: Some(e),
        })?;
    }

    let has_cancel_indet: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('task_attempts') WHERE name = 'cancel_indeterminate_reason'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_cancel_indet {
        conn.execute_batch(
            "ALTER TABLE task_attempts ADD COLUMN cancel_indeterminate_reason TEXT;",
        ).map_err(|e| PersistenceError::MigrationFailed {
            version: 2,
            message: format!("failed to add task_attempts.cancel_indeterminate_reason: {}", e),
            source: Some(e),
        })?;
    }

    Ok(())
}