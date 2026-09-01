//! Mega Brain V0 — Startup Recovery & Reconciliation (ADR-0010)
//!
//! Orchestrates crash recovery by scanning all entity types for non-terminal
//! state before the Hub accepts new commands. This module enforces:
//! - INV-019: Every external side effect produces a durable journal entry recoverable after crash.
//! - INV-031: Startup reconcile scans leases, operations, workspaces, sessions, and tasks
//!   before accepting new commands.
//!
//! Design principle: fail-closed. If any entity type cannot be scanned, startup
//! is blocked rather than proceeding with incomplete recovery.

pub mod reconciler;

pub use reconciler::{ReconcileReport, StartupReconciler};