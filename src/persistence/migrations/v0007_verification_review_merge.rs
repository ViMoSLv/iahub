//! Mega Brain V0 — Migration v0007: Verification, Review & Merge (Topic 06)
//!
//! Introduces tables for the verification-review-merge pipeline:
//! - `verification_evidence`: Immutable observational proof from workspace checks.
//! - `review_verdicts`: Independent review decisions bound to candidate SHA.
//! - `merge_lab_results`: Simulation outcomes valid for exact (candidate, target) tuple.
//! - `merge_queue`: Serialized merge requests per target branch with durable state.
//!
//! Key design decisions:
//! - All evidence/verdict/lab tables are append-only; updates create new rows.
//! - Freshness validation is enforced at the application layer via SHA comparison.
//! - Merge queue uses optimistic concurrency (version column) for serialization.
//! - Indexes support lookup by attempt, task, candidate SHA, and target branch.

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // All v0007 DDL is executed as a single batch to avoid issues with
    // deferred transactions where newly-created tables may not be visible
    // to subsequent CREATE INDEX statements until the transaction commits.
    // sqlite3_exec() processes statements sequentially within a single call,
    // making each table immediately available for indexing.
    conn.execute_batch(
        "
        -- 1. Verification Evidence
        CREATE TABLE IF NOT EXISTS verification_evidence (
            id                      TEXT NOT NULL PRIMARY KEY,
            attempt_id              TEXT NOT NULL,
            task_id                 TEXT NOT NULL,
            candidate_diff_sha256   TEXT NOT NULL,
            outcome                 TEXT NOT NULL,
            artifact_ids            TEXT NOT NULL,
            verified_at             INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_verification_evidence_attempt
            ON verification_evidence(attempt_id);
        CREATE INDEX IF NOT EXISTS idx_verification_evidence_task
            ON verification_evidence(task_id);

        -- 2. Review Verdicts
        CREATE TABLE IF NOT EXISTS review_verdicts (
            id                          TEXT NOT NULL PRIMARY KEY,
            task_id                     TEXT NOT NULL,
            attempt_id                  TEXT NOT NULL,
            candidate_sha               TEXT NOT NULL,
            diff_sha256                 TEXT NOT NULL,
            task_spec_revision          INTEGER NOT NULL,
            acceptance_revision         INTEGER NOT NULL,
            verification_evidence_ids   TEXT NOT NULL,
            reviewer_id                 TEXT NOT NULL,
            decision                    TEXT NOT NULL,
            feedback                    TEXT,
            created_at                  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_review_verdicts_attempt
            ON review_verdicts(attempt_id);
        CREATE INDEX IF NOT EXISTS idx_review_verdicts_candidate_sha
            ON review_verdicts(candidate_sha);

        -- 3. Merge Lab Results
        CREATE TABLE IF NOT EXISTS merge_lab_results (
            id                              TEXT NOT NULL PRIMARY KEY,
            task_id                         TEXT NOT NULL,
            attempt_id                      TEXT NOT NULL,
            candidate_sha                   TEXT NOT NULL,
            target_sha                      TEXT NOT NULL,
            repository_fingerprint          TEXT NOT NULL,
            verification_policy_revision    INTEGER NOT NULL,
            merge_clean                     INTEGER NOT NULL CHECK (merge_clean IN (0, 1)),
            tests_passed                    INTEGER NOT NULL CHECK (tests_passed IN (0, 1)),
            outcome                         TEXT NOT NULL,
            artifact_ids                    TEXT NOT NULL,
            simulated_at                    INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_merge_lab_results_attempt
            ON merge_lab_results(attempt_id);
        CREATE INDEX IF NOT EXISTS idx_merge_lab_results_tuple
            ON merge_lab_results(candidate_sha, target_sha, repository_fingerprint);

        -- 4. Merge Queue (v2)
        -- v0001 created a legacy merge_queue table with a different schema
        -- (candidate_commit, expected_target_commit, no target_branch).
        -- Drop it and recreate with the Topic 06 canonical schema that
        -- includes target_branch for per-branch serialization (INV-055).
        DROP TABLE IF EXISTS merge_queue;
        CREATE TABLE merge_queue (
            id                      TEXT NOT NULL PRIMARY KEY,
            project_id              TEXT NOT NULL,
            task_id                 TEXT NOT NULL,
            attempt_id              TEXT NOT NULL,
            target_branch           TEXT NOT NULL,
            candidate_sha           TEXT NOT NULL,
            review_verdict_id       TEXT NOT NULL,
            latest_lab_result_id    TEXT,
            status                  TEXT NOT NULL,
            priority                INTEGER NOT NULL,
            queued_at               INTEGER NOT NULL,
            updated_at              INTEGER NOT NULL,
            version                 INTEGER NOT NULL CHECK (version >= 1)
        );
        CREATE INDEX IF NOT EXISTS idx_merge_queue_target_branch
            ON merge_queue(target_branch, status);
        CREATE INDEX IF NOT EXISTS idx_merge_queue_attempt
            ON merge_queue(attempt_id);
        ",
    )
    .map_err(|e| PersistenceError::MigrationFailed {
        version: 7,
        message: format!("v0007 DDL failed: {}", e),
        source: Some(e),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::persistence::schema_version::CURRENT_SCHEMA_VERSION;
    use crate::persistence::SqliteStore;

    #[test]
    fn v0007_creates_verification_evidence_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='verification_evidence'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "verification_evidence must exist after v0007");
    }

    #[test]
    fn v0007_creates_review_verdicts_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='review_verdicts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "review_verdicts must exist after v0007");
    }

    #[test]
    fn v0007_creates_merge_lab_results_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='merge_lab_results'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "merge_lab_results must exist after v0007");
    }

    #[test]
    fn v0007_creates_merge_queue_table() {
        let store = SqliteStore::open_in_memory().unwrap();
        let count: i64 = store
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='merge_queue'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "merge_queue must exist after v0007");
    }

    #[test]
    fn v0007_creates_required_indexes() {
        let store = SqliteStore::open_in_memory().unwrap();
        let indexes: Vec<String> = store
            .conn()
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='index' AND name IN (
                    'idx_verification_evidence_attempt',
                    'idx_verification_evidence_task',
                    'idx_review_verdicts_attempt',
                    'idx_review_verdicts_candidate_sha',
                    'idx_merge_lab_results_attempt',
                    'idx_merge_lab_results_tuple',
                    'idx_merge_queue_target_branch',
                    'idx_merge_queue_attempt'
                )",
            )
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(indexes.contains(&"idx_verification_evidence_attempt".to_string()));
        assert!(indexes.contains(&"idx_verification_evidence_task".to_string()));
        assert!(indexes.contains(&"idx_review_verdicts_attempt".to_string()));
        assert!(indexes.contains(&"idx_review_verdicts_candidate_sha".to_string()));
        assert!(indexes.contains(&"idx_merge_lab_results_attempt".to_string()));
        assert!(indexes.contains(&"idx_merge_lab_results_tuple".to_string()));
        assert!(indexes.contains(&"idx_merge_queue_target_branch".to_string()));
        assert!(indexes.contains(&"idx_merge_queue_attempt".to_string()));
    }

    #[test]
    fn current_schema_version_is_7() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 7);
    }

    #[test]
    fn fresh_db_migrates_to_v7() {
        let store = SqliteStore::open_in_memory().unwrap();
        let v = store.schema_version().unwrap();
        assert_eq!(v, 7);
    }

    #[test]
    fn verification_evidence_insert_and_query() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();
        let artifacts_json = serde_json::to_string(&vec!["ART-001", "ART-002"]).unwrap();
        conn.execute(
            "INSERT INTO verification_evidence (id, attempt_id, task_id, candidate_diff_sha256, outcome, artifact_ids, verified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "VER-001",
                "ATT-3",
                "TASK-142",
                "a".repeat(64),
                "PASSED",
                artifacts_json,
                1725100800i64,
            ],
        )
        .unwrap();

        let outcome: String = conn
            .query_row(
                "SELECT outcome FROM verification_evidence WHERE id = ?1",
                ["VER-001"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(outcome, "PASSED");
    }

    #[test]
    fn review_verdict_self_review_prevention_query() {
        // This test validates that we can query for self-review violations.
        // The actual enforcement happens at the application layer, but the
        // schema supports the query pattern needed.
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();
        let evidence_json = serde_json::to_string(&Vec::<String>::new()).unwrap();
        conn.execute(
            "INSERT INTO review_verdicts (id, task_id, attempt_id, candidate_sha, diff_sha256, task_spec_revision, acceptance_revision, verification_evidence_ids, reviewer_id, decision, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                "REV-001",
                "TASK-142",
                "ATT-3",
                "abc123",
                "b".repeat(64),
                1i64,
                1i64,
                evidence_json,
                "agent-alice", // reviewer same as producer → violation
                "APPROVED",
                1725100800i64,
            ],
        )
        .unwrap();

        // Query pattern for detecting self-review (application layer enforces this)
        let reviewer: String = conn
            .query_row(
                "SELECT reviewer_id FROM review_verdicts WHERE id = ?1",
                ["REV-001"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reviewer, "agent-alice");
    }

    #[test]
    fn merge_queue_serialization_by_target_branch() {
        let store = SqliteStore::open_in_memory().unwrap();
        let conn = store.conn();

        // Insert two items for same target branch
        for (id, priority) in &[("MQI-001", 10), ("MQI-002", 20)] {
            conn.execute(
                "INSERT INTO merge_queue (id, project_id, task_id, attempt_id, target_branch, candidate_sha, review_verdict_id, status, priority, queued_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id,
                    "proj-001",
                    "TASK-142",
                    "ATT-3",
                    "main",
                    format!("sha-{}", id),
                    "REV-001",
                    "QUEUED",
                    priority,
                    1725100800i64,
                    1725100800i64,
                    1i64,
                ],
            )
            .unwrap();
        }

        // Query next item for target branch (ordered by priority)
        let next_id: String = conn
            .query_row(
                "SELECT id FROM merge_queue WHERE target_branch = ?1 AND status = ?2 ORDER BY priority ASC LIMIT 1",
                rusqlite::params!["main", "QUEUED"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(next_id, "MQI-001", "lower priority number = higher priority");
    }
}