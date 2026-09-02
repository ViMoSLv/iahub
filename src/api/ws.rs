//! Mega Brain V0 — WebSocket PTY Bridge (Phase 1, Gap 9)
//!
//! Implements the IA-Hub Protocol v1 for WebSocket communication:
//! - Binary frames: terminal I/O (high throughput ANSI data)
//! - Text/JSON frames: control commands (resize, spawn, interrupt, reconnect)
//!
//! Reconnection protocol (Gap 9):
//! - Frontend sends: {"type": "reconnect", "session_id": "uuid", "last_byte_offset": N}
//! - Backend replays scrollback from offset N, then continues live stream

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::runtime::supervisor::ProcessSupervisor;

/// Control commands received as JSON text frames from the frontend.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlCommand {
    /// Resize the terminal for a specific session.
    Resize {
        session_id: String,
        rows: u16,
        cols: u16,
    },
    /// Interrupt the agent process (Ctrl+C).
    Interrupt {
        session_id: String,
    },
    /// Reconnect with scrollback replay from a specific byte offset.
    Reconnect {
        session_id: String,
        last_byte_offset: u64,
    },
}

/// Shared state for WebSocket handlers.
pub struct WsState {
    pub supervisor: Arc<ProcessSupervisor>,
}

/// Handle an upgraded WebSocket connection for a specific session.
/// Spawns two tasks:
/// - Send task: PTY output → WebSocket binary frames
/// - Receive task: WebSocket frames → PTY input or control commands
pub async fn handle_session_socket(
    socket: WebSocket,
    session_id: String,
    state: Arc<WsState>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Get the PTY instance for this session
    let pty_instance = match state.supervisor.get_pty_instance(&session_id).await {
        Some(pty) => pty,
        None => {
            // Session not found — send error and close
            let err = serde_json::json!({
                "type": "error",
                "message": format!("Session {} not found", session_id)
            });
            let _ = ws_sender.send(Message::Text(err.to_string())).await;
            return;
        }
    };

    // Channel for PTY output → WebSocket
    let (tx_pty_to_ws, mut rx_pty_to_ws) = mpsc::channel::<Vec<u8>>(2048);
    // Separate channel for reconnect replay data (higher priority)
    let (tx_replay, mut rx_replay) = mpsc::channel::<Vec<u8>>(64);

    // Shared offset for reconnect replay — updated by read_task, read by recv_task
    let shared_offset = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Spawn a task to read from PTY scrollback and forward new data
    let pty_for_read = pty_instance.clone();
    let session_id_clone = session_id.clone();
    let shared_offset_for_read = shared_offset.clone();
    let read_task = tokio::spawn(async move {
        let mut last_offset: u64 = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await; // ~60fps
            let scrollback = pty_for_read.scrollback();
            // Lock, extract data, then drop the guard BEFORE any await
            let data_opt = {
                let sb = match scrollback.lock() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let current_total = sb.total_bytes();
                if current_total > last_offset {
                    let data = sb.read_from(last_offset);
                    last_offset = current_total;
                    shared_offset_for_read.store(last_offset, std::sync::atomic::Ordering::SeqCst);
                    Some(data)
                } else {
                    None
                }
            }; // MutexGuard dropped here — safe to await below
            if let Some(data) = data_opt {
                if tx_pty_to_ws.send(data).await.is_err() {
                    break; // WebSocket closed
                }
            }
            // Check if process exited
            if let Ok(Some(exit_code)) = pty_for_read.try_wait() {
                // Send exit notification as JSON text frame
                let exit_msg = serde_json::json!({
                    "type": "agent_exit",
                    "session_id": session_id_clone,
                    "exit_code": exit_code,
                    "message": format!("Agent process exited with code {}", exit_code)
                });
                let _ = tx_pty_to_ws.send(exit_msg.to_string().into_bytes()).await;
                break;
            }
        }
    });

    // SEND TASK: merges replay channel (priority) + live PTY output → WebSocket binary frames
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Replay data has priority — drain it first on reconnect
                Some(replay_bytes) = rx_replay.recv() => {
                    if ws_sender.send(Message::Binary(replay_bytes)).await.is_err() {
                        break;
                    }
                }
                // Live PTY output
                Some(bytes) = rx_pty_to_ws.recv() => {
                    if ws_sender.send(Message::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    // RECEIVE TASK: WebSocket → PTY input or control commands
    let pty_for_input = pty_instance.clone();
    let supervisor_for_ctrl = state.supervisor.clone();
    let tx_replay_for_recv = tx_replay.clone();
    let mut recv_task = tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Terminal input from UI → PTY
                    let _ = pty_for_input.send_input(data.to_vec()).await;
                }
                Ok(Message::Text(text)) => {
                    // Control command from UI
                    if let Ok(cmd) = serde_json::from_str::<ControlCommand>(&text) {
                        match cmd {
                            ControlCommand::Resize { session_id: sid, rows, cols } => {
                                if let Some(pty) = supervisor_for_ctrl.get_pty_instance(&sid).await {
                                    let _ = pty.resize(rows, cols);
                                }
                            }
                            ControlCommand::Interrupt { session_id: sid } => {
                                if let Some(pty) = supervisor_for_ctrl.get_pty_instance(&sid).await {
                                    let _ = pty.interrupt();
                                }
                            }
                            ControlCommand::Reconnect { session_id: sid, last_byte_offset } => {
                                // Replay scrollback from the requested offset via replay channel
                                if let Some(pty) = supervisor_for_ctrl.get_pty_instance(&sid).await {
                                    let replay_data = {
                                        let scrollback = pty.scrollback();
                                        let sb = match scrollback.lock() {
                                            Ok(s) => s,
                                            Err(_) => continue,
                                        };
                                        sb.read_from(last_byte_offset)
                                    }; // MutexGuard dropped here
                                    if !replay_data.is_empty() {
                                        tracing::info!(
                                            session_id = %sid,
                                            bytes = replay_data.len(),
                                            from_offset = last_byte_offset,
                                            "scrollback replay sent"
                                        );
                                        // Chunk large replays to avoid blocking
                                        for chunk in replay_data.chunks(32 * 1024) {
                                            if tx_replay_for_recv.send(chunk.to_vec()).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    });

    // If either task finishes, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
        _ = read_task => {
            send_task.abort();
            recv_task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_command_resize_deserializes() {
        let json = r#"{"type": "resize", "session_id": "sess-1", "rows": 30, "cols": 120}"#;
        let cmd: ControlCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ControlCommand::Resize { session_id, rows, cols } => {
                assert_eq!(session_id, "sess-1");
                assert_eq!(rows, 30);
                assert_eq!(cols, 120);
            }
            _ => panic!("expected Resize variant"),
        }
    }

    #[test]
    fn control_command_interrupt_deserializes() {
        let json = r#"{"type": "interrupt", "session_id": "sess-2"}"#;
        let cmd: ControlCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ControlCommand::Interrupt { session_id } => {
                assert_eq!(session_id, "sess-2");
            }
            _ => panic!("expected Interrupt variant"),
        }
    }

    #[test]
    fn control_command_reconnect_deserializes() {
        let json = r#"{"type": "reconnect", "session_id": "sess-3", "last_byte_offset": 45230}"#;
        let cmd: ControlCommand = serde_json::from_str(json).unwrap();
        match cmd {
            ControlCommand::Reconnect { session_id, last_byte_offset } => {
                assert_eq!(session_id, "sess-3");
                assert_eq!(last_byte_offset, 45230);
            }
            _ => panic!("expected Reconnect variant"),
        }
    }

    #[test]
    fn unknown_control_command_fails_deserialization() {
        let json = r#"{"type": "unknown_cmd", "session_id": "x"}"#;
        let result: Result<ControlCommand, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown control command must fail closed");
    }
}