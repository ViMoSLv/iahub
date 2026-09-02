//! Mega Brain V0 — Agent Message Bus (Phase 2)
//!
//! Durable agent-to-agent messaging via SQLite. Messages are hints/coordination,
//! never source of truth (Principle 3: state not conversation).
//!
//! Message types: Handoff, StatusReport, ReviewRequest, ReworkRequest, Question, Answer
//! All messages carry from_session, to_session, payload, and lifecycle status.

use serde::{Deserialize, Serialize};

/// Kind of agent-to-agent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MessageKind {
    /// Transfer of work with artifact references.
    Handoff,
    /// Progress update from a worker.
    StatusReport,
    /// Request for independent review.
    ReviewRequest,
    /// Feedback requesting rework on a previous attempt.
    ReworkRequest,
    /// Blocking question from one agent to another.
    Question,
    /// Answer to a previous question.
    Answer,
}

impl std::fmt::Display for MessageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Handoff => write!(f, "HANDOFF"),
            Self::StatusReport => write!(f, "STATUS_REPORT"),
            Self::ReviewRequest => write!(f, "REVIEW_REQUEST"),
            Self::ReworkRequest => write!(f, "REWORK_REQUEST"),
            Self::Question => write!(f, "QUESTION"),
            Self::Answer => write!(f, "ANSWER"),
        }
    }
}

/// Delivery status of an agent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MessageStatus {
    /// Message created but not yet delivered to recipient.
    Pending,
    /// Message delivered to recipient's session.
    Delivered,
    /// Message read and acknowledged by recipient.
    Read,
}

/// A durable agent-to-agent message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message identifier.
    pub id: String,
    /// Session that sent this message.
    pub from_session: String,
    /// Session that should receive this message.
    pub to_session: String,
    /// What kind of message this is.
    pub kind: MessageKind,
    /// JSON payload carrying the message content (artifact IDs, text, etc.).
    pub payload: String,
    /// Current delivery status.
    pub status: MessageStatus,
    /// When the message was created (unix timestamp seconds).
    pub created_at: i64,
    /// When the message was delivered (None if still pending).
    pub delivered_at: Option<i64>,
}

impl AgentMessage {
    /// Create a new pending message.
    pub fn new(
        id: &str,
        from_session: &str,
        to_session: &str,
        kind: MessageKind,
        payload: &str,
        now: i64,
    ) -> Self {
        Self {
            id: id.to_string(),
            from_session: from_session.to_string(),
            to_session: to_session.to_string(),
            kind,
            payload: payload.to_string(),
            status: MessageStatus::Pending,
            created_at: now,
            delivered_at: None,
        }
    }

    /// Mark this message as delivered.
    pub fn mark_delivered(&mut self, at: i64) {
        self.status = MessageStatus::Delivered;
        self.delivered_at = Some(at);
    }

    /// Mark this message as read.
    pub fn mark_read(&mut self) {
        self.status = MessageStatus::Read;
    }

    /// Returns true if this message has been delivered or read.
    pub fn is_delivered(&self) -> bool {
        matches!(self.status, MessageStatus::Delivered | MessageStatus::Read)
    }
}

/// In-memory message bus for MVP. Will be backed by SQLite table in production.
/// Messages are durable within the process lifetime; SQLite persistence comes
/// when the orchestrator runtime wires into the persistence layer.
pub struct MessageBus {
    messages: Vec<AgentMessage>,
}

impl MessageBus {
    /// Create an empty message bus.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Send a message from one session to another.
    pub fn send(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }

    /// Get all pending messages for a specific session.
    pub fn pending_for(&self, session_id: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.to_session == session_id && m.status == MessageStatus::Pending)
            .collect()
    }

    /// Get all messages for a specific session (any status).
    pub fn all_for(&self, session_id: &str) -> Vec<&AgentMessage> {
        self.messages
            .iter()
            .filter(|m| m.to_session == session_id || m.from_session == session_id)
            .collect()
    }

    /// Mark a message as delivered by ID.
    pub fn deliver(&mut self, message_id: &str, at: i64) -> bool {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.mark_delivered(at);
            true
        } else {
            false
        }
    }

    /// Mark a message as read by ID.
    pub fn mark_read(&mut self, message_id: &str) -> bool {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.mark_read();
            true
        } else {
            false
        }
    }

    /// Total number of messages in the bus.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the bus is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_and_receive_pending() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("msg-1", "session-a", "session-b", MessageKind::Handoff, r#"{"artifacts":["art-1"]}"#, 1000);
        bus.send(msg);

        let pending = bus.pending_for("session-b");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "msg-1");
        assert_eq!(pending[0].kind, MessageKind::Handoff);
    }

    #[test]
    fn pending_not_visible_to_sender() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("msg-2", "session-a", "session-b", MessageKind::StatusReport, "{}", 1000);
        bus.send(msg);

        let pending = bus.pending_for("session-a");
        assert!(pending.is_empty(), "sender should not see their own message as pending");
    }

    #[test]
    fn deliver_changes_status() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("msg-3", "a", "b", MessageKind::Question, r#"{"q":"why?"}"#, 1000);
        bus.send(msg);

        assert!(bus.deliver("msg-3", 1001));
        let pending = bus.pending_for("b");
        assert!(pending.is_empty(), "delivered message should not appear as pending");

        let all = bus.all_for("b");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, MessageStatus::Delivered);
        assert_eq!(all[0].delivered_at, Some(1001));
    }

    #[test]
    fn mark_read_changes_status() {
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("msg-4", "a", "b", MessageKind::Answer, r#"{"a":"because"}"#, 1000);
        bus.send(msg);
        bus.deliver("msg-4", 1001);
        bus.mark_read("msg-4");

        let all = bus.all_for("b");
        assert_eq!(all[0].status, MessageStatus::Read);
    }

    #[test]
    fn message_kind_serialization_roundtrip() {
        let kinds = [
            MessageKind::Handoff,
            MessageKind::StatusReport,
            MessageKind::ReviewRequest,
            MessageKind::ReworkRequest,
            MessageKind::Question,
            MessageKind::Answer,
        ];
        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let back: MessageKind = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, kind);
        }
    }

    #[test]
    fn agent_message_serialization_roundtrip() {
        let msg = AgentMessage::new("msg-5", "sess-x", "sess-y", MessageKind::ReviewRequest, r#"{"candidate_sha":"abc123"}"#, 2000);
        let json = serde_json::to_string(&msg).unwrap();
        let back: AgentMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn unknown_message_kind_fails_deserialization() {
        let json = "\"MAGIC_KIND\"";
        let result: Result<MessageKind, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown MessageKind must fail closed");
    }

    #[test]
    fn inv_058_messages_are_never_source_of_truth() {
        // INV-058: Agent messages are hints/coordination, never authoritative state.
        // The message bus stores messages but does not mutate any domain entity.
        // Task/Attempt/Artifact state lives in SQLite, not in messages.
        let mut bus = MessageBus::new();
        let msg = AgentMessage::new("msg-inv", "a", "b", MessageKind::Handoff, "{}", 1000);
        bus.send(msg);

        // Bus only stores messages — it has no method to mutate Tasks or Attempts
        // This is a structural guarantee: MessageBus has no access to domain state
        assert_eq!(bus.len(), 1);
        assert!(bus.is_empty() == false);
    }

    #[test]
    fn deliver_nonexistent_returns_false() {
        let mut bus = MessageBus::new();
        assert!(!bus.deliver("nonexistent", 1000));
    }

    #[test]
    fn all_for_includes_sent_and_received() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("m1", "a", "b", MessageKind::Handoff, "{}", 1000));
        bus.send(AgentMessage::new("m2", "b", "a", MessageKind::Answer, "{}", 1001));

        let all_a = bus.all_for("a");
        assert_eq!(all_a.len(), 2, "session a should see both sent and received");
    }
}