//! Mega Brain V0 — Command Store
//!
//! Persistence operations for the `commands` table. All access goes through
//! this module; no raw SQL in handlers or engine logic.

use rusqlite::params;
use rusqlite::OptionalExtension;

use crate::domain::{CommandId, Timestamp};
use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

/// Row read from the `commands` table.
#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub command_id: String,
    pub command_type: String,
    pub payload_hash: String,
    pub status: String,
    pub result_payload: Option<String>,
    pub error_payload: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

/// Insert a new command record with RECEIVED status.
/// Fails with constraint violation if command_id already exists.
pub fn insert_command(
    tx: &Transaction,
    command_id: &CommandId,
    command_type: &str,
    payload_hash: &str,
    issued_at: &Timestamp,
) -> Result<(), PersistenceError> {
    let ts: i64 = issued_at.0.parse().unwrap_or(0); // Timestamp is opaque string; parse as millis
    tx.conn()
        .execute(
            "INSERT INTO commands (command_id, command_type, payload_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![command_id.0, command_type, payload_hash, "RECEIVED", ts],
        )
        .map_err(PersistenceError::from)?;
    Ok(())
}

/// Look up an existing command by ID. Returns None if not found.
pub fn find_by_id(
    tx: &Transaction,
    command_id: &CommandId,
) -> Result<Option<CommandRecord>, PersistenceError> {
    let mut stmt = tx
        .conn()
        .prepare(
            "SELECT command_id, command_type, payload_hash, status,
                    result_payload, error_payload, created_at, completed_at
             FROM commands WHERE command_id = ?1",
        )
        .map_err(|e| PersistenceError::Transaction { source: e })?;

    let row = stmt
        .query_row(params![command_id.0], |r| {
            Ok(CommandRecord {
                command_id: r.get(0)?,
                command_type: r.get(1)?,
                payload_hash: r.get(2)?,
                status: r.get(3)?,
                result_payload: r.get(4)?,
                error_payload: r.get(5)?,
                created_at: r.get(6)?,
                completed_at: r.get(7)?,
            })
        })
        .optional()
        .map_err(|e| PersistenceError::Transaction { source: e })?;

    Ok(row)
}

/// Mark a command as SUCCEEDED with its result payload.
pub fn complete_success(
    tx: &Transaction,
    command_id: &CommandId,
    result_payload: &str,
    completed_at: &Timestamp,
) -> Result<(), PersistenceError> {
    let ts: i64 = completed_at.0.parse().unwrap_or(0);
    tx.conn()
        .execute(
            "UPDATE commands SET status = ?1, result_payload = ?2, completed_at = ?3
             WHERE command_id = ?4",
            params!["SUCCEEDED", result_payload, ts, command_id.0],
        )
        .map_err(|e| PersistenceError::Transaction { source: e })?;
    Ok(())
}

/// Mark a command as FAILED with its error payload.
pub fn complete_failure(
    tx: &Transaction,
    command_id: &CommandId,
    error_payload: &str,
    completed_at: &Timestamp,
) -> Result<(), PersistenceError> {
    let ts: i64 = completed_at.0.parse().unwrap_or(0);
    tx.conn()
        .execute(
            "UPDATE commands SET status = ?1, error_payload = ?2, completed_at = ?3
             WHERE command_id = ?4",
            params!["FAILED", error_payload, ts, command_id.0],
        )
        .map_err(|e| PersistenceError::Transaction { source: e })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    #[test]
    fn insert_and_find_command() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-001");
        let ts = Timestamp("1000".to_string());

        insert_command(&tx, &cmd_id, "CreateProject", "hash-abc", &ts).unwrap();

        let record = find_by_id(&tx, &cmd_id).unwrap().expect("must exist");
        assert_eq!(record.command_id, "cmd-001");
        assert_eq!(record.command_type, "CreateProject");
        assert_eq!(record.payload_hash, "hash-abc");
        assert_eq!(record.status, "RECEIVED");
        assert!(record.result_payload.is_none());
        assert!(record.completed_at.is_none());

        tx.commit().unwrap();
    }

    #[test]
    fn find_missing_returns_none() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let result = find_by_id(&tx, &CommandId::from("nonexistent")).unwrap();
        assert!(result.is_none());

        tx.commit().unwrap();
    }

    #[test]
    fn complete_success_updates_record() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-002");
        let ts = Timestamp("1000".to_string());
        insert_command(&tx, &cmd_id, "CreateRun", "hash-def", &ts).unwrap();

        let done_ts = Timestamp("2000".to_string());
        complete_success(&tx, &cmd_id, "{\"id\":\"r-1\"}", &done_ts).unwrap();

        let record = find_by_id(&tx, &cmd_id).unwrap().expect("must exist");
        assert_eq!(record.status, "SUCCEEDED");
        assert_eq!(record.result_payload.as_deref(), Some("{\"id\":\"r-1\"}"));
        assert_eq!(record.completed_at, Some(2000));

        tx.commit().unwrap();
    }

    #[test]
    fn complete_failure_updates_record() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-003");
        let ts = Timestamp("1000".to_string());
        insert_command(&tx, &cmd_id, "CreateTask", "hash-ghi", &ts).unwrap();

        let done_ts = Timestamp("3000".to_string());
        complete_failure(&tx, &cmd_id, "{\"error\":\"not_found\"}", &done_ts).unwrap();

        let record = find_by_id(&tx, &cmd_id).unwrap().expect("must exist");
        assert_eq!(record.status, "FAILED");
        assert_eq!(
            record.error_payload.as_deref(),
            Some("{\"error\":\"not_found\"}")
        );
        assert!(record.result_payload.is_none());

        tx.commit().unwrap();
    }

    #[test]
    fn duplicate_insert_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-dup");
        let ts = Timestamp("1000".to_string());
        insert_command(&tx, &cmd_id, "CreateProject", "hash-aaa", &ts).unwrap();

        let result = insert_command(&tx, &cmd_id, "CreateRun", "hash-bbb", &ts);
        assert!(result.is_err(), "duplicate command_id must be rejected");

        tx.rollback().unwrap();
    }
}
