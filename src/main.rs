//! Mega Brain V0 — Standalone Server Entry Point (Gap 11: Sidecar Architecture)
//!
//! This binary runs the Mega Brain backend as a standalone process.
//! In production, Tauri spawns this as a sidecar; for development,
//! run directly with `cargo run --bin mega-brain-server`.
//!
//! Startup sequence:
//! 1. Initialize tracing/logging (Gap 7: structured logging with correlation IDs)
//! 2. Resolve data directories (~/.iahub/data/, logs/, scrollback/, accounts/)
//! 3. Open SQLite database with migrations (Gap 15: path resolution + backup)
//! 4. Run startup reconciler (INV-019/031: scan all entities before accepting commands)
//! 5. Discover agent binaries on PATH (Gap 8: AgentBinaryResolver)
//! 6. Start API server with dynamic port discovery (Gap 13: port 8080-8090)
//! 7. Print IAHUB_LISTENING to stdout for Tauri sidecar discovery
//! 8. Wait for shutdown signal (SIGTERM/SIGINT)
//! 9. Graceful shutdown: terminate PTY sessions, flush journal, cleanup lock file

use std::path::PathBuf;

/// Resolve the base data directory for IA-Hub.
/// Unix: ~/.iahub/
/// Windows: %LOCALAPPDATA%/iahub/
fn resolve_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("iahub")
}

/// Ensure all required subdirectories exist.
fn ensure_directories(base: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(base.join("data"))?;
    std::fs::create_dir_all(base.join("logs"))?;
    std::fs::create_dir_all(base.join("scrollback"))?;
    std::fs::create_dir_all(base.join("accounts"))?;
    std::fs::create_dir_all(base.join("credentials"))?;
    std::fs::create_dir_all(base.join("data").join("backups"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Initialize structured logging (Gap 7)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();

    tracing::info!("Mega Brain V0 server starting");

    // Step 2: Resolve and create data directories
    let data_dir = resolve_data_dir();
    ensure_directories(&data_dir)?;
    tracing::info!(path = %data_dir.display(), "data directory initialized");

    // Step 3: Open SQLite database with migrations
    let db_path = data_dir.join("data").join("mega-brain.db");
    let mut store = mega_brain::persistence::SqliteStore::open(&db_path)?;
    let schema_version = store.schema_version()?;
    tracing::info!(
        path = %db_path.display(),
        schema_version = schema_version,
        "database opened"
    );

    // Step 4: Run startup reconciler (INV-019/031)
    {
        let tx = store.transaction()?;
        let report = mega_brain::recovery::StartupReconciler::reconcile(&tx)
            .map_err(|e| format!("startup reconcile failed: {}", e))?;
        tracing::info!(
            active_leases = report.active_leases,
            reconcilable_operations = report.reconcilable_operations,
            scan_complete = report.scan_complete,
            has_pending_work = report.has_pending_work(),
            "startup reconcile complete"
        );
        if report.has_pending_work() {
            tracing::warn!(
                "pending work detected — {} operations and {} leases need reconciliation",
                report.reconcilable_operations,
                report.active_leases
            );
        }
        tx.commit()?;
    }

    // Step 5: Discover agent binaries (Gap 8)
    let agents = mega_brain::adapters::AgentBinaryResolver::resolve_all();
    for agent in &agents {
        match &agent.status {
            mega_brain::adapters::AgentBinaryStatus::Ok { path, version } => {
                tracing::info!(
                    name = %agent.name,
                    path = %path,
                    version = ?version,
                    "agent binary discovered"
                );
            }
            mega_brain::adapters::AgentBinaryStatus::NotFound { name } => {
                tracing::info!(
                    name = %agent.name,
                    binary = %name,
                    "agent binary not found on PATH"
                );
            }
            mega_brain::adapters::AgentBinaryStatus::VersionUnknown { path } => {
                tracing::warn!(
                    name = %agent.name,
                    path = %path,
                    "agent binary found but version detection failed"
                );
            }
        }
    }

    // Step 6-7: Start API server with dynamic port discovery
    let server = mega_brain::api::ApiServer::start().await.map_err(|e| -> Box<dyn std::error::Error> { e })?;
    tracing::info!(port = server.port, "API server listening");

    // Step 8: Wait for shutdown signal
    tracing::info!("waiting for shutdown signal (Ctrl+C)");
    tokio::signal::ctrl_c().await?;

    // Step 9: Graceful shutdown
    tracing::info!("shutdown signal received, cleaning up");
    server.cleanup();
    tracing::info!("Mega Brain V0 server stopped");

    Ok(())
}