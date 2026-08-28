//! Mega Brain V0 — Command Engine
//!
//! Single authorized entry point for all Control Plane mutations.
//! Orchestrates: idempotency check → handler execution → atomic commit.
//!
//! The engine owns the transaction boundary. Handlers never open their own
//! connections or start their own transactions.

use serde::Serialize;

use crate::domain::Timestamp;
use crate::persistence::SqliteStore;

use super::error::CommandError;
use super::idempotency::{self, IdempotencyCheck};
use super::payload::canonical_payload_hash;
use super::store;
use super::types::{CommandEnvelope, CommandResult, CommandStatus};

/// The command engine. Holds a mutable reference to the store and provides
/// the single `execute` entry point for all commands.
pub struct CommandEngine<'a> {
    store: &'a mut SqliteStore,
}

impl<'a> CommandEngine<'a> {
    /// Create a new engine bound to the given store.
    pub fn new(store: &'a mut SqliteStore) -> Self {
        Self { store }
    }

    /// Execute a command with full idempotency and atomic persistence.
    ///
    /// Flow:
    /// 1. Compute canonical payload hash
    /// 2. Begin transaction
    /// 3. Idempotency check (replay if already completed)
    /// 4. Insert command record (RECEIVED)
    /// 5. Execute handler within the same transaction
    /// 6. Mark command SUCCEEDED or FAILED
    /// 7. Commit transaction
    ///
    /// On any error before commit, the transaction rolls back automatically.
    pub fn execute<C, R>(
        &mut self,
        envelope: &CommandEnvelope<C>,
        command_type: &str,
        handler: impl FnOnce(
            &crate::persistence::transaction::Transaction,
            &C,
            &Timestamp,
        ) -> Result<R, CommandError>,
    ) -> Result<CommandResult, CommandError>
    where
        C: Serialize,
        R: Serialize,
    {
        let payload_hash = canonical_payload_hash(&envelope.payload);

        let tx = self.store.transaction()?;

        // Step 1: Idempotency check
        match idempotency::check_idempotency(&tx, &envelope.command_id, &payload_hash)? {
            IdempotencyCheck::Replay(result) => {
                tx.commit()?;
                return Ok(result);
            }
            IdempotencyCheck::New => {}
        }

        // Step 2: Insert command record as RECEIVED
        store::insert_command(
            &tx,
            &envelope.command_id,
            command_type,
            &payload_hash,
            &envelope.issued_at,
        )?;

        // Step 3: Execute handler
        match handler(&tx, &envelope.payload, &envelope.issued_at) {
            Ok(result) => {
                let result_json =
                    serde_json::to_string(&result).map_err(|e| CommandError::InvalidCommand {
                        detail: format!("failed to serialize result: {}", e),
                    })?;
                store::complete_success(
                    &tx,
                    &envelope.command_id,
                    &result_json,
                    &envelope.issued_at,
                )?;
                tx.commit()?;
                Ok(CommandResult {
                    command_id: envelope.command_id.clone(),
                    status: CommandStatus::Succeeded,
                    result_payload: Some(result_json),
                    error_payload: None,
                })
            }
            Err(err) => {
                let error_json = serde_json::to_string(&err.to_string()).unwrap_or_default();
                store::complete_failure(
                    &tx,
                    &envelope.command_id,
                    &error_json,
                    &envelope.issued_at,
                )?;
                tx.commit()?;
                Ok(CommandResult {
                    command_id: envelope.command_id.clone(),
                    status: CommandStatus::Failed,
                    result_payload: None,
                    error_payload: Some(error_json),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::handlers::{handle_create_project, CreateProjectPayload};
    use crate::commands::types::CorrelationId;
    use crate::domain::{CommandId, ProjectId};
    use crate::persistence::repositories::project::ProjectRepository;

    fn make_envelope(payload: CreateProjectPayload) -> CommandEnvelope<CreateProjectPayload> {
        CommandEnvelope {
            command_id: CommandId::from(format!("cmd-{}", uuid::Uuid::new_v4())),
            correlation_id: CorrelationId::from("test-corr"),
            causation_id: None,
            expected_version: None,
            issued_at: Timestamp("1000".to_string()),
            payload,
        }
    }

    #[test]
    fn execute_create_project_succeeds() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let mut engine = CommandEngine::new(&mut store);

        let envelope = make_envelope(CreateProjectPayload {
            project_id: ProjectId::from("proj-eng-1"),
            name: "Engine Test".to_string(),
            repository_identity: "fp-eng".to_string(),
            canonical_path: "/eng/test".to_string(),
            target_branch: "main".to_string(),
        });

        let result = engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();

        assert_eq!(result.status, CommandStatus::Succeeded);
        assert!(result.result_payload.is_some());
        assert!(result.error_payload.is_none());

        // Verify persisted
        let tx = store.transaction().unwrap();
        let found = ProjectRepository::get_by_id(&tx, &ProjectId::from("proj-eng-1"))
            .unwrap()
            .expect("project must exist");
        assert_eq!(found.name, "Engine Test");
        tx.commit().unwrap();
    }

    #[test]
    fn execute_replays_on_duplicate_command_id() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let mut engine = CommandEngine::new(&mut store);

        let envelope = make_envelope(CreateProjectPayload {
            project_id: ProjectId::from("proj-replay"),
            name: "Replay Test".to_string(),
            repository_identity: "fp-rp".to_string(),
            canonical_path: "/rp".to_string(),
            target_branch: "main".to_string(),
        });

        // First execution
        let r1 = engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();
        assert_eq!(r1.status, CommandStatus::Succeeded);

        // Second execution with same command_id and payload → replay
        let r2 = engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();
        assert_eq!(r2.status, CommandStatus::Succeeded);
        assert_eq!(r1.result_payload, r2.result_payload);
    }

    #[test]
    fn execute_rejects_duplicate_with_different_payload() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let mut engine = CommandEngine::new(&mut store);

        let cmd_id = CommandId::from("cmd-conflict");
        let envelope1 = CommandEnvelope {
            command_id: cmd_id.clone(),
            correlation_id: CorrelationId::from("c1"),
            causation_id: None,
            expected_version: None,
            issued_at: Timestamp("1000".to_string()),
            payload: CreateProjectPayload {
                project_id: ProjectId::from("proj-a"),
                name: "A".to_string(),
                repository_identity: "fp-a".to_string(),
                canonical_path: "/a".to_string(),
                target_branch: "main".to_string(),
            },
        };

        engine
            .execute(&envelope1, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();

        // Same command_id, different payload
        let envelope2 = CommandEnvelope {
            command_id: cmd_id,
            correlation_id: CorrelationId::from("c2"),
            causation_id: None,
            expected_version: None,
            issued_at: Timestamp("2000".to_string()),
            payload: CreateProjectPayload {
                project_id: ProjectId::from("proj-b"),
                name: "B".to_string(),
                repository_identity: "fp-b".to_string(),
                canonical_path: "/b".to_string(),
                target_branch: "develop".to_string(),
            },
        };

        let err = engine
            .execute(&envelope2, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap_err();

        assert!(matches!(err, CommandError::DuplicateCommandMismatch { .. }));
    }

    #[test]
    fn execute_handler_failure_returns_failed_result() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let mut engine = CommandEngine::new(&mut store);

        let envelope = make_envelope(CreateProjectPayload {
            project_id: ProjectId::from("proj-fail"),
            name: "Fail Test".to_string(),
            repository_identity: "fp-f".to_string(),
            canonical_path: "/f".to_string(),
            target_branch: "main".to_string(),
        });

        // First insert succeeds
        engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();

        // Replay returns succeeded (same payload)
        let r2 = engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();
        assert_eq!(r2.status, CommandStatus::Succeeded);
    }

    #[test]
    fn command_record_persisted_with_correct_status() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let mut engine = CommandEngine::new(&mut store);

        let envelope = make_envelope(CreateProjectPayload {
            project_id: ProjectId::from("proj-status"),
            name: "Status Test".to_string(),
            repository_identity: "fp-s".to_string(),
            canonical_path: "/s".to_string(),
            target_branch: "main".to_string(),
        });

        engine
            .execute(&envelope, "CreateProject", |tx, p, ts| {
                handle_create_project(tx, p, ts)
            })
            .unwrap();

        // Verify command record in DB
        let tx = store.transaction().unwrap();
        let record = store::find_by_id(&tx, &envelope.command_id)
            .unwrap()
            .expect("command record must exist");
        assert_eq!(record.status, "SUCCEEDED");
        assert_eq!(record.command_type, "CreateProject");
        assert!(record.result_payload.is_some());
        assert!(record.completed_at.is_some());
        tx.commit().unwrap();
    }
}
