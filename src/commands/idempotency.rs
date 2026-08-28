//! Mega Brain V0 — Idempotency Check
//!
//! Before executing any command, the engine checks whether a command with
//! the same `command_id` already exists:
//!
//! - Not found → proceed with execution
//! - Found + same payload_hash + SUCCEEDED → return cached result (no re-execution)
//! - Found + same payload_hash + FAILED → return cached failure (deterministic retry)
//! - Found + different payload_hash → reject with DuplicateCommandMismatch
//!
//! This module is pure logic over store records; it performs no mutations.

use super::error::CommandError;
use super::store;
use super::types::{CommandResult, CommandStatus};
use crate::domain::CommandId;
use crate::persistence::transaction::Transaction;

/// Outcome of an idempotency check before command execution.
#[derive(Debug)]
pub enum IdempotencyCheck {
    /// No prior record exists; safe to execute.
    New,
    /// Prior record exists with matching hash and terminal status; replay result.
    Replay(CommandResult),
}

/// Check idempotency for a command. Returns either `New` (proceed) or
/// `Replay` (return cached result). Returns `Err` on payload mismatch.
pub fn check_idempotency(
    tx: &Transaction,
    command_id: &CommandId,
    payload_hash: &str,
) -> Result<IdempotencyCheck, CommandError> {
    let record = match store::find_by_id(tx, command_id)? {
        Some(r) => r,
        None => return Ok(IdempotencyCheck::New),
    };

    // Payload mismatch: same command_id, different intent
    if record.payload_hash != payload_hash {
        return Err(CommandError::DuplicateCommandMismatch {
            command_id: command_id.clone(),
        });
    }

    // Same payload: return cached result if terminal
    match record.status.as_str() {
        "SUCCEEDED" => Ok(IdempotencyCheck::Replay(CommandResult {
            command_id: command_id.clone(),
            status: CommandStatus::Succeeded,
            result_payload: record.result_payload,
            error_payload: None,
        })),
        "FAILED" => Ok(IdempotencyCheck::Replay(CommandResult {
            command_id: command_id.clone(),
            status: CommandStatus::Failed,
            result_payload: None,
            error_payload: record.error_payload,
        })),
        // RECEIVED but not yet completed: treat as in-flight.
        // For V0 single-writer this should not happen, but fail closed.
        _ => Err(CommandError::InvalidCommand {
            detail: format!(
                "command {} exists with non-terminal status {}",
                command_id, record.status
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::store;
    use crate::domain::Timestamp;
    use crate::persistence::SqliteStore;

    #[test]
    fn new_command_returns_new() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let result = check_idempotency(&tx, &CommandId::from("cmd-new"), "hash-abc").unwrap();
        assert!(matches!(result, IdempotencyCheck::New));

        tx.commit().unwrap();
    }

    #[test]
    fn succeeded_command_replays_result() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-ok");
        let ts = Timestamp("1000".to_string());
        store::insert_command(&tx, &cmd_id, "CreateProject", "hash-abc", &ts).unwrap();
        store::complete_success(&tx, &cmd_id, "{\"id\":\"p-1\"}", &ts).unwrap();

        let result = check_idempotency(&tx, &cmd_id, "hash-abc").unwrap();
        match result {
            IdempotencyCheck::Replay(r) => {
                assert_eq!(r.status, CommandStatus::Succeeded);
                assert_eq!(r.result_payload.as_deref(), Some("{\"id\":\"p-1\"}"));
                assert!(r.error_payload.is_none());
            }
            other => panic!("expected Replay, got {:?}", other),
        }

        tx.commit().unwrap();
    }

    #[test]
    fn failed_command_replays_failure() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-fail");
        let ts = Timestamp("1000".to_string());
        store::insert_command(&tx, &cmd_id, "CreateRun", "hash-def", &ts).unwrap();
        store::complete_failure(&tx, &cmd_id, "{\"error\":\"bad\"}", &ts).unwrap();

        let result = check_idempotency(&tx, &cmd_id, "hash-def").unwrap();
        match result {
            IdempotencyCheck::Replay(r) => {
                assert_eq!(r.status, CommandStatus::Failed);
                assert!(r.result_payload.is_none());
                assert_eq!(r.error_payload.as_deref(), Some("{\"error\":\"bad\"}"));
            }
            other => panic!("expected Replay, got {:?}", other),
        }

        tx.commit().unwrap();
    }

    #[test]
    fn different_payload_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-dup");
        let ts = Timestamp("1000".to_string());
        store::insert_command(&tx, &cmd_id, "CreateProject", "hash-aaa", &ts).unwrap();
        store::complete_success(&tx, &cmd_id, "{}", &ts).unwrap();

        let err = check_idempotency(&tx, &cmd_id, "hash-bbb").unwrap_err();
        assert!(matches!(err, CommandError::DuplicateCommandMismatch { .. }));

        tx.commit().unwrap();
    }

    #[test]
    fn received_non_terminal_status_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let cmd_id = CommandId::from("cmd-inflight");
        let ts = Timestamp("1000".to_string());
        store::insert_command(&tx, &cmd_id, "CreateProject", "hash-ccc", &ts).unwrap();
        // Do NOT complete — leave as RECEIVED

        let err = check_idempotency(&tx, &cmd_id, "hash-ccc").unwrap_err();
        assert!(matches!(err, CommandError::InvalidCommand { .. }));

        tx.rollback().unwrap();
    }
}
