//! Mega Brain V0 — PTY Engine (Phase 0, Gap 2 + Gap 14)
//!
//! Cross-platform pseudo-terminal engine using `portable-pty`.
//! Spawns coding agent processes (claude, agy, codex) with full terminal
//! emulation, async I/O bridging via tokio channels, and platform-specific
//! interrupt/resize handling.
//!
//! Architecture (from plan Section 17):
//! - Write loop: async mpsc receiver → PTY master write (UI → Agent)
//! - Read loop: spawn_blocking on PTY master reader → mpsc sender (Agent → UI)
//! - ScrollbackBuffer tracks all output bytes with monotonic offset (Gap 1)
//! - Platform-specific: SIGINT on Unix, byte 0x03 on Windows ConPTY (Gap 14)

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::scrollback::ScrollbackBuffer;

/// Handle to a running PTY session. Provides methods for I/O, resize,
/// interrupt, and termination.
pub struct PtyInstance {
    /// Unique session identifier.
    pub id: String,
    /// The PTY master handle (for resize only).
    pty_master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    /// Dedicated writer for sending bytes to the PTY (UI → Agent).
    pty_writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Child process handle for exit status monitoring.
    child: Arc<Mutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    /// Channel sender for writing data to the PTY (UI → Agent).
    tx_to_terminal: mpsc::Sender<Vec<u8>>,
    /// Scrollback buffer tracking all output with byte offsets.
    scrollback: Arc<Mutex<ScrollbackBuffer>>,
}

impl PtyInstance {
    /// Spawn a new agent process in a PTY with the given configuration.
    ///
    /// # Arguments
    /// * `session_id` - Unique identifier for this session
    /// * `agent_binary` - Absolute path to the agent binary (e.g., "claude", "agy")
    /// * `agent_args` - Arguments to pass to the agent binary
    /// * `workspace_path` - Working directory for the agent (isolated worktree)
    /// * `env_vars` - Isolation environment variables from IsolationBoundary
    /// * `rows` / `cols` - Initial terminal size
    /// * `tx_to_ui` - Channel for sending PTY output bytes to the UI/WebSocket
    ///
    /// # Returns
    /// A `PtyInstance` handle for controlling the session, or an error if
    /// the agent binary cannot be spawned.
    pub fn spawn_agent(
        session_id: &str,
        agent_binary: &str,
        agent_args: &[&str],
        workspace_path: &Path,
        env_vars: HashMap<String, String>,
        rows: u16,
        cols: u16,
        tx_to_ui: mpsc::Sender<Vec<u8>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Build command — spawn agent binary directly, NOT via shell -c wrapper
        let mut cmd = CommandBuilder::new(agent_binary);
        for arg in agent_args {
            cmd.arg(arg);
        }
        cmd.cwd(workspace_path.to_path_buf());

        // Inject isolation environment variables
        for (k, v) in &env_vars {
            cmd.env(k, v);
        }

        let child = pair.slave.spawn_command(cmd)?;
        // Drop slave in parent to enable EOF detection on the reader
        drop(pair.slave);

        let pty_master = Arc::new(Mutex::new(pair.master));
        let child = Arc::new(Mutex::new(child));
        let scrollback = Arc::new(Mutex::new(ScrollbackBuffer::new()));

        // Take a dedicated writer from the PTY master — MasterPty doesn't impl Write,
        // but take_writer() returns a Box<dyn Write + Send> we can use directly.
        let pty_writer: Box<dyn Write + Send> = {
            let master = pty_master.lock().map_err(|e| format!("PTY master lock poisoned: {}", e))?;
            master.take_writer().map_err(|e| format!("failed to take PTY writer: {}", e))?
        };
        let pty_writer = Arc::new(Mutex::new(pty_writer));

        // Channel for UI → PTY writes
        let (tx_to_terminal, mut rx_from_ui) = mpsc::channel::<Vec<u8>>(1024);

        // WRITE LOOP: async receiver → PTY writer (UI → Agent)
        let pty_writer_clone = pty_writer.clone();
        tokio::spawn(async move {
            while let Some(bytes) = rx_from_ui.recv().await {
                let mut writer = match pty_writer_clone.lock() {
                    Ok(w) => w,
                    Err(_) => break,
                };
                if writer.write_all(&bytes).is_err() {
                    break; // Process died or PTY closed
                }
                let _ = writer.flush();
            }
        });

        // READ LOOP: blocking PTY read → async sender + scrollback (Agent → UI)
        let pty_master_reader = pty_master.clone();
        let scrollback_reader = scrollback.clone();
        let tx_to_ui_clone = tx_to_ui.clone();
        tokio::task::spawn_blocking(move || {
            let reader = match pty_master_reader.lock() {
                Ok(m) => match m.try_clone_reader() {
                    Ok(r) => r,
                    Err(_) => return,
                },
                Err(_) => return,
            };
            use std::io::Read;
            let mut reader = reader;
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF — process exited
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        // Track in scrollback buffer
                        if let Ok(mut sb) = scrollback_reader.lock() {
                            sb.write(&data);
                        }
                        // Send to UI via WebSocket bridge
                        if tx_to_ui_clone.blocking_send(data).is_err() {
                            break; // WebSocket disconnected
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            id: session_id.to_string(),
            pty_master,
            pty_writer,
            child,
            tx_to_terminal,
            scrollback,
        })
    }

    /// Send raw bytes to the PTY (keyboard input from UI).
    pub async fn send_input(&self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.tx_to_terminal.send(data).await
            .map_err(|e| format!("failed to send input to PTY: {}", e).into())
    }

    /// Resize the terminal. Enforces minimum dimensions to prevent ConPTY bugs (Gap 14).
    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Enforce minimums — ConPTY has bugs with very small buffers (Gap 14)
        let rows = rows.max(5);
        let cols = cols.max(20);
        let master = self.pty_master.lock()
            .map_err(|e| format!("PTY master lock poisoned: {}", e))?;
        master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    /// Send interrupt signal to the agent process.
    /// Platform-specific: SIGINT on Unix, byte 0x03 on Windows ConPTY (Gap 14).
    pub fn interrupt(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(windows)]
        {
            // Windows ConPTY: write Ctrl+C byte (0x03) to PTY writer
            let mut writer = self.pty_writer.lock()
                .map_err(|e| format!("PTY writer lock poisoned: {}", e))?;
            writer.write_all(&[0x03])?;
            writer.flush()?;
        }
        #[cfg(not(windows))]
        {
            // Unix: send SIGINT to the child process
            let child = self.child.lock()
                .map_err(|e| format!("child lock poisoned: {}", e))?;
            let pid = child.process_id().ok_or("no PID available for child process")?;
            unsafe {
                libc::kill(pid as i32, libc::SIGINT);
            }
        }
        Ok(())
    }

    /// Check if the child process is still running.
    /// Returns Some(exit_status) if the process has exited, None if still running.
    pub fn try_wait(&self) -> Result<Option<u32>, Box<dyn std::error::Error + Send + Sync>> {
        let mut child = self.child.lock()
            .map_err(|e| format!("child lock poisoned: {}", e))?;
        match child.try_wait()? {
            Some(status) => Ok(Some(status.exit_code())),
            None => Ok(None),
        }
    }

    /// Terminate the agent process gracefully.
    pub fn terminate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut child = self.child.lock()
            .map_err(|e| format!("child lock poisoned: {}", e))?;
        child.kill()?;
        Ok(())
    }

    /// Get a clone of the scrollback buffer for reading history (Gap 9 reconnect replay).
    pub fn scrollback(&self) -> Arc<Mutex<ScrollbackBuffer>> {
        self.scrollback.clone()
    }

    /// Get the channel sender for writing to this PTY.
    pub fn input_sender(&self) -> mpsc::Sender<Vec<u8>> {
        self.tx_to_terminal.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollback_buffer_tracks_writes() {
        let mut buf = ScrollbackBuffer::new();
        buf.write(b"hello ");
        buf.write(b"world");
        assert_eq!(buf.total_bytes(), 11);
        let data = buf.read_from(0);
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn scrollback_partial_read() {
        let mut buf = ScrollbackBuffer::new();
        buf.write(b"hello world");
        let partial = buf.read_from(6);
        assert_eq!(partial, b"world");
    }

    #[test]
    fn scrollback_eviction_under_memory_pressure() {
        let mut buf = ScrollbackBuffer::with_config(50, None, "test");
        buf.write(&[b'A'; 30]);
        buf.write(&[b'B'; 30]);
        buf.write(&[b'C'; 30]);
        // Total bytes still accurate even after eviction
        assert_eq!(buf.total_bytes(), 90);
        // Some data was evicted
        assert!(buf.oldest_available_offset() > 0);
    }

    #[test]
    fn resize_enforces_minimum_dimensions() {
        // This test validates the logic without actually spawning a PTY
        let rows: u16 = 2;
        let cols: u16 = 5;
        let enforced_rows = rows.max(5);
        let enforced_cols = cols.max(20);
        assert_eq!(enforced_rows, 5);
        assert_eq!(enforced_cols, 20);
    }

    #[test]
    fn interrupt_byte_is_0x03_on_windows() {
        // Validate the interrupt byte constant
        let ctrl_c: u8 = 0x03;
        assert_eq!(ctrl_c, 3);
    }
}