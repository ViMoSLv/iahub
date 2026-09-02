//! Mega Brain V0 — API Layer (Phase 1)
//!
//! HTTP/WebSocket server running on localhost for frontend communication.
//! Implements the IA-Hub Protocol v1:
//! - Binary WebSocket frames: terminal I/O (high throughput ANSI data)
//! - Text/JSON WebSocket frames: control commands (resize, spawn, interrupt)
//! - REST endpoints: CRUD for ProviderAccounts, Projects, Sessions, Health
//!
//! Architecture (from plan Section 6):
//! - Axum HTTP server bound to 127.0.0.1 only (localhost security)
//! - Dynamic port discovery with lock file (Gap 13)
//! - WebSocket PTY bridge per session (binary + JSON dual-frame)
//! - Health endpoint for frontend readiness probe (Gap 6)
//! - Structured logging with correlation IDs (Gap 7)

pub mod server;
pub mod ws;
pub mod routes;
pub mod health;

pub use server::ApiServer;
pub use health::HealthResponse;