//! Mega Brain V0 — Persistence Error Types
//!
//! Typed errors for the persistence layer. `anyhow` is not used in public APIs;
//! it may appear only at binary/CLI boundaries.

use std::fmt;

/// All errors originating from the persistence layer.
#[derive(Debug)]
pub enum PersistenceError {
    /// Failed to open or create the database file.
    DatabaseOpen {
        path: String,
        source: rusqlite::Error,
    },

    /// A migration failed to apply.
    MigrationFailed {
        version: i64,
        message: String,
        source: Option<rusqlite::Error>,
    },

    /// The database schema version is newer than this binary supports.
    SchemaTooNew { found: i64, supported: i64 },

    /// A constraint was violated (FK, UNIQUE, CHECK).
    ConstraintViolation { table: String, detail: String },

    /// Entity not found by ID.
    NotFound { entity: &'static str, id: String },

    /// Serialization/deserialization of a domain value failed.
    Serialization {
        context: &'static str,
        detail: String,
    },

    /// A persisted state value could not be decoded into a known domain enum.
    StateDecode {
        entity: &'static str,
        field: &'static str,
        raw_value: String,
    },

    /// Transaction execution failed.
    Transaction { source: rusqlite::Error },

    /// SQLite returned SQLITE_BUSY after timeout.
    Busy { detail: String },

    /// Database integrity check failed.
    Corrupt { detail: String },

    /// Optimistic concurrency conflict (version mismatch).
    VersionConflict {
        entity: &'static str,
        id: String,
        expected_version: i64,
    },

    /// IO error related to database path or directory creation.
    Io {
        path: String,
        source: std::io::Error,
    },
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseOpen { path, source } => {
                write!(f, "failed to open database at {}: {}", path, source)
            }
            Self::MigrationFailed {
                version,
                message,
                source,
            } => {
                write!(f, "migration v{} failed: {}", version, message)?;
                if let Some(e) = source {
                    write!(f, " ({})", e)?;
                }
                Ok(())
            }
            Self::SchemaTooNew { found, supported } => {
                write!(
                    f,
                    "database schema version {} is newer than supported version {}; \
                     upgrade Mega Brain to access this database",
                    found, supported
                )
            }
            Self::ConstraintViolation { table, detail } => {
                write!(f, "constraint violation on {}: {}", table, detail)
            }
            Self::NotFound { entity, id } => {
                write!(f, "{} not found: {}", entity, id)
            }
            Self::Serialization { context, detail } => {
                write!(f, "serialization error in {}: {}", context, detail)
            }
            Self::StateDecode {
                entity,
                field,
                raw_value,
            } => {
                write!(
                    f,
                    "unknown persisted state for {}.{}: \"{}\"",
                    entity, field, raw_value
                )
            }
            Self::Transaction { source } => {
                write!(f, "transaction failed: {}", source)
            }
            Self::Busy { detail } => {
                write!(f, "database busy: {}", detail)
            }
            Self::Corrupt { detail } => {
                write!(f, "database corrupt: {}", detail)
            }
            Self::VersionConflict {
                entity,
                id,
                expected_version,
            } => {
                write!(
                    f,
                    "version conflict on {} {}: expected version {}",
                    entity, id, expected_version
                )
            }
            Self::Io { path, source } => {
                write!(f, "IO error at {}: {}", path, source)
            }
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DatabaseOpen { source, .. } => Some(source),
            Self::MigrationFailed {
                source: Some(s), ..
            } => Some(s),
            Self::Transaction { source } => Some(source),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for PersistenceError {
    fn from(e: rusqlite::Error) -> Self {
        match &e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::DatabaseBusy =>
            {
                Self::Busy {
                    detail: e.to_string(),
                }
            }
            rusqlite::Error::SqliteFailure(err, msg)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Self::ConstraintViolation {
                    table: "unknown".to_string(),
                    detail: msg.clone().unwrap_or_else(|| e.to_string()),
                }
            }
            _ => Self::Transaction { source: e },
        }
    }
}
