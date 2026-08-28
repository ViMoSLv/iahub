//! Mega Brain V0 — Authority & Lease Service (Topic 04)
//!
//! Durable lease management with monotonic fencing tokens per ADR-0004.
//! This module is the single authority boundary: no command handler may grant,
//! validate, or revoke authority without going through this service.
//!
//! Key guarantees:
//! - Fencing tokens are monotonically increasing per (resource_type, resource_id)
//! - Tokens survive restarts (persisted in SQLite, never regenerated from memory)
//! - Only one ACTIVE lease per resource at any time
//! - Stale authority is mathematically impossible to reuse
//! - Heartbeat updates liveness but NEVER extends expiry
//! - Explicit renewal is separate from heartbeat
//! - Cancellation requires evidence; ACTIVE → CANCELLED direct transition forbidden

pub mod error;
pub mod model;
pub mod repository;
pub mod service;

pub use error::AuthorityError;
pub use model::{LeaseRecord, LeaseStatus, ResourceId};
pub use service::LeaseService;
