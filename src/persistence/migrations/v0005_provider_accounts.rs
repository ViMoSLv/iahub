//! Mega Brain V0 — Migration v0005: Provider Accounts (ADR-0012)
//!
//! Introduces the `provider_accounts` table to support multi-account provider
//! identity and authentication isolation. This is a domain/architecture
//! preparation change; no runtime session or adapter logic is implemented here.
//!
//! Key design decisions:
//! - Generic schema: no provider-specific columns (e.g., google_email, claude_email).
//!   Provider-specific configuration belongs in typed adapter config, not this table.
//! - Credentials referenced indirectly via `auth_profile_id`; plaintext secrets
//!   are never stored in this table.
//! - Optimistic concurrency via `version` column following existing OCC patterns.
//! - `identity_hint` is optional metadata, not authentication material.
//! - Forward-only migration; v0001–v0004 are not modified.

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS provider_accounts (
            id                       TEXT NOT NULL PRIMARY KEY,
            provider_kind            TEXT NOT NULL,
            label                    TEXT NOT NULL,
            identity_hint            TEXT,
            auth_profile_id          TEXT NOT NULL,
            status                   TEXT NOT NULL,
            max_concurrent_sessions  INTEGER NOT NULL CHECK (max_concurrent_sessions >= 0),
            created_at               INTEGER NOT NULL,
            updated_at               INTEGER NOT NULL,
            version                  INTEGER NOT NULL CHECK (version >= 1)
        );",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 5,
        message: format!("failed to create provider_accounts: {}", e),
        source: Some(e),
    })?;

    // Index for efficient lookup of accounts by provider kind.
    // Supports future scheduler queries: "find all active accounts for provider X".
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_provider_accounts_provider_kind
         ON provider_accounts(provider_kind);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 5,
        message: format!("failed to create provider_kind index: {}", e),
        source: Some(e),
    })?;

    // Index for efficient lookup by status.
    // Supports future scheduler queries: "find all active accounts".
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_provider_accounts_status
         ON provider_accounts(status);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 5,
        message: format!("failed to create status index: {}", e),
        source: Some(e),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::persistence::schema_version::CURRENT_SCHEMA_VERSION;
    use crate::persistence::SqliteStore;

    #[test]
    fn v0005_creates_provider_accounts_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='provider_accounts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "provider_accounts must exist after v0005");
    }

    #[test]
    fn v0005_creates_provider_kind_index() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_provider_accounts_provider_kind'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "provider_kind index must exist after v0005");
    }

    #[test]
    fn v0005_creates_status_index() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_provider_accounts_status'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "status index must exist after v0005");
    }

    #[test]
    fn current_schema_version_is_at_least_5() {
        const { assert!(CURRENT_SCHEMA_VERSION >= 5) };
    }

    #[test]
    fn fresh_db_migrates_to_current() {
        let store = SqliteStore::open_in_memory().unwrap();
        let v = store.schema_version().unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn provider_accounts_insert_and_query() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();

        conn.execute(
            "INSERT INTO provider_accounts (id, provider_kind, label, identity_hint, auth_profile_id, status, max_concurrent_sessions, created_at, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                "PA-CLAUDE-A",
                "claude",
                "Claude A",
                Some("user-a@example.com"),
                "auth-claude-a",
                "ACTIVE",
                2i64,
                1725100800i64,
                1725100800i64,
                1i64,
            ],
        )
        .unwrap();

        let label: String = conn
            .query_row(
                "SELECT label FROM provider_accounts WHERE id = ?1",
                ["PA-CLAUDE-A"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(label, "Claude A");
    }

    #[test]
    fn provider_accounts_no_provider_specific_columns() {
        // Verify the table schema does not contain provider-specific columns.
        // This is an architectural invariant: generic model only.
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();

        let mut stmt = conn.prepare("PRAGMA table_info(provider_accounts)").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(!columns.iter().any(|c| c.contains("google")), "no google-specific columns");
        assert!(!columns.iter().any(|c| c.contains("claude")), "no claude-specific columns (provider_kind is generic)");
        assert!(columns.contains(&"auth_profile_id".to_string()), "must have auth_profile_id");
        assert!(columns.contains(&"identity_hint".to_string()), "must have identity_hint");
        assert!(columns.contains(&"version".to_string()), "must have version for OCC");
    }
}