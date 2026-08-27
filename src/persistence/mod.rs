//! Mega Brain V0 — Persistence Layer (Topic 03)
//!
//! SQLite is the source of truth for orchestration state. This module owns
//! all database access: connection factory, pragma configuration, migrations,
//! schema versioning, transaction abstraction, and repository implementations.
//!
//! Domain types (`crate::domain`) have zero dependency on this module.
//! Mapping between domain entities and persistence rows happens here.

pub mod config;
pub mod database;
pub mod error;
pub mod migrations;
pub mod repositories;
pub mod schema_version;
pub mod transaction;

pub use database::SqliteStore;
pub use error::PersistenceError;
pub use transaction::Transaction;
