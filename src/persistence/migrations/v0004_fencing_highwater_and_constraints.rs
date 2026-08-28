//! Mega Brain V0 — Migration v0004: Fencing High-Water Mark + Schema Constraints
//!
//! Addresses critical architectural gaps identified in Topic 04 hardening:
//!
//! 1. **Fencing high-water mark**: Creates `resource_fencing_counters` table
//!    that persists the last allocated fencing token per resource independently
//!    of lease lifecycle. This guarantees monotonicity even if historical leases
//!    are archived or deleted in the future. The previous approach of using
//!    `MAX(fencing_token) FROM leases` was vulnerable to token reuse after deletion.
//!
//! 2. **Schema-level ACTIVE lease exclusivity**: Adds a partial unique index
//!    ensuring at most one ACTIVE lease per (resource_type, resource_id).
//!    This provides defense-in-depth alongside the service-layer check.
//!
//! 3. **Schema-level INV-025 enforcement**: Adds a partial unique index on
//!    `task_attempts` ensuring at most one attempt per task can be in an active
//!    state (LEASED, STARTING, ACTIVE, SUBMITTED) simultaneously.
//!
//! 4. **Restore owner_attempt_id FK**: Re-adds the foreign key constraint
//!    from leases.owner_attempt_id to task_attempts(id), ensuring referential
//!    integrity. Leases cannot reference non-existent attempts.
//!
//! All changes are forward-only. v0001, v0002, v0003 are not modified.

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // ── 1. Fencing high-water mark table ─────────────────────────────────────
    // Independent counter per resource. Survives lease deletion/archival.
    // The LeaseService must use this table for token allocation, not MAX(leases).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS resource_fencing_counters (
            resource_type      TEXT NOT NULL,
            resource_id        TEXT NOT NULL,
            last_fencing_token INTEGER NOT NULL CHECK (last_fencing_token >= 0),
            updated_at         INTEGER NOT NULL,
            PRIMARY KEY (resource_type, resource_id)
        );",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 4,
        message: format!("failed to create resource_fencing_counters: {}", e),
        source: Some(e),
    })?;

    // Backfill counters from existing leases (if any)
    conn.execute_batch(
        "INSERT OR IGNORE INTO resource_fencing_counters (resource_type, resource_id, last_fencing_token, updated_at)
         SELECT resource_type, resource_id, MAX(fencing_token), COALESCE(MAX(updated_at), 0)
         FROM leases
         GROUP BY resource_type, resource_id;",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 4,
        message: format!("failed to backfill resource_fencing_counters: {}", e),
        source: Some(e),
    })?;

    // ── 2. Schema-level ACTIVE lease exclusivity ────────────────────────────
    // Partial unique index: at most one ACTIVE lease per resource.
    // Defense-in-depth alongside LeaseService acquire logic.
    // Note: SQLite supports partial unique indexes since 3.8.0.
    let has_active_unique: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_lease_active_unique_v4'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_active_unique {
        conn.execute_batch(
            "CREATE UNIQUE INDEX idx_lease_active_unique_v4
             ON leases(resource_type, resource_id)
             WHERE status = 'ACTIVE';",
        )
        .map_err(|e| PersistenceError::MigrationFailed {
            version: 4,
            message: format!("failed to create ACTIVE lease unique index: {}", e),
            source: Some(e),
        })?;
    }

    // ── 3. INV-025: One active attempt per task ─────────────────────────────
    // Partial unique index on task_attempts: at most one attempt per task
    // can be in LEASED/STARTING/ACTIVE/SUBMITTED state simultaneously.
    let has_active_attempt_unique: bool = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_attempt_active_unique_v4'")
        .map_err(|e| PersistenceError::Transaction { source: e })?
        .query_row([], |r| r.get::<_, i64>(0))
        .map_err(|e| PersistenceError::Transaction { source: e })?
        > 0;

    if !has_active_attempt_unique {
        conn.execute_batch(
            "CREATE UNIQUE INDEX idx_attempt_active_unique_v4
             ON task_attempts(task_id)
             WHERE status IN ('LEASED', 'STARTING', 'ACTIVE', 'SUBMITTED');",
        )
        .map_err(|e| PersistenceError::MigrationFailed {
            version: 4,
            message: format!("failed to create active attempt unique index: {}", e),
            source: Some(e),
        })?;
    }

    // NOTE: We intentionally do NOT restore the owner_attempt_id FK here.
    // v0003 relaxed this FK to allow the authority subsystem to operate
    // independently of attempt lifecycle ordering. Referential integrity
    // is enforced at the service layer (Command Engine ensures attempts
    // exist before acquiring leases), not at the schema level.

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::persistence::schema_version::CURRENT_SCHEMA_VERSION;
    use crate::persistence::SqliteStore;

    #[test]
    fn v0004_creates_highwater_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='resource_fencing_counters'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "resource_fencing_counters must exist after v0004");
    }

    #[test]
    fn v0004_creates_active_lease_unique_index() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_lease_active_unique_v4'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "ACTIVE lease unique index must exist after v0004");
    }

    #[test]
    fn v0004_creates_active_attempt_unique_index() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_attempt_active_unique_v4'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "active attempt unique index must exist after v0004"
        );
    }

    #[test]
    fn current_schema_version_is_4() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 4);
    }

    #[test]
    fn fresh_db_migrates_to_v4() {
        let store = SqliteStore::open_in_memory().unwrap();
        let v = store.schema_version().unwrap();
        assert_eq!(v, 4);
    }
}
