//! Mega Brain V0 — REST API Routes (Phase 1)
//!
//! HTTP endpoints for frontend CRUD operations and health checks.
//! All routes are bound to localhost only (security boundary).

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::health::HealthResponse;
use super::ws::WsState;

/// Application state shared across all route handlers.
pub struct AppState {
    pub ws_state: Arc<WsState>,
    pub start_time: std::time::Instant,
    pub schema_version: i64,
}

/// Build the API router with all REST endpoints.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/sessions", get(list_sessions_handler).post(spawn_session_handler))
        .route("/api/agents", get(list_agents_handler))
        .with_state(state)
}

/// GET /health — readiness probe for frontend (Gap 6).
async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let active_sessions = state.ws_state.supervisor.total_active_sessions().await;

    // Check if onboarding is needed (no projects in DB)
    // For now, always false — will be wired to persistence layer
    let onboarding_required = false;

    let mut resp = HealthResponse::ready(active_sessions, state.schema_version, onboarding_required);
    resp.uptime_seconds = uptime;

    // Populate agent discovery status
    let agents = crate::adapters::AgentBinaryResolver::resolve_all();
    resp.agents = agents.iter().map(|a| {
        use crate::adapters::AgentBinaryStatus;
        match &a.status {
            AgentBinaryStatus::Ok { path, version } => super::health::AgentBinaryStatus {
                name: a.name.clone(),
                path: Some(path.clone()),
                version: version.clone(),
                status: "ok".to_string(),
            },
            AgentBinaryStatus::NotFound { name: _ } => super::health::AgentBinaryStatus {
                name: a.name.clone(),
                path: None,
                version: None,
                status: "not_found".to_string(),
            },
            AgentBinaryStatus::VersionUnknown { path } => super::health::AgentBinaryStatus {
                name: a.name.clone(),
                path: Some(path.clone()),
                version: None,
                status: "version_unknown".to_string(),
            },
        }
    }).collect();

    (StatusCode::OK, Json(resp))
}

// ── Session endpoints ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SpawnSessionRequest {
    /// Which agent binary to use (e.g., "claude", "codex")
    agent_binary: String,
    /// Working directory for the agent (workspace path)
    #[serde(default)]
    workspace_path: Option<String>,
    /// Optional label for the session
    #[serde(default)]
    #[allow(dead_code)]
    label: Option<String>,
}

#[derive(Debug, Serialize)]
struct SpawnSessionResponse {
    session_id: String,
    status: String,
    agent_binary: String,
    workspace_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionListItem {
    id: String,
    agent_binary: String,
    account_id: String,
    provider: String,
    status: String,
}

/// POST /api/sessions — spawn a new PTY session with an agent.
async fn spawn_session_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SpawnSessionRequest>,
) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();

    // Determine workspace path
    let workspace_path = req.workspace_path.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Build isolation environment
    let base_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("iahub")
        .join("accounts");
    let boundary = crate::runtime::IsolationBoundary::new(&session_id, &base_data_dir);
    let _ = boundary.provision();
    let env_vars = boundary.generate_env_map();

    // Create channel for PTY output → WebSocket
    let (tx_to_ui, _rx_from_pty) = tokio::sync::mpsc::channel::<Vec<u8>>(2048);

    // Spawn the PTY process
    let workspace = std::path::Path::new(&workspace_path);
    match crate::runtime::PtyInstance::spawn_agent(
        &session_id,
        &req.agent_binary,
        &[],
        workspace,
        env_vars,
        24,
        80,
        tx_to_ui,
    ) {
        Ok(pty_instance) => {
            let pty_arc = Arc::new(pty_instance);

            // Register with supervisor
            let register_result = state.ws_state.supervisor.register_session(
                &session_id,
                "default",
                &req.agent_binary,
                pty_arc,
            ).await;

            if let Err(event) = register_result {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": format!("{:?}", event),
                    })),
                );
            }

            (
                StatusCode::CREATED,
                Json(serde_json::to_value(SpawnSessionResponse {
                    session_id,
                    status: "active".to_string(),
                    agent_binary: req.agent_binary,
                    workspace_path,
                }).unwrap()),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to spawn agent: {}", e),
            })),
        ),
    }
}

/// GET /api/sessions — list all active sessions.
async fn list_sessions_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let summaries = state.ws_state.supervisor.list_sessions().await;
    let sessions: Vec<SessionListItem> = summaries.into_iter().map(|s| {
        SessionListItem {
            id: s.id,
            agent_binary: s.agent_binary,
            account_id: s.account_id,
            provider: "local".to_string(),
            status: "active".to_string(),
        }
    }).collect();

    (StatusCode::OK, Json(sessions))
}

/// GET /api/agents — list discovered agent binaries.
async fn list_agents_handler() -> impl IntoResponse {
    let agents = crate::adapters::AgentBinaryResolver::resolve_all();
    let result: Vec<serde_json::Value> = agents.iter().map(|a| {
        use crate::adapters::AgentBinaryStatus;
        match &a.status {
            AgentBinaryStatus::Ok { path, version } => serde_json::json!({
                "name": a.name,
                "binary": a.binary_name,
                "path": path,
                "version": version,
                "status": "ok",
                "capabilities": a.capabilities,
            }),
            AgentBinaryStatus::NotFound { name } => serde_json::json!({
                "name": a.name,
                "binary": name,
                "path": null,
                "version": null,
                "status": "not_found",
                "capabilities": a.capabilities,
            }),
            AgentBinaryStatus::VersionUnknown { path } => serde_json::json!({
                "name": a.name,
                "binary": a.binary_name,
                "path": path,
                "version": null,
                "status": "version_unknown",
                "capabilities": a.capabilities,
            }),
        }
    }).collect();

    (StatusCode::OK, Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::supervisor::ProcessSupervisor;
    use tokio::sync::mpsc;

    fn make_test_state() -> Arc<AppState> {
        let (tx, _rx) = mpsc::channel(100);
        let supervisor = ProcessSupervisor::new(tx);
        let ws_state = WsState {
            supervisor: Arc::new(supervisor),
        };
        Arc::new(AppState {
            ws_state: Arc::new(ws_state),
            start_time: std::time::Instant::now(),
            schema_version: 7,
        })
    }

    #[tokio::test]
    async fn health_endpoint_returns_ready() {
        let state = make_test_state();
        let app = build_router(state);

        let response = axum::http::Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = tower::ServiceExt::oneshot(app, response).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "ready");
        assert_eq!(health.schema_version, 7);
        assert_eq!(health.active_sessions, 0);
    }

    #[tokio::test]
    async fn list_sessions_returns_empty_initially() {
        let state = make_test_state();
        let app = build_router(state);

        let response = axum::http::Request::builder()
            .uri("/api/sessions")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = tower::ServiceExt::oneshot(app, response).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let sessions: Vec<SessionListItem> = serde_json::from_slice(&body).unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn list_agents_returns_discovered_binaries() {
        let response = axum::http::Request::builder()
            .uri("/api/agents")
            .body(axum::body::Body::empty())
            .unwrap();

        let app = build_router(make_test_state());
        let resp = tower::ServiceExt::oneshot(app, response).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let agents: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(agents.len(), 4, "should discover 4 known agent types");
    }
}