//! Mega Brain V0 — Migration v0001: Initial Schema
//!
//! Creates all V0 tables, indexes, and constraints as specified in Topic 03.
//! This migration is transactable and idempotent (guarded by schema version).

use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

pub fn apply(tx: &Transaction) -> Result<(), PersistenceError> {
    let conn = tx.conn();

    // ── projects ────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE projects (
            id                  TEXT PRIMARY KEY,
            name                TEXT NOT NULL,
            repository_identity TEXT NOT NULL,
            canonical_path      TEXT NOT NULL,
            target_branch       TEXT NOT NULL,
            created_at          INTEGER NOT NULL,
            updated_at          INTEGER NOT NULL,
            version             INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;

    // ── runs ────────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE runs (
            id              TEXT PRIMARY KEY,
            project_id      TEXT NOT NULL REFERENCES projects(id),
            objective       TEXT NOT NULL,
            status          TEXT NOT NULL,
            version         INTEGER NOT NULL CHECK (version >= 1),
            created_at      INTEGER NOT NULL,
            updated_at      INTEGER NOT NULL,
            started_at      INTEGER,
            finished_at     INTEGER,
            terminal_reason TEXT
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_runs_project_status ON runs(project_id, status);")?;

    // ── tasks ───────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE tasks (
            id                   TEXT PRIMARY KEY,
            run_id               TEXT NOT NULL REFERENCES runs(id),
            title                TEXT NOT NULL,
            objective            TEXT NOT NULL,
            status               TEXT NOT NULL,
            priority             INTEGER NOT NULL DEFAULT 0,
            version              INTEGER NOT NULL CHECK (version >= 1),
            base_commit          TEXT,
            expected_write_scope TEXT,
            acceptance_contract  TEXT,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL,
            started_at           INTEGER,
            finished_at          INTEGER
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_tasks_run_status ON tasks(run_id, status);")?;

    // ── task_dependencies ───────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE task_dependencies (
            task_id          TEXT NOT NULL REFERENCES tasks(id),
            depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
            reason           TEXT,
            created_at       INTEGER NOT NULL,
            PRIMARY KEY (task_id, depends_on_task_id),
            CHECK (task_id != depends_on_task_id)
        );",
    )?;

    // ── task_attempts ───────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE task_attempts (
            id               TEXT PRIMARY KEY,
            task_id          TEXT NOT NULL REFERENCES tasks(id),
            attempt_number   INTEGER NOT NULL CHECK (attempt_number > 0),
            agent_id         TEXT,
            workspace_id     TEXT,
            status           TEXT NOT NULL,
            version          INTEGER NOT NULL CHECK (version >= 1),
            base_commit      TEXT,
            candidate_commit TEXT,
            started_at       INTEGER,
            finished_at      INTEGER,
            created_at       INTEGER NOT NULL,
            updated_at       INTEGER NOT NULL,
            UNIQUE (task_id, attempt_number)
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_attempts_task ON task_attempts(task_id);")?;
    conn.execute_batch("CREATE INDEX idx_attempts_status ON task_attempts(status);")?;

    // ── agents ──────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE agents (
            id                 TEXT PRIMARY KEY,
            name               TEXT NOT NULL,
            provider           TEXT NOT NULL,
            adapter_kind       TEXT NOT NULL,
            status             TEXT NOT NULL,
            max_parallel_tasks INTEGER NOT NULL CHECK (max_parallel_tasks >= 1),
            created_at         INTEGER NOT NULL,
            updated_at         INTEGER NOT NULL,
            version            INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;

    // ── agent_capabilities ──────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE agent_capabilities (
            agent_id   TEXT NOT NULL REFERENCES agents(id),
            capability TEXT NOT NULL,
            confidence REAL,
            PRIMARY KEY (agent_id, capability)
        );",
    )?;

    // ── agent_sessions ──────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE agent_sessions (
            id                  TEXT PRIMARY KEY,
            agent_id            TEXT NOT NULL REFERENCES agents(id),
            attempt_id          TEXT REFERENCES task_attempts(id),
            workspace_id        TEXT,
            provider_session_id TEXT,
            status              TEXT NOT NULL,
            process_identity    TEXT,
            started_at          INTEGER,
            last_observed_at    INTEGER,
            ended_at            INTEGER,
            version             INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;

    // ── workspaces ──────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE workspaces (
            id          TEXT PRIMARY KEY,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            attempt_id  TEXT REFERENCES task_attempts(id),
            mode        TEXT NOT NULL,
            path        TEXT NOT NULL,
            branch_name TEXT,
            base_commit TEXT,
            status      TEXT NOT NULL,
            version     INTEGER NOT NULL CHECK (version >= 1),
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL,
            removed_at  INTEGER
        );",
    )?;

    // ── leases ──────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE leases (
            id                 TEXT PRIMARY KEY,
            resource_type      TEXT NOT NULL,
            resource_id        TEXT NOT NULL,
            attempt_id         TEXT REFERENCES task_attempts(id),
            lease_token_hash   TEXT NOT NULL,
            fencing_token      INTEGER NOT NULL CHECK (fencing_token >= 0),
            issued_at          INTEGER NOT NULL,
            heartbeat_at       INTEGER,
            expires_at         INTEGER NOT NULL,
            revoked_at         INTEGER,
            version            INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_leases_resource ON leases(resource_type, resource_id);")?;
    conn.execute_batch("CREATE INDEX idx_leases_expires ON leases(expires_at);")?;

    // ── commands ────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE commands (
            command_id     TEXT PRIMARY KEY,
            command_type   TEXT NOT NULL,
            payload_hash   TEXT NOT NULL,
            status         TEXT NOT NULL,
            result_payload TEXT,
            error_payload  TEXT,
            created_at     INTEGER NOT NULL,
            completed_at   INTEGER
        );",
    )?;

    // ── events ──────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE events (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type        TEXT NOT NULL,
            aggregate_type    TEXT NOT NULL,
            aggregate_id      TEXT NOT NULL,
            aggregate_version INTEGER NOT NULL,
            payload           TEXT NOT NULL,
            created_at        INTEGER NOT NULL,
            correlation_id    TEXT,
            causation_id      TEXT,
            command_id        TEXT REFERENCES commands(command_id)
        );",
    )?;
    conn.execute_batch(
        "CREATE INDEX idx_events_aggregate ON events(aggregate_type, aggregate_id);",
    )?;
    conn.execute_batch("CREATE INDEX idx_events_command ON events(command_id);")?;

    // ── outbox ──────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE outbox (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id       INTEGER NOT NULL REFERENCES events(id),
            created_at     INTEGER NOT NULL,
            published_at   INTEGER,
            attempt_count  INTEGER NOT NULL DEFAULT 0,
            last_error     TEXT
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_outbox_pending ON outbox(published_at);")?;

    // ── operations ──────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE operations (
            id                 TEXT PRIMARY KEY,
            operation_type     TEXT NOT NULL,
            status             TEXT NOT NULL,
            project_id         TEXT REFERENCES projects(id),
            task_id            TEXT REFERENCES tasks(id),
            attempt_id         TEXT REFERENCES task_attempts(id),
            command_id         TEXT REFERENCES commands(command_id),
            preconditions      TEXT,
            input_payload      TEXT,
            result_payload     TEXT,
            external_reference TEXT,
            prepared_at        INTEGER,
            started_at         INTEGER,
            committed_at       INTEGER,
            failed_at          INTEGER,
            last_error         TEXT,
            version            INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_operations_status ON operations(status);")?;

    // ── artifacts ───────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE artifacts (
            id         TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id),
            run_id     TEXT REFERENCES runs(id),
            task_id    TEXT REFERENCES tasks(id),
            attempt_id TEXT REFERENCES task_attempts(id),
            type       TEXT NOT NULL,
            sha256     TEXT NOT NULL,
            path       TEXT NOT NULL,
            size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
            metadata   TEXT,
            created_at INTEGER NOT NULL
        );",
    )?;

    // ── context_packs ───────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE context_packs (
            id             TEXT PRIMARY KEY,
            task_id        TEXT NOT NULL REFERENCES tasks(id),
            attempt_id     TEXT REFERENCES task_attempts(id),
            schema_version TEXT NOT NULL,
            sha256         TEXT NOT NULL,
            payload        TEXT NOT NULL,
            created_at     INTEGER NOT NULL
        );",
    )?;

    // ── reviews ─────────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE reviews (
            id                 TEXT PRIMARY KEY,
            task_id            TEXT NOT NULL REFERENCES tasks(id),
            attempt_id         TEXT NOT NULL REFERENCES task_attempts(id),
            candidate_commit   TEXT NOT NULL,
            reviewer_agent_id  TEXT REFERENCES agents(id),
            status             TEXT NOT NULL,
            verdict            TEXT,
            evidence           TEXT,
            created_at         INTEGER NOT NULL,
            updated_at         INTEGER NOT NULL,
            version            INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;
    conn.execute_batch("CREATE INDEX idx_reviews_task_status ON reviews(task_id, status);")?;

    // ── merge_queue ─────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE merge_queue (
            id                     TEXT PRIMARY KEY,
            project_id             TEXT NOT NULL REFERENCES projects(id),
            task_id                TEXT NOT NULL REFERENCES tasks(id),
            candidate_commit       TEXT NOT NULL,
            expected_target_commit TEXT NOT NULL,
            status                 TEXT NOT NULL,
            priority               INTEGER NOT NULL DEFAULT 0,
            enqueued_at            INTEGER NOT NULL,
            started_at             INTEGER,
            finished_at            INTEGER,
            version                INTEGER NOT NULL CHECK (version >= 1)
        );",
    )?;
    conn.execute_batch(
        "CREATE INDEX idx_merge_queue_order ON merge_queue(project_id, status, priority, enqueued_at);",
    )?;

    // ── decisions ───────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE decisions (
            id                  TEXT PRIMARY KEY,
            project_id          TEXT NOT NULL REFERENCES projects(id),
            run_id              TEXT NOT NULL REFERENCES runs(id),
            task_id             TEXT REFERENCES tasks(id),
            key                 TEXT NOT NULL,
            value               TEXT NOT NULL,
            status              TEXT NOT NULL,
            revision            INTEGER NOT NULL DEFAULT 1,
            source_artifact_id  TEXT REFERENCES artifacts(id),
            created_at          INTEGER NOT NULL,
            updated_at          INTEGER NOT NULL
        );",
    )?;

    // ── resource_claims ─────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE resource_claims (
            id               TEXT PRIMARY KEY,
            task_id          TEXT NOT NULL REFERENCES tasks(id),
            attempt_id       TEXT REFERENCES task_attempts(id),
            resource_type    TEXT NOT NULL,
            resource_pattern TEXT NOT NULL,
            mode             TEXT NOT NULL,
            created_at       INTEGER NOT NULL,
            released_at      INTEGER
        );",
    )?;

    // ── file_touches ────────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE file_touches (
            task_id       TEXT NOT NULL REFERENCES tasks(id),
            attempt_id    TEXT NOT NULL REFERENCES task_attempts(id),
            path          TEXT NOT NULL,
            change_kind   TEXT NOT NULL,
            first_seen_at INTEGER NOT NULL,
            last_seen_at  INTEGER NOT NULL,
            PRIMARY KEY (task_id, attempt_id, path)
        );",
    )?;
    conn.execute_batch(
        "CREATE INDEX idx_file_touches_attempt ON file_touches(task_id, attempt_id);",
    )?;

    Ok(())
}
