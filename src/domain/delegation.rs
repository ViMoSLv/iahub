//! Mega Brain V0 — Delegation Domain Model (Architectural Hardening)
//!
//! Pure domain types for the Orchestrator → Delegation → Worker → Verification
//! pipeline. These types define the structural boundaries that prevent prompts
//! from becoming coordination state and prevent workers from self-certifying.
//!
//! Reference: Architectural Adendo "Orchestrator / Delegação"
//!
//! Key invariants enforced by these types:
//! - INV-037: Prompts are compiled artifacts, not coordination state
//! - INV-038: Every delegation must reference a versioned context snapshot
//! - INV-039: Agent selection is capability-based, not hardcoded
//! - INV-040: Workers encountering out-of-authority decisions must stop and escalate
//! - INV-041: Only verified evidence may transition work to certified DONE

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{AgentId, ArtifactId, TaskId};

// ---------------------------------------------------------------------------
// Strongly-typed IDs (newtypes)
// ---------------------------------------------------------------------------

macro_rules! define_delegation_id {
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

define_delegation_id!(
    DelegationId,
    "Unique identifier for one delegation of a task."
);
define_delegation_id!(
    ContextSnapshotId,
    "Immutable reference to a versioned context snapshot used during delegation."
);
define_delegation_id!(
    CompiledPromptId,
    "Unique identifier for a compiled prompt artifact produced by the PromptCompiler."
);

// ---------------------------------------------------------------------------
// Agent Capabilities (INV-039)
// ---------------------------------------------------------------------------

/// Discrete capabilities that agents can declare and the Orchestrator matches against.
/// Agent selection is capability-based, never hardcoded to a specific model name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentCapability {
    CodeGeneration,
    CodeReview,
    ArchitectureReview,
    Research,
    WebResearch,
    GitOperations,
    Testing,
    SecurityReview,
    PromptEngineering,
}

impl fmt::Display for AgentCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CodeGeneration => write!(f, "code-generation"),
            Self::CodeReview => write!(f, "code-review"),
            Self::ArchitectureReview => write!(f, "architecture-review"),
            Self::Research => write!(f, "research"),
            Self::WebResearch => write!(f, "web-research"),
            Self::GitOperations => write!(f, "git-operations"),
            Self::Testing => write!(f, "testing"),
            Self::SecurityReview => write!(f, "security-review"),
            Self::PromptEngineering => write!(f, "prompt-engineering"),
        }
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
// Delegation Status
// ---------------------------------------------------------------------------

/// Lifecycle states for a Delegation. Distinct from TaskStatus — a single Task
/// may have multiple Delegations across retries, reviews, and recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DelegationStatus {
    /// Delegation created but not yet dispatched to an agent.
    Pending,
    /// Dispatched to agent, awaiting session connection.
    Dispatched,
    /// Agent is actively executing.
    Executing,
    /// Agent reported completion; awaiting verification.
    Reported,
    /// Independent verification passed; result accepted.
    Verified,
    /// Agent stopped due to a stop condition; needs Orchestrator decision.
    StoppedNeedsDecision,
    /// Delegation failed irrecoverably.
    Failed,
    /// Delegation cancelled before completion.
    Cancelled,
}

impl DelegationStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Verified | Self::StoppedNeedsDecision | Self::Failed | Self::Cancelled
        )
    }
}

// ---------------------------------------------------------------------------
// Authority Scope
// ---------------------------------------------------------------------------

/// Explicit boundary of what a delegated agent is permitted to do.
/// Violations are failures, not suggestions.
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
    /// Whether the agent may merge to canonical target branch.
    pub can_merge: bool,
}

// ---------------------------------------------------------------------------
// Context Snapshot (INV-038)
// ---------------------------------------------------------------------------

/// Immutable, versioned snapshot of all contextual information available at the
/// moment a delegation was created. Enables future audit/replay of "with what
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
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Compiled Prompt (INV-037)
// ---------------------------------------------------------------------------

/// A compiled prompt is an artifact produced by the PromptCompiler from a
/// Delegation + ContextSnapshot. It is NOT coordination state — it is a
/// communication artifact that can be regenerated, versioned, and audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledPrompt {
    pub id: CompiledPromptId,
    pub task_id: TaskId,
    pub delegation_id: DelegationId,
    /// Identifies which compiler version/template produced this prompt.
    pub compiler_version: String,
    pub context_snapshot_id: ContextSnapshotId,
    pub agent_id: AgentId,
    /// SHA-256 hash of the prompt content for integrity verification.
    pub content_hash: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Worker Report (INV-005, INV-006)
// ---------------------------------------------------------------------------

/// What a worker claims about its execution. This is REPORTED state,
/// never trusted as OBSERVED reality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerReport {
    pub delegation_id: DelegationId,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEvidence {
    pub delegation_id: DelegationId,
    /// The observed outcome after independent checks.
    pub observed_status: String,
    /// Individual checks that were performed and their outcomes.
    pub checks_passed: Vec<String>,
    /// Any violations detected during verification.
    pub violations: Vec<String>,
    /// Artifact IDs containing verification evidence (test reports, diffs, etc.).
    pub evidence_artifact_ids: Vec<ArtifactId>,
}

// ---------------------------------------------------------------------------
// Delegation Entity
// ---------------------------------------------------------------------------

/// One delegation of a Task to a specific agent with explicit authority,
/// context, and stop conditions. A Task may have many Delegations over its
/// lifetime (retries, reviews, recovery).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub id: DelegationId,
    pub task_id: TaskId,
    pub agent_id: AgentId,
    pub status: DelegationStatus,
    pub authority_scope: AuthorityScope,
    pub stop_conditions: Vec<StopCondition>,
    pub context_snapshot_id: ContextSnapshotId,
    pub compiled_prompt_id: Option<CompiledPromptId>,
    pub worker_report: Option<WorkerReport>,
    pub verification_evidence: Option<VerificationEvidence>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegation_id_newtype_works() {
        let id = DelegationId::from("DEL-001");
        assert_eq!(id.to_string(), "DEL-001");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"DEL-001\"");
        let back: DelegationId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn context_snapshot_id_newtype_works() {
        let id = ContextSnapshotId::from("SNAP-abc");
        assert_eq!(id.to_string(), "SNAP-abc");
    }

    #[test]
    fn compiled_prompt_id_newtype_works() {
        let id = CompiledPromptId::from("PROMPT-xyz");
        assert_eq!(id.to_string(), "PROMPT-xyz");
    }

    #[test]
    fn agent_capability_display_kebab_case() {
        assert_eq!(
            AgentCapability::CodeGeneration.to_string(),
            "code-generation"
        );
        assert_eq!(
            AgentCapability::ArchitectureReview.to_string(),
            "architecture-review"
        );
        assert_eq!(AgentCapability::WebResearch.to_string(), "web-research");
        assert_eq!(
            AgentCapability::SecurityReview.to_string(),
            "security-review"
        );
    }

    #[test]
    fn agent_capability_serializes_kebab_case() {
        let cap = AgentCapability::PromptEngineering;
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, "\"prompt-engineering\"");
        let back: AgentCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn unknown_agent_capability_fails_deserialization() {
        let json = "\"teleportation\"";
        let result: Result<AgentCapability, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown capability must fail closed");
    }

    #[test]
    fn delegation_status_terminal_states() {
        assert!(DelegationStatus::Verified.is_terminal());
        assert!(DelegationStatus::StoppedNeedsDecision.is_terminal());
        assert!(DelegationStatus::Failed.is_terminal());
        assert!(DelegationStatus::Cancelled.is_terminal());

        assert!(!DelegationStatus::Pending.is_terminal());
        assert!(!DelegationStatus::Dispatched.is_terminal());
        assert!(!DelegationStatus::Executing.is_terminal());
        assert!(!DelegationStatus::Reported.is_terminal());
    }

    #[test]
    fn delegation_status_roundtrip() {
        let statuses = [
            DelegationStatus::Pending,
            DelegationStatus::Dispatched,
            DelegationStatus::Executing,
            DelegationStatus::Reported,
            DelegationStatus::Verified,
            DelegationStatus::StoppedNeedsDecision,
            DelegationStatus::Failed,
            DelegationStatus::Cancelled,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: DelegationStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn unknown_delegation_status_fails_deserialization() {
        let json = "\"MAGIC_DONE\"";
        let result: Result<DelegationStatus, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown DelegationStatus must fail closed");
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
    fn authority_scope_default_deny_merge() {
        let scope = AuthorityScope {
            allowed_paths: vec!["src/**".to_string()],
            denied_paths: vec![".git/**".to_string(), "adr/**".to_string()],
            can_create_files: true,
            can_delete_files: false,
            can_execute_commands: true,
            can_modify_architecture: false,
            can_merge: false, // Workers NEVER merge (INV-013)
        };
        assert!(!scope.can_merge);
        assert!(!scope.can_modify_architecture);
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
            created_at: "2026-08-28T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: ContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project_revision, "abc123");
        assert_eq!(back.schema_version, 1);
    }

    #[test]
    fn compiled_prompt_references_snapshot_and_delegation() {
        let prompt = CompiledPrompt {
            id: CompiledPromptId::from("p-1"),
            task_id: TaskId::from("TASK-142"),
            delegation_id: DelegationId::from("DEL-3"),
            compiler_version: "v1.0".to_string(),
            context_snapshot_id: ContextSnapshotId::from("snap-1"),
            agent_id: AgentId::from("claude-code-cli"),
            content_hash: "sha256abcdef".to_string(),
            created_at: "2026-08-28T12:00:00Z".to_string(),
        };
        assert_eq!(prompt.context_snapshot_id.to_string(), "snap-1");
        assert_eq!(prompt.delegation_id.to_string(), "DEL-3");
    }

    #[test]
    fn worker_report_is_separate_from_verification() {
        let report = WorkerReport {
            delegation_id: DelegationId::from("DEL-1"),
            reported_status: "DONE".to_string(),
            summary: "All tests pass".to_string(),
            claims: vec!["79 tests passed".to_string()],
            artifact_ids: vec![ArtifactId::from("art-1")],
        };

        let evidence = VerificationEvidence {
            delegation_id: DelegationId::from("DEL-1"),
            observed_status: "VERIFIED".to_string(),
            checks_passed: vec!["tests_pass".to_string(), "lint_clean".to_string()],
            violations: vec![],
            evidence_artifact_ids: vec![ArtifactId::from("art-test-report")],
        };

        // They are distinct types — cannot accidentally use report as evidence
        assert_ne!(report.reported_status, evidence.observed_status);
        assert_eq!(report.delegation_id, evidence.delegation_id);
    }

    #[test]
    fn delegation_full_lifecycle_serializes() {
        let del = Delegation {
            id: DelegationId::from("DEL-full"),
            task_id: TaskId::from("TASK-1"),
            agent_id: AgentId::from("codex-cli"),
            status: DelegationStatus::Executing,
            authority_scope: AuthorityScope {
                allowed_paths: vec!["src/auth/**".to_string()],
                denied_paths: vec![".git/**".to_string()],
                can_create_files: true,
                can_delete_files: false,
                can_execute_commands: true,
                can_modify_architecture: false,
                can_merge: false,
            },
            stop_conditions: vec![
                StopCondition::ArchitecturalContradiction,
                StopCondition::OutOfAuthority,
            ],
            context_snapshot_id: ContextSnapshotId::from("snap-42"),
            compiled_prompt_id: Some(CompiledPromptId::from("p-42")),
            worker_report: None,
            verification_evidence: None,
            created_at: "2026-08-28T12:00:00Z".to_string(),
            started_at: Some("2026-08-28T12:00:05Z".to_string()),
            finished_at: None,
        };

        let json = serde_json::to_string(&del).unwrap();
        let back: Delegation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, del.id);
        assert_eq!(back.status, DelegationStatus::Executing);
        assert!(back.worker_report.is_none());
        assert!(back.verification_evidence.is_none());
        assert_eq!(back.stop_conditions.len(), 2);
    }
}
