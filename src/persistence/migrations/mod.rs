//! Mega Brain V0 — Migration System
//!
//! Ordered, deterministic, transactable migrations. Each migration is a
//! function that receives a `&Transaction` and returns `Result`. Migrations
//! are applied sequentially inside a single transaction per version step.
//! Re-running an already-applied migration is a no-op (idempotent guard).

use rusqlite::Connection;

use super::error::PersistenceError;
use super::schema_version::{self, CURRENT_SCHEMA_VERSION};
use super::transaction::Transaction;

mod v0001_initial;
mod v0002_leases_operations;
mod v0003_lease_fk_relaxation;
mod v0004_fencing_highwater_and_constraints;

/// A single named migration step.
struct Migration {
    version: i64,
    name: &'static str,
    apply: fn(&Transaction) -> Result<(), PersistenceError>,
}

/// All migrations in strict ascending order. Never reorder or remove entries.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "v0001_initial",
        apply: v0001_initial::apply,
    },
    Migration {
        version: 2,
        name: "v0002_leases_operations",
        apply: v0002_leases_operations::apply,
    },
    Migration {
        version: 3,
        name: "v0003_lease_fk_relaxation",
        apply: v0003_lease_fk_relaxation::apply,
    },
    Migration {
        version: 4,
        name: "v0004_fencing_highwater_and_constraints",
        apply: v0004_fencing_highwater_and_constraints::apply,
    },
];

/// Run all pending migrations up to `CURRENT_SCHEMA_VERSION`.
/// Safe to call on every open; already-applied versions are skipped.
pub fn run_migrations(conn: &mut Connection) -> Result<(), PersistenceError> {
    let stored = schema_version::read_schema_version(conn)?;
    schema_version::validate_schema_version(stored)?;

    for migration in MIGRATIONS {
        if migration.version <= stored {
            continue;
        }
        if migration.version > CURRENT_SCHEMA_VERSION {
            break;
        }

        let tx = Transaction::begin(conn)?;
        (migration.apply)(&tx).map_err(|e| PersistenceError::MigrationFailed {
            version: migration.version,
            message: format!("{} failed: {}", migration.name, e),
            source: None,
        })?;
        schema_version::set_schema_version(tx.conn(), migration.version)?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::config;

    fn fresh_conn() -> Connection {
        let conn = Connection::open(":memory:").unwrap();
        config::apply_and_verify_pragmas(&conn).unwrap();
        conn
    }

    #[test]
    fn fresh_database_migrates_to_current() {
        let mut conn = fresh_conn();
        run_migrations(&mut conn).unwrap();
        let v = schema_version::read_schema_version(&conn).unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn rerun_migrations_is_idempotent() {
        let mut conn = fresh_conn();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();
        let v = schema_version::read_schema_version(&conn).unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let mut conn = fresh_conn();
        schema_version::set_schema_version(&conn, CURRENT_SCHEMA_VERSION + 10).unwrap();
        let err = run_migrations(&mut conn).unwrap_err();
        match err {
            PersistenceError::SchemaTooNew { found, supported } => {
                assert_eq!(found, CURRENT_SCHEMA_VERSION + 10);
                assert_eq!(supported, CURRENT_SCHEMA_VERSION);
            }
            other => panic!("expected SchemaTooNew, got {:?}", other),
        }
    }
}
