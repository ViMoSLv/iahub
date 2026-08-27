# ADR-0006: Independent Verification and Review

## Status
ACCEPTED

## Context
Mega Brain V0 must ensure that no agent can certify its own work as complete or correct. Self-certification is a primary failure mode in multi-agent systems: agents hallucinate test passes, claim fixes that do not exist, or approve their own broken code. The system must enforce separation between production, verification, and approval.

Key constraints:
- Workers cannot certify their own success (Constitutional Principle 6).
- Reported state is not observed reality (Constitutional Principle 5).
- Verification must be reproducible and evidence-based.
- Review must be independent of the producing agent.
- Unknown outcomes must remain unknown, not default to pass/fail.
- All evidence must be immutable and hash-verifiable.

## Decision
We enforce a three-stage gate between Attempt submission and merge authorization:

### Stage 1: Observational Verification
Performed by the Hub or a dedicated verification agent (never the producing agent). Includes:
- Git diff validation against write scope
- Test execution in isolated environment
- Lint/format checks
- Security pattern scanning
- Artifact generation with SHA-256 hashes

Outcome: PASS, FAIL, or UNKNOWN. Only PASS proceeds to review. FAIL returns to rework. UNKNOWN escalates or quarantines.

### Stage 2: Independent Review
Performed by a reviewer agent whose `agent_id` differs from the producing Attempt's `agent_id`. Reviewer receives only:
- Candidate commit SHA
- Diff artifact
- Test report artifact
- Acceptance criteria
- Verification artifacts

Reviewer never sees agent chat logs, terminal output, or self-reported completion messages. Verdict binds to candidate SHA; any subsequent change invalidates the review.

### Stage 3: Merge Authorization
Only after both verification PASS and review APPROVED does the Task transition to MERGE_READY. Merge itself is performed by the Hub via Merge Laboratory, never by any agent.

### Evidence Binding
All verification and review artifacts are stored with:
- Content SHA-256
- Producer identity
- Timestamp
- Schema version
- Reference to candidate SHA and base SHA

Any mismatch between referenced SHA and actual artifact content invalidates the entire gate.

## Consequences
### Positive
- Agent self-certification is structurally impossible.
- Review quality improves by focusing on evidence, not narrative.
- Verification failures are caught before human or agent review time is spent.
- Audit trail is complete and tamper-evident via hashes.
- Unknown states are preserved rather than masked.

### Negative
- Three-stage gate adds latency between submission and merge.
- Requires additional agent capacity for verification and review.
- Some correct work may be rejected due to verification false negatives.
- Artifact storage grows with evidence retention.
- Developers must write machine-verifiable acceptance criteria.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Verification agent is buggy or biased | Rotate verification agents; chaos test verification pipeline; versioned policies |
| Reviewer rubber-stamps without reading | Require structured verdict with evidence references; audit review duration; sample human review |
| Candidate modified after verification/review | SHA binding invalidates stale approvals; re-run full gate on any change |
| Verification false negative blocks correct work | Retry with different verifier; human escalation; preserve reported outcome for debugging |
| Reviewer collusion with producer | Random reviewer assignment; cross-provider review; audit logs; anomaly detection |
| Artifact storage bloat | Retention policy; compression; metadata-only in SQLite; external object store for large blobs |

## Related
- INV-005: Reported not observed
- INV-006: No self-review
- INV-016: Done requires evidence
- INV-028: Review evidence binding
- INV-029: Merge evidence binding
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md](../TOPICOS/06-VERIFICACAO-REVIEW-E-MERGE.md)