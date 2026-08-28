//! Mega Brain V0 — Transaction Abstraction
//!
//! Centralized transaction API. All state mutations that must be atomic go
//! through this abstraction. Rollback is automatic on drop if not committed.

use rusqlite::{Connection, Transaction as RusqliteTransaction};

use super::error::PersistenceError;

/// A database transaction wrapper with automatic rollback on drop.
pub struct Transaction<'conn> {
    inner: Option<RusqliteTransaction<'conn>>,
}

impl<'conn> Transaction<'conn> {
    /// Begin a new deferred transaction.
    ///
    /// Accepts `&mut Connection` directly. The caller (`SqliteStore`) is
    /// responsible for providing safe mutable access via `UnsafeCell`.
    pub fn begin(conn: &'conn mut Connection) -> Result<Self, PersistenceError> {
        let tx = conn
            .transaction()
            .map_err(|e| PersistenceError::Transaction { source: e })?;
        Ok(Self { inner: Some(tx) })
    }

    /// Commit the transaction. Consumes self to prevent further use.
    pub fn commit(mut self) -> Result<(), PersistenceError> {
        if let Some(tx) = self.inner.take() {
            tx.commit()
                .map_err(|e| PersistenceError::Transaction { source: e })?;
        }
        Ok(())
    }

    /// Explicit rollback. Normally not needed — drop handles it.
    pub fn rollback(mut self) -> Result<(), PersistenceError> {
        if let Some(tx) = self.inner.take() {
            tx.rollback()
                .map_err(|e| PersistenceError::Transaction { source: e })?;
        }
        Ok(())
    }

    /// Access the underlying rusqlite connection for executing statements
    /// within this transaction.
    pub fn conn(&self) -> &Connection {
        self.inner.as_ref().expect("transaction already consumed")
    }

    /// Create a savepoint within this transaction.
    ///
    /// Savepoints allow partial rollback: if a handler fails, we can roll back
    /// only the handler's mutations while preserving the command record and
    /// idempotency state inserted before the savepoint.
    ///
    /// Returns a `Savepoint` guard that rolls back on drop unless committed.
    pub fn savepoint(&self, name: &str) -> Result<Savepoint<'_>, PersistenceError> {
        self.conn()
            .execute_batch(&format!("SAVEPOINT {};", name))
            .map_err(|e| PersistenceError::Transaction { source: e })?;
        Ok(Savepoint {
            conn: self.conn(),
            name: name.to_string(),
            committed: false,
        })
    }
}

/// A savepoint within a transaction. Rolls back on drop unless explicitly
/// released (committed). This enables atomic handler execution: if the handler
/// fails, its mutations are discarded while the surrounding transaction
/// (command record, idempotency state) remains intact.
pub struct Savepoint<'conn> {
    conn: &'conn Connection,
    name: String,
    committed: bool,
}

impl Savepoint<'_> {
    /// Release (commit) the savepoint, making its changes permanent within
    /// the enclosing transaction.
    pub fn release(mut self) -> Result<(), PersistenceError> {
        self.committed = true;
        self.conn
            .execute_batch(&format!("RELEASE SAVEPOINT {};", self.name))
            .map_err(|e| PersistenceError::Transaction { source: e })
    }

    /// Explicitly roll back to this savepoint, discarding all changes made
    /// after it was created. The enclosing transaction continues.
    pub fn rollback(mut self) -> Result<(), PersistenceError> {
        self.committed = true; // Prevent double-rollback in Drop
        self.conn
            .execute_batch(&format!("ROLLBACK TO SAVEPOINT {};", self.name))
            .map_err(|e| PersistenceError::Transaction { source: e })
    }

    /// Access the underlying connection for executing statements within
    /// this savepoint scope.
    pub fn conn(&self) -> &Connection {
        self.conn
    }
}

impl Drop for Savepoint<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // Auto-rollback on drop if not explicitly released or rolled back.
            // Ignore errors during drop — the enclosing transaction will
            // handle cleanup.
            let _ = self
                .conn
                .execute_batch(&format!("ROLLBACK TO SAVEPOINT {};", self.name));
        }
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        // If inner is still Some, the transaction was neither committed nor
        // explicitly rolled back. rusqlite's Transaction::drop rolls back
        // automatically, so we just let it happen.
    }
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
    fn committed_transaction_persists() {
        let mut conn = fresh_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT);")
            .unwrap();

        {
            let tx = Transaction::begin(&mut conn).unwrap();
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (1, 'hello')", [])
                .unwrap();
            tx.commit().unwrap();
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn dropped_transaction_rolls_back() {
        let mut conn = fresh_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT);")
            .unwrap();

        {
            let tx = Transaction::begin(&mut conn).unwrap();
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (1, 'hello')", [])
                .unwrap();
            // tx dropped here without commit → rollback
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn explicit_rollback_discards_changes() {
        let mut conn = fresh_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT);")
            .unwrap();

        {
            let tx = Transaction::begin(&mut conn).unwrap();
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (1, 'hello')", [])
                .unwrap();
            tx.rollback().unwrap();
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn failed_second_insert_rolls_back_first() {
        let mut conn = fresh_conn();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY UNIQUE, val TEXT NOT NULL);")
            .unwrap();

        let result = (|| -> Result<(), PersistenceError> {
            let tx = Transaction::begin(&mut conn)?;
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (1, 'a')", [])
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            // This should fail due to NOT NULL violation
            tx.conn()
                .execute("INSERT INTO t (id, val) VALUES (2, NULL)", [])
                .map_err(|e| PersistenceError::Transaction { source: e })?;
            tx.commit()
        })();

        assert!(result.is_err());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "both inserts must be rolled back");
    }
}
