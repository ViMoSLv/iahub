//! Mega Brain V0 — Task Decomposer (Phase 2)
//!
//! Template-based task decomposition for MVP. Breaks a user objective into
//! subtasks with role assignments. LLM-based decomposition is P2.
//!
//! MVP template: scout → implement → test → review
//! Each subtask gets a role that maps to a ProviderAccount selection strategy.

use serde::{Deserialize, Serialize};

/// A decomposed subtask with role assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecomposedTask {
    /// Sequential order (0 = first to execute).
    pub order: usize,
    /// Role for this subtask — used by the dispatcher to select a ProviderAccount.
    pub role: AgentRole,
    /// Human-readable description of what this task should accomplish.
    pub description: String,
    /// Whether this task can run in parallel with others at the same order level.
    pub parallelizable: bool,
    /// IDs of tasks that must complete before this one can start.
    pub depends_on: Vec<usize>,
}

/// Agent roles for task assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum AgentRole {
    /// Analyze codebase, gather context, identify relevant files.
    Scout,
    /// Write code, implement features, fix bugs.
    Coder,
    /// Write and run tests, verify behavior.
    Tester,
    /// Independent review of changes — must differ from Coder.
    Reviewer,
    /// General-purpose shell execution (builds, installs, etc.).
    Shell,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scout => write!(f, "scout"),
            Self::Coder => write!(f, "coder"),
            Self::Tester => write!(f, "tester"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Shell => write!(f, "shell"),
        }
    }
}

/// Template-based task decomposer.
pub struct TaskDecomposer;

impl TaskDecomposer {
    /// Decompose a user objective into subtasks using the standard template.
    ///
    /// MVP template: scout → implement → test → review
    /// - Scout runs first (order 0)
    /// - Coder runs after scout (order 1, depends on 0)
    /// - Tester runs after coder (order 2, depends on 1)
    /// - Reviewer runs after tester (order 3, depends on 2)
    ///   Reviewer MUST be assigned a different ProviderAccount than Coder (INV-006).
    pub fn decompose(objective: &str) -> Vec<DecomposedTask> {
        vec![
            DecomposedTask {
                order: 0,
                role: AgentRole::Scout,
                description: format!("Analyze the codebase and gather context for: {}", objective),
                parallelizable: false,
                depends_on: vec![],
            },
            DecomposedTask {
                order: 1,
                role: AgentRole::Coder,
                description: format!("Implement the changes for: {}", objective),
                parallelizable: false,
                depends_on: vec![0],
            },
            DecomposedTask {
                order: 2,
                role: AgentRole::Tester,
                description: format!("Write and run tests for: {}", objective),
                parallelizable: false,
                depends_on: vec![1],
            },
            DecomposedTask {
                order: 3,
                role: AgentRole::Reviewer,
                description: format!("Independently review the changes for: {}", objective),
                parallelizable: false,
                depends_on: vec![2],
            },
        ]
    }

    /// Decompose with a custom template (for future extensibility).
    pub fn decompose_with_roles(objective: &str, roles: &[AgentRole]) -> Vec<DecomposedTask> {
        roles
            .iter()
            .enumerate()
            .map(|(i, role)| DecomposedTask {
                order: i,
                role: *role,
                description: format!("{} for: {}", role, objective),
                parallelizable: false,
                depends_on: if i > 0 { vec![i - 1] } else { vec![] },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_decomposition_produces_four_tasks() {
        let tasks = TaskDecomposer::decompose("implement JWT auth");
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].role, AgentRole::Scout);
        assert_eq!(tasks[1].role, AgentRole::Coder);
        assert_eq!(tasks[2].role, AgentRole::Tester);
        assert_eq!(tasks[3].role, AgentRole::Reviewer);
    }

    #[test]
    fn dependency_chain_is_sequential() {
        let tasks = TaskDecomposer::decompose("fix login bug");
        assert!(tasks[0].depends_on.is_empty());
        assert_eq!(tasks[1].depends_on, vec![0]);
        assert_eq!(tasks[2].depends_on, vec![1]);
        assert_eq!(tasks[3].depends_on, vec![2]);
    }

    #[test]
    fn descriptions_include_objective() {
        let tasks = TaskDecomposer::decompose("add dark mode");
        assert!(tasks[0].description.contains("add dark mode"));
        assert!(tasks[1].description.contains("add dark mode"));
        assert!(tasks[2].description.contains("add dark mode"));
        assert!(tasks[3].description.contains("add dark mode"));
    }

    #[test]
    fn custom_roles_template() {
        let roles = vec![AgentRole::Coder, AgentRole::Reviewer];
        let tasks = TaskDecomposer::decompose_with_roles("quick fix", &roles);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].role, AgentRole::Coder);
        assert_eq!(tasks[1].role, AgentRole::Reviewer);
        assert_eq!(tasks[1].depends_on, vec![0]);
    }

    #[test]
    fn agent_role_display() {
        assert_eq!(AgentRole::Scout.to_string(), "scout");
        assert_eq!(AgentRole::Coder.to_string(), "coder");
        assert_eq!(AgentRole::Tester.to_string(), "tester");
        assert_eq!(AgentRole::Reviewer.to_string(), "reviewer");
        assert_eq!(AgentRole::Shell.to_string(), "shell");
    }

    #[test]
    fn agent_role_serialization_roundtrip() {
        let role = AgentRole::Reviewer;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"REVIEWER\"");
        let back: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, role);
    }

    #[test]
    fn decomposed_task_serialization_roundtrip() {
        let tasks = TaskDecomposer::decompose("test objective");
        let json = serde_json::to_string(&tasks).unwrap();
        let back: Vec<DecomposedTask> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, tasks);
    }

    #[test]
    fn inv_006_reviewer_is_separate_from_coder() {
        // INV-006: The reviewer must be a different entity from the coder.
        // At the decomposition level, they are separate tasks with separate
        // role assignments. The dispatcher enforces account-level separation.
        let tasks = TaskDecomposer::decompose("any task");
        let coder_idx = tasks.iter().position(|t| t.role == AgentRole::Coder).unwrap();
        let reviewer_idx = tasks.iter().position(|t| t.role == AgentRole::Reviewer).unwrap();
        assert_ne!(coder_idx, reviewer_idx, "coder and reviewer must be distinct tasks");
        // Reviewer depends on tester which depends on coder — no overlap
        assert!(tasks[reviewer_idx].depends_on.contains(&(reviewer_idx - 1)));
    }
}