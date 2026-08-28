//! Mega Brain V0 — Command Handlers
//!
//! Each handler implements the pure domain decision + atomic persistence for
//! one command type. Handlers receive a `&Transaction` and return either a
//! typed result or a `CommandError`. No handler opens its own connection or
//! starts its own transaction.

use serde::{Deserialize, Serialize};

use crate::domain::{EntityVersion, ProjectId, Timestamp};
use crate::persistence::repositories::project::{ProjectRepository, ProjectRow};
use crate::persistence::transaction::Transaction;

use super::error::CommandError;

// ---------------------------------------------------------------------------
// CreateProject
// ---------------------------------------------------------------------------

/// Payload for the `CreateProject` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProjectPayload {
    pub project_id: ProjectId,
    pub name: String,
    pub repository_identity: String,
    pub canonical_path: String,
    pub target_branch: String,
}

/// Result returned on successful project creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProjectResult {
    pub project_id: ProjectId,
}

/// Execute the CreateProject command within an existing transaction.
pub fn handle_create_project(
    tx: &Transaction,
    payload: &CreateProjectPayload,
    issued_at: &Timestamp,
) -> Result<CreateProjectResult, CommandError> {
    // The `projects` table stores timestamps as INTEGER (unix millis).
    // Timestamp is an opaque string in the domain; parse to i64 for persistence.
    let ts_str = format!("{}", issued_at.0.parse::<i64>().unwrap_or(0));
    let ts = Timestamp(ts_str);

    let row = ProjectRow {
        id: payload.project_id.clone(),
        name: payload.name.clone(),
        repository_identity: payload.repository_identity.clone(),
        canonical_path: payload.canonical_path.clone(),
        target_branch: payload.target_branch.clone(),
        created_at: ts.clone(),
        updated_at: ts,
        version: EntityVersion::INITIAL,
    };

    ProjectRepository::insert(tx, &row)?;

    Ok(CreateProjectResult {
        project_id: payload.project_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SqliteStore;

    #[test]
    fn create_project_succeeds() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let payload = CreateProjectPayload {
            project_id: ProjectId::from("proj-1"),
            name: "Test Project".to_string(),
            repository_identity: "fp-abc".to_string(),
            canonical_path: "/tmp/test".to_string(),
            target_branch: "main".to_string(),
        };
        let ts = Timestamp("1000".to_string());

        let result = handle_create_project(&tx, &payload, &ts).unwrap();
        assert_eq!(result.project_id.0, "proj-1");

        // Verify persisted
        let found = ProjectRepository::get_by_id(&tx, &ProjectId::from("proj-1"))
            .unwrap()
            .expect("project must exist after insert");
        assert_eq!(found.name, "Test Project");

        tx.commit().unwrap();
    }

    #[test]
    fn create_duplicate_project_fails() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();

        let payload = CreateProjectPayload {
            project_id: ProjectId::from("proj-dup"),
            name: "First".to_string(),
            repository_identity: "fp-1".to_string(),
            canonical_path: "/a".to_string(),
            target_branch: "main".to_string(),
        };
        let ts = Timestamp("1000".to_string());

        handle_create_project(&tx, &payload, &ts).unwrap();

        // Second insert with same ID must fail
        let err = handle_create_project(&tx, &payload, &ts).unwrap_err();
        assert!(
            matches!(err, CommandError::Persistence(_)),
            "duplicate PK must produce persistence error"
        );

        tx.rollback().unwrap();
    }
}
