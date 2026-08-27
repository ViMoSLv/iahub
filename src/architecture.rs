//! Mega Brain V0 — Architecture Constitution
//!
//! These are product invariants, not suggestions. Every implementation decision
//! must be traceable to one or more of these principles. Violations are rejected
//! at PR review regardless of functional correctness.

/// The seven foundational constitutional principles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstitutionalPrinciple {
    /// 1. Agents are disposable. State is durable.
    AgentsDisposableStateDurable,
    /// 2. Agents do not share working trees.
    NoSharedWorkingTrees,
    /// 3. Agents communicate through state, not conversation.
    StateNotConversation,
    /// 4. Every side effect must be replay-safe.
    ReplaySafeSideEffects,
    /// 5. Reported state is not observed reality.
    ReportedIsNotObserved,
    /// 6. Workers cannot certify their own success.
    NoSelfCertification,
    /// 7. Unknown state must remain unknown.
    UnknownRemainsUnknown,
}

/// Additional non-negotiable architectural principles (8–20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonNegotiablePrinciple {
    /// 8. Git is the source of truth for code reality.
    GitIsSourceOfTruth,
    /// 9. SQLite is the source of truth for orchestration state.
    SqliteIsSourceOfTruth,
    /// 10. Filesystem watchers are hints, never authority.
    FsWatchersAreHints,
    /// 11. MCP is an adapter, not the core architecture.
    McpIsAdapter,
    /// 12. No agent may directly mutate the canonical integration workspace.
    NoDirectCanonicalMutation,
    /// 13. No agent may merge the target branch.
    NoAgentMerge,
    /// 14. The Hub owns all consequential state transitions.
    HubOwnsTransitions,
    /// 15. The same logical task survives retries, reviews, rework, and agent replacement.
    TaskSurvivesLifecycle,
    /// 16. A task is not complete because an agent says "done".
    DoneRequiresEvidence,
    /// 17. The UI must be disposable.
    UiIsDisposable,
    /// 18. No critical mutable orchestration state may exist only in RAM.
    NoRamOnlyState,
    /// 19. All external side effects must be journaled.
    JournalAllSideEffects,
    /// 20. Failures must be classified, not flattened into generic FAILED.
    ClassifyFailures,
}

/// Anti-patterns that must be rejected at PR review (Appendix O).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiPattern {
    AgentToAgentChatAsAuthority,
    SharedMutableWorkingTree,
    TaskStatusFromTerminalParsing,
    ProviderSpecificFieldsInCoreTask,
    UiOwnedSchedulerState,
    FilesystemWatcherAsFinalTruth,
    SilentMergeConflictResolution,
    UnboundedAutonomousRetryLoop,
    MutableFrozenPlanSpec,
    PidOnlyProcessOwnership,
    ImplicitGlobalWriteScope,
    NonAtomicCriticalJsonRewrite,
    NewSideEffectWithoutRecoverySemantics,
    NewStatusWithoutExhaustiveConsequenceMapping,
}

impl AntiPattern {
    /// Returns a human-readable description matching the blueprint wording.
    pub fn description(&self) -> &'static str {
        match self {
            Self::AgentToAgentChatAsAuthority => {
                "agent-to-agent chat as authoritative workflow state"
            }
            Self::SharedMutableWorkingTree => {
                "shared mutable working tree between active coding agents"
            }
            Self::TaskStatusFromTerminalParsing => "Task status controlled by terminal parsing",
            Self::ProviderSpecificFieldsInCoreTask => "provider-specific fields in core Task state",
            Self::UiOwnedSchedulerState => "UI-owned scheduler state",
            Self::FilesystemWatcherAsFinalTruth => "filesystem watcher as final truth",
            Self::SilentMergeConflictResolution => "silent merge conflict resolution",
            Self::UnboundedAutonomousRetryLoop => "unbounded autonomous retry loop",
            Self::MutableFrozenPlanSpec => "mutable frozen PlanSpec",
            Self::PidOnlyProcessOwnership => "PID-only process ownership",
            Self::ImplicitGlobalWriteScope => "implicit global write scope",
            Self::NonAtomicCriticalJsonRewrite => "critical JSON state rewritten non-atomically",
            Self::NewSideEffectWithoutRecoverySemantics => {
                "new side effect without recovery semantics"
            }
            Self::NewStatusWithoutExhaustiveConsequenceMapping => {
                "new status without exhaustive consequence mapping"
            }
        }
    }
}

/// Pre-implementation freeze checklist items (Appendix Q).
/// Each variant corresponds to one checkbox that must be completed before MB-BOOTSTRAP-001.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeChecklistItem {
    ArchitectureMd,
    StateMachinesMd,
    InvariantsMd,
    Adr0001RustHubSqliteGitCli,
    Adr0002IsolatedWorktreePerAttempt,
    Adr0003CommandEventOperationSeparation,
    Adr0004LeasesAndFencingTokens,
    Adr0005ObservedVsReportedState,
    Adr0006IndependentVerificationReview,
    Adr0007MergeLabAndSerializedQueue,
    Adr0008McpAsAdapterNotCore,
    Adr0009ProviderManifestsNativeAdapters,
    Adr0010ReconcileOnStartup,
    PinToolchainVersions,
    DefineWindowsCiLane,
    DefineLinuxCiLane,
    AddMacosLaneWhenPtyWorktreeBegins,
}

impl FreezeChecklistItem {
    /// Returns the expected filename or action description.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ArchitectureMd => "ARCHITECTURE.md",
            Self::StateMachinesMd => "STATE-MACHINES.md",
            Self::InvariantsMd => "INVARIANTS.md",
            Self::Adr0001RustHubSqliteGitCli => "ADR-0001: Rust Hub + SQLite + Git CLI",
            Self::Adr0002IsolatedWorktreePerAttempt => {
                "ADR-0002: isolated worktree per writing Attempt"
            }
            Self::Adr0003CommandEventOperationSeparation => {
                "ADR-0003: Command/Event/Operation separation"
            }
            Self::Adr0004LeasesAndFencingTokens => "ADR-0004: leases + fencing tokens",
            Self::Adr0005ObservedVsReportedState => "ADR-0005: observed vs reported state",
            Self::Adr0006IndependentVerificationReview => {
                "ADR-0006: independent verification/review"
            }
            Self::Adr0007MergeLabAndSerializedQueue => {
                "ADR-0007: Merge Laboratory + serialized Merge Queue"
            }
            Self::Adr0008McpAsAdapterNotCore => "ADR-0008: MCP as adapter, not core",
            Self::Adr0009ProviderManifestsNativeAdapters => {
                "ADR-0009: provider manifests + native adapters"
            }
            Self::Adr0010ReconcileOnStartup => "ADR-0010: Reconcile on startup",
            Self::PinToolchainVersions => "Pin toolchain versions",
            Self::DefineWindowsCiLane => "Define Windows CI lane from day one",
            Self::DefineLinuxCiLane => "Define Linux CI lane from day one",
            Self::AddMacosLaneWhenPtyWorktreeBegins => {
                "Add macOS lane when PTY/worktree layer begins"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_anti_patterns_have_non_empty_description() {
        let patterns = [
            AntiPattern::AgentToAgentChatAsAuthority,
            AntiPattern::SharedMutableWorkingTree,
            AntiPattern::TaskStatusFromTerminalParsing,
            AntiPattern::ProviderSpecificFieldsInCoreTask,
            AntiPattern::UiOwnedSchedulerState,
            AntiPattern::FilesystemWatcherAsFinalTruth,
            AntiPattern::SilentMergeConflictResolution,
            AntiPattern::UnboundedAutonomousRetryLoop,
            AntiPattern::MutableFrozenPlanSpec,
            AntiPattern::PidOnlyProcessOwnership,
            AntiPattern::ImplicitGlobalWriteScope,
            AntiPattern::NonAtomicCriticalJsonRewrite,
            AntiPattern::NewSideEffectWithoutRecoverySemantics,
            AntiPattern::NewStatusWithoutExhaustiveConsequenceMapping,
        ];
        for p in &patterns {
            assert!(!p.description().is_empty(), "{:?} has empty description", p);
        }
    }

    #[test]
    fn freeze_checklist_has_17_items() {
        let items = [
            FreezeChecklistItem::ArchitectureMd,
            FreezeChecklistItem::StateMachinesMd,
            FreezeChecklistItem::InvariantsMd,
            FreezeChecklistItem::Adr0001RustHubSqliteGitCli,
            FreezeChecklistItem::Adr0002IsolatedWorktreePerAttempt,
            FreezeChecklistItem::Adr0003CommandEventOperationSeparation,
            FreezeChecklistItem::Adr0004LeasesAndFencingTokens,
            FreezeChecklistItem::Adr0005ObservedVsReportedState,
            FreezeChecklistItem::Adr0006IndependentVerificationReview,
            FreezeChecklistItem::Adr0007MergeLabAndSerializedQueue,
            FreezeChecklistItem::Adr0008McpAsAdapterNotCore,
            FreezeChecklistItem::Adr0009ProviderManifestsNativeAdapters,
            FreezeChecklistItem::Adr0010ReconcileOnStartup,
            FreezeChecklistItem::PinToolchainVersions,
            FreezeChecklistItem::DefineWindowsCiLane,
            FreezeChecklistItem::DefineLinuxCiLane,
            FreezeChecklistItem::AddMacosLaneWhenPtyWorktreeBegins,
        ];
        assert_eq!(
            items.len(),
            17,
            "Freeze checklist must have exactly 17 items per Appendix Q"
        );
    }

    #[test]
    fn constitutional_principles_count_is_seven() {
        let principles = [
            ConstitutionalPrinciple::AgentsDisposableStateDurable,
            ConstitutionalPrinciple::NoSharedWorkingTrees,
            ConstitutionalPrinciple::StateNotConversation,
            ConstitutionalPrinciple::ReplaySafeSideEffects,
            ConstitutionalPrinciple::ReportedIsNotObserved,
            ConstitutionalPrinciple::NoSelfCertification,
            ConstitutionalPrinciple::UnknownRemainsUnknown,
        ];
        assert_eq!(
            principles.len(),
            7,
            "Seven constitutional principles per Section 1.1"
        );
    }

    #[test]
    fn non_negotiable_principles_count_is_thirteen() {
        // Principles 8 through 20 inclusive = 13 items
        let principles = [
            NonNegotiablePrinciple::GitIsSourceOfTruth,
            NonNegotiablePrinciple::SqliteIsSourceOfTruth,
            NonNegotiablePrinciple::FsWatchersAreHints,
            NonNegotiablePrinciple::McpIsAdapter,
            NonNegotiablePrinciple::NoDirectCanonicalMutation,
            NonNegotiablePrinciple::NoAgentMerge,
            NonNegotiablePrinciple::HubOwnsTransitions,
            NonNegotiablePrinciple::TaskSurvivesLifecycle,
            NonNegotiablePrinciple::DoneRequiresEvidence,
            NonNegotiablePrinciple::UiIsDisposable,
            NonNegotiablePrinciple::NoRamOnlyState,
            NonNegotiablePrinciple::JournalAllSideEffects,
            NonNegotiablePrinciple::ClassifyFailures,
        ];
        assert_eq!(
            principles.len(),
            13,
            "Non-negotiable principles 8-20 = 13 items per Section 1.2"
        );
    }
}
