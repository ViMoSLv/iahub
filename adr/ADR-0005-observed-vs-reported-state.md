# ADR-0005: Observed vs Reported State

## Status
ACCEPTED

## Context
Coding agents frequently report completion, success, or specific file changes that do not match actual filesystem or Git state. Trusting agent self-reports leads to silent corruption, false positives in verification, and merged code that does not satisfy acceptance criteria. The system must distinguish what an agent claims from what can be independently verified.

Key constraints:
- Agents are untrusted workers; they may hallucinate, lie, or lose context.
- Terminal output parsing is fragile and provider-specific.
- Filesystem state can diverge from Git state (uncommitted changes, stale caches).
- Verification must be reproducible and auditable after the fact.
- Unknown or indeterminate state must not default to success or failure.

## Decision
We maintain a strict separation between **reported state** (what the agent claims) and **observed state** (what the system independently verifies). Only observed state drives authoritative transitions.

### Reported State
- Stored in `task_attempts.reported_outcome` and `agent_sessions.observed_json`.
- Treated as advisory input, never as ground truth.
- Used for debugging, user feedback, and handoff context.
- Never triggers VERIFYING → REVIEWING or MERGE_READY transitions directly.

### Observed State
- Derived from Git diffs, test results, lint output, and filesystem inspection performed by the Hub or independent verification agents.
- Stored in artifacts (test-report, diff, security-report) with SHA-256 hashes.
- Drives all authoritative state transitions in Task and Review state machines.
- Survives agent crashes and session restarts.

### Verification Gate
Every Attempt in SUBMITTED state must pass through VERIFYING before REVIEWING. Verification includes:
1. Git diff against base commit matches write scope.
2. Tests pass in isolated environment.
3. Lints/formatters succeed.
4. No forbidden patterns detected.
5. Artifacts produced and hashed.

If verification cannot determine outcome (e.g., test timeout, infra failure), state transitions to UNKNOWN or FAILED with classified reason — never to REVIEWING or DONE.

### Review Independence
Reviewers receive only observed evidence (artifacts, diffs, test reports). They never see agent chat logs or terminal output as primary evidence. Review verdict binds to candidate SHA; any change invalidates the review.

## Consequences
### Positive
- Agent hallucinations cannot corrupt canonical state.
- Verification is reproducible and auditable via artifact hashes.
- Unknown states are preserved rather than masked as success/failure.
- Review quality improves by focusing on evidence, not narrative.
- Debugging is easier: reported vs observed discrepancy is explicit.

### Negative
- Verification adds latency between submission and review.
- Some agent work may be discarded if verification fails despite correct intent.
- Artifact storage grows with verification evidence.
- Developers must design acceptance criteria that are machine-verifiable.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Verification false negative rejects correct work | Retry with different verification agent; human escalation path; preserve reported outcome for debugging |
| Artifact storage bloat | Retention policy; compress large artifacts; store only metadata + hash in SQLite |
| Verification itself is buggy | Independent verification agents; chaos testing of verification pipeline; versioned verification policies |
| Agent gaming verification | Write scope enforcement; diff-based scope drift detection; randomized verification checks |
| Observed state lags behind reality | Real-time Git polling; event-driven triggers; reconcile on startup |

## Related
- INV-005: Reported not observed
- INV-007: Unknown remains unknown
- INV-016: Done requires evidence
- INV-028: Review evidence binding
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md](../TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md)