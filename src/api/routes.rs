//! Mega Brain V0 — REST API Routes (Phase 1)
//!
//! HTTP endpoints for frontend CRUD operations and health checks.
//! All routes are bound to localhost only (security boundary).

use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::health::HealthResponse;
use super::ws::{self, WsState};

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
        .route("/api/sessions/:id", delete(delete_session_handler))
        .route("/api/agents", get(list_agents_handler))
        .route("/api/accounts", get(list_accounts_handler).post(create_account_handler))
        .route("/api/orchestrate", axum::routing::post(orchestrate_handler))
        .route("/ws/session/:id", get(ws_upgrade_handler))
        .with_state(state)
}

/// WebSocket upgrade handler for PTY session bridge.
async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let ws_state = state.ws_state.clone();
    ws.on_upgrade(move |socket| ws::handle_session_socket(socket, session_id, ws_state))
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
    /// ProviderAccount ID to bind this session to for isolation.
    /// If omitted, a unique per-session account is generated.
    #[serde(default)]
    account_id: Option<String>,
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

    // Determine workspace path — use provided path or current directory.
    // If the path is a git repo, provision an isolated worktree for this session.
    let base_workspace = req.workspace_path.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Attempt git worktree provisioning for isolated workspace per session.
    // Falls back to the base path if not a git repo or worktree creation fails.
    let workspace_path = {
        let base = std::path::Path::new(&base_workspace);
        let worktree_root = base.join(".ia-hub").join("attempts");
        let manager = crate::git::WorktreeManager::new(base, &worktree_root);
        match manager.provision_worktree(&session_id) {
            Ok(info) => {
                tracing::info!(
                    session_id = %session_id,
                    worktree = %info.worktree_path.display(),
                    branch = %info.branch_name,
                    "git worktree provisioned for session"
                );
                info.worktree_path.to_string_lossy().to_string()
            }
            Err(e) => {
                tracing::warn!(
                    session_id = %session_id,
                    error = %e,
                    "worktree provisioning failed, using base workspace"
                );
                base_workspace.clone()
            }
        }
    };

    // Determine the account_id for isolation — use provided account_id or
    // generate a unique per-session one. This ensures each session gets its
    // own IsolationBoundary directory tree, never sharing with others.
    let effective_account_id = req.account_id.unwrap_or_else(|| {
        format!("session-{}", uuid::Uuid::new_v4())
    });

    // Build isolation environment bound to the account_id (not session_id)
    let base_data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("iahub")
        .join("accounts");
    let boundary = crate::runtime::IsolationBoundary::new(&effective_account_id, &base_data_dir);
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

            // Register with supervisor using the effective account_id
            let register_result = state.ws_state.supervisor.register_session(
                &session_id,
                &effective_account_id,
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

/// DELETE /api/sessions/:id — terminate a running session (P0-3).
async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.ws_state.supervisor.terminate_session(&session_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "terminated",
                "session_id": session_id,
            })),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": e,
            })),
        ),
    }
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

// ── Provider Account endpoints ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateAccountRequest {
    provider: String,
    label: String,
    #[serde(default)]
    identity_hint: Option<String>,
    #[serde(default = "default_max_concurrent")]
    max_concurrent_sessions: u32,
}

fn default_max_concurrent() -> u32 { 2 }

#[derive(Debug, Serialize)]
struct AccountResponse {
    id: String,
    provider: String,
    label: String,
    status: String,
    max_concurrent_sessions: u32,
    active_sessions: usize,
}

/// In-memory account registry for MVP — will be backed by SQLite in production.
use std::sync::{Mutex, LazyLock};

#[derive(Debug, Clone)]
struct StoredAccount {
    id: String,
    provider: String,
    label: String,
    #[allow(dead_code)]
    identity_hint: Option<String>,
    max_concurrent_sessions: u32,
}

static ACCOUNTS: LazyLock<Mutex<Vec<StoredAccount>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// POST /api/accounts — register a new ProviderAccount.
async fn create_account_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAccountRequest>,
) -> impl IntoResponse {
    let account_id = uuid::Uuid::new_v4().to_string();
    let account = StoredAccount {
        id: account_id.clone(),
        provider: req.provider.clone(),
        label: req.label.clone(),
        identity_hint: req.identity_hint,
        max_concurrent_sessions: req.max_concurrent_sessions,
    };

    // Register concurrency limit with supervisor
    state.ws_state.supervisor.set_account_limit(&account_id, req.max_concurrent_sessions).await;

    // Store in registry
    {
        let mut accounts = ACCOUNTS.lock().unwrap();
        accounts.push(account);
    }

    let resp = AccountResponse {
        id: account_id,
        provider: req.provider,
        label: req.label,
        status: "active".to_string(),
        max_concurrent_sessions: req.max_concurrent_sessions,
        active_sessions: 0,
    };

    (StatusCode::CREATED, Json(resp))
}

/// GET /api/accounts — list all registered ProviderAccounts.
async fn list_accounts_handler(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let accounts = ACCOUNTS.lock().unwrap().clone();
    let result: Vec<AccountResponse> = accounts.into_iter().map(|a| {
        AccountResponse {
            id: a.id.clone(),
            provider: a.provider,
            label: a.label,
            status: "active".to_string(),
            max_concurrent_sessions: a.max_concurrent_sessions,
            active_sessions: 0, // TODO: query supervisor for real count
        }
    }).collect();

    (StatusCode::OK, Json(result))
}

// ── Orchestrator endpoint ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OrchestrateRequest {
    /// The user objective to decompose into tasks.
    objective: String,
    /// Optional custom roles (defaults to scout → coder → tester → reviewer).
    #[serde(default)]
    roles: Option<Vec<String>>,
}

/// POST /api/orchestrate — decompose an objective into tasks with account assignments.
async fn orchestrate_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<OrchestrateRequest>,
) -> impl IntoResponse {
    use crate::orchestrator::{TaskDecomposer, SessionDispatcher};
    use crate::orchestrator::dispatcher::AccountSlot;

    // Decompose the objective into subtasks
    let tasks = if let Some(role_names) = &req.roles {
        let roles: Vec<crate::orchestrator::decomposer::AgentRole> = role_names
            .iter()
            .filter_map(|r| match r.as_str() {
                "scout" => Some(crate::orchestrator::decomposer::AgentRole::Scout),
                "coder" => Some(crate::orchestrator::decomposer::AgentRole::Coder),
                "tester" => Some(crate::orchestrator::decomposer::AgentRole::Tester),
                "reviewer" => Some(crate::orchestrator::decomposer::AgentRole::Reviewer),
                "shell" => Some(crate::orchestrator::decomposer::AgentRole::Shell),
                _ => None,
            })
            .collect();
        if roles.is_empty() {
            TaskDecomposer::decompose(&req.objective)
        } else {
            TaskDecomposer::decompose_with_roles(&req.objective, &roles)
        }
    } else {
        TaskDecomposer::decompose(&req.objective)
    };

    // Build account slots from registered accounts
    let accounts = ACCOUNTS.lock().unwrap().clone();
    let slots: Vec<AccountSlot> = accounts.iter().map(|a| AccountSlot {
        account_id: a.id.clone(),
        provider: a.provider.clone(),
        max_concurrent: a.max_concurrent_sessions,
        active_sessions: 0, // TODO: query real count from supervisor
        available: true,
    }).collect();

    // Attempt dispatch
    let result = if slots.is_empty() {
        // No accounts registered — return tasks without assignments
        let task_list: Vec<serde_json::Value> = tasks.iter().map(|t| serde_json::json!({
            "order": t.order,
            "role": t.role.to_string(),
            "description": t.description,
            "parallelizable": t.parallelizable,
            "depends_on": t.depends_on,
            "account_id": null,
            "provider": null,
        })).collect();
        serde_json::json!({
            "objective": req.objective,
            "tasks": task_list,
            "assignments": [],
            "warning": "No ProviderAccounts registered. Add accounts via POST /api/accounts first.",
        })
    } else {
        let dispatcher = SessionDispatcher::new(slots);
        match dispatcher.dispatch(&tasks) {
            Ok(assignments) => {
                let task_list: Vec<serde_json::Value> = assignments.iter().map(|a| serde_json::json!({
                    "order": a.task.order,
                    "role": a.task.role.to_string(),
                    "description": a.task.description,
                    "parallelizable": a.task.parallelizable,
                    "depends_on": a.task.depends_on,
                    "account_id": a.account_id,
                    "provider": a.provider,
                })).collect();
                serde_json::json!({
                    "objective": req.objective,
                    "tasks": task_list,
                    "assignments": task_list,
                })
            }
            Err(e) => {
                // Dispatch failed — return tasks without assignments + error
                let task_list: Vec<serde_json::Value> = tasks.iter().map(|t| serde_json::json!({
                    "order": t.order,
                    "role": t.role.to_string(),
                    "description": t.description,
                    "parallelizable": t.parallelizable,
                    "depends_on": t.depends_on,
                    "account_id": null,
                    "provider": null,
                })).collect();
                serde_json::json!({
                    "objective": req.objective,
                    "tasks": task_list,
                    "assignments": [],
                    "error": format!("{:?}", e),
                })
            }
        }
    };

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