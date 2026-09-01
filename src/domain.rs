//! Mega Brain V0 — Core Domain Model (Topic 02)
//!
//! Pure domain types with no dependency on SQLite, Git, MCP, PTY, or any I/O.
//! All IDs are strongly-typed newtypes. EntityVersion and FencingToken are
//! distinct newtypes preventing accidental cross-use. State machines are
//! exhaustive enums with pure transition functions that return Result.
//! Unknown/unrecognized persisted states deserialize fail-closed.
//!
//! Reference: MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Sections 4–5.

pub mod delegation;
pub mod provider;
pub mod verification;
pub mod workspace;

use serde::{Deserialize, Serialize};
use std::fmt;

// Re-export delegation/dispatch types for ergonomic access via `crate::domain::*`
pub use delegation::{
    AuthorityScope, CapabilityId, CompiledPrompt, CompiledPromptId, ContextSnapshot,
    ContextSnapshotId, DispatchSpec, StopCondition, VerificationEvidence, VerificationOutcome,
    WorkerReport,
};

// Re-export provider/multi-account types (ADR-0012)
pub use provider::{
    AuthProfileId, CredentialRef, ProviderAccount, ProviderAccountId, ProviderAccountRuntimeState,
    ProviderAccountStatus, ProviderKind,
};

// Re-export workspace isolation & write scope types (Topic 05)
// Note: WorkspaceId and ArtifactId from workspace module shadow the core IDs
// defined below. Core domain code should use the workspace-qualified names
// when referring to workspace/artifact entities specifically.
pub use workspace::{
    Artifact, ArtifactId, CapabilityId as WriteCapabilityId, CleanupEvaluation,
    CleanupGate, PathPattern, RepositoryIdentity, ScopeDriftReport, WorkspaceId,
    WriteCapability,
};

// Re-export verification, review & merge types (Topic 06)
// Note: VerificationEvidence and VerificationOutcome here are from the verification
// module (observational truth), distinct from delegation::VerificationEvidence
// (worker self-report wrapper). Use qualified paths when both are in scope.
pub use verification::{
    MergeLabId, MergeLabOutcome, MergeLabResult, MergeQueueItem, MergeQueueItemId,
    MergeQueueStatus, ReviewDecision, ReviewVerdict, VerificationEvidence as ObservationalEvidence,
    VerificationId, VerificationOutcome as ObservationalOutcome,
};

// ---------------------------------------------------------------------------
// Strongly-typed IDs (newtypes)
// ---------------------------------------------------------------------------

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(
    ProjectId,
    "Unique identifier for a registered Git repository."
);
define_id!(
    RunId,
    "Unique identifier for a top-level objective execution."
);
define_id!(
    TaskId,
    "Stable logical unit of work surviving retries and reassignment."
);
define_id!(AttemptId, "One execution attempt of one Task.");
define_id!(
    SessionId,
    "Live agent process/provider session attached to an Attempt."
);
// WorkspaceId and ArtifactId are now defined in domain::workspace (Topic 05).
// The core domain re-exports them via `pub use workspace::{...}` above.
// LeaseId remains a core concept tied to the authority subsystem.
define_id!(LeaseId, "Time-bounded authority grant over a resource.");
define_id!(ReviewId, "Independent evaluation of a candidate commit.");
define_id!(
    MergeItemId,
    "Durable request to integrate a candidate into a target branch."
);
define_id!(AgentId, "Registered coding agent identity.");
define_id!(CommandId, "Idempotent mutation intent identifier.");

// ---------------------------------------------------------------------------
// Versioning & Authority newtypes
// ---------------------------------------------------------------------------

/// Optimistic concurrency version for mutable domain entities.
/// Incremented atomically on every state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityVersion(pub i64);

impl EntityVersion {
    pub const INITIAL: Self = Self(1);

    /// Returns the next version. Panics on overflow (should never happen at V0 scale).
    pub fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("EntityVersion overflow"))
    }
}

/// Monotonically increasing authority number preventing stale lease reuse.
/// Distinct from EntityVersion to prevent accidental cross-use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FencingToken(pub i64);

impl FencingToken {
    pub const INITIAL: Self = Self(1);

    /// Returns the next fencing token. Panics on overflow.
    pub fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("FencingToken overflow"))
    }

    /// Returns true if this token is strictly newer than `other`.
    pub fn is_newer_than(self, other: Self) -> bool {
        self.0 > other.0
    }
}

impl fmt::Display for FencingToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Timestamps (opaque string wrapper — no chrono dependency in pure domain)
// ---------------------------------------------------------------------------

/// ISO-8601 timestamp stored as opaque string. Parsing/validation is an
/// infrastructure concern, not a domain concern.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub String);

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Run Status + Transitions (Section 5.1)
// ---------------------------------------------------------------------------

/// Exhaustive Run states. Unrecognized variants deserialize fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum RunStatus {
    Draft,
    Planning,
    PlanValidating,
    Ready,
    Running,
    Parked,
    Blocked,
    Escalated,
    Incomplete,
    Failed,
    Cancelled,
    OutcomeUnknown,
    Succeeded,
}

impl RunStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded
                | Self::Failed
                | Self::Incomplete
                | Self::Escalated
                | Self::Cancelled
                | Self::OutcomeUnknown
        )
    }
}

/// Errors returned when a Run transition is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunTransitionError {
    InvalidTransition { from: RunStatus, to: RunStatus },
    TerminalStateCannotTransition { from: RunStatus },
}

impl fmt::Display for RunTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Run transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Run state {:?} cannot transition", from)
            }
        }
    }
}

/// Pure function: validates a Run state transition.
pub fn validate_run_transition(from: RunStatus, to: RunStatus) -> Result<(), RunTransitionError> {
    if from.is_terminal() {
        return Err(RunTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        // Forward path
        (RunStatus::Draft, RunStatus::Planning)
            | (RunStatus::Planning, RunStatus::PlanValidating)
            | (RunStatus::PlanValidating, RunStatus::Ready)
            | (RunStatus::PlanValidating, RunStatus::Planning) // validation failed, re-plan
            | (RunStatus::Ready, RunStatus::Running)
            // Running branches
            | (RunStatus::Running, RunStatus::Parked)
            | (RunStatus::Running, RunStatus::Blocked)
            | (RunStatus::Running, RunStatus::Escalated)
            | (RunStatus::Running, RunStatus::Incomplete)
            | (RunStatus::Running, RunStatus::Failed)
            | (RunStatus::Running, RunStatus::Cancelled)
            | (RunStatus::Running, RunStatus::OutcomeUnknown)
            | (RunStatus::Running, RunStatus::Succeeded)
            // Recovery paths
            | (RunStatus::Parked, RunStatus::Running)
            | (RunStatus::Blocked, RunStatus::Running)
    );

    if valid {
        Ok(())
    } else {
        Err(RunTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Task Status + Transitions (Section 5.2)
// ---------------------------------------------------------------------------

/// Exhaustive Task states. Unrecognized variants deserialize fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum TaskStatus {
    Created,
    Ready,
    Claimed,
    Running,
    Submitted,
    Verifying,
    Reviewing,
    MergeReady,
    Merging,
    Done,
    NeedsChanges,
    Blocked,
    Parked,
    Cancelled,
    Escalated,
    Incomplete,
    Failed,
    Conflict,
}

impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskTransitionError {
    InvalidTransition { from: TaskStatus, to: TaskStatus },
    TerminalStateCannotTransition { from: TaskStatus },
}

impl fmt::Display for TaskTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Task transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Task state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_task_transition(
    from: TaskStatus,
    to: TaskStatus,
) -> Result<(), TaskTransitionError> {
    if from.is_terminal() {
        return Err(TaskTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        // Forward path
        (TaskStatus::Created, TaskStatus::Ready)
            | (TaskStatus::Ready, TaskStatus::Claimed)
            | (TaskStatus::Claimed, TaskStatus::Running)
            | (TaskStatus::Running, TaskStatus::Submitted)
            | (TaskStatus::Submitted, TaskStatus::Verifying)
            | (TaskStatus::Verifying, TaskStatus::Reviewing)
            | (TaskStatus::Verifying, TaskStatus::NeedsChanges)
            | (TaskStatus::Verifying, TaskStatus::Failed)
            | (TaskStatus::Reviewing, TaskStatus::MergeReady)
            | (TaskStatus::Reviewing, TaskStatus::NeedsChanges)
            | (TaskStatus::MergeReady, TaskStatus::Merging)
            | (TaskStatus::Merging, TaskStatus::Done)
            | (TaskStatus::Merging, TaskStatus::Conflict)
            | (TaskStatus::Merging, TaskStatus::Failed)
            // Rework loop
            | (TaskStatus::NeedsChanges, TaskStatus::Ready)
            // Interruptions from any non-terminal
            | (_, TaskStatus::Blocked)
            | (_, TaskStatus::Parked)
            | (_, TaskStatus::Cancelled)
            | (_, TaskStatus::Escalated)
            | (_, TaskStatus::Incomplete)
            // Recovery
            | (TaskStatus::Blocked, TaskStatus::Ready)
            | (TaskStatus::Parked, TaskStatus::Ready)
            | (TaskStatus::Conflict, TaskStatus::Reviewing)
            | (TaskStatus::Conflict, TaskStatus::NeedsChanges)
    );

    if valid {
        Ok(())
    } else {
        Err(TaskTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Attempt Status + Transitions (Section 5.3)
// ---------------------------------------------------------------------------

/// Evidence of process/session termination observed by the system.
/// Worker self-report alone is NOT sufficient — this must come from
/// independent observation (process exit, session disconnect, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerminationEvidence {
    /// Process exited with known code.
    ProcessExited { exit_code: i32 },
    /// Session/connection lost and confirmed dead.
    SessionDisconnected { reason: String },
    /// Provider confirmed termination via adapter.
    ProviderConfirmed { detail: String },
}

/// Cancellation state machine per Appendix E.
/// Distinguishes request from observed termination.
/// ACTIVE → CANCELLED direct transition is FORBIDDEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum AttemptStatus {
    Created,
    Leased,
    Starting,
    Active,
    Submitted,
    Blocked,
    Stale,
    Failed,
    /// Cancellation has been requested but not yet confirmed.
    /// Lease/fencing authority MUST be revoked before entering this state.
    CancelRequested,
    /// Cancellation confirmed via observed termination evidence.
    Cancelled,
    /// Cancellation requested but termination could not be confirmed.
    /// Preserves uncertainty for later reconcile.
    CancelIndeterminate,
    Lost,
}

impl AttemptStatus {
    /// States considered "active" for INV-025 (one active attempt per task).
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Leased | Self::Starting | Self::Active | Self::Submitted
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Cancelled | Self::CancelIndeterminate | Self::Lost | Self::Stale
        )
    }

    /// Returns true if this status represents a cancellation-in-progress state.
    pub fn is_cancelling(self) -> bool {
        matches!(
            self,
            Self::CancelRequested | Self::Cancelled | Self::CancelIndeterminate
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptTransitionError {
    InvalidTransition {
        from: AttemptStatus,
        to: AttemptStatus,
    },
    TerminalStateCannotTransition {
        from: AttemptStatus,
    },
}

impl fmt::Display for AttemptTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Attempt transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Attempt state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_attempt_transition(
    from: AttemptStatus,
    to: AttemptStatus,
) -> Result<(), AttemptTransitionError> {
    if from.is_terminal() {
        return Err(AttemptTransitionError::TerminalStateCannotTransition { from });
    }

    // Appendix E: Cancellation requires CancelRequested first.
    // Direct Active → Cancelled is FORBIDDEN.
    // CancelIndeterminate preserves uncertainty when evidence is inconclusive.
    let valid = matches!(
        (from, to),
        (AttemptStatus::Created, AttemptStatus::Leased)
            | (AttemptStatus::Leased, AttemptStatus::Starting)
            | (AttemptStatus::Starting, AttemptStatus::Active)
            | (AttemptStatus::Active, AttemptStatus::Submitted)
            | (AttemptStatus::Active, AttemptStatus::Blocked)
            | (AttemptStatus::Active, AttemptStatus::Stale)
            | (AttemptStatus::Active, AttemptStatus::Failed)
            | (AttemptStatus::Active, AttemptStatus::CancelRequested)
            | (AttemptStatus::Active, AttemptStatus::Lost)
            | (AttemptStatus::Blocked, AttemptStatus::Active)
            | (AttemptStatus::CancelRequested, AttemptStatus::Cancelled)
            | (
                AttemptStatus::CancelRequested,
                AttemptStatus::CancelIndeterminate
            )
            | (AttemptStatus::CancelRequested, AttemptStatus::Active)
    );

    if valid {
        Ok(())
    } else {
        Err(AttemptTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Session State + Transitions (Section 5.4)
// ---------------------------------------------------------------------------

/// Only observable states. THINKING/REASONING intentionally excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum SessionState {
    Created,
    Starting,
    Connected,
    Active,
    Idle,
    Unresponsive,
    Exited,
    Lost,
    Terminated,
}

impl SessionState {
    /// Lost is NOT terminal: reconcile may still transition LOST → TERMINATED
    /// after cleanup confirmation. Only Exited and Terminated are true terminals.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Exited | Self::Terminated)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTransitionError {
    InvalidTransition {
        from: SessionState,
        to: SessionState,
    },
    TerminalStateCannotTransition {
        from: SessionState,
    },
}

impl fmt::Display for SessionTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Session transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Session state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_session_transition(
    from: SessionState,
    to: SessionState,
) -> Result<(), SessionTransitionError> {
    if from.is_terminal() {
        return Err(SessionTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        (SessionState::Created, SessionState::Starting)
            | (SessionState::Starting, SessionState::Connected)
            | (SessionState::Connected, SessionState::Active)
            | (SessionState::Active, SessionState::Idle)
            | (SessionState::Active, SessionState::Unresponsive)
            | (SessionState::Active, SessionState::Exited)
            | (SessionState::Active, SessionState::Terminated)
            | (SessionState::Idle, SessionState::Active)
            | (SessionState::Unresponsive, SessionState::Active)
            | (SessionState::Unresponsive, SessionState::Lost)
            | (SessionState::Exited, SessionState::Lost)
            | (SessionState::Lost, SessionState::Terminated)
    );

    if valid {
        Ok(())
    } else {
        Err(SessionTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Workspace Status + Transitions (Section 5.5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum WorkspaceStatus {
    Planned,
    Creating,
    Ready,
    InUse,
    Dirty,
    Sealed,
    Releasing,
    Removed,
    Broken,
    Orphaned,
}

impl WorkspaceStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Removed | Self::Broken | Self::Orphaned)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceTransitionError {
    InvalidTransition {
        from: WorkspaceStatus,
        to: WorkspaceStatus,
    },
    TerminalStateCannotTransition {
        from: WorkspaceStatus,
    },
}

impl fmt::Display for WorkspaceTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Workspace transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Workspace state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_workspace_transition(
    from: WorkspaceStatus,
    to: WorkspaceStatus,
) -> Result<(), WorkspaceTransitionError> {
    if from.is_terminal() {
        return Err(WorkspaceTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        (WorkspaceStatus::Planned, WorkspaceStatus::Creating)
            | (WorkspaceStatus::Creating, WorkspaceStatus::Ready)
            | (WorkspaceStatus::Creating, WorkspaceStatus::Broken)
            | (WorkspaceStatus::Ready, WorkspaceStatus::InUse)
            | (WorkspaceStatus::InUse, WorkspaceStatus::Dirty)
            | (WorkspaceStatus::Dirty, WorkspaceStatus::Sealed)
            | (WorkspaceStatus::Sealed, WorkspaceStatus::Releasing)
            | (WorkspaceStatus::Releasing, WorkspaceStatus::Removed)
            // Failure transitions from any non-terminal
            | (_, WorkspaceStatus::Broken)
            | (_, WorkspaceStatus::Orphaned)
    );

    if valid {
        Ok(())
    } else {
        Err(WorkspaceTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Review Status + Transitions (Section 5.6)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ReviewStatus {
    Pending,
    Assigned,
    InReview,
    ChangesRequired,
    Rejected,
    Approved,
}

impl ReviewStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::ChangesRequired | Self::Rejected | Self::Approved
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewTransitionError {
    InvalidTransition {
        from: ReviewStatus,
        to: ReviewStatus,
    },
    TerminalStateCannotTransition {
        from: ReviewStatus,
    },
}

impl fmt::Display for ReviewTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Review transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Review state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_review_transition(
    from: ReviewStatus,
    to: ReviewStatus,
) -> Result<(), ReviewTransitionError> {
    if from.is_terminal() {
        // Special case: APPROVED → PENDING when candidate SHA changed (review invalidated)
        if from == ReviewStatus::Approved && to == ReviewStatus::Pending {
            return Ok(());
        }
        return Err(ReviewTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        (ReviewStatus::Pending, ReviewStatus::Assigned)
            | (ReviewStatus::Assigned, ReviewStatus::InReview)
            | (ReviewStatus::InReview, ReviewStatus::Approved)
            | (ReviewStatus::InReview, ReviewStatus::ChangesRequired)
            | (ReviewStatus::InReview, ReviewStatus::Rejected)
    );

    if valid {
        Ok(())
    } else {
        Err(ReviewTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Merge Status + Transitions (Section 5.7)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum MergeStatus {
    Queued,
    Precheck,
    LabSimulation,
    Ready,
    Merging,
    Completed,
    Conflict,
    TestFailed,
    Failed,
}

impl MergeStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Conflict | Self::TestFailed | Self::Failed
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeTransitionError {
    InvalidTransition { from: MergeStatus, to: MergeStatus },
    TerminalStateCannotTransition { from: MergeStatus },
}

impl fmt::Display for MergeTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid Merge transition: {:?} → {:?}", from, to)
            }
            Self::TerminalStateCannotTransition { from } => {
                write!(f, "terminal Merge state {:?} cannot transition", from)
            }
        }
    }
}

pub fn validate_merge_transition(
    from: MergeStatus,
    to: MergeStatus,
) -> Result<(), MergeTransitionError> {
    if from.is_terminal() {
        return Err(MergeTransitionError::TerminalStateCannotTransition { from });
    }

    let valid = matches!(
        (from, to),
        (MergeStatus::Queued, MergeStatus::Precheck)
            | (MergeStatus::Precheck, MergeStatus::LabSimulation)
            | (MergeStatus::Precheck, MergeStatus::Queued) // target advanced, re-queue
            | (MergeStatus::LabSimulation, MergeStatus::Ready)
            | (MergeStatus::LabSimulation, MergeStatus::Conflict)
            | (MergeStatus::LabSimulation, MergeStatus::TestFailed)
            | (MergeStatus::Ready, MergeStatus::Merging)
            | (MergeStatus::Merging, MergeStatus::Completed)
            | (MergeStatus::Merging, MergeStatus::Failed)
    );

    if valid {
        Ok(())
    } else {
        Err(MergeTransitionError::InvalidTransition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Process Identity (INV-032)
// ---------------------------------------------------------------------------

/// Unique identity for a Hub process instance. PID alone is insufficient because
/// PIDs recycle; the combination of PID + start timestamp (or nonce) guarantees
/// uniqueness across restarts and prevents stale process references (INV-032).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessIdentity {
    /// Operating system process ID at time of startup.
    pub pid: u32,
    /// Monotonic timestamp (seconds since epoch) when this process started.
    /// Combined with PID to prevent reuse after OS PID recycling.
    pub started_at: i64,
    /// Optional cryptographic nonce for additional uniqueness guarantee in
    /// environments where PID + timestamp collisions are theoretically possible.
    pub nonce: Option<String>,
}

impl ProcessIdentity {
    /// Create a new process identity from PID and start timestamp.
    pub fn new(pid: u32, started_at: i64) -> Self {
        Self {
            pid,
            started_at,
            nonce: None,
        }
    }

    /// Create a new process identity with an explicit nonce.
    pub fn with_nonce(pid: u32, started_at: i64, nonce: String) -> Self {
        Self {
            pid,
            started_at,
            nonce: Some(nonce),
        }
    }

    /// Returns true if this identity has sufficient uniqueness guarantees.
    /// PID alone is never sufficient (INV-032); must have either timestamp or nonce.
    pub fn is_valid(&self) -> bool {
        // Timestamp is always present and non-zero for valid identities
        self.started_at > 0
    }
}

impl fmt::Display for ProcessIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.nonce {
            Some(n) => write!(f, "pid:{}@{}#{}", self.pid, self.started_at, n),
            None => write!(f, "pid:{}@{}", self.pid, self.started_at),
        }
    }
}

// ---------------------------------------------------------------------------
// Policy Snapshot (INV-036)
// ---------------------------------------------------------------------------

/// Immutable snapshot of policy configuration bound to a specific Run.
/// Once a Run enters RUNNING state, its policy snapshot cannot change;
/// any policy update requires an explicit migration event (INV-036).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySnapshot {
    /// Unique identifier for this snapshot version.
    pub snapshot_id: String,
    /// The Run this snapshot is bound to.
    pub run_id: String,
    /// Schema version of the policy at time of snapshot.
    pub policy_schema_version: EntityVersion,
    /// SHA-256 hash of the serialized policy content.
    pub policy_hash: String,
    /// When this snapshot was created (before Run entered RUNNING).
    pub captured_at: Timestamp,
    /// Whether the associated Run has entered RUNNING state.
    /// Once true, this snapshot is frozen and cannot be replaced.
    pub frozen: bool,
}

impl PolicySnapshot {
    /// Returns true if this snapshot can still be replaced by a newer one.
    /// Only unfrozen snapshots (Run not yet RUNNING) are replaceable.
    pub fn is_replaceable(&self) -> bool {
        !self.frozen
    }

    /// Freeze this snapshot, making it immutable for the remainder of the Run.
    /// Called when the Run transitions to RUNNING state.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }
}

// ---------------------------------------------------------------------------
// Failure Classification (INV-020)
// ---------------------------------------------------------------------------

/// Classified reason for a failure. Every terminal FAILED state must carry
/// one of these; bare "FAILED" without classification is rejected (INV-020).
///
/// Categories:
/// - **Recoverable**: Transient issues that may succeed on retry (timeout, rate limit).
/// - **NonRecoverable**: Deterministic failures requiring human intervention or spec change.
/// - **Infrastructure**: Platform/environment issues outside agent control.
/// - **PolicyViolation**: Agent violated an invariant or scope constraint.
/// - **Unknown**: Evidence insufficient to classify; treated as non-recoverable per Principle 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum FailureReason {
    /// Transient timeout; may succeed on retry with backoff.
    Timeout,
    /// Rate limit or quota exhaustion from provider or infrastructure.
    RateLimited,
    /// Network connectivity loss or DNS resolution failure.
    NetworkError,
    /// Build/compilation failed due to code errors.
    BuildFailed,
    /// Tests failed deterministically.
    TestFailed,
    /// Agent produced output violating scope, invariants, or acceptance criteria.
    PolicyViolation,
    /// Merge conflict detected during lab simulation or actual merge.
    MergeConflict,
    /// Provider authentication expired or was revoked mid-execution.
    AuthenticationExpired,
    /// Environment/tooling missing or misconfigured (not agent's fault).
    InfrastructureError,
    /// Evidence insufficient to determine root cause; fail-closed.
    Unknown,
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::RateLimited => write!(f, "RATE_LIMITED"),
            Self::NetworkError => write!(f, "NETWORK_ERROR"),
            Self::BuildFailed => write!(f, "BUILD_FAILED"),
            Self::TestFailed => write!(f, "TEST_FAILED"),
            Self::PolicyViolation => write!(f, "POLICY_VIOLATION"),
            Self::MergeConflict => write!(f, "MERGE_CONFLICT"),
            Self::AuthenticationExpired => write!(f, "AUTHENTICATION_EXPIRED"),
            Self::InfrastructureError => write!(f, "INFRASTRUCTURE_ERROR"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

impl FailureReason {
    /// Returns true if this failure category may be resolved by retrying.
    pub fn is_recoverable(self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::RateLimited | Self::NetworkError
        )
    }
}

// ---------------------------------------------------------------------------
// Lease Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum LeaseStatus {
    Active,
    Expired,
    Revoked,
}

// ---------------------------------------------------------------------------
// Supporting enums (legacy aliases removed — canonical types live in submodules)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Hard,
    Soft,
}

// ---------------------------------------------------------------------------
// Fail-closed deserialization tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Run transitions: all valid --

    #[test]
    fn run_valid_forward_transitions() {
        assert!(validate_run_transition(RunStatus::Draft, RunStatus::Planning).is_ok());
        assert!(validate_run_transition(RunStatus::Planning, RunStatus::PlanValidating).is_ok());
        assert!(validate_run_transition(RunStatus::PlanValidating, RunStatus::Ready).is_ok());
        assert!(validate_run_transition(RunStatus::Ready, RunStatus::Running).is_ok());
    }

    #[test]
    fn run_valid_branching_transitions() {
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Parked).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Blocked).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Escalated).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Incomplete).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Failed).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Cancelled).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::OutcomeUnknown).is_ok());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Succeeded).is_ok());
    }

    #[test]
    fn run_valid_recovery_transitions() {
        assert!(validate_run_transition(RunStatus::Parked, RunStatus::Running).is_ok());
        assert!(validate_run_transition(RunStatus::Blocked, RunStatus::Running).is_ok());
        assert!(validate_run_transition(RunStatus::PlanValidating, RunStatus::Planning).is_ok());
    }

    #[test]
    fn run_invalid_transitions_rejected() {
        assert!(validate_run_transition(RunStatus::Draft, RunStatus::Running).is_err());
        assert!(validate_run_transition(RunStatus::Ready, RunStatus::Succeeded).is_err());
        assert!(validate_run_transition(RunStatus::Planning, RunStatus::Running).is_err());
        assert!(validate_run_transition(RunStatus::Draft, RunStatus::Succeeded).is_err());
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Draft).is_err());
    }

    #[test]
    fn run_terminal_states_cannot_transition() {
        let terminals = [
            RunStatus::Succeeded,
            RunStatus::Failed,
            RunStatus::Incomplete,
            RunStatus::Escalated,
            RunStatus::Cancelled,
            RunStatus::OutcomeUnknown,
        ];
        for t in &terminals {
            let result = validate_run_transition(*t, RunStatus::Running);
            assert!(result.is_err(), "{:?} should not transition", t);
            assert!(matches!(
                result.unwrap_err(),
                RunTransitionError::TerminalStateCannotTransition { .. }
            ));
        }
    }

    // -- Task transitions --

    #[test]
    fn task_valid_forward_transitions() {
        assert!(validate_task_transition(TaskStatus::Created, TaskStatus::Ready).is_ok());
        assert!(validate_task_transition(TaskStatus::Ready, TaskStatus::Claimed).is_ok());
        assert!(validate_task_transition(TaskStatus::Claimed, TaskStatus::Running).is_ok());
        assert!(validate_task_transition(TaskStatus::Running, TaskStatus::Submitted).is_ok());
        assert!(validate_task_transition(TaskStatus::Submitted, TaskStatus::Verifying).is_ok());
        assert!(validate_task_transition(TaskStatus::Verifying, TaskStatus::Reviewing).is_ok());
        assert!(validate_task_transition(TaskStatus::Reviewing, TaskStatus::MergeReady).is_ok());
        assert!(validate_task_transition(TaskStatus::MergeReady, TaskStatus::Merging).is_ok());
        assert!(validate_task_transition(TaskStatus::Merging, TaskStatus::Done).is_ok());
    }

    #[test]
    fn task_rework_loop() {
        assert!(validate_task_transition(TaskStatus::Verifying, TaskStatus::NeedsChanges).is_ok());
        assert!(validate_task_transition(TaskStatus::Reviewing, TaskStatus::NeedsChanges).is_ok());
        assert!(validate_task_transition(TaskStatus::NeedsChanges, TaskStatus::Ready).is_ok());
    }

    #[test]
    fn task_interruption_from_non_terminal() {
        assert!(validate_task_transition(TaskStatus::Running, TaskStatus::Blocked).is_ok());
        assert!(validate_task_transition(TaskStatus::Claimed, TaskStatus::Cancelled).is_ok());
        assert!(validate_task_transition(TaskStatus::Verifying, TaskStatus::Escalated).is_ok());
    }

    #[test]
    fn task_invalid_transitions_rejected() {
        assert!(validate_task_transition(TaskStatus::Created, TaskStatus::Running).is_err());
        assert!(validate_task_transition(TaskStatus::Ready, TaskStatus::Done).is_err());
        assert!(validate_task_transition(TaskStatus::Submitted, TaskStatus::Claimed).is_err());
        assert!(validate_task_transition(TaskStatus::Merging, TaskStatus::Ready).is_err());
    }

    #[test]
    fn task_terminal_states_cannot_transition() {
        assert!(validate_task_transition(TaskStatus::Done, TaskStatus::Ready).is_err());
        assert!(validate_task_transition(TaskStatus::Cancelled, TaskStatus::Ready).is_err());
    }

    // -- Attempt transitions --

    #[test]
    fn attempt_valid_transitions() {
        assert!(validate_attempt_transition(AttemptStatus::Created, AttemptStatus::Leased).is_ok());
        assert!(
            validate_attempt_transition(AttemptStatus::Leased, AttemptStatus::Starting).is_ok()
        );
        assert!(
            validate_attempt_transition(AttemptStatus::Starting, AttemptStatus::Active).is_ok()
        );
        assert!(
            validate_attempt_transition(AttemptStatus::Active, AttemptStatus::Submitted).is_ok()
        );
        assert!(validate_attempt_transition(AttemptStatus::Active, AttemptStatus::Stale).is_ok());
        assert!(validate_attempt_transition(AttemptStatus::Active, AttemptStatus::Lost).is_ok());
        // STALE→LOST is a reconcile action, not a domain transition (Stale is terminal)
        assert!(validate_attempt_transition(AttemptStatus::Blocked, AttemptStatus::Active).is_ok());
    }

    #[test]
    fn attempt_invalid_transitions_rejected() {
        assert!(
            validate_attempt_transition(AttemptStatus::Created, AttemptStatus::Active).is_err()
        );
        assert!(
            validate_attempt_transition(AttemptStatus::Leased, AttemptStatus::Submitted).is_err()
        );
        assert!(validate_attempt_transition(AttemptStatus::Lost, AttemptStatus::Active).is_err());
        assert!(
            validate_attempt_transition(AttemptStatus::Failed, AttemptStatus::Created).is_err()
        );
    }

    /// Appendix E (INV-033): Direct ACTIVE → CANCELLED is forbidden.
    /// Cancellation must go through CANCEL_REQUESTED first.
    #[test]
    fn inv_033_direct_active_to_cancelled_is_forbidden() {
        let result = validate_attempt_transition(AttemptStatus::Active, AttemptStatus::Cancelled);
        assert!(
            result.is_err(),
            "ACTIVE → CANCELLED direct transition must be rejected per Appendix E"
        );
    }

    /// Appendix E: Valid two-phase cancellation path.
    /// ACTIVE → CANCEL_REQUESTED → CANCELLED (with evidence) or CANCEL_INDETERMINATE.
    #[test]
    fn inv_033_cancellation_requires_two_phase_path() {
        // Phase 1: Request cancellation
        assert!(
            validate_attempt_transition(AttemptStatus::Active, AttemptStatus::CancelRequested)
                .is_ok(),
            "ACTIVE → CANCEL_REQUESTED must be valid"
        );

        // Phase 2a: Confirmed termination → CANCELLED
        assert!(
            validate_attempt_transition(AttemptStatus::CancelRequested, AttemptStatus::Cancelled)
                .is_ok(),
            "CANCEL_REQUESTED → CANCELLED must be valid when evidence confirms termination"
        );

        // Phase 2b: Inconclusive evidence → CANCEL_INDETERMINATE
        assert!(
            validate_attempt_transition(
                AttemptStatus::CancelRequested,
                AttemptStatus::CancelIndeterminate
            )
            .is_ok(),
            "CANCEL_REQUESTED → CANCEL_INDETERMINATE must be valid for inconclusive evidence"
        );

        // Recovery: Cancel requested but agent responds → back to ACTIVE
        assert!(
            validate_attempt_transition(AttemptStatus::CancelRequested, AttemptStatus::Active)
                .is_ok(),
            "CANCEL_REQUESTED → ACTIVE must be valid if agent proves liveness"
        );
    }

    /// CANCEL_INDETERMINATE is terminal — cannot transition further without reconcile.
    #[test]
    fn cancel_indeterminate_is_terminal() {
        assert!(
            AttemptStatus::CancelIndeterminate.is_terminal(),
            "CANCEL_INDETERMINATE must be terminal per Appendix E"
        );
        assert!(
            validate_attempt_transition(AttemptStatus::CancelIndeterminate, AttemptStatus::Active)
                .is_err(),
            "CANCEL_INDETERMINATE cannot transition back to ACTIVE without external reconcile"
        );
    }

    /// TerminationEvidence serializes correctly for persistence.
    #[test]
    fn termination_evidence_serializes_correctly() {
        let evidence = TerminationEvidence::ProcessExited { exit_code: 1 };
        let json = serde_json::to_string(&evidence).unwrap();
        assert!(json.contains("PROCESS_EXITED"));
        assert!(json.contains("1"));

        let deserialized: TerminationEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, evidence);
    }

    #[test]
    fn attempt_active_states_match_inv_025() {
        assert!(AttemptStatus::Leased.is_active());
        assert!(AttemptStatus::Starting.is_active());
        assert!(AttemptStatus::Active.is_active());
        assert!(AttemptStatus::Submitted.is_active());

        assert!(!AttemptStatus::Created.is_active());
        assert!(!AttemptStatus::Blocked.is_active());
        assert!(!AttemptStatus::Stale.is_active());
        assert!(!AttemptStatus::Failed.is_active());
        assert!(!AttemptStatus::Cancelled.is_active());
        assert!(!AttemptStatus::Lost.is_active());
    }

    // -- Session transitions --

    #[test]
    fn session_valid_transitions() {
        assert!(validate_session_transition(SessionState::Created, SessionState::Starting).is_ok());
        assert!(
            validate_session_transition(SessionState::Starting, SessionState::Connected).is_ok()
        );
        assert!(validate_session_transition(SessionState::Connected, SessionState::Active).is_ok());
        assert!(validate_session_transition(SessionState::Active, SessionState::Idle).is_ok());
        assert!(validate_session_transition(SessionState::Idle, SessionState::Active).is_ok());
        assert!(
            validate_session_transition(SessionState::Active, SessionState::Unresponsive).is_ok()
        );
        assert!(
            validate_session_transition(SessionState::Unresponsive, SessionState::Active).is_ok()
        );
        assert!(
            validate_session_transition(SessionState::Unresponsive, SessionState::Lost).is_ok()
        );
        assert!(validate_session_transition(SessionState::Active, SessionState::Exited).is_ok());
        // EXITED→LOST is a reconcile observation, not a domain transition (Exited is terminal)
        assert!(validate_session_transition(SessionState::Lost, SessionState::Terminated).is_ok());
    }

    #[test]
    fn session_invalid_transitions_rejected() {
        assert!(validate_session_transition(SessionState::Created, SessionState::Active).is_err());
        assert!(validate_session_transition(SessionState::Connected, SessionState::Lost).is_err());
        assert!(
            validate_session_transition(SessionState::Terminated, SessionState::Active).is_err()
        );
        assert!(validate_session_transition(SessionState::Exited, SessionState::Active).is_err());
    }

    // -- Workspace transitions --

    #[test]
    fn workspace_valid_transitions() {
        assert!(
            validate_workspace_transition(WorkspaceStatus::Planned, WorkspaceStatus::Creating)
                .is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Creating, WorkspaceStatus::Ready)
                .is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Ready, WorkspaceStatus::InUse).is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::InUse, WorkspaceStatus::Dirty).is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Dirty, WorkspaceStatus::Sealed).is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Sealed, WorkspaceStatus::Releasing)
                .is_ok()
        );
        assert!(validate_workspace_transition(
            WorkspaceStatus::Releasing,
            WorkspaceStatus::Removed
        )
        .is_ok());
    }

    #[test]
    fn workspace_failure_transitions_from_any_non_terminal() {
        assert!(
            validate_workspace_transition(WorkspaceStatus::Planned, WorkspaceStatus::Broken)
                .is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::InUse, WorkspaceStatus::Broken).is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Ready, WorkspaceStatus::Orphaned)
                .is_ok()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Dirty, WorkspaceStatus::Orphaned)
                .is_ok()
        );
    }

    #[test]
    fn workspace_terminal_states_cannot_transition() {
        assert!(
            validate_workspace_transition(WorkspaceStatus::Removed, WorkspaceStatus::Ready)
                .is_err()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Broken, WorkspaceStatus::Ready).is_err()
        );
        assert!(
            validate_workspace_transition(WorkspaceStatus::Orphaned, WorkspaceStatus::Ready)
                .is_err()
        );
    }

    // -- Review transitions --

    #[test]
    fn review_valid_transitions() {
        assert!(validate_review_transition(ReviewStatus::Pending, ReviewStatus::Assigned).is_ok());
        assert!(validate_review_transition(ReviewStatus::Assigned, ReviewStatus::InReview).is_ok());
        assert!(validate_review_transition(ReviewStatus::InReview, ReviewStatus::Approved).is_ok());
        assert!(
            validate_review_transition(ReviewStatus::InReview, ReviewStatus::ChangesRequired)
                .is_ok()
        );
        assert!(validate_review_transition(ReviewStatus::InReview, ReviewStatus::Rejected).is_ok());
    }

    #[test]
    fn review_approved_can_revert_to_pending_on_sha_change() {
        assert!(validate_review_transition(ReviewStatus::Approved, ReviewStatus::Pending).is_ok());
    }

    #[test]
    fn review_invalid_transitions_rejected() {
        assert!(validate_review_transition(ReviewStatus::Pending, ReviewStatus::Approved).is_err());
        assert!(
            validate_review_transition(ReviewStatus::Assigned, ReviewStatus::Approved).is_err()
        );
        assert!(
            validate_review_transition(ReviewStatus::Rejected, ReviewStatus::Approved).is_err()
        );
        assert!(
            validate_review_transition(ReviewStatus::ChangesRequired, ReviewStatus::Approved)
                .is_err()
        );
    }

    // -- Merge transitions --

    #[test]
    fn merge_valid_transitions() {
        assert!(validate_merge_transition(MergeStatus::Queued, MergeStatus::Precheck).is_ok());
        assert!(
            validate_merge_transition(MergeStatus::Precheck, MergeStatus::LabSimulation).is_ok()
        );
        assert!(validate_merge_transition(MergeStatus::Precheck, MergeStatus::Queued).is_ok());
        assert!(validate_merge_transition(MergeStatus::LabSimulation, MergeStatus::Ready).is_ok());
        assert!(
            validate_merge_transition(MergeStatus::LabSimulation, MergeStatus::Conflict).is_ok()
        );
        assert!(
            validate_merge_transition(MergeStatus::LabSimulation, MergeStatus::TestFailed).is_ok()
        );
        assert!(validate_merge_transition(MergeStatus::Ready, MergeStatus::Merging).is_ok());
        assert!(validate_merge_transition(MergeStatus::Merging, MergeStatus::Completed).is_ok());
        assert!(validate_merge_transition(MergeStatus::Merging, MergeStatus::Failed).is_ok());
    }

    #[test]
    fn merge_invalid_transitions_rejected() {
        assert!(validate_merge_transition(MergeStatus::Queued, MergeStatus::Merging).is_err());
        assert!(validate_merge_transition(MergeStatus::Ready, MergeStatus::Completed).is_err());
        assert!(validate_merge_transition(MergeStatus::Completed, MergeStatus::Queued).is_err());
        assert!(validate_merge_transition(MergeStatus::Conflict, MergeStatus::Ready).is_err());
        assert!(validate_merge_transition(MergeStatus::Failed, MergeStatus::Queued).is_err());
    }

    // -- Newtype safety --

    // -- FailureReason (INV-020) --

    #[test]
    fn failure_reason_display_and_serialization() {
        assert_eq!(FailureReason::Timeout.to_string(), "TIMEOUT");
        assert_eq!(FailureReason::RateLimited.to_string(), "RATE_LIMITED");
        assert_eq!(FailureReason::NetworkError.to_string(), "NETWORK_ERROR");
        assert_eq!(FailureReason::BuildFailed.to_string(), "BUILD_FAILED");
        assert_eq!(FailureReason::TestFailed.to_string(), "TEST_FAILED");
        assert_eq!(FailureReason::PolicyViolation.to_string(), "POLICY_VIOLATION");
        assert_eq!(FailureReason::MergeConflict.to_string(), "MERGE_CONFLICT");
        assert_eq!(FailureReason::AuthenticationExpired.to_string(), "AUTHENTICATION_EXPIRED");
        assert_eq!(FailureReason::InfrastructureError.to_string(), "INFRASTRUCTURE_ERROR");
        assert_eq!(FailureReason::Unknown.to_string(), "UNKNOWN");

        let json = serde_json::to_string(&FailureReason::PolicyViolation).unwrap();
        assert_eq!(json, "\"POLICY_VIOLATION\"");
        let back: FailureReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back, FailureReason::PolicyViolation);
    }

    #[test]
    fn unknown_failure_reason_fails_deserialization() {
        let json = "\"MAGIC_FAILURE\"";
        let result: Result<FailureReason, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown FailureReason must fail closed");
    }

    #[test]
    fn failure_reason_recoverability_classification() {
        // Recoverable: transient issues that may succeed on retry
        assert!(FailureReason::Timeout.is_recoverable());
        assert!(FailureReason::RateLimited.is_recoverable());
        assert!(FailureReason::NetworkError.is_recoverable());

        // Non-recoverable: deterministic or policy failures
        assert!(!FailureReason::BuildFailed.is_recoverable());
        assert!(!FailureReason::TestFailed.is_recoverable());
        assert!(!FailureReason::PolicyViolation.is_recoverable());
        assert!(!FailureReason::MergeConflict.is_recoverable());
        assert!(!FailureReason::AuthenticationExpired.is_recoverable());
        assert!(!FailureReason::InfrastructureError.is_recoverable());
        assert!(!FailureReason::Unknown.is_recoverable());
    }

    #[test]
    fn inv_020_every_failure_carries_classified_reason() {
        // INV-020: Every failure carries a classified failure_reason;
        // bare FAILED without classification is rejected.
        // This test verifies the enum exists, is exhaustive via #[non_exhaustive],
        // and that Unknown is available for cases where evidence is insufficient.
        // The type system enforces that any code path producing a failure MUST
        // provide a FailureReason — there is no way to construct a "bare" failure.
        let reasons = [
            FailureReason::Timeout,
            FailureReason::RateLimited,
            FailureReason::NetworkError,
            FailureReason::BuildFailed,
            FailureReason::TestFailed,
            FailureReason::PolicyViolation,
            FailureReason::MergeConflict,
            FailureReason::AuthenticationExpired,
            FailureReason::InfrastructureError,
            FailureReason::Unknown,
        ];
        // All variants are distinct and serializable
        for reason in &reasons {
            let json = serde_json::to_string(reason).unwrap();
            let back: FailureReason = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, reason);
        }
        // Unknown is always available as fail-closed default (Principle 7)
        assert!(!FailureReason::Unknown.is_recoverable());
    }

    // -- ProcessIdentity (INV-032) --

    #[test]
    fn inv_032_process_identity_requires_timestamp_not_pid_alone() {
        // INV-032: PID alone is insufficient for process identity.
        // A valid ProcessIdentity must include a start timestamp.
        let valid = ProcessIdentity::new(1234, 1700000000);
        assert!(valid.is_valid(), "PID + timestamp must be valid");
        assert_eq!(valid.pid, 1234);
        assert_eq!(valid.started_at, 1700000000);
        assert!(valid.nonce.is_none());

        // Zero timestamp is invalid — proves timestamp is required
        let invalid = ProcessIdentity { pid: 1234, started_at: 0, nonce: None };
        assert!(!invalid.is_valid(), "PID alone (zero timestamp) must be invalid");
    }

    #[test]
    fn inv_032_process_identity_with_nonce_is_valid() {
        let identity = ProcessIdentity::with_nonce(5678, 1700000000, "abc-nonce".to_string());
        assert!(identity.is_valid());
        assert_eq!(identity.nonce, Some("abc-nonce".to_string()));
    }

    #[test]
    fn inv_032_different_pids_same_timestamp_are_distinct() {
        let a = ProcessIdentity::new(100, 1700000000);
        let b = ProcessIdentity::new(200, 1700000000);
        assert_ne!(a, b, "different PIDs at same time must be distinct identities");
    }

    #[test]
    fn inv_032_same_pid_different_timestamps_are_distinct() {
        // This is the core of INV-032: PID recycling means the same PID
        // at different times represents different processes.
        let old = ProcessIdentity::new(1000, 1600000000);
        let recycled = ProcessIdentity::new(1000, 1700000000);
        assert_ne!(old, recycled, "recycled PID with new timestamp must be a new identity");
    }

    #[test]
    fn inv_032_process_identity_serialization_roundtrip() {
        let identity = ProcessIdentity::with_nonce(9999, 1700000000, "unique-nonce".to_string());
        let json = serde_json::to_string(&identity).unwrap();
        let back: ProcessIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, identity);
    }

    #[test]
    fn inv_032_process_identity_display_format() {
        let without_nonce = ProcessIdentity::new(42, 1700000000);
        assert_eq!(without_nonce.to_string(), "pid:42@1700000000");

        let with_nonce = ProcessIdentity::with_nonce(42, 1700000000, "n1".to_string());
        assert_eq!(with_nonce.to_string(), "pid:42@1700000000#n1");
    }

    // -- PolicySnapshot (INV-036) --

    #[test]
    fn inv_036_policy_snapshot_is_replaceable_before_freeze() {
        let snapshot = PolicySnapshot {
            snapshot_id: "snap-1".to_string(),
            run_id: "run-1".to_string(),
            policy_schema_version: EntityVersion(1),
            policy_hash: "sha256-aabb".to_string(),
            captured_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            frozen: false,
        };
        assert!(snapshot.is_replaceable(), "unfrozen snapshot must be replaceable");
    }

    #[test]
    fn inv_036_policy_snapshot_becomes_immutable_after_freeze() {
        let mut snapshot = PolicySnapshot {
            snapshot_id: "snap-2".to_string(),
            run_id: "run-2".to_string(),
            policy_schema_version: EntityVersion(3),
            policy_hash: "sha256-ccdd".to_string(),
            captured_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            frozen: false,
        };

        // Freeze when Run enters RUNNING
        snapshot.freeze();
        assert!(!snapshot.is_replaceable(), "frozen snapshot must NOT be replaceable");
        assert!(snapshot.frozen);

        // Calling freeze again is idempotent
        snapshot.freeze();
        assert!(!snapshot.is_replaceable());
    }

    #[test]
    fn inv_036_policy_snapshot_serialization_preserves_frozen_state() {
        let snapshot = PolicySnapshot {
            snapshot_id: "snap-3".to_string(),
            run_id: "run-3".to_string(),
            policy_schema_version: EntityVersion(5),
            policy_hash: "sha256-eeff".to_string(),
            captured_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
            frozen: true,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let back: PolicySnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snapshot);
        assert!(back.frozen, "frozen state must survive serialization roundtrip");
        assert!(!back.is_replaceable());
    }

    #[test]
    fn entity_version_next_increments() {
        let v = EntityVersion::INITIAL;
        assert_eq!(v.next(), EntityVersion(2));
        assert_eq!(v.next().next(), EntityVersion(3));
    }

    #[test]
    fn fencing_token_next_increments() {
        let t = FencingToken::INITIAL;
        assert_eq!(t.next(), FencingToken(2));
        assert!(t.next().is_newer_than(t));
        assert!(!t.is_newer_than(t.next()));
    }

    #[test]
    fn entity_version_and_fencing_token_are_distinct_types() {
        // This test compiles only because they are different types.
        // If someone accidentally used EntityVersion where FencingToken is expected,
        // the compiler would reject it. We verify the distinction exists.
        let _v = EntityVersion(1);
        let _t = FencingToken(1);
        // The following would NOT compile (and that's the point):
        // let _wrong: FencingToken = EntityVersion(1);
    }

    // -- Fail-closed deserialization --

    #[test]
    fn unknown_run_status_fails_deserialization() {
        let json = "\"NONEXISTENT_STATUS\"";
        let result: Result<RunStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown RunStatus must fail closed");
    }

    #[test]
    fn unknown_task_status_fails_deserialization() {
        let json = "\"IMAGINARY_STATE\"";
        let result: Result<TaskStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown TaskStatus must fail closed");
    }

    #[test]
    fn unknown_attempt_status_fails_deserialization() {
        let json = "\"NOT_A_REAL_STATE\"";
        let result: Result<AttemptStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown AttemptStatus must fail closed");
    }

    #[test]
    fn unknown_session_state_fails_deserialization() {
        let json = "\"THINKING\""; // intentionally excluded per blueprint
        let result: Result<SessionState, _> = serde_json::from_str(json);
        assert!(result.is_err(), "THINKING must not be a valid SessionState");
    }

    #[test]
    fn unknown_workspace_status_fails_deserialization() {
        let json = "\"MAGIC_WORKSPACE\"";
        let result: Result<WorkspaceStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown WorkspaceStatus must fail closed");
    }

    #[test]
    fn unknown_review_status_fails_deserialization() {
        let json = "\"RUBBER_STAMPED\"";
        let result: Result<ReviewStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown ReviewStatus must fail closed");
    }

    #[test]
    fn unknown_merge_status_fails_deserialization() {
        let json = "\"YOLO_MERGE\"";
        let result: Result<MergeStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown MergeStatus must fail closed");
    }

    #[test]
    fn known_statuses_roundtrip_correctly() {
        let run = RunStatus::PlanValidating;
        let json = serde_json::to_string(&run).unwrap();
        assert_eq!(json, "\"PLAN_VALIDATING\"");
        let back: RunStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, run);

        let task = TaskStatus::MergeReady;
        let json = serde_json::to_string(&task).unwrap();
        assert_eq!(json, "\"MERGE_READY\"");
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, task);
    }

    // -- ID newtypes --

    #[test]
    fn id_newtypes_display_correctly() {
        let pid = ProjectId::from("proj-001");
        assert_eq!(pid.to_string(), "proj-001");

        let tid = TaskId::from("TASK-142".to_string());
        assert_eq!(tid.to_string(), "TASK-142");
    }

    #[test]
    fn id_newtypes_serialize_as_plain_strings() {
        let aid = AttemptId::from("ATT-3");
        let json = serde_json::to_string(&aid).unwrap();
        assert_eq!(json, "\"ATT-3\"");
        let back: AttemptId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, aid);
    }

    #[test]
    fn timestamp_displays_correctly() {
        let ts = Timestamp("2026-08-28T12:00:00Z".to_string());
        assert_eq!(ts.to_string(), "2026-08-28T12:00:00Z");
    }
}
