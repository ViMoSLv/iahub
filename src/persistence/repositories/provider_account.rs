//! Mega Brain V0 — Provider Account Repository (P0-4: Persist Accounts)
//!
//! Persistence operations for the ProviderAccount aggregate. Maps between
//! domain types and SQLite rows in the `provider_accounts` table (v0005).
//!
//! This replaces the in-memory LazyLock<Mutex<Vec>> that lost all accounts
//! on backend restart.

use rusqlite::params;

use crate::domain::{EntityVersion, ProviderAccountId, ProviderKind, ProviderAccountStatus, Timestamp};
use crate::persistence::error::PersistenceError;
use crate::persistence::transaction::Transaction;

/// Row representation of a provider account in the database.
#[derive(Debug, Clone)]
pub struct ProviderAccountRow {
    pub id: ProviderAccountId,
    pub provider_kind: String,
    pub label: String,
    pub identity_hint: Option<String>,
    pub auth_profile_id: String,
    pub status: String,
    pub max_concurrent_sessions: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: EntityVersion,
}

/// Repository for ProviderAccount persistence operations.
pub struct ProviderAccountRepository;

impl ProviderAccountRepository {
    /// Insert a new provider account. Fails if ID already exists.
    pub fn insert(tx: &Transaction, row: &ProviderAccountRow) -> Result<(), PersistenceError> {
        let created_at: i64 = row.created_at.0.parse().map_err(|e| PersistenceError::Serialization {
            context: "provider_accounts.created_at",
            detail: format!("invalid timestamp '{}': {}", row.created_at.0, e),
        })?;
        let updated_at: i64 = row.updated_at.0.parse().map_err(|e| PersistenceError::Serialization {
            context: "provider_accounts.updated_at",
            detail: format!("invalid timestamp '{}': {}", row.updated_at.0, e),
        })?;

        tx.conn()
            .execute(
                "INSERT INTO provider_accounts (id, provider_kind, label, identity_hint, auth_profile_id, status, max_concurrent_sessions, created_at, updated_at, version)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    row.id.0,
                    row.provider_kind,
                    row.label,
                    row.identity_hint,
                    row.auth_profile_id,
                    row.status,
                    row.max_concurrent_sessions,
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
                        table: "provider_accounts".to_string(),
                        detail: format!("duplicate provider account id: {}", row.id),
                    }
                }
                other => PersistenceError::Transaction { source: other },
            })?;
        Ok(())
    }

    /// Fetch a provider account by ID. Returns None if not found.
    pub fn get_by_id(
        tx: &Transaction,
        id: &ProviderAccountId,
    ) -> Result<Option<ProviderAccountRow>, PersistenceError> {
        let result = tx.conn().query_row(
            "SELECT id, provider_kind, label, identity_hint, auth_profile_id, status, max_concurrent_sessions, created_at, updated_at, version
             FROM provider_accounts WHERE id = ?1",
            [&id.0],
            |row| {
                let created_at_i64: i64 = row.get(7)?;
                let updated_at_i64: i64 = row.get(8)?;
                Ok(ProviderAccountRow {
                    id: ProviderAccountId(row.get::<_, String>(0)?),
                    provider_kind: row.get(1)?,
                    label: row.get(2)?,
                    identity_hint: row.get(3)?,
                    auth_profile_id: row.get(4)?,
                    status: row.get(5)?,
                    max_concurrent_sessions: row.get(6)?,
                    created_at: Timestamp(created_at_i64.to_string()),
                    updated_at: Timestamp(updated_at_i64.to_string()),
                    version: EntityVersion(row.get(9)?),
                })
            },
        );

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PersistenceError::Transaction { source: e }),
        }
    }

    /// List all provider accounts, optionally filtered by provider kind.
    pub fn list(tx: &Transaction, provider_kind_filter: Option<&str>) -> Result<Vec<ProviderAccountRow>, PersistenceError> {
        let mut accounts = Vec::new();

        if let Some(kind) = provider_kind_filter {
            let mut stmt = tx.conn().prepare(
                "SELECT id, provider_kind, label, identity_hint, auth_profile_id, status, max_concurrent_sessions, created_at, updated_at, version
                 FROM provider_accounts WHERE provider_kind = ?1 ORDER BY created_at"
            ).map_err(|e| PersistenceError::Transaction { source: e })?;

            let rows = stmt.query_map([kind], |row| {
                let created_at_i64: i64 = row.get(7)?;
                let updated_at_i64: i64 = row.get(8)?;
                Ok(ProviderAccountRow {
                    id: ProviderAccountId(row.get::<_, String>(0)?),
                    provider_kind: row.get(1)?,
                    label: row.get(2)?,
                    identity_hint: row.get(3)?,
                    auth_profile_id: row.get(4)?,
                    status: row.get(5)?,
                    max_concurrent_sessions: row.get(6)?,
                    created_at: Timestamp(created_at_i64.to_string()),
                    updated_at: Timestamp(updated_at_i64.to_string()),
                    version: EntityVersion(row.get(9)?),
                })
            }).map_err(|e| PersistenceError::Transaction { source: e })?;

            for row in rows {
                accounts.push(row.map_err(|e| PersistenceError::Transaction { source: e })?);
            }
        } else {
            let mut stmt = tx.conn().prepare(
                "SELECT id, provider_kind, label, identity_hint, auth_profile_id, status, max_concurrent_sessions, created_at, updated_at, version
                 FROM provider_accounts ORDER BY created_at"
            ).map_err(|e| PersistenceError::Transaction { source: e })?;

            let rows = stmt.query_map([], |row| {
                let created_at_i64: i64 = row.get(7)?;
                let updated_at_i64: i64 = row.get(8)?;
                Ok(ProviderAccountRow {
                    id: ProviderAccountId(row.get::<_, String>(0)?),
                    provider_kind: row.get(1)?,
                    label: row.get(2)?,
                    identity_hint: row.get(3)?,
                    auth_profile_id: row.get(4)?,
                    status: row.get(5)?,
                    max_concurrent_sessions: row.get(6)?,
                    created_at: Timestamp(created_at_i64.to_string()),
                    updated_at: Timestamp(updated_at_i64.to_string()),
                    version: EntityVersion(row.get(9)?),
                })
            }).map_err(|e| PersistenceError::Transaction { source: e })?;

            for row in rows {
                accounts.push(row.map_err(|e| PersistenceError::Transaction { source: e })?);
            }
        }

        Ok(accounts)
    }

    /// Delete a provider account by ID.
    pub fn delete(tx: &Transaction, id: &ProviderAccountId) -> Result<bool, PersistenceError> {
        let affected = tx.conn()
            .execute("DELETE FROM provider_accounts WHERE id = ?1", [&id.0])
            .map_err(|e| PersistenceError::Transaction { source: e })?;
        Ok(affected > 0)
    }

    /// Update a provider account with optimistic concurrency control.
    pub fn update(
        tx: &Transaction,
        row: &ProviderAccountRow,
        expected_version: EntityVersion,
    ) -> Result<(), PersistenceError> {
        let updated_at: i64 = row.updated_at.0.parse().map_err(|e| PersistenceError::Serialization {
            context: "provider_accounts.updated_at",
            detail: format!("invalid timestamp '{}': {}", row.updated_at.0, e),
        })?;

        let affected = tx.conn()
            .execute(
                "UPDATE provider_accounts SET provider_kind = ?1, label = ?2, identity_hint = ?3,
                 auth_profile_id = ?4, status = ?5, max_concurrent_sessions = ?6,
                 updated_at = ?7, version = ?8
                 WHERE id = ?9 AND version = ?10",
                params![
                    row.provider_kind,
                    row.label,
                    row.identity_hint,
                    row.auth_profile_id,
                    row.status,
                    row.max_concurrent_sessions,
                    updated_at,
                    row.version.0,
                    row.id.0,
                    expected_version.0,
                ],
            )
            .map_err(|e| PersistenceError::Transaction { source: e })?;

        if affected == 0 {
            return Err(PersistenceError::VersionConflict {
                entity: "ProviderAccount",
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

    fn make_account(id: &str, provider: &str) -> ProviderAccountRow {
        ProviderAccountRow {
            id: ProviderAccountId::from(id),
            provider_kind: provider.to_string(),
            label: format!("{} Account", provider),
            identity_hint: Some(format!("user-{}@example.com", id)),
            auth_profile_id: format!("auth-{}", id),
            status: "ACTIVE".to_string(),
            max_concurrent_sessions: 2,
            created_at: Timestamp("1000".to_string()),
            updated_at: Timestamp("1000".to_string()),
            version: EntityVersion::INITIAL,
        }
    }

    #[test]
    fn insert_and_get_by_id() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let acc = make_account("PA-1", "claude");
        ProviderAccountRepository::insert(&tx, &acc).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let fetched = ProviderAccountRepository::get_by_id(&tx2, &ProviderAccountId::from("PA-1"))
            .unwrap()
            .expect("must find inserted account");
        assert_eq!(fetched.id, acc.id);
        assert_eq!(fetched.label, acc.label);
        assert_eq!(fetched.provider_kind, "claude");
        assert_eq!(fetched.max_concurrent_sessions, 2);
    }

    #[test]
    fn list_all_accounts() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        ProviderAccountRepository::insert(&tx, &make_account("PA-A", "claude")).unwrap();
        ProviderAccountRepository::insert(&tx, &make_account("PA-B", "antigravity")).unwrap();
        ProviderAccountRepository::insert(&tx, &make_account("PA-C", "claude")).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let all = ProviderAccountRepository::list(&tx2, None).unwrap();
        assert_eq!(all.len(), 3);

        let claude_only = ProviderAccountRepository::list(&tx2, Some("claude")).unwrap();
        assert_eq!(claude_only.len(), 2);
    }

    #[test]
    fn delete_account() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        ProviderAccountRepository::insert(&tx, &make_account("PA-DEL", "codex")).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let deleted = ProviderAccountRepository::delete(&tx2, &ProviderAccountId::from("PA-DEL")).unwrap();
        assert!(deleted);
        tx2.commit().unwrap();

        let tx3 = store.transaction().unwrap();
        let result = ProviderAccountRepository::get_by_id(&tx3, &ProviderAccountId::from("PA-DEL")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_insert_rejected() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let acc = make_account("PA-DUP", "claude");
        ProviderAccountRepository::insert(&tx, &acc).unwrap();
        let err = ProviderAccountRepository::insert(&tx, &acc).unwrap_err();
        match err {
            PersistenceError::ConstraintViolation { table, .. } => {
                assert_eq!(table, "provider_accounts");
            }
            other => panic!("expected ConstraintViolation, got {:?}", other),
        }
    }

    #[test]
    fn update_with_stale_version_fails() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let tx = store.transaction().unwrap();
        let acc = make_account("PA-STALE", "claude");
        ProviderAccountRepository::insert(&tx, &acc).unwrap();
        tx.commit().unwrap();

        let tx2 = store.transaction().unwrap();
        let mut updated = acc.clone();
        updated.label = "Updated".to_string();
        updated.version = updated.version.next();
        let err = ProviderAccountRepository::update(&tx2, &updated, EntityVersion(999)).unwrap_err();
        match err {
            PersistenceError::VersionConflict { entity, .. } => {
                assert_eq!(entity, "ProviderAccount");
            }
            other => panic!("expected VersionConflict, got {:?}", other),
        }
    }

    #[test]
    fn accounts_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("accounts_persist.db");

        // Session 1: insert accounts
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();
            ProviderAccountRepository::insert(&tx, &make_account("PA-PERSIST-1", "claude")).unwrap();
            ProviderAccountRepository::insert(&tx, &make_account("PA-PERSIST-2", "antigravity")).unwrap();
            tx.commit().unwrap();
        }

        // Session 2: verify accounts survived
        {
            let mut store = SqliteStore::open(&db_path).unwrap();
            let tx = store.transaction().unwrap();
            let all = ProviderAccountRepository::list(&tx, None).unwrap();
            assert_eq!(all.len(), 2, "accounts must survive restart");
            let fetched = ProviderAccountRepository::get_by_id(&tx, &ProviderAccountId::from("PA-PERSIST-1"))
                .unwrap()
                .expect("PA-PERSIST-1 must exist after restart");
            assert_eq!(fetched.provider_kind, "claude");
        }
    }
}