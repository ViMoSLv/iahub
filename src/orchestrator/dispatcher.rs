//! Mega Brain V0 — Session Dispatcher (Phase 2)
//!
//! Selects ProviderAccounts for decomposed tasks respecting concurrency limits,
//! role affinity, and INV-006 (reviewer != coder account). Wires together
//! RuntimeIdentity + Workspace + PTY + Adapter for each task.

use serde::{Deserialize, Serialize};

use super::decomposer::{AgentRole, DecomposedTask};

/// Result of dispatching a decomposed task to a ProviderAccount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchAssignment {
    /// The task being assigned.
    pub task: DecomposedTask,
    /// The selected ProviderAccount ID.
    pub account_id: String,
    /// The provider kind (e.g., "claude", "antigravity").
    pub provider: String,
}

/// Error when no suitable account is available for a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchError {
    /// No accounts available for the required role.
    NoAvailableAccount { role: String, reason: String },
    /// All accounts for this provider are at capacity.
    AccountAtCapacity { account_id: String, max: u32, current: usize },
    /// INV-006 violation: reviewer would be same account as coder.
    SelfReviewViolation { coder_account: String, reviewer_account: String },
}

/// A registered ProviderAccount with its current state.
#[derive(Debug, Clone)]
pub struct AccountSlot {
    pub account_id: String,
    pub provider: String,
    pub max_concurrent: u32,
    pub active_sessions: usize,
    pub available: bool,
}

/// Session dispatcher that assigns tasks to ProviderAccounts.
pub struct SessionDispatcher {
    accounts: Vec<AccountSlot>,
}

impl SessionDispatcher {
    /// Create a new dispatcher with the given account slots.
    pub fn new(accounts: Vec<AccountSlot>) -> Self {
        Self { accounts }
    }

    /// Dispatch a list of decomposed tasks to available accounts.
    /// Enforces:
    /// - Concurrency limits per account
    /// - INV-006: reviewer account != coder account
    /// - Availability status
    pub fn dispatch(&self, tasks: &[DecomposedTask]) -> Result<Vec<DispatchAssignment>, DispatchError> {
        let mut assignments = Vec::with_capacity(tasks.len());
        let mut coder_account: Option<String> = None;

        for task in tasks {
            let account = self.select_account(task.role, coder_account.as_deref())?;

            if task.role == AgentRole::Coder {
                coder_account = Some(account.account_id.clone());
            }

            assignments.push(DispatchAssignment {
                task: task.clone(),
                account_id: account.account_id.clone(),
                provider: account.provider.clone(),
            });
        }

        Ok(assignments)
    }

    /// Select an available account for the given role, respecting constraints.
    fn select_account(&self, role: AgentRole, coder_account: Option<&str>) -> Result<&AccountSlot, DispatchError> {
        for account in &self.accounts {
            if !account.available {
                continue;
            }
            if account.active_sessions >= account.max_concurrent as usize {
                continue;
            }
            // INV-006: reviewer must not be the same account as coder
            if role == AgentRole::Reviewer {
                if let Some(coder) = coder_account {
                    if account.account_id == coder {
                        continue; // Skip this account — try next
                    }
                }
            }
            return Ok(account);
        }

        // Check if the only reason is INV-006 constraint
        if let Some(coder) = coder_account {
            if role == AgentRole::Reviewer {
                let all_at_capacity_or_self = self.accounts.iter().all(|a| {
                    !a.available
                        || a.active_sessions >= a.max_concurrent as usize
                        || a.account_id == coder
                });
                if all_at_capacity_or_self {
                    return Err(DispatchError::SelfReviewViolation {
                        coder_account: coder.to_string(),
                        reviewer_account: "no other account available".to_string(),
                    });
                }
            }
        }

        Err(DispatchError::NoAvailableAccount {
            role: role.to_string(),
            reason: "all accounts unavailable or at capacity".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::decomposer::TaskDecomposer;

    fn make_accounts() -> Vec<AccountSlot> {
        vec![
            AccountSlot {
                account_id: "claude-a".to_string(),
                provider: "claude".to_string(),
                max_concurrent: 2,
                active_sessions: 0,
                available: true,
            },
            AccountSlot {
                account_id: "claude-b".to_string(),
                provider: "claude".to_string(),
                max_concurrent: 2,
                active_sessions: 0,
                available: true,
            },
            AccountSlot {
                account_id: "agy-a".to_string(),
                provider: "antigravity".to_string(),
                max_concurrent: 1,
                active_sessions: 0,
                available: true,
            },
        ]
    }

    #[test]
    fn dispatch_assigns_all_four_roles() {
        let dispatcher = SessionDispatcher::new(make_accounts());
        let tasks = TaskDecomposer::decompose("implement feature");
        let assignments = dispatcher.dispatch(&tasks).unwrap();
        assert_eq!(assignments.len(), 4);
        assert_eq!(assignments[0].task.role, AgentRole::Scout);
        assert_eq!(assignments[1].task.role, AgentRole::Coder);
        assert_eq!(assignments[2].task.role, AgentRole::Tester);
        assert_eq!(assignments[3].task.role, AgentRole::Reviewer);
    }

    #[test]
    fn inv_006_reviewer_gets_different_account_than_coder() {
        let dispatcher = SessionDispatcher::new(make_accounts());
        let tasks = TaskDecomposer::decompose("any task");
        let assignments = dispatcher.dispatch(&tasks).unwrap();
        let coder = &assignments[1]; // Coder is index 1
        let reviewer = &assignments[3]; // Reviewer is index 3
        assert_ne!(
            coder.account_id, reviewer.account_id,
            "INV-006: reviewer must use a different account than coder"
        );
    }

    #[test]
    fn dispatch_fails_when_no_accounts_available() {
        let accounts = vec![AccountSlot {
            account_id: "disabled".to_string(),
            provider: "claude".to_string(),
            max_concurrent: 2,
            active_sessions: 0,
            available: false,
        }];
        let dispatcher = SessionDispatcher::new(accounts);
        let tasks = TaskDecomposer::decompose("test");
        let result = dispatcher.dispatch(&tasks);
        assert!(result.is_err());
    }

    #[test]
    fn dispatch_fails_when_all_at_capacity() {
        let accounts = vec![AccountSlot {
            account_id: "busy".to_string(),
            provider: "claude".to_string(),
            max_concurrent: 1,
            active_sessions: 1, // at capacity
            available: true,
        }];
        let dispatcher = SessionDispatcher::new(accounts);
        let tasks = TaskDecomposer::decompose("test");
        let result = dispatcher.dispatch(&tasks);
        assert!(result.is_err());
    }

    #[test]
    fn inv_006_single_account_cannot_be_both_coder_and_reviewer() {
        // Only one account available — cannot satisfy INV-006
        let accounts = vec![
            AccountSlot {
                account_id: "only-one".to_string(),
                provider: "claude".to_string(),
                max_concurrent: 4,
                active_sessions: 0,
                available: true,
            },
        ];
        let dispatcher = SessionDispatcher::new(accounts);
        let tasks = TaskDecomposer::decompose("test");
        let result = dispatcher.dispatch(&tasks);
        // Should fail because reviewer cannot use the same account as coder
        assert!(result.is_err(), "single account cannot satisfy INV-006");
        match result.unwrap_err() {
            DispatchError::SelfReviewViolation { .. } => {}
            other => panic!("expected SelfReviewViolation, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_assignment_serialization_roundtrip() {
        let dispatcher = SessionDispatcher::new(make_accounts());
        let tasks = TaskDecomposer::decompose("serialize test");
        let assignments = dispatcher.dispatch(&tasks).unwrap();
        let json = serde_json::to_string(&assignments).unwrap();
        let back: Vec<DispatchAssignment> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), assignments.len());
        assert_eq!(back[0].account_id, assignments[0].account_id);
    }
}