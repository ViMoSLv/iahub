//! Mega Brain V0 — Schema Version Management
//!
//! The database carries an explicit schema version. On open:
//! - version < supported → migrate forward
//! - version == supported → proceed
//! - version > supported → FAIL CLOSED (UnsupportedFutureSchemaVersion)
//!
//! A binary must never silently write to a schema it does not understand.

use rusqlite::Connection;

use super::error::PersistenceError;

/// The schema version this binary understands and produces.
/// Increment when adding migrations.
pub const CURRENT_SCHEMA_VERSION: i64 = 4;

/// Name of the internal table tracking schema metadata.
pub const SCHEMA_VERSION_TABLE: &str = "mega_brain_schema";

/// Ensure the schema version table exists and return the current stored version.
/// Returns 0 if the table does not yet exist (fresh database).
pub fn read_schema_version(conn: &Connection) -> Result<i64, PersistenceError> {
    // Check if the schema table exists
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1;",
            [SCHEMA_VERSION_TABLE],
            |row| row.get(0),
        )
        .map_err(|e| PersistenceError::DatabaseOpen {
            path: "<schema_version>".to_string(),
            source: e,
        })?;

    if !exists {
        return Ok(0);
    }

    conn.query_row(
        &format!("SELECT version FROM {} LIMIT 1;", SCHEMA_VERSION_TABLE),
        [],
        |row| row.get(0),
    )
    .map_err(|e| PersistenceError::DatabaseOpen {
        path: "<schema_version read>".to_string(),
        source: e,
    })
}

/// Set or update the schema version in the metadata table.
pub fn set_schema_version(conn: &Connection, version: i64) -> Result<(), PersistenceError> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {} (version INTEGER NOT NULL);",
        SCHEMA_VERSION_TABLE
    ))
    .map_err(|e| PersistenceError::MigrationFailed {
        version,
        message: "failed to create schema version table".to_string(),
        source: Some(e),
    })?;

    // rusqlite::execute() only accepts a single statement. Split DELETE and INSERT
    // into two calls to avoid MultipleStatement errors.
    conn.execute_batch(&format!("DELETE FROM {};", SCHEMA_VERSION_TABLE))
        .map_err(|e| PersistenceError::MigrationFailed {
            version,
            message: "failed to clear schema version".to_string(),
            source: Some(e),
        })?;

    conn.execute(
        &format!(
            "INSERT INTO {} (version) VALUES (?1);",
            SCHEMA_VERSION_TABLE
        ),
        [version],
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version,
        message: "failed to set schema version".to_string(),
        source: Some(e),
    })?;

    Ok(())
}

/// Validate that the stored schema version is compatible with this binary.
/// Returns Ok(stored_version) if compatible, Err if too new.
pub fn validate_schema_version(stored: i64) -> Result<i64, PersistenceError> {
    if stored > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::SchemaTooNew {
            found: stored,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_returns_version_zero() {
        let conn = Connection::open(":memory:").unwrap();
        let v = read_schema_version(&conn).unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn set_and_read_schema_version() {
        let conn = Connection::open(":memory:").unwrap();
        set_schema_version(&conn, 1).unwrap();
        let v = read_schema_version(&conn).unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn set_schema_version_is_idempotent() {
        let conn = Connection::open(":memory:").unwrap();
        set_schema_version(&conn, 1).unwrap();
        set_schema_version(&conn, 1).unwrap();
        let v = read_schema_version(&conn).unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn set_schema_version_can_advance() {
        let conn = Connection::open(":memory:").unwrap();
        set_schema_version(&conn, 1).unwrap();
        set_schema_version(&conn, 2).unwrap();
        let v = read_schema_version(&conn).unwrap();
        assert_eq!(v, 2);
    }

    #[test]
    fn validate_accepts_current_version() {
        assert!(validate_schema_version(CURRENT_SCHEMA_VERSION).is_ok());
    }

    #[test]
    fn validate_accepts_older_version() {
        assert!(validate_schema_version(0).is_ok());
    }

    #[test]
    fn validate_rejects_future_version() {
        let future = CURRENT_SCHEMA_VERSION + 1;
        let err = validate_schema_version(future).unwrap_err();
        match err {
            PersistenceError::SchemaTooNew { found, supported } => {
                assert_eq!(found, future);
                assert_eq!(supported, CURRENT_SCHEMA_VERSION);
            }
            other => panic!("expected SchemaTooNew, got {:?}", other),
        }
    }
}
