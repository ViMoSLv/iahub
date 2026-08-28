//! Mega Brain V0 — Command Types and Envelope
//!
//! Strongly-typed command envelope and status lifecycle.
//! All commands flow through this structure before reaching handlers.

use serde::{Deserialize, Serialize};

use crate::domain::{AttemptId, CommandId, EntityVersion, FencingToken, LeaseId, Timestamp};

/// The type of actor issuing a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    Agent,
    User,
    System,
    Adapter,
}

/// Identity of the actor issuing a command.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Actor {
    pub r#type: ActorType,
    pub id: String,
}

/// Correlation ID for tracing related commands/events across the system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);

impl From<String> for CorrelationId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for CorrelationId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Command lifecycle statuses persisted in the `commands` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandStatus {
    /// Command received but not yet executed.
    Received,
    /// Command executed and committed successfully.
    Succeeded,
    /// Command executed and failed deterministically (business rejection).
    Failed,
}

/// Typed envelope wrapping every command entering the engine.
///
/// The envelope carries identity, correlation, versioning, authority, and payload
/// separately so that idempotency checks can operate on the hash without
/// deserializing the payload. Matches the canonical envelope in Topic 04 §8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope<C> {
    /// Unique command identifier for idempotency.
    pub command_id: CommandId,
    /// Identity of the actor issuing this command.
    pub actor: Actor,
    /// Correlation ID linking related commands and events.
    pub correlation_id: CorrelationId,
    /// Optional causation: the command or event that triggered this one.
    pub causation_id: Option<CommandId>,
    /// Expected entity version for optimistic concurrency (if applicable).
    pub expected_version: Option<EntityVersion>,
    /// Attempt ID required for mutating attempt commands.
    pub attempt_id: Option<AttemptId>,
    /// Lease ID proving current authority over the attempt/resource.
    pub lease_id: Option<LeaseId>,
    /// Fencing token preventing stale authority reuse.
    pub fencing_token: Option<FencingToken>,
    /// When the command was issued (opaque timestamp).
    pub issued_at: Timestamp,
    /// The typed command payload.
    pub payload: C,
}

/// Result of a successfully executed command, persisted for idempotent replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    /// The command that produced this result.
    pub command_id: CommandId,
    /// Final status after execution.
    pub status: CommandStatus,
    /// Serialized result payload (command-specific).
    pub result_payload: Option<String>,
    /// Serialized error payload if status is Failed.
    pub error_payload: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_status_roundtrip() {
        let statuses = [
            CommandStatus::Received,
            CommandStatus::Succeeded,
            CommandStatus::Failed,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: CommandStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn unknown_command_status_fails_deserialization() {
        let json = "\"EXECUTING\"";
        let result: Result<CommandStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown CommandStatus must fail closed");
    }

    #[test]
    fn correlation_id_from_str() {
        let cid = CorrelationId::from("corr-123");
        assert_eq!(cid.0, "corr-123");
    }
}
