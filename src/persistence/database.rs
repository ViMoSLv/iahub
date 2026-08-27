//! Mega Brain V0 — SqliteStore (Connection Factory)
//!
//! Single canonical entry point for opening the coordination database.
//! Applies pragmas, verifies SQLite version, runs migrations, and exposes
//! repository access. No other module should call `Connection::open` directly.

use rusqlite::Connection;
use std::cell::UnsafeCell;
use std::path::{Path, PathBuf};

use super::config;
use super::error::PersistenceError;
use super::migrations;
use super::schema_version;
use super::transaction::Transaction;

/// The central persistence handle. Owns the SQLite connection and provides
/// transactional access to all repositories.
///
/// Uses `UnsafeCell` to allow `Transaction::begin(&self)` to obtain a
/// `&mut Connection` from `&self`. This is sound because:
/// - `SqliteStore` is not `Sync` (cannot be shared across threads).
/// - Only one `Transaction` can be live at a time (borrows `&self` exclusively via `'conn`).
/// - SQLite WAL guarantees single-writer semantics at the engine level.
pub struct SqliteStore {
    conn: UnsafeCell<Connection>,
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
            conn: UnsafeCell::new(conn),
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
            conn: UnsafeCell::new(conn),
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Begin a new transaction.
    pub fn transaction(&self) -> Result<Transaction<'_>, PersistenceError> {
        // SAFETY: SqliteStore is !Send + !Sync. Only one Transaction can borrow
        // self at a time. The returned Transaction holds &'_ self, preventing
        // concurrent access through Rust's borrow checker.
        Transaction::begin(unsafe { &mut *self.conn.get() })
    }

    /// Access the underlying connection for read-only queries outside transactions.
    pub fn conn(&self) -> &Connection {
        // SAFETY: No mutable borrow can coexist with this shared reference
        // because Transaction borrows &mut self exclusively via 'conn lifetime.
        unsafe { &*self.conn.get() }
    }

    /// Return the path this store was opened with.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Return the current schema version stored in the database.
    pub fn schema_version(&self) -> Result<i64, PersistenceError> {
        // SAFETY: No mutable borrow can coexist with this shared reference
        // because Transaction borrows &mut self exclusively via 'conn lifetime.
        schema_version::read_schema_version(unsafe { &*self.conn.get() })
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
            let store = SqliteStore::open(&db_path).expect("first open must succeed");
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
            rusqlite::params!["run-1", "nonexistent-project", "obj", "DRAFT", 1i64, 1000i64, 1000i64],
        );
        assert!(result.is_err(), "FK violation must be rejected");
    }

    #[test]
    fn duplicate_command_id_is_rejected() {
        let store = SqliteStore::open_in_memory().unwrap();
        store
            .conn()
            .execute(
                "INSERT INTO commands (command_id, command_type, payload_hash, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params!["cmd-1", "TestCmd", "hash-abc", "PENDING", 1000i64],
            )
            .unwrap();

        let result = store.conn().execute(
            "INSERT INTO commands (command_id, command_type, payload_hash, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["cmd-1", "OtherCmd", "hash-def", "PENDING", 2000i64],
        );
        assert!(result.is_err(), "duplicate command_id must be rejected");
    }

    #[test]
    fn self_dependency_is_rejected() {
        let store = SqliteStore::open_in_memory().unwrap();

        // Need a project → run → task first
        store
            .conn()
            .execute_batch(
                "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES ('p1', 'P', 'fp', '/p', 'main', 0, 0, 1);
                 INSERT INTO runs (id, project_id, objective, status, version, created_at, updated_at) VALUES ('r1', 'p1', 'obj', 'DRAFT', 1, 0, 0);
                 INSERT INTO tasks (id, run_id, title, objective, status, priority, version, created_at, updated_at) VALUES ('t1', 'r1', 'T', 'obj', 'CREATED', 0, 1, 0, 0);",
            )
            .unwrap();

        let result = store.conn().execute(
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
        let store = SqliteStore::open_in_memory().unwrap();

        let result = (|| -> Result<(), PersistenceError> {
            let tx = store.transaction()?;
            tx.conn()
                .execute(
                    "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    rusqlite::params!["p1", "P", "fp", "/p", "main", 0i64, 0i64, 1i64],
                )
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            // Force error
            tx.conn()
                .execute("INSERT INTO nonexistent_table VALUES (1)", [])
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            tx.commit()
        })();

        assert!(result.is_err());

        let count: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM projects;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "project insert must be rolled back");
    }

    #[test]
    fn wal_mode_allows_concurrent_readers() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("wal_test.db");

        let store_a = SqliteStore::open(&db_path).unwrap();
        let store_b = SqliteStore::open(&db_path).unwrap();

        // Writer A inserts
        let tx = store_a.transaction().unwrap();
        tx.conn()
            .execute(
                "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params!["p1", "P", "fp", "/p", "main", 0i64, 0i64, 1i64],
            )
            .unwrap();
        tx.commit().unwrap();

        // Reader B can see it
        let count: i64 = store_b
            .conn()
            .query_row("SELECT COUNT(*) FROM projects;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
