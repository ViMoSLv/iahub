//! Mega Brain V0 — Project Repository
//!
//! Persistence operations for the Project aggregate. Maps between domain
//! `ProjectId` / `EntityVersion` and persistence rows. Domain module has
//! zero dependency on rusqlite; all mapping happens here.

use rusqlite::params;

use crate::domain::{EntityVersion, ProjectId, Timestamp};
use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

/// Row representation of a project in the database.
#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: ProjectId,
    pub name: String,
    pub repository_identity: String,
    pub canonical_path: String,
    pub target_branch: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: EntityVersion,
}

/// Repository for Project aggregate operations.
pub struct ProjectRepository;

impl ProjectRepository {
    /// Insert a new project. Fails if ID already exists (PK constraint).
    pub fn insert(tx: &Transaction, row: &ProjectRow) -> Result<(), PersistenceError> {
        // Timestamps are stored as INTEGER (unix millis) in SQLite.
        // Parse the opaque Timestamp string to i64 for persistence.
        // Fail closed on invalid timestamps — never silently use epoch zero.
        let created_at: i64 =
            row.created_at
                .0
                .parse()
                .map_err(|e| PersistenceError::Serialization {
                    context: "projects.created_at",
                    detail: format!("invalid timestamp '{}': {}", row.created_at.0, e),
                })?;
        let updated_at: i64 =
            row.updated_at
                .0
                .parse()
                .map_err(|e| PersistenceError::Serialization {
                    context: "projects.updated_at",
                    detail: format!("invalid timestamp '{}': {}", row.updated_at.0, e),
                })?;

        tx.conn()
            .execute(
                "INSERT INTO projects (id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    row.id.0,
                    row.name,
                    row.repository_identity,
                    row.canonical_path,
                    row.target_branch,
                    created_at,
                    updated_at,
                    row.version.0,
                ],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    PersistenceError::ConstraintViolation {
                        table: "projects".to_string(),
                        detail: format!("duplicate project id: {}", row.id),
                    }
                }
                other => PersistenceError::Transaction { source: other },
            })?;
        Ok(())
    }

    /// Fetch a project by ID. Returns None if not found.
    pub fn get_by_id(
        tx: &Transaction,
        id: &ProjectId,
    ) -> Result<Option<ProjectRow>, PersistenceError> {
        let result = tx.conn().query_row(
            "SELECT id, name, repository_identity, canonical_path, target_branch, created_at, updated_at, version
             FROM projects WHERE id = ?1",
            [&id.0],
            |row| {
                // created_at and updated_at are stored as INTEGER in SQLite.
                // Read as i64 and convert back to opaque Timestamp string.
                let created_at_i64: i64 = row.get(5)?;
                let updated_at_i64: i64 = row.get(6)?;
                Ok(ProjectRow {
                    id: ProjectId(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    repository_identity: row.get(2)?,
                    canonical_path: row.get(3)?,
                    target_branch: row.get(4)?,
                    created_at: Timestamp(created_at_i64.to_string()),
                    updated_at: Timestamp(updated_at_i64.to_string()),
                    version: EntityVersion(row.get(7)?),
                })
            },
        );

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PersistenceError::Transaction { source: e }),
        }
    }

    /// Update a project with optimistic concurrency control.
    /// Returns Err(VersionConflict) if the stored version does not match expected.
    pub fn update(
        tx: &Transaction,
        row: &ProjectRow,
        expected_version: EntityVersion,
    ) -> Result<(), PersistenceError> {
        // Timestamps are stored as INTEGER (unix millis) in SQLite.
        // Fail closed on invalid timestamps — never silently use epoch zero.
        let updated_at: i64 =
            row.updated_at
                .0
                .parse()
                .map_err(|e| PersistenceError::Serialization {
                    context: "projects.updated_at",
                    detail: format!("invalid timestamp '{}': {}", row.updated_at.0, e),
                })?;

        let affected = tx
            .conn()
            .execute(
                "UPDATE projects SET name = ?1, repository_identity = ?2, canonical_path = ?3,
                 target_branch = ?4, updated_at = ?5, version = ?6
                 WHERE id = ?7 AND version = ?8",
                params![
                    row.name,
                    row.repository_identity,
                    row.canonical_path,
                    row.target_branch,
                    updated_at,
                    row.version.0,
                    row.id.0,
                    expected_version.0,
                ],
            )
            .map_err(|e| PersistenceError::Transaction { source: e })?;

        if affected == 0 {
            return Err(PersistenceError::VersionConflict {
                entity: "Project",
                id: row.id.0.clone(),
                expected_version: expected_version.0,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::SqliteStore;

    fn make_project(id: &str) -> ProjectRow {
        ProjectRow {
            id: ProjectId::from(id),
            name: format!("Project {}", id),
            repository_identity: format!("fp-{}", id),
            canonical_path: format!("/repos/{}", id),
            target_branch: "main".to_string(),
            // Use integer-compatible timestamps since schema stores INTEGER
            created_at: Timestamp("1000".to_string()),
            updated_at: Timestamp("1000".to_string()),
            version: EntityVersion::INITIAL,
        }
    }

    #[test]
    fn insert_and_get_by_id() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let p = make_project("proj-1");
        ProjectRepository::insert(&tx, &p).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let fetched = ProjectRepository::get_by_id(&tx2, &ProjectId::from("proj-1"))
            .unwrap()
            .expect("must find inserted project");
        assert_eq!(fetched.id, p.id);
        assert_eq!(fetched.name, p.name);
        assert_eq!(fetched.repository_identity, p.repository_identity);
        assert_eq!(fetched.version, EntityVersion::INITIAL);
    }

    #[test]
    fn get_by_id_returns_none_for_missing() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let result = ProjectRepository::get_by_id(&tx, &ProjectId::from("nonexistent")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_insert_is_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let p = make_project("proj-dup");
        ProjectRepository::insert(&tx, &p).unwrap();
        let err = ProjectRepository::insert(&tx, &p).unwrap_err();
        match err {
            PersistenceError::ConstraintViolation { table, .. } => {
                assert_eq!(table, "projects");
            }
            other => panic!("expected ConstraintViolation, got {:?}", other),
        }
    }

    #[test]
    fn update_with_correct_version_succeeds() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let mut p = make_project("proj-upd");
        ProjectRepository::insert(&tx, &p).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        p.name = "Updated Name".to_string();
        p.version = p.version.next();
        p.updated_at = Timestamp("2000".to_string());
        ProjectRepository::update(&tx2, &p, EntityVersion::INITIAL).unwrap();
        tx2.commit().unwrap();

        let tx3 = store.transaction().unwrap();
        let fetched = ProjectRepository::get_by_id(&tx3, &p.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Updated Name");
        assert_eq!(fetched.version, EntityVersion(2));
    }

    #[test]
    fn update_with_stale_version_returns_conflict() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let p = make_project("proj-stale");
        ProjectRepository::insert(&tx, &p).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let err = ProjectRepository::update(&tx2, &p, EntityVersion(999)).unwrap_err();
        match err {
            PersistenceError::VersionConflict {
                entity,
                id,
                expected_version,
            } => {
                assert_eq!(entity, "Project");
                assert_eq!(id, "proj-stale");
                assert_eq!(expected_version, 999);
            }
            other => panic!("expected VersionConflict, got {:?}", other),
        }
    }
}
