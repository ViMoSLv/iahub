//! Mega Brain V0 — SqliteStore (Connection Factory)
//!
//! Single canonical entry point for opening the coordination database.
//! Applies pragmas, verifies SQLite version, runs migrations, and exposes
//! repository access. No other module should call `Connection::open` directly.

#![forbid(unsafe_code)]

use rusqlite::Connection;
use std::path::{Path, PathBuf};

use super::config;
use super::error::PersistenceError;
use super::migrations;
use super::schema_version;
use super::transaction::Transaction;

/// The central persistence handle. Owns the SQLite connection and provides
/// transactional access to all repositories.
///
/// Mutating operations (`transaction`) require `&mut self`, enforcing exclusive
/// access at compile time without any `unsafe` interior mutability.
/// Read-only operations (`conn`, `schema_version`, `path`) accept `&self`.
pub struct SqliteStore {
    conn: Connection,
    db_path: PathBuf,
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore")
            .field("db_path", &self.db_path)
            .finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open (or create) the database at the given path, apply all mandatory
    /// configuration, verify version compatibility, and run pending migrations.
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| PersistenceError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        let mut conn = Connection::open(path).map_err(|e| PersistenceError::DatabaseOpen {
            path: path.display().to_string(),
            source: e,
        })?;

        // Apply and verify all mandatory pragmas
        config::apply_and_verify_pragmas(&conn)?;

        // Verify bundled SQLite version meets constitutional minimum
        let _version = config::verify_sqlite_version(&conn)?;

        // Run integrity check on open
        let check: String = conn
            .query_row("PRAGMA quick_check;", [], |row| row.get(0))
            .map_err(|e| PersistenceError::Corrupt {
                detail: format!("quick_check failed: {}", e),
            })?;
        if check != "ok" {
            return Err(PersistenceError::Corrupt {
                detail: format!("PRAGMA quick_check returned: {}", check),
            });
        }

        // Run migrations (idempotent, transacted per version step)
        migrations::run_migrations(&mut conn)?;

        Ok(Self {
            conn,
            db_path: path.to_path_buf(),
        })
    }

    /// Open an in-memory database for testing. Same configuration as file-backed.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let mut conn =
            Connection::open(":memory:").map_err(|e| PersistenceError::DatabaseOpen {
                path: ":memory:".to_string(),
                source: e,
            })?;
        config::apply_and_verify_pragmas(&conn)?;
        migrations::run_migrations(&mut conn)?;
        Ok(Self {
            conn,
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Begin a new transaction. Requires exclusive access (`&mut self`).
    pub fn transaction(&mut self) -> Result<Transaction<'_>, PersistenceError> {
        Transaction::begin(&mut self.conn)
    }

    /// Access the underlying connection for read-only queries outside transactions.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Return the path this store was opened with.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Return the current schema version stored in the database.
    pub fn schema_version(&self) -> Result<i64, PersistenceError> {
        schema_version::read_schema_version(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::schema_version::CURRENT_SCHEMA_VERSION;

    #[test]
    fn in_memory_store_opens_and_migrates() {
        let store = SqliteStore::open_in_memory().expect("in-memory store must open");
        let v = store.schema_version().unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn file_backed_store_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // First open: create + migrate
        {
            let mut store = SqliteStore::open(&db_path).expect("first open must succeed");
            let tx = store.transaction().unwrap();
            tx.conn()
                .execute(
                    "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params!["proj-1", "Test", "fp-abc", "/tmp/test", "main", 1000i64, 1000i64, 1i64],
                )
                .unwrap();
            tx.commit().unwrap();
        }

        // Second open: verify data persisted
        {
            let store = SqliteStore::open(&db_path).expect("reopen must succeed");
            let name: String = store
                .conn()
                .query_row("SELECT name FROM projects WHERE id = ?1", ["proj-1"], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(name, "Test");
        }
    }

    #[test]
    fn future_schema_is_rejected_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("future.db");

        // Create DB with artificially high schema version
        {
            let conn = Connection::open(&db_path).unwrap();
            config::apply_and_verify_pragmas(&conn).unwrap();
            schema_version::set_schema_version(&conn, CURRENT_SCHEMA_VERSION + 100).unwrap();
        }

        let err = SqliteStore::open(&db_path).unwrap_err();
        match err {
            PersistenceError::SchemaTooNew { found, supported } => {
                assert_eq!(found, CURRENT_SCHEMA_VERSION + 100);
                assert_eq!(supported, CURRENT_SCHEMA_VERSION);
            }
            other => panic!("expected SchemaTooNew, got {:?}", other),
        }
    }

    #[test]
    fn foreign_key_enforcement_works() {
        let store = SqliteStore::open_in_memory().unwrap();
        let result = store.conn().execute(
            "INSERT INTO runs (id, project_id, objective, status, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["run-orphan", "nonexistent-project", "test", "pending", 1i64, 0i64, 0i64],
        );
        assert!(
            result.is_err(),
            "FK constraint must reject orphan run insert"
        );
    }

    #[test]
    fn duplicate_command_id_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        tx.conn()
            .execute(
                "INSERT INTO commands (command_id, command_type, payload_hash, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params!["cmd-1", "CreateProject", "hash-aaa", "SUCCEEDED", 0i64],
            )
            .unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let result = tx2.conn().execute(
            "INSERT INTO commands (command_id, command_type, payload_hash, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["cmd-1", "CreateRun", "hash-bbb", "RECEIVED", 0i64],
        );
        assert!(
            result.is_err(),
            "duplicate command_id must be rejected by PK constraint"
        );
    }

    #[test]
    fn self_dependency_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        // Insert a task first so FK is satisfied
        let tx = store.transaction().unwrap();
        tx.conn()
            .execute(
                "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params!["p1", "P", "fp", "/p", "main", 0i64, 0i64, 1i64],
            )
            .unwrap();
        tx.conn()
            .execute(
                "INSERT INTO runs (id, project_id, objective, status, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params!["r1", "p1", "obj", "pending", 1i64, 0i64, 0i64],
            )
            .unwrap();
        tx.conn()
            .execute(
                "INSERT INTO tasks (id, run_id, title, objective, status, priority, version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params!["t1", "r1", "Test Task", "obj", "CREATED", 0i64, 1i64, 0i64, 0i64],
            )
            .unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let result = tx2.conn().execute(
            "INSERT INTO task_dependencies (task_id, depends_on_task_id, reason, created_at) VALUES (?1, ?1, ?2, ?3)",
            rusqlite::params!["t1", "self-ref", 0i64],
        );
        assert!(
            result.is_err(),
            "self-dependency must be rejected by CHECK constraint"
        );
    }

    #[test]
    fn transaction_rollback_on_error() {
        let mut store = SqliteStore::open_in_memory().unwrap();

        // Create a test table
        store
            .conn()
            .execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT NOT NULL);")
            .unwrap();

        // Attempt a transaction where second insert fails
        let result = (|| -> Result<(), PersistenceError> {
            let tx = store.transaction()?;
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (1, 'a')", [])
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            // This violates NOT NULL
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (2, NULL)", [])
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            tx.commit()
        })();

        assert!(result.is_err());

        // Verify rollback: no rows should exist
        let count: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM t;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "failed transaction must roll back all changes");
    }

    #[test]
    fn wal_mode_allows_concurrent_readers() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wal_test.db");

        let mut store = SqliteStore::open(&db_path).unwrap();

        // Write some data
        let tx = store.transaction().unwrap();
        tx.conn()
            .execute(
                "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params!["wal-proj", "WAL Test", "fp-wal", "/wal", "main", 0i64, 0i64, 1i64],
            )
            .unwrap();
        tx.commit().unwrap();

        // Open a second read-only connection while store is still alive
        let reader = Connection::open(&db_path).unwrap();
        let name: String = reader
            .query_row(
                "SELECT name FROM projects WHERE id = ?1",
                ["wal-proj"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "WAL Test");
    }
}
