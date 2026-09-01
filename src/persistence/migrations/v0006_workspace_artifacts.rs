//! Mega Brain V0 — Migration v0006: Workspaces & Artifacts (Topic 05)
//!
//! Introduces tables for workspace lifecycle tracking and content-addressable
//! artifact metadata. This is domain/architecture preparation; no Git worktree
//! creation or filesystem I/O is implemented here.
//!
//! Key design decisions:
//! - `workspaces` tracks lifecycle state, ownership (attempt_id), and cleanup gates.
//! - `artifacts` stores metadata + SHA-256 hash; large content stays on disk.
//! - `write_capabilities` persists the explicit authority grants per attempt.
//! - All tables follow existing OCC patterns where applicable.
//! - Forward-only migration; v0001–v0005 are not modified.

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // ── 1. Workspaces table ───────────────────────────────────────────────────
    // v0001 created a legacy workspaces table with a different schema
    // (project_id, mode, path, branch_name, base_commit, removed_at).
    // Drop it and recreate with the Topic 05 canonical schema that includes
    // task_id, repository_fingerprint, and workspace_mode for proper lifecycle tracking.
    conn.execute_batch(
        "DROP TABLE IF EXISTS workspaces;
        CREATE TABLE workspaces (
            id                    TEXT NOT NULL PRIMARY KEY,
            attempt_id            TEXT NOT NULL,
            task_id               TEXT NOT NULL,
            repository_fingerprint TEXT NOT NULL,
            workspace_mode        TEXT NOT NULL,
            root_path             TEXT NOT NULL,
            status                TEXT NOT NULL,
            created_at            INTEGER NOT NULL,
            updated_at            INTEGER NOT NULL,
            version               INTEGER NOT NULL CHECK (version >= 1)
        );",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create workspaces: {}", e),
        source: Some(e),
    })?;

    // Index for lookup by attempt (primary ownership relationship)
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_workspaces_attempt_id
         ON workspaces(attempt_id);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create workspaces attempt index: {}", e),
        source: Some(e),
    })?;

    // Index for cleanup queries (find orphaned/broken workspaces)
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_workspaces_status
         ON workspaces(status);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create workspaces status index: {}", e),
        source: Some(e),
    })?;

    // ── 2. Write Capabilities table ───────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS write_capabilities (
            id              TEXT NOT NULL PRIMARY KEY,
            task_id         TEXT NOT NULL,
            attempt_id      TEXT NOT NULL,
            fencing_token   INTEGER NOT NULL,
            allow_patterns  TEXT NOT NULL,
            deny_patterns   TEXT NOT NULL,
            expires_at      TEXT NOT NULL,
            created_at      INTEGER NOT NULL
        );",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create write_capabilities: {}", e),
        source: Some(e),
    })?;

    // Index for capability lookup by attempt
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_write_capabilities_attempt
         ON write_capabilities(attempt_id);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create capabilities attempt index: {}", e),
        source: Some(e),
    })?;

    // ── 3. Artifacts table ─────────────────────────────────────────────────────
    // v0001 created a legacy artifacts table with different columns
    // (type, path, metadata; no artifact_type, content_path, schema_version, producer).
    // Drop and recreate with the Topic 05 canonical schema.
    conn.execute_batch(
        "DROP TABLE IF EXISTS artifacts;
        CREATE TABLE artifacts (
            id              TEXT NOT NULL PRIMARY KEY,
            artifact_type   TEXT NOT NULL,
            project_id      TEXT NOT NULL,
            run_id          TEXT,
            task_id         TEXT,
            attempt_id      TEXT,
            content_path    TEXT NOT NULL,
            sha256          TEXT NOT NULL,
            size_bytes      INTEGER NOT NULL CHECK (size_bytes >= 0),
            schema_version  INTEGER NOT NULL CHECK (schema_version >= 1),
            producer        TEXT NOT NULL,
            created_at      INTEGER NOT NULL
        );",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create artifacts: {}", e),
        source: Some(e),
    })?;

    // Index for artifact lookup by attempt
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_artifacts_attempt
         ON artifacts(attempt_id);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create artifacts attempt index: {}", e),
        source: Some(e),
    })?;

    // Index for content-addressable lookup (find duplicate artifacts)
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_artifacts_sha256
         ON artifacts(sha256);",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 6,
        message: format!("failed to create artifacts sha256 index: {}", e),
        source: Some(e),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::persistence::schema_version::CURRENT_SCHEMA_VERSION;
    use crate::persistence::SqliteStore;

    #[test]
    fn v0006_creates_workspaces_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='workspaces'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "workspaces must exist after v0006");
    }

    #[test]
    fn v0006_creates_write_capabilities_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='write_capabilities'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "write_capabilities must exist after v0006");
    }

    #[test]
    fn v0006_creates_artifacts_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifacts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "artifacts must exist after v0006");
    }

    #[test]
    fn v0006_creates_required_indexes() {
        let store = SqliteStore::open_in_memory().unwrap();
        let indexes: Vec<String> = store
            .conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%_v6%' OR name IN ('idx_workspaces_attempt_id', 'idx_workspaces_status', 'idx_write_capabilities_attempt', 'idx_artifacts_attempt', 'idx_artifacts_sha256')")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(indexes.contains(&"idx_workspaces_attempt_id".to_string()));
        assert!(indexes.contains(&"idx_workspaces_status".to_string()));
        assert!(indexes.contains(&"idx_write_capabilities_attempt".to_string()));
        assert!(indexes.contains(&"idx_artifacts_attempt".to_string()));
        assert!(indexes.contains(&"idx_artifacts_sha256".to_string()));
    }

    #[test]
    fn current_schema_version_is_at_least_6() {
        const { assert!(CURRENT_SCHEMA_VERSION >= 6) };
    }

    #[test]
    fn fresh_db_migrates_to_current() {
        let store = SqliteStore::open_in_memory().unwrap();
        let v = store.schema_version().unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn workspaces_insert_and_query() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();
        conn.execute(
            "INSERT INTO workspaces (id, attempt_id, task_id, repository_fingerprint, workspace_mode, root_path, status, created_at, updated_at, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                "WS-TASK142-ATT01",
                "ATT-01",
                "TASK-142",
                "fp-abc123",
                "GIT_WORKTREE",
                "/tmp/workspaces/TASK-142-ATT-01",
                "READY",
                1725100800i64,
                1725100800i64,
                1i64,
            ],
        )
        .unwrap();

        let mode: String = conn
            .query_row(
                "SELECT workspace_mode FROM workspaces WHERE id = ?1",
                ["WS-TASK142-ATT01"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mode, "GIT_WORKTREE");
    }

    #[test]
    fn artifacts_content_addressable_lookup() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();
        let hash = "a".repeat(64);
        conn.execute(
            "INSERT INTO artifacts (id, artifact_type, project_id, content_path, sha256, size_bytes, schema_version, producer, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "ART-001",
                "DIFF",
                "proj-001",
                "artifacts/ART-001.patch",
                &hash,
                4096i64,
                1i64,
                "agent-claude-a",
                1725100800i64,
            ],
        )
        .unwrap();

        let found_id: String = conn
            .query_row(
                "SELECT id FROM artifacts WHERE sha256 = ?1",
                [&hash],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(found_id, "ART-001");
    }
}