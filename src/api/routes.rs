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

    (StatusCode::OK, Json(resp))
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
}