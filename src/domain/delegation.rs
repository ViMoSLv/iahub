//! Mega Brain V0 — Dispatch Specification & Delegation Value Objects
//!
//! Pure domain types for the Orchestrator → Attempt dispatch pipeline.
//! These are **value objects**, not independent aggregates with their own
//! lifecycle. The execution identity remains `AttemptId`; a retry creates
//! `Attempt-2`, not `Attempt-2 + Delegation-9`.
//!
//! Reference: ADR-0011 (AMENDED), Architectural Adendo "Orchestrator / Delegação"
//!
//! Key invariants enforced by these types:
//! - INV-037: Prompts are compiled artifacts, not coordination state
//! - INV-038: Every dispatch must reference a versioned context snapshot
//! - INV-039: Agent selection is capability-based, not hardcoded
//! - INV-040: Workers encountering out-of-authority decisions must stop and escalate
//! - INV-041: Only verified evidence may authorize certified completion
//! - INV-013: Workers never merge target branch (no `can_merge` in AuthorityScope)

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{AgentId, ArtifactId, AttemptId, TaskId, Timestamp};

// ---------------------------------------------------------------------------
// Strongly-typed IDs (newtypes)
// ---------------------------------------------------------------------------

macro_rules! define_dispatch_id {
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

define_dispatch_id!(
    ContextSnapshotId,
    "Immutable reference to a versioned context snapshot used during dispatch."
);
define_dispatch_id!(
    CompiledPromptId,
    "Unique identifier for a compiled prompt artifact produced by the PromptCompiler."
);

// ---------------------------------------------------------------------------
// CapabilityId — data-driven, not enum (PARTE 6)
// ---------------------------------------------------------------------------

/// Opaque capability identifier. Data-driven: new capabilities can be added
/// via provider manifests without recompiling the Core.
///
/// Known capabilities are exposed as associated constants for convenience,
/// but the type accepts any valid string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    pub const CODE_GENERATION: &'static str = "code-generation";
    pub const CODE_REVIEW: &'static str = "code-review";
    pub const ARCHITECTURE_REVIEW: &'static str = "architecture-review";
    pub const RESEARCH: &'static str = "research";
    pub const WEB_RESEARCH: &'static str = "web-research";
    pub const GIT_OPERATIONS: &'static str = "git-operations";
    pub const TESTING: &'static str = "testing";
    pub const SECURITY_REVIEW: &'static str = "security-review";
    pub const PROMPT_ENGINEERING: &'static str = "prompt-engineering";

    /// Create a new CapabilityId from a string. No validation beyond non-empty.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns true if this capability matches a known constant.
    pub fn is_known(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::CODE_GENERATION
                | Self::CODE_REVIEW
                | Self::ARCHITECTURE_REVIEW
                | Self::RESEARCH
                | Self::WEB_RESEARCH
                | Self::GIT_OPERATIONS
                | Self::TESTING
                | Self::SECURITY_REVIEW
                | Self::PROMPT_ENGINEERING
        )
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Stop Conditions (INV-040)
// ---------------------------------------------------------------------------

/// Conditions under which a worker MUST stop execution and escalate to the Orchestrator.
/// Workers cannot improvise solutions when any of these conditions are encountered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StopCondition {
    /// Task requires an architectural decision outside the worker's authority.
    ArchitecturalContradiction,
    /// A prerequisite dependency or resource is missing.
    MissingPrerequisite,
    /// Execution would violate a frozen invariant.
    InvariantViolation,
    /// Human intervention or approval is required.
    NeedsHumanDecision,
    /// Requested action exceeds the delegation's authority scope.
    OutOfAuthority,
    /// An unplanned destructive operation was encountered.
    UnplannedDestructiveAction,
    /// Custom stop condition with a descriptive reason.
    Other(String),
}

// ---------------------------------------------------------------------------
// Authority Scope (PARTE 7 — no can_merge)
// ---------------------------------------------------------------------------

/// Explicit boundary of what a dispatched agent is permitted to do.
/// Violations are failures, not suggestions.
///
/// NOTE: `can_merge` is intentionally absent. Workers NEVER merge target
/// branches (INV-013). Merge authority belongs exclusively to the Merge Engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityScope {
    /// File/path patterns the agent MAY modify.
    pub allowed_paths: Vec<String>,
    /// File/path patterns the agent MUST NOT modify.
    pub denied_paths: Vec<String>,
    /// Whether the agent may create new files.
    pub can_create_files: bool,
    /// Whether the agent may delete files.
    pub can_delete_files: bool,
    /// Whether the agent may execute shell commands.
    pub can_execute_commands: bool,
    /// Whether the agent may modify architecture/ADR/invariant documents.
    pub can_modify_architecture: bool,
}

// ---------------------------------------------------------------------------
// Context Snapshot (INV-038)
// ---------------------------------------------------------------------------

/// Immutable, versioned snapshot of all contextual information available at the
/// moment a dispatch was created. Enables future audit/replay of "with what
/// context did this agent make this decision?"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub id: ContextSnapshotId,
    /// Git commit SHA of the project at snapshot time.
    pub project_revision: String,
    /// Version hash of ARCHITECTURE.md at snapshot time.
    pub architecture_revision: String,
    /// Version hash of INVARIANTS.md at snapshot time.
    pub invariant_revision: String,
    /// Version hash of ADR directory at snapshot time.
    pub adr_revision: String,
    /// Schema version of the coordination database at snapshot time.
    pub schema_version: i64,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Compiled Prompt (INV-037)
// ---------------------------------------------------------------------------

/// A compiled prompt is an artifact produced by the PromptCompiler from a
/// DispatchSpec + ContextSnapshot. It is NOT coordination state — it is a
/// communication artifact that can be regenerated, versioned, and audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledPrompt {
    pub id: CompiledPromptId,
    pub task_id: TaskId,
    pub attempt_id: AttemptId,
    /// Identifies which compiler version/template produced this prompt.
    pub compiler_version: String,
    pub context_snapshot_id: ContextSnapshotId,
    pub agent_id: AgentId,
    /// SHA-256 hash of the prompt content for integrity verification.
    pub content_hash: String,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Worker Report (INV-005, INV-006)
// ---------------------------------------------------------------------------

/// What a worker claims about its execution. This is REPORTED state,
/// never trusted as OBSERVED reality. References AttemptId, not DelegationId.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerReport {
    pub attempt_id: AttemptId,
    /// The status the worker claims to have achieved.
    pub reported_status: String,
    /// Free-text summary from the worker.
    pub summary: String,
    /// Structured claims (e.g., "tests passed", "files modified").
    pub claims: Vec<String>,
    /// Artifact IDs the worker says it produced.
    pub artifact_ids: Vec<ArtifactId>,
}

// ---------------------------------------------------------------------------
// Verification Evidence (INV-041)
// ---------------------------------------------------------------------------

/// Independent observational evidence produced by verification, separate from
/// worker self-report. Only this evidence can authorize transitions to DONE.
/// Must reference the candidate commit SHA it verified against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEvidence {
    pub attempt_id: AttemptId,
    /// The candidate commit SHA this evidence was produced against.
    /// If the candidate changes, this evidence is stale and invalid.
    pub candidate_commit_sha: String,
    /// Typed verification outcome.
    pub outcome: VerificationOutcome,
    /// Individual checks that were performed and their outcomes.
    pub checks_passed: Vec<String>,
    /// Any violations detected during verification.
    pub violations: Vec<String>,
    /// Artifact IDs containing verification evidence (test reports, diffs, etc.).
    pub evidence_artifact_ids: Vec<ArtifactId>,
}

/// Typed verification outcome. Unknown is first-class (INV-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationOutcome {
    Passed,
    Failed,
    Unknown,
}

// ---------------------------------------------------------------------------
// DispatchSpec — immutable value object attached to an Attempt (PARTE 2)
// ---------------------------------------------------------------------------

/// Immutable dispatch specification for one Attempt. This is a value object,
/// NOT an independent aggregate. It has no lifecycle of its own — it is created
/// once per Attempt and never mutated.
///
/// Replaces the former `Delegation` entity. The execution identity remains
/// `AttemptId`; retries create new Attempts, not new Delegations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchSpec {
    pub attempt_id: AttemptId,
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub authority_scope: AuthorityScope,
    pub stop_conditions: Vec<StopCondition>,
    pub context_snapshot_id: ContextSnapshotId,
    pub compiled_prompt_id: Option<CompiledPromptId>,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_snapshot_id_newtype_works() {
        let id = ContextSnapshotId::from("SNAP-abc");
        assert_eq!(id.to_string(), "SNAP-abc");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"SNAP-abc\"");
        let back: ContextSnapshotId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn compiled_prompt_id_newtype_works() {
        let id = CompiledPromptId::from("PROMPT-xyz");
        assert_eq!(id.to_string(), "PROMPT-xyz");
    }

    #[test]
    fn capability_id_data_driven() {
        // Known capabilities work
        let cap = CapabilityId::new(CapabilityId::CODE_GENERATION);
        assert_eq!(cap.to_string(), "code-generation");
        assert!(cap.is_known());

        // Unknown/custom capabilities also work without recompilation
        let custom = CapabilityId::new("provider.custom-capability");
        assert_eq!(custom.to_string(), "provider.custom-capability");
        assert!(!custom.is_known());

        // Serialization roundtrip
        let json = serde_json::to_string(&custom).unwrap();
        let back: CapabilityId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, custom);
    }

    #[test]
    fn agent_capability_display_kebab_case() {
        assert_eq!(CapabilityId::CODE_GENERATION, "code-generation");
        assert_eq!(CapabilityId::ARCHITECTURE_REVIEW, "architecture-review");
        assert_eq!(CapabilityId::WEB_RESEARCH, "web-research");
        assert_eq!(CapabilityId::SECURITY_REVIEW, "security-review");
    }

    #[test]
    fn agent_capability_serializes_kebab_case() {
        let cap = CapabilityId::new(CapabilityId::PROMPT_ENGINEERING);
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, "\"prompt-engineering\"");
        let back: CapabilityId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn unknown_agent_capability_still_deserializes() {
        // Data-driven: unknown capabilities deserialize successfully
        // (unlike the old enum which failed closed on unknown variants)
        let json = "\"teleportation\"";
        let result: Result<CapabilityId, _> = serde_json::from_str(json);
        assert!(result.is_ok(), "data-driven capabilities accept any string");
        assert!(!result.unwrap().is_known());
    }

    #[test]
    fn stop_condition_variants_serialize() {
        let conditions = vec![
            StopCondition::ArchitecturalContradiction,
            StopCondition::MissingPrerequisite,
            StopCondition::InvariantViolation,
            StopCondition::NeedsHumanDecision,
            StopCondition::OutOfAuthority,
            StopCondition::UnplannedDestructiveAction,
            StopCondition::Other("custom reason".to_string()),
        ];
        for c in &conditions {
            let json = serde_json::to_string(c).unwrap();
            let back: StopCondition = serde_json::from_str(&json).unwrap();
            assert_eq!(*c, back);
        }
    }

    #[test]
    fn authority_scope_has_no_can_merge() {
        let scope = AuthorityScope {
            allowed_paths: vec!["src/**".to_string()],
            denied_paths: vec![".git/**".to_string(), "adr/**".to_string()],
            can_create_files: true,
            can_delete_files: false,
            can_execute_commands: true,
            can_modify_architecture: false,
            // No can_merge field — workers NEVER merge (INV-013)
        };
        assert!(!scope.can_modify_architecture);
        // Compile-time proof: scope.can_merge would not compile
    }

    #[test]
    fn context_snapshot_carries_all_revisions() {
        let snap = ContextSnapshot {
            id: ContextSnapshotId::from("snap-1"),
            project_revision: "abc123".to_string(),
            architecture_revision: "arch-hash-1".to_string(),
            invariant_revision: "inv-hash-1".to_string(),
            adr_revision: "adr-hash-1".to_string(),
            schema_version: 1,
            created_at: Timestamp("2026-08-28T12:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: ContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_revision, "abc123");
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.created_at.to_string(), "2026-08-28T12:00:00Z");
    }

    #[test]
    fn compiled_prompt_references_snapshot_and_attempt() {
        let prompt = CompiledPrompt {
            id: CompiledPromptId::from("p-1"),
            task_id: TaskId::from("TASK-142"),
            attempt_id: AttemptId::from("ATT-3"),
            compiler_version: "v1.0".to_string(),
            context_snapshot_id: ContextSnapshotId::from("snap-1"),
            agent_id: AgentId::from("claude-code-cli"),
            content_hash: "sha256abcdef".to_string(),
            created_at: Timestamp("2026-08-28T12:00:00Z".to_string()),
        };
        assert_eq!(prompt.context_snapshot_id.to_string(), "snap-1");
        assert_eq!(prompt.attempt_id.to_string(), "ATT-3");
    }

    #[test]
    fn worker_report_is_separate_from_verification() {
        let report = WorkerReport {
            attempt_id: AttemptId::from("ATT-1"),
            reported_status: "DONE".to_string(),
            summary: "All tests pass".to_string(),
            claims: vec!["79 tests passed".to_string()],
            artifact_ids: vec![ArtifactId::from("art-1")],
        };

        let evidence = VerificationEvidence {
            attempt_id: AttemptId::from("ATT-1"),
            candidate_commit_sha: "abc123def".to_string(),
            outcome: VerificationOutcome::Passed,
            checks_passed: vec!["tests_pass".to_string(), "lint_clean".to_string()],
            violations: vec![],
            evidence_artifact_ids: vec![ArtifactId::from("art-test-report")],
        };

        // They are distinct types — cannot accidentally use report as evidence
        assert_ne!(report.reported_status, format!("{:?}", evidence.outcome));
        assert_eq!(report.attempt_id, evidence.attempt_id);
    }

    #[test]
    fn verification_outcome_roundtrip() {
        let outcomes = [
            VerificationOutcome::Passed,
            VerificationOutcome::Failed,
            VerificationOutcome::Unknown,
        ];
        for o in &outcomes {
            let json = serde_json::to_string(o).unwrap();
            let back: VerificationOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(*o, back);
        }
    }

    #[test]
    fn unknown_verification_outcome_fails_deserialization() {
        let json = "\"MAGIC_PASSED\"";
        let result: Result<VerificationOutcome, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unknown VerificationOutcome must fail closed"
        );
    }

    #[test]
    fn dispatch_spec_full_lifecycle_serializes() {
        let spec = DispatchSpec {
            attempt_id: AttemptId::from("ATT-full"),
            task_id: TaskId::from("TASK-1"),
            agent_id: AgentId::from("codex-cli"),
            authority_scope: AuthorityScope {
                allowed_paths: vec!["src/auth/**".to_string()],
                denied_paths: vec![".git/**".to_string()],
                can_create_files: true,
                can_delete_files: false,
                can_execute_commands: true,
                can_modify_architecture: false,
            },
            stop_conditions: vec![
                StopCondition::ArchitecturalContradiction,
                StopCondition::OutOfAuthority,
            ],
            context_snapshot_id: ContextSnapshotId::from("snap-42"),
            compiled_prompt_id: Some(CompiledPromptId::from("p-42")),
            created_at: Timestamp("2026-08-28T12:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&spec).unwrap();
        let back: DispatchSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attempt_id, spec.attempt_id);
        assert_eq!(back.agent_id, spec.agent_id);
        assert_eq!(back.stop_conditions.len(), 2);
        assert_eq!(back.compiled_prompt_id, spec.compiled_prompt_id);
    }

    #[test]
    fn delegation_id_no_longer_exists() {
        // Compile-time proof: DelegationId should not exist anymore.
        // If someone tries to use it, compilation fails.
        // This test documents the intentional removal.
        // The following would NOT compile (and that's the point):
        // let _id = DelegationId::from("DEL-1");
    }

    #[test]
    fn delegation_status_no_longer_exists() {
        // Compile-time proof: DelegationStatus should not exist anymore.
        // The following would NOT compile (and that's the point):
        // let _s = DelegationStatus::Executing;
    }
}