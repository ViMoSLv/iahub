//! Mega Brain V0 — Orchestrator (Phase 2)
//!
//! Minimum viable orchestration: template-based task decomposition,
//! provider account selection with concurrency awareness, session spawning,
//! and durable agent-to-agent messaging via SQLite.
//!
//! MVP Orchestrator flow:
//! User Objective → decompose → Tasks → role assignment → ProviderAccount selection
//! → execution → reports → verification → result
//!
//! The orchestrator works over durable Mega Brain structures (Tasks, Attempts,
//! Artifacts in SQLite), never using prompt text as source of truth.

pub mod decomposer;
pub mod dispatcher;
pub mod message_bus;

pub use decomposer::TaskDecomposer;
pub use dispatcher::SessionDispatcher;
pub use message_bus::{AgentMessage, MessageBus, MessageKind};