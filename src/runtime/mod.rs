//! Mega Brain V0 — Runtime Layer (Phase 0)
//!
//! Process lifecycle management, PTY engine, runtime isolation, and agent
//! binary discovery. This module bridges the pure domain model to real
//! OS processes running coding agents.
//!
//! Key components:
//! - **PtyEngine**: Cross-platform pseudo-terminal (ConPTY on Windows, POSIX PTY on Unix)
//! - **ProcessSupervisor**: Lifecycle management, health monitoring, graceful shutdown
//! - **IsolationManager**: Per-account HOME/env/config/cache separation
//! - **ScrollbackBuffer**: Ring buffer + disk overflow for terminal history
//!
//! Invariants enforced:
//! - INV-001: Agents are disposable; PTY processes can die without losing orchestration state
//! - INV-056: PTY sessions survive UI disconnect (sidecar architecture)
//! - INV-057: Each ProviderAccount gets unique HOME/env (isolation boundary)

pub mod pty;
pub mod supervisor;
pub mod isolation;
pub mod scrollback;

pub use pty::PtyInstance;
pub use supervisor::ProcessSupervisor;
pub use isolation::IsolationBoundary;
pub use scrollback::ScrollbackBuffer;