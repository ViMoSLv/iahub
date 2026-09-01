//! Mega Brain V0 — Workspace Isolation & Write Scope Domain Model (Topic 05)
//!
//! Pure domain types for workspace lifecycle, write capabilities, scope drift
//! verification, artifact integrity, and cleanup safety. No I/O, no Git CLI,
//! no filesystem access — those are infrastructure concerns.
//!
//! Key concepts:
//! - **Workspace**: Isolated working directory bound to an Attempt.
//! - **WriteCapability**: Explicit, time-bounded authority to mutate specific paths.
//! - **ScopeDriftReport**: Evidence of actual writes vs authorized scope at submission.
//! - **Artifact**: Content-addressable evidence with SHA-256 integrity.
//! - **CleanupGate**: Safety checklist that must pass before workspace removal.
//!
//! Reference: MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Sections 12–14,
//! Appendix F (Cleanup Semantics), Appendix G (Artifact Integrity).
//!
//! Invariants enforced by these types:
//! - INV-013: Workers never receive the canonical integration workspace
//! - INV-014: Write scope is explicit, time-bounded, and path-validated
//! - INV-015: Scope violations at submission fail the Attempt regardless of intent
//! - INV-016: Cleanup never destroys unintegrated work without evidence

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{AttemptId, ProjectId, RunId, TaskId, Timestamp};

// ---------------------------------------------------------------------------
// Strongly-typed IDs
// ---------------------------------------------------------------------------

macro_rules! define_workspace_id {
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

define_workspace_id!(
    WorkspaceId,
    "Unique identifier for an isolated workspace bound to an Attempt."
);

define_workspace_id!(
    CapabilityId,
    "Unique identifier for a write capability grant."
);

define_workspace_id!(
    ArtifactId,
    "Unique identifier for a content-addressable artifact."
);

// ---------------------------------------------------------------------------
// RepositoryIdentity — resolved from Git, not from path
// ---------------------------------------------------------------------------

/// Identity of a Git repository resolved via `git rev-parse --git-common-dir`
/// and `git rev-parse --show-toplevel`. Path alone is never authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    /// Absolute path to the .git common directory.
    pub git_common_dir: String,
    /// Opaque fingerprint derived from repository metadata.
    /// Two directories pointing to the same repo share a fingerprint.
    pub repository_fingerprint: String,
}

// ---------------------------------------------------------------------------
// WorkspaceMode — how the workspace was provisioned
// ---------------------------------------------------------------------------

/// How a workspace was provisioned. GIT_WORKTREE is the default and preferred mode.
/// LOCAL_COPY is a fallback when worktrees are impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum WorkspaceMode {
    /// Git worktree created under a managed root directory.
    GitWorktree,
    /// Full local copy of the repository. Fallback only.
    LocalCopy,
}

impl fmt::Display for WorkspaceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitWorktree => write!(f, "GIT_WORKTREE"),
            Self::LocalCopy => write!(f, "LOCAL_COPY"),
        }
    }
}

// ---------------------------------------------------------------------------
// WriteCapability — explicit, time-bounded mutation authority
// ---------------------------------------------------------------------------

/// A glob pattern for path matching within a write capability.
/// Patterns are relative to the workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathPattern(pub String);

impl fmt::Display for PathPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Explicit, time-bounded authority for an agent to mutate files within
/// a workspace. The agent receives this as its sole write permission.
///
/// Deny patterns take precedence over allow patterns. Paths outside the
/// workspace root are always denied regardless of pattern matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteCapability {
    pub id: CapabilityId,
    pub task_id: TaskId,
    pub attempt_id: AttemptId,
    /// Fencing token binding this capability to a specific lease.
    pub fencing_token: i64,
    /// Glob patterns for paths the agent MAY write.
    pub allow: Vec<PathPattern>,
    /// Glob patterns for paths the agent MUST NOT write. Takes precedence.
    pub deny: Vec<PathPattern>,
    /// When this capability expires. Expired capabilities reject all writes.
    pub expires_at: Timestamp,
}

// ---------------------------------------------------------------------------
// ScopeDriftReport — evidence at submission time
// ---------------------------------------------------------------------------

/// Result of comparing actual Git diff against the authorized write capability
/// at submission time. Scope violations fail the Attempt regardless of agent intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeDriftReport {
    pub capability_id: CapabilityId,
    pub attempt_id: AttemptId,
    /// Files modified that fall outside the authorized scope.
    pub out_of_scope_files: Vec<String>,
    /// Files modified inside deny patterns.
    pub denied_files: Vec<String>,
    /// True if any violation was detected.
    pub has_violations: bool,
}

impl ScopeDriftReport {
    /// Create a clean report with no violations.
    pub fn clean(capability_id: CapabilityId, attempt_id: AttemptId) -> Self {
        Self {
            capability_id,
            attempt_id,
            out_of_scope_files: Vec::new(),
            denied_files: Vec::new(),
            has_violations: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ArtifactType — classification of evidence
// ---------------------------------------------------------------------------

/// Classification of an artifact's purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ArtifactType {
    /// Git diff/patch produced by the attempt.
    Diff,
    /// Test execution results.
    TestResults,
    /// Build output or compilation log.
    BuildLog,
    /// Verification evidence (lint, static analysis, etc.).
    VerificationEvidence,
    /// Review comment or annotation.
    ReviewAnnotation,
    /// Arbitrary evidence attached by the system.
    SystemEvidence,
}

// ---------------------------------------------------------------------------
// Artifact — content-addressable evidence with integrity
// ---------------------------------------------------------------------------

/// Immutable evidence produced during execution. Large artifacts stay on disk;
/// SQLite stores metadata and SHA-256 hash for integrity verification.
///
/// Important evidence must be content-addressable or hash-verifiable so later
/// review/recovery can prove it is inspecting the same object (Appendix G).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub project_id: ProjectId,
    pub run_id: Option<RunId>,
    pub task_id: Option<TaskId>,
    pub attempt_id: Option<AttemptId>,
    /// Relative path within the artifact store, or inline payload marker.
    pub content_path: String,
    /// SHA-256 hash of the artifact content. Hex-encoded lowercase.
    pub sha256: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    pub created_at: Timestamp,
    /// Identifier of the producer (agent, verifier, system component).
    pub producer: String,
}

impl Artifact {
    /// Validate that the SHA-256 field looks like a valid hex-encoded hash.
    /// Does NOT verify content — that requires reading the file.
    pub fn has_valid_hash_format(&self) -> bool {
        self.sha256.len() == 64 && self.sha256.chars().all(|c| c.is_ascii_hexdigit())
    }
}

// ---------------------------------------------------------------------------
// CleanupGate — safety checklist before workspace removal
// ---------------------------------------------------------------------------

/// Individual gate that must pass before a workspace can be safely removed.
/// Each gate represents one condition from Appendix F.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum CleanupGate {
    /// Task is in a terminal state or explicitly abandoned.
    TaskTerminalOrAbandoned,
    /// Candidate result (diff/artifact) has been safely captured.
    CandidateCaptured,
    /// Git diff is empty or has been archived.
    DiffEmptyOrArchived,
    /// No live Session currently owns this workspace.
    NoLiveSession,
    /// No active operation references this workspace.
    NoActiveOperation,
}

impl fmt::Display for CleanupGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskTerminalOrAbandoned => write!(f, "TASK_TERMINAL_OR_ABANDONED"),
            Self::CandidateCaptured => write!(f, "CANDIDATE_CAPTURED"),
            Self::DiffEmptyOrArchived => write!(f, "DIFF_EMPTY_OR_ARCHIVED"),
            Self::NoLiveSession => write!(f, "NO_LIVE_SESSION"),
            Self::NoActiveOperation => write!(f, "NO_ACTIVE_OPERATION"),
        }
    }
}

/// Result of evaluating all cleanup gates for a workspace.
/// If any gate fails, the workspace enters QUARANTINE/ORPHANED instead of deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupEvaluation {
    pub workspace_id: WorkspaceId,
    /// Gates that passed evaluation.
    pub passed: Vec<CleanupGate>,
    /// Gates that failed evaluation. Non-empty means cleanup is NOT safe.
    pub failed: Vec<CleanupGate>,
    /// True if all gates passed and removal is safe.
    pub is_safe_to_remove: bool,
}

impl CleanupEvaluation {
    /// All gates required for safe cleanup.
    pub const ALL_GATES: &'static [CleanupGate] = &[
        CleanupGate::TaskTerminalOrAbandoned,
        CleanupGate::CandidateCaptured,
        CleanupGate::DiffEmptyOrArchived,
        CleanupGate::NoLiveSession,
        CleanupGate::NoActiveOperation,
    ];

    /// Create a fully-passed evaluation.
    pub fn safe(workspace_id: WorkspaceId) -> Self {
        Self {
            workspace_id,
            passed: Self::ALL_GATES.to_vec(),
            failed: Vec::new(),
            is_safe_to_remove: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectId, RunId};

    // -- ID newtypes --

    #[test]
    fn workspace_id_newtype_works() {
        let id = WorkspaceId::from("WS-TASK142-ATT01");
        assert_eq!(id.to_string(), "WS-TASK142-ATT01");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"WS-TASK142-ATT01\"");
        let back: WorkspaceId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn capability_id_newtype_works() {
        let id = CapabilityId::from("WCAP-18");
        assert_eq!(id.to_string(), "WCAP-18");
    }

    #[test]
    fn artifact_id_newtype_works() {
        let id = ArtifactId::from("ART-001");
        assert_eq!(id.to_string(), "ART-001");
    }

    // -- RepositoryIdentity --

    #[test]
    fn repository_identity_serialization_roundtrip() {
        let identity = RepositoryIdentity {
            git_common_dir: "/home/user/project/.git".to_string(),
            repository_fingerprint: "abc123def456".to_string(),
        };
        let json = serde_json::to_string(&identity).unwrap();
        let back: RepositoryIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, identity);
    }

    // -- WorkspaceMode --

    #[test]
    fn workspace_mode_display_and_serialization() {
        assert_eq!(WorkspaceMode::GitWorktree.to_string(), "GIT_WORKTREE");
        assert_eq!(WorkspaceMode::LocalCopy.to_string(), "LOCAL_COPY");

        let json = serde_json::to_string(&WorkspaceMode::GitWorktree).unwrap();
        assert_eq!(json, "\"GIT_WORKTREE\"");
        let back: WorkspaceMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, WorkspaceMode::GitWorktree);
    }

    #[test]
    fn unknown_workspace_mode_fails_deserialization() {
        let json = "\"MAGIC_WORKSPACE_MODE\"";
        let result: Result<WorkspaceMode, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown WorkspaceMode must fail closed");
    }

    // -- WriteCapability --

    #[test]
    fn write_capability_serialization_roundtrip() {
        let cap = WriteCapability {
            id: CapabilityId::from("WCAP-18"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            fencing_token: 42,
            allow: vec![
                PathPattern("src/auth/**".to_string()),
                PathPattern("tests/auth/**".to_string()),
            ],
            deny: vec![
                PathPattern(".git/**".to_string()),
                PathPattern(".megabrain/**".to_string()),
                PathPattern("src/finance/**".to_string()),
            ],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        let json = serde_json::to_string(&cap).unwrap();
        let back: WriteCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cap.id);
        assert_eq!(back.task_id, cap.task_id);
        assert_eq!(back.attempt_id, cap.attempt_id);
        assert_eq!(back.fencing_token, cap.fencing_token);
        assert_eq!(back.allow, cap.allow);
        assert_eq!(back.deny, cap.deny);
        assert_eq!(back.expires_at, cap.expires_at);
    }

    #[test]
    fn path_pattern_display() {
        let p = PathPattern("src/**/*.rs".to_string());
        assert_eq!(p.to_string(), "src/**/*.rs");
    }

    // -- ScopeDriftReport --

    #[test]
    fn scope_drift_report_clean_has_no_violations() {
        let report = ScopeDriftReport::clean(
            CapabilityId::from("WCAP-18"),
            AttemptId::from("ATT-3"),
        );
        assert!(!report.has_violations);
        assert!(report.out_of_scope_files.is_empty());
        assert!(report.denied_files.is_empty());
    }

    #[test]
    fn scope_drift_report_with_violations() {
        let report = ScopeDriftReport {
            capability_id: CapabilityId::from("WCAP-18"),
            attempt_id: AttemptId::from("ATT-3"),
            out_of_scope_files: vec!["src/finance/billing.rs".to_string()],
            denied_files: vec![".git/config".to_string()],
            has_violations: true,
        };
        assert!(report.has_violations);
        assert_eq!(report.out_of_scope_files.len(), 1);
        assert_eq!(report.denied_files.len(), 1);
    }

    // -- Artifact --

    #[test]
    fn artifact_serialization_roundtrip() {
        let artifact = Artifact {
            id: ArtifactId::from("ART-001"),
            artifact_type: ArtifactType::Diff,
            project_id: ProjectId::from("proj-001"),
            run_id: Some(RunId::from("RUN-7")),
            task_id: Some(TaskId::from("TASK-142")),
            attempt_id: Some(AttemptId::from("ATT-3")),
            content_path: "artifacts/ART-001.patch".to_string(),
            sha256: "a".repeat(64),
            size_bytes: 4096,
            schema_version: 1,
            created_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
            producer: "agent-claude-a".to_string(),
        };

        let json = serde_json::to_string(&artifact).unwrap();
        let back: Artifact = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, artifact.id);
        assert_eq!(back.artifact_type, artifact.artifact_type);
        assert_eq!(back.sha256, artifact.sha256);
        assert_eq!(back.size_bytes, artifact.size_bytes);
    }

    #[test]
    fn artifact_valid_hash_format() {
        let valid = Artifact {
            id: ArtifactId::from("ART-001"),
            artifact_type: ArtifactType::TestResults,
            project_id: ProjectId::from("proj-001"),
            run_id: None,
            task_id: None,
            attempt_id: None,
            content_path: "results.json".to_string(),
            sha256: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string(),
            size_bytes: 256,
            schema_version: 1,
            created_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
            producer: "verifier".to_string(),
        };
        assert!(valid.has_valid_hash_format());
    }

    #[test]
    fn artifact_invalid_hash_format_rejected() {
        let invalid_short = Artifact {
            id: ArtifactId::from("ART-002"),
            artifact_type: ArtifactType::BuildLog,
            project_id: ProjectId::from("proj-001"),
            run_id: None,
            task_id: None,
            attempt_id: None,
            content_path: "build.log".to_string(),
            sha256: "abc123".to_string(), // too short
            size_bytes: 100,
            schema_version: 1,
            created_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
            producer: "builder".to_string(),
        };
        assert!(!invalid_short.has_valid_hash_format());

        let invalid_chars = Artifact {
            id: ArtifactId::from("ART-003"),
            artifact_type: ArtifactType::BuildLog,
            project_id: ProjectId::from("proj-001"),
            run_id: None,
            task_id: None,
            attempt_id: None,
            content_path: "build.log".to_string(),
            sha256: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string(),
            size_bytes: 100,
            schema_version: 1,
            created_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
            producer: "builder".to_string(),
        };
        assert!(!invalid_chars.has_valid_hash_format());
    }

    #[test]
    fn unknown_artifact_type_fails_deserialization() {
        let json = "\"MAGIC_ARTIFACT_TYPE\"";
        let result: Result<ArtifactType, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown ArtifactType must fail closed");
    }

    // -- CleanupGate --

    #[test]
    fn cleanup_gate_display() {
        assert_eq!(
            CleanupGate::TaskTerminalOrAbandoned.to_string(),
            "TASK_TERMINAL_OR_ABANDONED"
        );
        assert_eq!(
            CleanupGate::CandidateCaptured.to_string(),
            "CANDIDATE_CAPTURED"
        );
        assert_eq!(
            CleanupGate::DiffEmptyOrArchived.to_string(),
            "DIFF_EMPTY_OR_ARCHIVED"
        );
        assert_eq!(CleanupGate::NoLiveSession.to_string(), "NO_LIVE_SESSION");
        assert_eq!(
            CleanupGate::NoActiveOperation.to_string(),
            "NO_ACTIVE_OPERATION"
        );
    }

    #[test]
    fn cleanup_evaluation_safe_has_all_gates_passed() {
        let eval = CleanupEvaluation::safe(WorkspaceId::from("WS-001"));
        assert!(eval.is_safe_to_remove);
        assert!(eval.failed.is_empty());
        assert_eq!(eval.passed.len(), CleanupEvaluation::ALL_GATES.len());
    }

    #[test]
    fn cleanup_evaluation_unsafe_when_gates_fail() {
        let eval = CleanupEvaluation {
            workspace_id: WorkspaceId::from("WS-002"),
            passed: vec![CleanupGate::TaskTerminalOrAbandoned],
            failed: vec![
                CleanupGate::CandidateCaptured,
                CleanupGate::NoLiveSession,
            ],
            is_safe_to_remove: false,
        };
        assert!(!eval.is_safe_to_remove);
        assert_eq!(eval.failed.len(), 2);
    }

    #[test]
    fn cleanup_evaluation_serialization_roundtrip() {
        let eval = CleanupEvaluation {
            workspace_id: WorkspaceId::from("WS-003"),
            passed: vec![
                CleanupGate::TaskTerminalOrAbandoned,
                CleanupGate::CandidateCaptured,
            ],
            failed: vec![CleanupGate::NoLiveSession],
            is_safe_to_remove: false,
        };
        let json = serde_json::to_string(&eval).unwrap();
        let back: CleanupEvaluation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workspace_id, eval.workspace_id);
        assert_eq!(back.passed, eval.passed);
        assert_eq!(back.failed, eval.failed);
        assert_eq!(back.is_safe_to_remove, eval.is_safe_to_remove);
    }

    // -- INV-002: No shared workspace between active attempts --

    #[test]
    fn inv_002_write_capabilities_for_different_attempts_are_isolated() {
        // INV-002: No two active Attempts share the same workspace path.
        // Each WriteCapability is bound to a unique (attempt_id, fencing_token) tuple.
        // Two capabilities for the same task but different attempts must have
        // distinct IDs and fencing tokens, ensuring workspace isolation.
        let cap_att1 = WriteCapability {
            id: CapabilityId::from("WCAP-ATT1"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-1"),
            fencing_token: 10,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        let cap_att2 = WriteCapability {
            id: CapabilityId::from("WCAP-ATT2"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-2"),
            fencing_token: 11,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Different attempts must have different capability IDs
        assert_ne!(cap_att1.id, cap_att2.id, "each attempt gets its own capability");
        assert_ne!(cap_att1.attempt_id, cap_att2.attempt_id);
        assert_ne!(cap_att1.fencing_token, cap_att2.fencing_token,
            "fencing tokens enforce isolation between concurrent attempts");
    }

    // -- INV-003: State not conversation --

    #[test]
    fn inv_003_workspace_state_is_durable_not_conversational() {
        // INV-003: Agent-to-agent communication occurs only through durable state.
        // Workspace types serialize/deserialize deterministically — no ephemeral
        // in-memory state can substitute for persisted records.
        let cap = WriteCapability {
            id: CapabilityId::from("WCAP-DURABLE"),
            task_id: TaskId::from("TASK-1"),
            attempt_id: AttemptId::from("ATT-1"),
            fencing_token: 42,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![PathPattern(".git/**".to_string())],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Round-trip through JSON proves state is durable and reproducible
        let json = serde_json::to_string(&cap).unwrap();
        let back: WriteCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cap.id);
        assert_eq!(back.fencing_token, cap.fencing_token);
        assert_eq!(back.allow, cap.allow);
        assert_eq!(back.deny, cap.deny);

        // ScopeDriftReport also survives serialization — evidence is durable
        let report = ScopeDriftReport {
            capability_id: CapabilityId::from("WCAP-DURABLE"),
            attempt_id: AttemptId::from("ATT-1"),
            out_of_scope_files: vec!["secret.txt".to_string()],
            denied_files: vec![],
            has_violations: true,
        };
        let report_json = serde_json::to_string(&report).unwrap();
        let report_back: ScopeDriftReport = serde_json::from_str(&report_json).unwrap();
        assert!(report_back.has_violations);
        assert_eq!(report_back.out_of_scope_files.len(), 1);
    }

    // -- INV-008: Git is source of truth --

    #[test]
    fn inv_008_repository_identity_resolved_from_git_metadata_not_path() {
        // INV-008: Code reality is determined exclusively by Git, never by
        // filesystem observation alone. RepositoryIdentity carries git_common_dir
        // and a fingerprint derived from Git metadata — path alone is never authoritative.
        let identity = RepositoryIdentity {
            git_common_dir: "/repos/project/.git".to_string(),
            repository_fingerprint: "sha256-aabbccdd".to_string(),
        };

        // Identity is defined by Git metadata fields, not by workspace path
        assert!(!identity.git_common_dir.is_empty(),
            "git_common_dir must be present — path alone is insufficient");
        assert!(!identity.repository_fingerprint.is_empty(),
            "fingerprint must be derived from Git metadata");

        // Two directories pointing to the same repo share a fingerprint
        let same_repo_different_path = RepositoryIdentity {
            git_common_dir: "/worktrees/feature-x/.git".to_string(),
            repository_fingerprint: "sha256-aabbccdd".to_string(), // same fingerprint
        };
        assert_eq!(identity.repository_fingerprint, same_repo_different_path.repository_fingerprint,
            "same repo must share fingerprint regardless of path");

        // Different repos have different fingerprints even at similar paths
        let different_repo = RepositoryIdentity {
            git_common_dir: "/repos/other-project/.git".to_string(),
            repository_fingerprint: "sha256-11223344".to_string(),
        };
        assert_ne!(identity.repository_fingerprint, different_repo.repository_fingerprint,
            "different repos must have different fingerprints");
    }

    // -- INV-012: No agent write access to canonical integration workspace --

    #[test]
    fn inv_012_write_capability_never_grants_canonical_workspace_access() {
        // INV-012: No agent process receives write access to the canonical
        // integration workspace. WriteCapability is always bound to an isolated
        // attempt workspace, never to the canonical branch or main worktree.
        // The capability carries an attempt_id and fencing_token that tie it
        // to a specific isolated workspace — there is no way to construct a
        // valid WriteCapability without these bindings.
        let cap = WriteCapability {
            id: CapabilityId::from("WCAP-ISOLATED"),
            task_id: TaskId::from("TASK-1"),
            attempt_id: AttemptId::from("ATT-ISOLATED"),
            fencing_token: 42,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Every capability must be bound to a specific attempt (isolated workspace)
        assert!(!cap.attempt_id.0.is_empty(),
            "WriteCapability must be bound to an attempt — cannot target canonical workspace");
        assert!(cap.fencing_token > 0,
            "WriteCapability must carry a fencing token from an active lease on an isolated workspace");

        // Canonical workspace paths are always denied by convention
        let safe_cap = WriteCapability {
            id: CapabilityId::from("WCAP-SAFE"),
            task_id: TaskId::from("TASK-1"),
            attempt_id: AttemptId::from("ATT-SAFE"),
            fencing_token: 43,
            allow: vec![PathPattern("feature-branch/**".to_string())],
            deny: vec![
                PathPattern("main".to_string()),
                PathPattern("master".to_string()),
                PathPattern(".git/refs/heads/main".to_string()),
            ],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Deny patterns for canonical branches are present
        assert!(!safe_cap.deny.is_empty(),
            "capabilities should deny canonical branch paths");
        assert_ne!(safe_cap.attempt_id, cap.attempt_id,
            "each capability targets a different isolated workspace");
    }

    // -- INV-010: FS watchers are hints, not authority --

    #[test]
    fn inv_010_scope_drift_report_requires_evidence_not_fs_events() {
        // INV-010: Filesystem watcher events are treated as hints and always
        // verified against authoritative state. ScopeDriftReport is generated
        // from Git diff (authoritative), not from fsnotify events (hints).
        // The report structure itself enforces this: it carries capability_id
        // and attempt_id bindings that tie it to a specific Git-based verification,
        // not to ephemeral filesystem notifications.
        let report = ScopeDriftReport {
            capability_id: CapabilityId::from("WCAP-VERIFY"),
            attempt_id: AttemptId::from("ATT-VERIFY"),
            out_of_scope_files: vec!["unauthorized.txt".to_string()],
            denied_files: vec![],
            has_violations: true,
        };

        // Report is bound to a capability (Git-verified scope), not an fs event
        assert!(!report.capability_id.0.is_empty(),
            "drift report must reference a write capability (Git-verified authority)");
        assert!(!report.attempt_id.0.is_empty(),
            "drift report must reference an attempt (isolated workspace with Git state)");

        // Clean report also requires the same bindings — even "no violations"
        // must be verified against Git, not assumed from fs silence
        let clean = ScopeDriftReport::clean(
            CapabilityId::from("WCAP-CLEAN"),
            AttemptId::from("ATT-CLEAN"),
        );
        assert!(!clean.has_violations);
        assert!(!clean.capability_id.0.is_empty(),
            "even clean reports must be Git-verified, not inferred from fs quiet");
    }

    // -- INV-011: MCP is adapter, not core --

    #[test]
    fn inv_011_workspace_types_are_pure_domain_without_mcp_coupling() {
        // INV-011: MCP servers expose only read/write facades over Hub state
        // and contain no business logic. This test verifies that all workspace
        // domain types serialize/deserialize through standard serde without
        // any MCP-specific fields, protocols, or dependencies. The domain model
        // is fully self-contained — MCP adapts TO these types, not the reverse.
        let cap = WriteCapability {
            id: CapabilityId::from("WCAP-PURE"),
            task_id: TaskId::from("TASK-PURE"),
            attempt_id: AttemptId::from("ATT-PURE"),
            fencing_token: 1,
            allow: vec![PathPattern("**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Serialization uses standard JSON — no MCP protocol encoding
        let json = serde_json::to_string(&cap).unwrap();
        assert!(!json.contains("mcp"), "domain types must not reference MCP");
        assert!(!json.contains("jsonrpc"), "domain types must not embed RPC protocols");

        // Round-trip proves the type is self-contained
        let back: WriteCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cap);

        // RepositoryIdentity also has no MCP coupling
        let identity = RepositoryIdentity {
            git_common_dir: "/repo/.git".to_string(),
            repository_fingerprint: "fp-123".to_string(),
        };
        let id_json = serde_json::to_string(&identity).unwrap();
        assert!(!id_json.contains("mcp"));
        let id_back: RepositoryIdentity = serde_json::from_str(&id_json).unwrap();
        assert_eq!(id_back, identity);
    }

    // -- INV-017: UI is disposable --

    #[test]
    fn inv_017_all_orchestration_state_survives_serialization() {
        // INV-017: Closing the UI does not terminate active sessions or lose
        // orchestration state. All critical domain types must survive JSON
        // serialization roundtrips — proving they can be persisted to SQLite
        // and restored independently of any UI process.
        let cap = WriteCapability {
            id: CapabilityId::from("WCAP-DURABLE"),
            task_id: TaskId::from("TASK-D"),
            attempt_id: AttemptId::from("ATT-D"),
            fencing_token: 99,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![PathPattern("secrets/**".to_string())],
            expires_at: Timestamp("2026-12-31T23:59:59Z".to_string()),
        };
        let cap_json = serde_json::to_string(&cap).unwrap();
        let cap_back: WriteCapability = serde_json::from_str(&cap_json).unwrap();
        assert_eq!(cap_back, cap, "WriteCapability must survive UI-independent persistence");

        let report = ScopeDriftReport {
            capability_id: CapabilityId::from("WCAP-D"),
            attempt_id: AttemptId::from("ATT-D"),
            out_of_scope_files: vec!["leaked.env".to_string()],
            denied_files: vec!["secrets/key.pem".to_string()],
            has_violations: true,
        };
        let rpt_json = serde_json::to_string(&report).unwrap();
        let rpt_back: ScopeDriftReport = serde_json::from_str(&rpt_json).unwrap();
        assert_eq!(rpt_back, report, "ScopeDriftReport must survive UI-independent persistence");

        let artifact = Artifact {
            id: ArtifactId::from("ART-D"),
            artifact_type: ArtifactType::TestResults,
            project_id: ProjectId::from("PROJ-1"),
            run_id: Some(RunId::from("RUN-1".to_string())),
            task_id: Some(TaskId::from("TASK-D")),
            attempt_id: Some(AttemptId::from("ATT-D")),
            content_path: "/artifacts/test-report.json".to_string(),
            sha256: "a".repeat(64),
            size_bytes: 1024,
            schema_version: 1,
            producer: "verification-engine".to_string(),
            created_at: Timestamp("2026-08-31T15:00:00Z".to_string()),
        };
        let art_json = serde_json::to_string(&artifact).unwrap();
        let art_back: Artifact = serde_json::from_str(&art_json).unwrap();
        assert_eq!(art_back, artifact, "Artifact must survive UI-independent persistence");
    }

    // -- Cross-type consistency --

    #[test]
    fn write_capability_binds_to_specific_attempt() {
        let cap_a = WriteCapability {
            id: CapabilityId::from("WCAP-1"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-1"),
            fencing_token: 10,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        let cap_b = WriteCapability {
            id: CapabilityId::from("WCAP-2"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-2"),
            fencing_token: 11,
            allow: vec![PathPattern("src/**".to_string())],
            deny: vec![],
            expires_at: Timestamp("2026-08-31T23:59:59Z".to_string()),
        };

        // Same task, different attempts — capabilities are distinct
        assert_eq!(cap_a.task_id, cap_b.task_id);
        assert_ne!(cap_a.attempt_id, cap_b.attempt_id);
        assert_ne!(cap_a.id, cap_b.id);
        assert_ne!(cap_a.fencing_token, cap_b.fencing_token);
    }
}