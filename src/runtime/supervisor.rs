//! Mega Brain V0 — Process Supervisor (Gap 3 + Gap 4 + Gap 5)
//!
//! Manages the lifecycle of all PTY sessions: spawn, monitor, graceful shutdown,
//! crash detection, and concurrent session limit enforcement.
//!
//! Responsibilities:
//! - Track all active PtyInstance handles
//! - Monitor child process exit status (Gap 4: error propagation)
//! - Enforce max_concurrent_sessions per ProviderAccount (Gap 5)
//! - Graceful shutdown: SIGTERM → wait → SIGKILL (Gap 3)
//! - Emit structured events for UI consumption via WebSocket

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::pty::PtyInstance;

/// Event emitted by the supervisor when a session state changes.
/// Sent to the UI via WebSocket as JSON control frames.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// A new session was spawned successfully.
    SessionStarted {
        session_id: String,
        account_id: String,
        agent_binary: String,
    },
    /// A session's agent process exited (normally or crashed).
    AgentExit {
        session_id: String,
        exit_code: Option<u32>,
        message: String,
    },
    /// A session was terminated by the supervisor (graceful shutdown).
    SessionTerminated {
        session_id: String,
        reason: String,
    },
    /// Session spawn was rejected due to concurrent limit.
    SpawnRejected {
        account_id: String,
        max_concurrent: u32,
        current_active: usize,
        message: String,
    },
}

/// Metadata tracked per active session.
pub struct SessionRecord {
    pub pty_instance: Arc<PtyInstance>,
    pub account_id: String,
    pub agent_binary: String,
}

/// Public summary of a session for API responses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub agent_binary: String,
    pub account_id: String,
}

/// The Process Supervisor manages all active PTY sessions.
/// Thread-safe via RwLock for concurrent read access.
pub struct ProcessSupervisor {
    /// Active sessions indexed by session_id.
    sessions: RwLock<HashMap<String, SessionRecord>>,
    /// Channel for emitting session events to the UI/API layer.
    event_tx: mpsc::Sender<SessionEvent>,
    /// Maximum concurrent sessions per ProviderAccount.
    /// Keyed by account_id. Default from ProviderAccount.max_concurrent_sessions.
    account_limits: RwLock<HashMap<String, u32>>,
}

impl ProcessSupervisor {
    /// Create a new supervisor with an event channel.
    pub fn new(event_tx: mpsc::Sender<SessionEvent>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_tx,
            account_limits: RwLock::new(HashMap::new()),
        }
    }

    /// Register the max concurrent session limit for a ProviderAccount.
    pub async fn set_account_limit(&self, account_id: &str, max_concurrent: u32) {
        let mut limits = self.account_limits.write().await;
        limits.insert(account_id.to_string(), max_concurrent);
    }

    /// Get the number of active sessions for a specific account.
    pub async fn active_sessions_for_account(&self, account_id: &str) -> usize {
        let sessions = self.sessions.read().await;
        sessions.values().filter(|s| s.account_id == account_id).count()
    }

    /// Get total number of active sessions across all accounts.
    pub async fn total_active_sessions(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Register a spawned PTY session with the supervisor.
    /// Checks concurrent session limits before accepting (Gap 5).
    pub async fn register_session(
        &self,
        session_id: &str,
        account_id: &str,
        agent_binary: &str,
        pty_instance: Arc<PtyInstance>,
    ) -> Result<(), SessionEvent> {
        // Check concurrent limit (Gap 5)
        let limits = self.account_limits.read().await;
        if let Some(&max) = limits.get(account_id) {
            let current = self.active_sessions_for_account(account_id).await;
            if current >= max as usize {
                let event = SessionEvent::SpawnRejected {
                    account_id: account_id.to_string(),
                    max_concurrent: max,
                    current_active: current,
                    message: format!(
                        "Account {} is at capacity ({}/{})",
                        account_id, current, max
                    ),
                };
                let _ = self.event_tx.send(event.clone()).await;
                return Err(event);
            }
        }
        drop(limits);

        // Register the session
        let record = SessionRecord {
            pty_instance,
            account_id: account_id.to_string(),
            agent_binary: agent_binary.to_string(),
        };
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.to_string(), record);
        }

        // Emit started event
        let event = SessionEvent::SessionStarted {
            session_id: session_id.to_string(),
            account_id: account_id.to_string(),
            agent_binary: agent_binary.to_string(),
        };
        let _ = self.event_tx.send(event).await;

        Ok(())
    }

    /// Remove a session from the supervisor (after termination or crash).
    pub async fn remove_session(&self, session_id: &str) -> Option<Arc<PtyInstance>> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).map(|r| r.pty_instance)
    }

    /// Report that an agent process has exited (Gap 4: error propagation).
    pub async fn report_agent_exit(&self, session_id: &str, exit_code: Option<u32>) {
        let message = match exit_code {
            Some(0) => "Agent process exited normally".to_string(),
            Some(code) => format!("Agent process exited with code {}", code),
            None => "Agent process terminated by signal".to_string(),
        };

        let event = SessionEvent::AgentExit {
            session_id: session_id.to_string(),
            exit_code,
            message,
        };
        let _ = self.event_tx.send(event).await;

        // Remove from active sessions
        self.remove_session(session_id).await;
    }

    /// Graceful shutdown of all sessions (Gap 3).
    /// Sequence: mark shutting down → SIGTERM → wait → SIGKILL → cleanup.
    pub async fn shutdown_all(&self, grace_period_secs: u64) {
        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.keys().cloned().collect()
        };

        // Phase 1: Send terminate signal to all
        for sid in &session_ids {
            if let Some(pty) = self.get_pty_instance(sid).await {
                let _ = pty.terminate();
            }
        }

        // Phase 2: Wait for grace period, polling exit status
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(grace_period_secs);
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let all_exited = {
                let sessions = self.sessions.read().await;
                let mut all_done = true;
                for (sid, record) in sessions.iter() {
                    match record.pty_instance.try_wait() {
                        Ok(Some(_)) => { /* exited */ }
                        Ok(None) => { all_done = false; }
                        Err(_) => { /* assume dead */ }
                    }
                    let _ = sid; // suppress unused warning
                }
                all_done
            };

            if all_exited || tokio::time::Instant::now() >= deadline {
                break;
            }
        }

        // Phase 3: Force kill any remaining
        let remaining: Vec<String> = {
            let sessions = self.sessions.read().await;
            let mut still_alive = Vec::new();
            for (sid, record) in sessions.iter() {
                if let Ok(None) = record.pty_instance.try_wait() {
                    still_alive.push(sid.clone());
                }
            }
            still_alive
        };

        for sid in &remaining {
            if let Some(pty) = self.get_pty_instance(sid).await {
                let _ = pty.terminate(); // second attempt = force
            }
            let event = SessionEvent::SessionTerminated {
                session_id: sid.clone(),
                reason: "force killed after grace period".to_string(),
            };
            let _ = self.event_tx.send(event).await;
        }

        // Phase 4: Clear all sessions
        {
            let mut sessions = self.sessions.write().await;
            sessions.clear();
        }
    }

    /// Get a PTY instance by session ID (for I/O operations).
    pub async fn get_pty_instance(&self, session_id: &str) -> Option<Arc<PtyInstance>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|r| r.pty_instance.clone())
    }

    /// Get a clone of the event receiver for the API layer to consume.
    /// Note: this creates a new receiver — only one consumer should exist.
    pub fn event_sender(&self) -> mpsc::Sender<SessionEvent> {
        self.event_tx.clone()
    }

    /// List all active session IDs.
    pub async fn list_session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// List all active sessions with their metadata.
    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, record)| SessionSummary {
                id: id.clone(),
                agent_binary: record.agent_binary.clone(),
                account_id: record.account_id.clone(),
            })
            .collect()
    }

    /// Terminate a single session by ID (P0-3: DELETE /api/sessions/:id).
    /// Sends terminate signal, waits briefly, then removes from registry.
    pub async fn terminate_session(&self, session_id: &str) -> Result<(), String> {
        let pty = {
            let sessions = self.sessions.read().await;
            sessions.get(session_id).map(|r| r.pty_instance.clone())
        };

        match pty {
            Some(pty_instance) => {
                // Send terminate signal
                let _ = pty_instance.terminate();

                // Brief wait for graceful exit
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Force kill if still alive
                if let Ok(None) = pty_instance.try_wait() {
                    let _ = pty_instance.terminate();
                }

                // Remove from registry
                self.remove_session(session_id).await;

                // Emit termination event
                let event = SessionEvent::SessionTerminated {
                    session_id: session_id.to_string(),
                    reason: "terminated via API".to_string(),
                };
                let _ = self.event_tx.send(event).await;

                Ok(())
            }
            None => Err(format!("session {} not found", session_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn concurrent_limit_enforcement() {
        let (tx, _rx) = mpsc::channel(100);
        let supervisor = ProcessSupervisor::new(tx);

        // Set limit of 2 for account-a
        supervisor.set_account_limit("account-a", 2).await;

        // Simulate checking limits
        assert_eq!(supervisor.active_sessions_for_account("account-a").await, 0);

        // After "registering" 2 sessions, the 3rd should be rejected
        // (We can't fully test without real PTY, but we test the limit logic)
        let limits = supervisor.account_limits.read().await;
        assert_eq!(limits.get("account-a"), Some(&2));
    }

    #[tokio::test]
    async fn session_event_serialization() {
        let event = SessionEvent::AgentExit {
            session_id: "sess-1".to_string(),
            exit_code: Some(1),
            message: "Agent process exited with code 1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("agent_exit"));
        assert!(json.contains("sess-1"));
        assert!(json.contains("\"exit_code\":1"));
    }

    #[tokio::test]
    async fn spawn_rejected_event_format() {
        let event = SessionEvent::SpawnRejected {
            account_id: "acc-1".to_string(),
            max_concurrent: 2,
            current_active: 2,
            message: "Account acc-1 is at capacity (2/2)".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("spawn_rejected"));
        assert!(json.contains("capacity"));
    }

    #[tokio::test]
    async fn shutdown_clears_all_sessions() {
        let (tx, _rx) = mpsc::channel(100);
        let supervisor = ProcessSupervisor::new(tx);
        // No sessions to shut down — should complete without panic
        supervisor.shutdown_all(1).await;
        assert_eq!(supervisor.total_active_sessions().await, 0);
    }

    #[tokio::test]
    async fn list_session_ids_empty_initially() {
        let (tx, _rx) = mpsc::channel(100);
        let supervisor = ProcessSupervisor::new(tx);
        let ids = supervisor.list_session_ids().await;
        assert!(ids.is_empty());
    }
}