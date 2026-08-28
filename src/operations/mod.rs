//! Mega Brain V0 — Operation Journal (ADR-0003)
//!
//! Append-only journal for external side effects. Every consequential operation
//! (worktree creation, Git ref creation, agent spawn, merge execution) must have
//! a PREPARED journal entry BEFORE execution begins. This enables deterministic
//! crash recovery and auditability.
//!
//! States: PREPARED → EXECUTING → SIDE_EFFECT_OBSERVED → COMMITTED | ROLLED_BACK
//!         | REQUIRES_RECONCILE | FAILED
//!
//! Unknown states fail closed on deserialization.

pub mod error;
pub mod model;
pub mod repository;
pub mod service;

pub use error::OperationError;
pub use model::{OperationId, OperationRecord, OperationStatus, OperationType};
pub use service::OperationService;
