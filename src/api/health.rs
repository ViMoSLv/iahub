//! Mega Brain V0 — Health Check Endpoint (Gap 6)
//!
//! Provides a readiness probe for the frontend to poll before connecting
//! WebSocket. Returns status of all subsystems and active session count.

use serde::{Deserialize, Serialize};

/// Health response returned by GET /health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status: "starting", "ready", "degraded", "shutting_down"
    pub status: String,
    /// Seconds since the server started.
    pub uptime_seconds: u64,
    /// Status of each subsystem.
    pub subsystems: SubsystemStatus,
    /// Number of currently active PTY sessions.
    pub active_sessions: usize,
    /// Current database schema version.
    pub schema_version: i64,
    /// Whether onboarding is required (no projects exist yet).
    pub onboarding_required: bool,
    /// Agent binary discovery status (Gap 8).
    pub agents: Vec<AgentBinaryStatus>,
}

/// Status of individual subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemStatus {
    pub sqlite: String,
    pub pty_engine: String,
    pub credential_store: String,
}

/// Status of a discovered agent binary (Gap 8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBinaryStatus {
    pub name: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub status: String, // "ok", "not_found", "version_unknown"
}

impl HealthResponse {
    /// Create a "ready" health response with default subsystem statuses.
    pub fn ready(active_sessions: usize, schema_version: i64, onboarding_required: bool) -> Self {
        Self {
            status: "ready".to_string(),
            uptime_seconds: 0, // will be set by server
            subsystems: SubsystemStatus {
                sqlite: "ok".to_string(),
                pty_engine: "ok".to_string(),
                credential_store: "ok".to_string(),
            },
            active_sessions,
            schema_version,
            onboarding_required,
            agents: Vec::new(),
        }
    }

    /// Create a "starting" health response.
    pub fn starting() -> Self {
        Self {
            status: "starting".to_string(),
            uptime_seconds: 0,
            subsystems: SubsystemStatus {
                sqlite: "initializing".to_string(),
                pty_engine: "initializing".to_string(),
                credential_store: "initializing".to_string(),
            },
            active_sessions: 0,
            schema_version: 0,
            onboarding_required: false,
            agents: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes_to_json() {
        let resp = HealthResponse::ready(3, 7, false);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ready\""));
        assert!(json.contains("\"active_sessions\":3"));
        assert!(json.contains("\"schema_version\":7"));
        assert!(json.contains("\"sqlite\":\"ok\""));
    }

    #[test]
    fn health_starting_status() {
        let resp = HealthResponse::starting();
        assert_eq!(resp.status, "starting");
        assert_eq!(resp.active_sessions, 0);
    }

    #[test]
    fn agent_binary_status_serializes() {
        let agent = AgentBinaryStatus {
            name: "claude".to_string(),
            path: Some("/usr/local/bin/claude".to_string()),
            version: Some("1.2.3".to_string()),
            status: "ok".to_string(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"name\":\"claude\""));
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn agent_not_found_status() {
        let agent = AgentBinaryStatus {
            name: "antigravity".to_string(),
            path: None,
            version: None,
            status: "not_found".to_string(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"not_found\""));
        assert!(json.contains("null"));
    }
}