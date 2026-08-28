//! Mega Brain V0 — Command Engine
//!
//! Single authorized entry point for all Control Plane mutations.
//! Orchestrates: idempotency check → handler execution → atomic commit.
//!
//! The engine owns the transaction boundary. Handlers never open their own
//! connections or start their own transactions.

use serde::Serialize;

use crate::authority::LeaseService;
use crate::domain::Timestamp;
use crate::persistence::SqliteStore;

use super::error::CommandError;
use super::idempotency::{self, IdempotencyCheck};
use super::payload::canonical_payload_hash;
use super::policy::AuthorityRequirement;
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

    /// Execute a command with full idempotency, authority validation, and atomic persistence.
    ///
    /// Flow:
    /// 1. Validate authority requirement BEFORE any DB access (fail fast)
    /// 2. Compute canonical payload hash
    /// 3. Begin transaction
    /// 4. Idempotency check (replay if already completed)
    /// 5. Insert command record (RECEIVED)
    /// 6. Validate lease authority within transaction (if required)
    /// 7. Execute handler within the same transaction
    /// 8. Mark command SUCCEEDED or FAILED
    /// 9. Commit transaction
    ///
    /// On any error before commit, the transaction rolls back automatically.
    /// Authority validation happens BOTH before the transaction (fail fast on
    /// missing fields) AND inside it (stale token / expired lease checks).
    pub fn execute<C, R>(
        &mut self,
        envelope: &CommandEnvelope<C>,
        command_type: &str,
        authority_req: AuthorityRequirement,
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
        // Step 0: Pre-flight authority field validation (before DB access).
        // If the command requires lease authority but the envelope is missing
        // attempt_id, lease_id, or fencing_token, reject immediately.
        let resolved_authority =
            if authority_req.requires_lease() {
                Some(authority_req.resolve(envelope).ok_or_else(|| {
                    CommandError::InvalidCommand {
                        detail: format!(
                            "command {} requires lease authority but envelope is missing \
                         attempt_id, lease_id, or fencing_token",
                            command_type
                        ),
                    }
                })?)
            } else {
                None
            };

        let payload_hash = canonical_payload_hash(&envelope.payload).map_err(|e| {
            CommandError::InvalidCommand {
                detail: format!("failed to hash command payload: {}", e),
            }
        })?;

        let tx = self.store.transaction()?;

        // Step 1: Idempotency check (includes command_type in identity)
        match idempotency::check_idempotency(
            &tx,
            &envelope.command_id,
            command_type,
            &payload_hash,
        )? {
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

        // Step 3: Validate lease authority within transaction (if required).
        // This checks expiry, revocation, fencing token validity, and ownership
        // against the current DB state. Handler MUST NOT execute if this fails.
        if let Some(ref auth) = resolved_authority {
            let now: i64 =
                envelope
                    .issued_at
                    .0
                    .parse()
                    .map_err(|e| CommandError::InvalidCommand {
                        detail: format!(
                            "invalid issued_at timestamp '{}' for authority validation: {}",
                            envelope.issued_at.0, e
                        ),
                    })?;
            LeaseService::validate_authority(
                &tx,
                &auth.lease_id,
                auth.fencing_token,
                &auth.resource,
                &auth.attempt_id.0,
                now,
            )
            .map_err(|e| match e {
                crate::authority::AuthorityError::StaleAuthority { reason, .. } => {
                    CommandError::StaleAuthority {
                        resource_type: auth.resource.resource_type.clone(),
                        resource_id: auth.resource.resource_id.clone(),
                        presented_token: auth.fencing_token,
                        current_token: auth.fencing_token,
                        reason,
                    }
                }
                _other => CommandError::StaleAuthority {
                    resource_type: auth.resource.resource_type.clone(),
                    resource_id: auth.resource.resource_id.clone(),
                    presented_token: auth.fencing_token,
                    current_token: auth.fencing_token,
                    reason: crate::authority::model::StaleReason::LeaseNotActive,
                },
            })?;
        }

        // Step 4: Execute handler within a savepoint for atomicity.
        // If the handler fails, its mutations are rolled back via the savepoint,
        // but the command record (RECEIVED) persists so we can mark it FAILED.
        // This prevents partial state from leaking into the database.
        //
        // The handler still receives &Transaction (unchanged API). The savepoint
        // is created on the same connection and controls rollback scope.
        let sp = tx.savepoint("handler_effects")?;
        let handler_result = handler(&tx, &envelope.payload, &envelope.issued_at);
        match handler_result {
            Ok(result) => {
                sp.release()?;
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
                // Roll back handler mutations; command record survives.
                sp.rollback()?;
                let error_json = serde_json::to_string(&err.to_string()).map_err(|e| {
                    CommandError::InvalidCommand {
                        detail: format!("failed to serialize command error: {}", e),
                    }
                })?;
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
    use crate::commands::types::{Actor, ActorType, CorrelationId};
    use crate::domain::{CommandId, ProjectId};
    use crate::persistence::repositories::project::ProjectRepository;

    fn make_envelope(payload: CreateProjectPayload) -> CommandEnvelope<CreateProjectPayload> {
        CommandEnvelope {
            command_id: CommandId::from(format!("cmd-{}", uuid::Uuid::new_v4())),
            actor: Actor {
                r#type: ActorType::System,
                id: "test".to_string(),
            },
            correlation_id: CorrelationId::from("test-corr"),
            causation_id: None,
            expected_version: None,
            attempt_id: None,
            lease_id: None,
            fencing_token: None,
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
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
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
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
            .unwrap();
        assert_eq!(r1.status, CommandStatus::Succeeded);

        // Second execution with same command_id and payload → replay
        let r2 = engine
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
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
            actor: Actor {
                r#type: ActorType::System,
                id: "test".to_string(),
            },
            correlation_id: CorrelationId::from("c1"),
            causation_id: None,
            expected_version: None,
            attempt_id: None,
            lease_id: None,
            fencing_token: None,
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
            .execute(
                &envelope1,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
            .unwrap();

        // Same command_id, different payload
        let envelope2 = CommandEnvelope {
            command_id: cmd_id,
            actor: Actor {
                r#type: ActorType::System,
                id: "test".to_string(),
            },
            correlation_id: CorrelationId::from("c2"),
            causation_id: None,
            expected_version: None,
            attempt_id: None,
            lease_id: None,
            fencing_token: None,
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
            .execute(
                &envelope2,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
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
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
            .unwrap();

        // Replay returns succeeded (same payload)
        let r2 = engine
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
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
            .execute(
                &envelope,
                "CreateProject",
                AuthorityRequirement::None,
                handle_create_project,
            )
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
