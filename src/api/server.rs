//! Mega Brain V0 — API Server (Phase 1, Gap 13)
//!
//! Axum HTTP/WebSocket server with dynamic port discovery and lock file.
//! Binds to 127.0.0.1 only (localhost security boundary).
//!
//! Port discovery (Gap 13):
//! - Try ports 8080..8090 sequentially
//! - Write PID + port to ~/.iahub/server.lock
//! - Print "IAHUB_LISTENING 127.0.0.1:PORT" to stdout for Tauri sidecar

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::routes::{build_router, AppState};
use super::ws::WsState;
use crate::runtime::supervisor::ProcessSupervisor;

/// Default port range for dynamic discovery.
const PORT_RANGE_START: u16 = 8080;
const PORT_RANGE_END: u16 = 8090;

/// The API server instance.
pub struct ApiServer {
    /// The port the server is listening on.
    pub port: u16,
    /// Path to the lock file.
    lock_path: PathBuf,
}

impl ApiServer {
    /// Start the API server with dynamic port discovery.
    /// Returns the server instance with the bound port, or an error if
    /// no port in the range is available.
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create supervisor and event channel
        let (event_tx, mut event_rx) = mpsc::channel::<crate::runtime::supervisor::SessionEvent>(256);
        let supervisor = Arc::new(ProcessSupervisor::new(event_tx));

        // Spawn event consumer (logs events for now; will forward to WebSocket broadcast)
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                tracing::info!(?event, "session event");
            }
        });

        let ws_state = WsState { supervisor };
        let app_state = Arc::new(AppState {
            ws_state: Arc::new(ws_state),
            start_time: std::time::Instant::now(),
            schema_version: 7, // current schema version
        });

        let router = build_router(app_state);

        // Dynamic port discovery (Gap 13)
        let lock_path = Self::lock_file_path();
        let mut bound_port: Option<u16> = None;

        for port in PORT_RANGE_START..=PORT_RANGE_END {
            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    // Write lock file
                    if let Some(parent) = lock_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let lock_content = format!("{}:{}\n", std::process::id(), port);
                    let _ = std::fs::write(&lock_path, &lock_content);

                    // Print for Tauri sidecar discovery
                    println!("IAHUB_LISTENING 127.0.0.1:{}", port);

                    tracing::info!(port = port, "API server starting");

                    // Spawn the server
                    tokio::spawn(async move {
                        if let Err(e) = axum::serve(listener, router).await {
                            tracing::error!(error = %e, "API server error");
                        }
                    });

                    bound_port = Some(port);
                    break;
                }
                Err(_) => continue, // Try next port
            }
        }

        let port = bound_port.ok_or_else(|| {
            format!("No available port in range {}-{}", PORT_RANGE_START, PORT_RANGE_END)
        })?;

        Ok(Self { port, lock_path })
    }

    /// Get the lock file path.
    fn lock_file_path() -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("iahub");
        base.join("server.lock")
    }

    /// Clean up the lock file on shutdown.
    pub fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_file_path_is_under_data_dir() {
        let path = ApiServer::lock_file_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("iahub"));
        assert!(path_str.ends_with("server.lock"));
    }

    #[test]
    fn port_range_is_valid() {
        assert!(PORT_RANGE_START < PORT_RANGE_END);
        assert!(PORT_RANGE_START >= 1024); // Non-privileged ports
    }
}