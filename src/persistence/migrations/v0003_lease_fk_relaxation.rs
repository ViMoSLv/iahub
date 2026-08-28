//! Mega Brain V0 — Migration v0003: Relax Legacy Lease FK Constraint
//!
//! The v0001 `leases.attempt_id` column has a FOREIGN KEY reference to
//! `task_attempts(id)`. This prevents the Lease Service from operating
//! independently of attempt creation order, which violates the architectural
//! principle that authority management is a standalone concern (ADR-0004).
//!
//! Authority correctness is guaranteed by fencing tokens + service-level
//! validation, not by FK constraints. The legacy `attempt_id` column is
//! retained for backward compatibility but its FK is removed.
//!
//! SQLite does not support DROP CONSTRAINT or ALTER COLUMN to remove FKs.
//! We recreate the table without the FK, preserving all data and indexes.
use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // Check if the FK still exists by inspecting foreign_key_list
    let has_fk: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_foreign_key_list('leases') WHERE \"table\" = 'task_attempts'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_fk {
        // Already migrated or never had the FK
        return Ok(());
    }

    // Recreate leases table without FK on attempt_id.
    // All columns preserved; only the FK constraint is removed.
    conn.execute_batch(
        "
        CREATE TABLE leases_new (
            id                 TEXT PRIMARY KEY,
            resource_type      TEXT NOT NULL,
            resource_id        TEXT NOT NULL,
            attempt_id         TEXT,
            owner_attempt_id   TEXT,
            lease_token_hash   TEXT NOT NULL,
            fencing_token      INTEGER NOT NULL CHECK (fencing_token >= 0),
            status             TEXT NOT NULL DEFAULT 'ACTIVE',
            issued_at          INTEGER NOT NULL,
            heartbeat_at       INTEGER,
            expires_at         INTEGER NOT NULL,
            revoked_at         INTEGER,
            version            INTEGER NOT NULL CHECK (version >= 1),
            created_at         INTEGER NOT NULL DEFAULT 0,
            updated_at         INTEGER NOT NULL DEFAULT 0
        );

        INSERT INTO leases_new (
            id, resource_type, resource_id, attempt_id, owner_attempt_id,
            lease_token_hash, fencing_token, status, issued_at, heartbeat_at,
            expires_at, revoked_at, version, created_at, updated_at
        ) SELECT
            id, resource_type, resource_id, attempt_id, owner_attempt_id,
            lease_token_hash, fencing_token, COALESCE(status, 'ACTIVE'),
            issued_at, heartbeat_at, expires_at, revoked_at, version,
            COALESCE(created_at, 0), COALESCE(updated_at, 0)
        FROM leases;

        DROP TABLE leases;

        ALTER TABLE leases_new RENAME TO leases;

        CREATE INDEX IF NOT EXISTS idx_leases_resource ON leases(resource_type, resource_id);
        CREATE INDEX IF NOT EXISTS idx_leases_expires ON leases(expires_at);
        CREATE INDEX IF NOT EXISTS idx_leases_owner_v2 ON leases(owner_attempt_id);
        CREATE INDEX IF NOT EXISTS idx_leases_expiry_v2 ON leases(status, expires_at);
        ",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 3,
        message: format!("v0003 lease FK relaxation failed: {}", e),
        source: Some(e),
    })?;

    Ok(())
}
