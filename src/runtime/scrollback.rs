//! Mega Brain V0 — Scrollback Buffer (Gap 1)
//!
//! Ring buffer in memory with disk overflow for terminal output history.
//! Prevents OOM when multiple PTY sessions generate continuous ANSI output.
//!
//! Design:
//! - In-memory ring buffer: configurable size (default 10MB per session)
//! - Disk overflow: when ring is full, oldest data is flushed to
//!   `~/.iahub/scrollback/<session_id>.log`
//! - Byte offset tracking: each byte written increments a monotonic offset,
//!   enabling WebSocket reconnect replay from a specific position (Gap 9)

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Default in-memory scrollback limit: 10 MB per session.
pub const DEFAULT_MEMORY_LIMIT: usize = 10 * 1024 * 1024;

/// Chunk size for disk overflow writes.
const DISK_CHUNK_SIZE: usize = 64 * 1024;

/// A scrollback buffer that stores terminal output bytes with monotonic
/// byte offset tracking. Supports in-memory ring buffer with disk overflow.
pub struct ScrollbackBuffer {
    /// Monotonically increasing byte offset — never resets, even after
    /// ring buffer wraps. Used for WebSocket reconnect replay.
    total_bytes_written: AtomicU64,

    /// In-memory ring buffer storing the most recent output.
    /// Each entry is a chunk of bytes with its starting offset.
    ring: VecDeque<RingEntry>,

    /// Current memory usage in bytes.
    memory_used: usize,

    /// Maximum memory before overflow to disk.
    memory_limit: usize,

    /// Path for disk overflow file (None = no disk overflow).
    overflow_path: Option<PathBuf>,

    /// Bytes written to disk overflow so far.
    disk_bytes_written: u64,

    /// Pending bytes waiting to be flushed to disk.
    pending_overflow: Vec<u8>,
}

struct RingEntry {
    start_offset: u64,
    data: Vec<u8>,
}

impl ScrollbackBuffer {
    /// Create a new scrollback buffer with default memory limit and no disk overflow.
    pub fn new() -> Self {
        Self {
            total_bytes_written: AtomicU64::new(0),
            ring: VecDeque::new(),
            memory_used: 0,
            memory_limit: DEFAULT_MEMORY_LIMIT,
            overflow_path: None,
            disk_bytes_written: 0,
            pending_overflow: Vec::new(),
        }
    }

    /// Create a new scrollback buffer with custom memory limit and optional disk overflow.
    pub fn with_config(memory_limit: usize, overflow_dir: Option<&Path>, session_id: &str) -> Self {
        let overflow_path = overflow_dir.map(|dir| dir.join(format!("{}.log", session_id)));
        Self {
            total_bytes_written: AtomicU64::new(0),
            ring: VecDeque::new(),
            memory_used: 0,
            memory_limit,
            overflow_path,
            disk_bytes_written: 0,
            pending_overflow: Vec::new(),
        }
    }

    /// Write bytes into the scrollback buffer. Returns the starting byte offset
    /// of this write within the total stream.
    pub fn write(&mut self, data: &[u8]) -> u64 {
        if data.is_empty() {
            return self.total_bytes_written.load(Ordering::SeqCst);
        }

        let start_offset = self.total_bytes_written.fetch_add(data.len() as u64, Ordering::SeqCst);

        // Add to ring buffer
        let entry = RingEntry {
            start_offset,
            data: data.to_vec(),
        };
        self.memory_used += data.len();
        self.ring.push_back(entry);

        // Evict oldest entries if over memory limit
        while self.memory_used > self.memory_limit && self.ring.len() > 1 {
            if let Some(evicted) = self.ring.pop_front() {
                self.memory_used -= evicted.data.len();
                // Queue for disk overflow
                if self.overflow_path.is_some() {
                    self.pending_overflow.extend_from_slice(&evicted.data);
                }
            }
        }

        // Flush to disk if pending overflow exceeds chunk size
        if self.pending_overflow.len() >= DISK_CHUNK_SIZE {
            let _ = self.flush_overflow();
        }

        start_offset
    }

    /// Read bytes from the scrollback buffer starting at a given byte offset.
    /// Returns all available bytes from that offset onward (up to what's in memory).
    /// If the requested offset has been evicted from memory, returns bytes from
    /// the oldest available offset.
    pub fn read_from(&self, from_offset: u64) -> Vec<u8> {
        let mut result = Vec::new();
        for entry in &self.ring {
            let entry_end = entry.start_offset + entry.data.len() as u64;
            if entry_end <= from_offset {
                continue; // entirely before our start point
            }
            if entry.start_offset >= from_offset {
                result.extend_from_slice(&entry.data);
            } else {
                // Partial overlap — skip the bytes before from_offset
                let skip = (from_offset - entry.start_offset) as usize;
                result.extend_from_slice(&entry.data[skip..]);
            }
        }
        result
    }

    /// Get the current total bytes written (monotonic offset).
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_written.load(Ordering::SeqCst)
    }

    /// Get the oldest byte offset still available in memory.
    pub fn oldest_available_offset(&self) -> u64 {
        self.ring.front().map(|e| e.start_offset).unwrap_or(0)
    }

    /// Get current memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.memory_used
    }

    /// Flush pending overflow data to disk. Returns Ok(()) on success or
    /// if no overflow path is configured.
    fn flush_overflow(&mut self) -> std::io::Result<()> {
        if self.pending_overflow.is_empty() {
            return Ok(());
        }
        if let Some(ref path) = self.overflow_path {
            use std::io::Write;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            file.write_all(&self.pending_overflow)?;
            self.disk_bytes_written += self.pending_overflow.len() as u64;
            self.pending_overflow.clear();
        }
        Ok(())
    }

    /// Force flush all pending data to disk. Call during graceful shutdown.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.flush_overflow()
    }
}

impl Default for ScrollbackBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        let mut buf = ScrollbackBuffer::new();
        let data = b"hello world from terminal";
        let offset = buf.write(data);
        assert_eq!(offset, 0);
        assert_eq!(buf.total_bytes(), data.len() as u64);

        let read_back = buf.read_from(0);
        assert_eq!(read_back, data);
    }

    #[test]
    fn monotonic_offset_tracking() {
        let mut buf = ScrollbackBuffer::new();
        let off1 = buf.write(b"first ");
        let off2 = buf.write(b"second");
        assert_eq!(off1, 0);
        assert_eq!(off2, 6);
        assert_eq!(buf.total_bytes(), 12);
    }

    #[test]
    fn read_from_partial_offset() {
        let mut buf = ScrollbackBuffer::new();
        buf.write(b"hello world");
        let partial = buf.read_from(6);
        assert_eq!(partial, b"world");
    }

    #[test]
    fn ring_eviction_respects_memory_limit() {
        let mut buf = ScrollbackBuffer::with_config(100, None, "test");
        // Write 150 bytes — should evict oldest entries
        buf.write(&[b'A'; 60]);
        buf.write(&[b'B'; 60]);
        buf.write(&[b'C'; 60]);

        // Memory should be at or under limit
        assert!(buf.memory_usage() <= 100 + 60); // last entry may push slightly over before eviction
        // Total bytes still tracks everything
        assert_eq!(buf.total_bytes(), 180);
        // Oldest available should be > 0 (some data evicted)
        assert!(buf.oldest_available_offset() > 0);
    }

    #[test]
    fn empty_write_returns_current_offset() {
        let mut buf = ScrollbackBuffer::new();
        buf.write(b"data");
        let off = buf.write(b"");
        assert_eq!(off, 4);
        assert_eq!(buf.total_bytes(), 4);
    }

    #[test]
    fn read_from_beyond_available_returns_empty() {
        let mut buf = ScrollbackBuffer::new();
        buf.write(b"short");
        let result = buf.read_from(1000);
        assert!(result.is_empty());
    }

    #[test]
    fn disk_overflow_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut buf = ScrollbackBuffer::with_config(50, Some(dir.path()), "sess-1");
        // Write enough to trigger eviction and overflow
        buf.write(&[b'X'; 30]);
        buf.write(&[b'Y'; 30]);
        buf.write(&[b'Z'; 30]);
        // Force flush
        buf.flush().unwrap();

        let overflow_file = dir.path().join("sess-1.log");
        assert!(overflow_file.exists(), "overflow file must be created");
        let content = std::fs::read(&overflow_file).unwrap();
        assert!(!content.is_empty(), "overflow file must contain evicted data");
    }
}