//! Mega Brain V0 — Command Engine (Topic 04)
//!
//! Single authorized entry point for all Control Plane mutations.
//! Every consequential state change flows through:
//!
//! ```text
//! Command → Idempotency Check → Precondition/Version Check
//!       → Domain Decision → Single SQLite Transaction
//!       → State Mutation + Event + Outbox → Commit → Result
//! ```
//!
//! Guarantees:
//! - At-least-once delivery with idempotent effects
//! - Optimistic concurrency via EntityVersion
//! - Atomic persistence of entity + event + outbox
//! - Fail-closed for unknown payloads and future schema versions

pub mod engine;
pub mod error;
pub mod handlers;
pub mod idempotency;
pub mod payload;
pub mod store;
pub mod types;

pub use engine::CommandEngine;
pub use error::CommandError;
pub use types::{CommandEnvelope, CommandStatus};
