# Mega Brain V0 — State Machines

> **Status:** FROZEN FOR IMPLEMENTATION
> **Source:** MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md, Section 5
> **Enforcement:** All transitions implemented as pure functions in Rust. Arbitrary status mutations are forbidden. Unlisted states do not exist.

## 5.1 Run State Machine

```text
DRAFT
  ↓
PLANNING
  ↓
PLAN_VALIDATING
  ↓
READY
  ↓
RUNNING
  ├────────→ PARKED
  ├────────→ BLOCKED
  ├────────→ ESCALATED
  ├────────→ INCOMPLETE
  ├────────→ FAILED
  ├────────→ CANCELLED
  ├────────→ OUTCOME_UNKNOWN
  └────────→ SUCCEEDED
```

### Terminal States
- `SUCCEEDED`
- `FAILED`
- `INCOMPLETE`
- `ESCALATED`
- `CANCELLED`
- `OUTCOME_UNKNOWN`

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| DRAFT | PLANNING | PlanCreate command | Valid objective, project registered |
| PLANNING | PLAN_VALIDATING | PlanFreeze command | PlanSpec syntactically valid |
| PLAN_VALIDATING | READY | Validation passed | No cycles, write scopes valid, deps satisfiable |
| PLAN_VALIDATING | PLANNING | Validation failed | Fixable issues detected |
| READY | RUNNING | RealityGate passed | All preconditions met, resources available |
| RUNNING | PARKED | ParkRun command | User-initiated pause |
| RUNNING | BLOCKED | External dependency unavailable | Classified reason recorded |
| RUNNING | ESCALATED | Human intervention required | Cannot proceed autonomously |
| RUNNING | INCOMPLETE | Budget exhausted or timeout | Partial results preserved |
| RUNNING | FAILED | Unrecoverable error | Classified failure_reason |
| RUNNING | CANCELLED | CancelRun command + termination confirmed | Lease revoked, workspace quarantined if uncertain |
| RUNNING | OUTCOME_UNKNOWN | Cannot determine result | Evidence insufficient for success/failure |
| RUNNING | SUCCEEDED | All tasks DONE + verified outcome matches objective | Final verification passed |
| PARKED | RUNNING | ResumeRun command | Resources still available |
| BLOCKED | RUNNING | Dependency resolved | Blocking condition cleared |

---

## 5.2 Task State Machine

```text
CREATED
  ↓
READY
  ↓
CLAIMED
  ↓
RUNNING
  ↓
SUBMITTED
  ↓
VERIFYING
  ├────→ NEEDS_CHANGES
  ├────→ FAILED
  ↓
REVIEWING
  ├────→ NEEDS_CHANGES
  ↓
MERGE_READY
  ↓
MERGING
  ├────→ CONFLICT
  ├────→ FAILED
  ↓
DONE
```

### Additional States
- `BLOCKED` — dependency not yet satisfied
- `PARKED` — user-initiated pause
- `CANCELLED` — explicitly abandoned
- `ESCALATED` — requires human decision
- `INCOMPLETE` — partial work, cannot continue

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| CREATED | READY | Dependencies satisfied | All hard deps in DONE or skipped |
| READY | CLAIMED | TaskClaim command + lease acquired | Valid fencing token, no active attempt |
| CLAIMED | RUNNING | Session connected + workspace ready | Heartbeat received, write scope granted |
| RUNNING | SUBMITTED | TaskSubmit command | Candidate commit exists, diff within scope |
| SUBMITTED | VERIFYING | Auto-triggered on submit | Verification agent assigned |
| VERIFYING | REVIEWING | Verification PASS | All checks passed, artifacts produced |
| VERIFYING | NEEDS_CHANGES | Verification FAIL (fixable) | Classified reason, context preserved |
| VERIFYING | FAILED | Verification FAIL (unfixable) | Classified failure_reason |
| REVIEWING | MERGE_READY | Review APPROVED | Verdict binds to candidate SHA |
| REVIEWING | NEEDS_CHANGES | Review CHANGES_REQUIRED | Structured feedback with evidence refs |
| MERGE_READY | MERGING | Dequeued from merge queue | Lab simulation CLEAN against current target |
| MERGING | DONE | Merge completed successfully | Canonical branch updated, task marked complete |
| MERGING | CONFLICT | Merge lab detects conflict | Returns to REVIEWING or NEEDS_CHANGES |
| MERGING | FAILED | Merge execution error | Classified failure_reason |
| Any non-terminal | BLOCKED | Dependency became unsatisfied | Reason recorded |
| Any non-terminal | PARKED | ParkTask command | User-initiated |
| Any non-terminal | CANCELLED | CancelTask command | Lease revoked, workspace handled per cleanup policy |
| Any non-terminal | ESCALATED | Human intervention needed | Cannot proceed autonomously |
| NEEDS_CHANGES | READY | New attempt created | Previous attempt context preserved in handoff artifact |

---

## 5.3 Attempt State Machine

```text
CREATED
  ↓
LEASED
  ↓
STARTING
  ↓
ACTIVE
  ├────→ SUBMITTED
  ├────→ BLOCKED
  ├────→ STALE
  ├────→ FAILED
  ├────→ CANCELLED
  └────→ LOST
```

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| CREATED | LEASED | Lease acquired | Fencing token assigned, expires_at set |
| LEASED | STARTING | Workspace creation initiated | Write scope validated |
| STARTING | ACTIVE | Session connected + first heartbeat | Process identity verified |
| ACTIVE | SUBMITTED | TaskSubmit command with valid fencing | Diff within write scope |
| ACTIVE | BLOCKED | Agent reports blocker or external dep fails | Lease preserved, scheduler notified |
| ACTIVE | STALE | Lease expired without renewal | Fencing token incremented, reassignment triggered |
| ACTIVE | FAILED | Agent-reported or observed unrecoverable error | Classified failure_reason |
| ACTIVE | CANCELLED | Cancel confirmed + process terminated | Lease revoked, workspace quarantined if uncertain |
| ACTIVE | LOST | Process death detected without clean exit | Identity verification failed, workspace quarantined |
| BLOCKED | ACTIVE | Blocker resolved | Lease still valid |
| STALE | LOST | Reconciliation confirms no live process | After grace period |

---

## 5.4 Session State Machine

Only observable states. Do not invent `THINKING`/`REASONING` unless provider emits trustworthy telemetry.

```text
CREATED
  ↓
STARTING
  ↓
CONNECTED
  ↓
ACTIVE
  ├────→ IDLE
  ├────→ UNRESPONSIVE
  ├────→ EXITED
  ├────→ LOST
  └────→ TERMINATED
```

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| CREATED | STARTING | Spawn command issued | Adapter selected, config validated |
| STARTING | CONNECTED | PTY/RPC connection established | Process identity captured |
| CONNECTED | ACTIVE | First meaningful output or heartbeat | Session holder confirms liveness |
| ACTIVE | IDLE | No activity for configured threshold | Heartbeat still arriving |
| IDLE | ACTIVE | New output or steer command received | Session still alive |
| ACTIVE | UNRESPONSIVE | Heartbeat timeout exceeded | Process may still exist; not yet confirmed dead |
| UNRESPONSIVE | ACTIVE | Heartbeat resumed | False alarm resolved |
| UNRESPONSIVE | LOST | Process identity check fails | PID recycled or nonce mismatch |
| ACTIVE | EXITED | Clean exit code observed | Process terminated normally |
| ACTIVE | TERMINATED | Kill signal sent + confirmation | Intentional termination by Hub |
| EXITED | LOST | Exit code indicates crash or corruption | Evidence preserved for debugging |
| LOST | TERMINATED | Cleanup confirmed | After quarantine/reconcile |

---

## 5.5 Workspace State Machine

```text
PLANNED
  ↓
CREATING
  ↓
READY
  ↓
IN_USE
  ↓
DIRTY
  ↓
SEALED
  ↓
RELEASING
  ↓
REMOVED
```

### Failure States
- `BROKEN` — filesystem missing or corrupted
- `ORPHANED` — no owning attempt, uncertain contents

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| PLANNED | CREATING | WorkspaceCreate operation prepared | Path validated, base commit recorded |
| CREATING | READY | Git worktree created successfully | Filesystem verified |
| CREATING | BROKEN | Worktree creation failed | Classified error, cleanup initiated |
| READY | IN_USE | Attempt transitions to ACTIVE | Lease active, write scope granted |
| IN_USE | DIRTY | File modification detected | Within write scope |
| DIRTY | SEALED | Submission received, diff captured | Candidate commit created |
| SEALED | RELEASING | Attempt terminal or workspace reassigned | Cleanup safety checks passed (Appendix F) |
| RELEASING | REMOVED | Cleanup completed | Evidence archived or integrated |
| Any state | BROKEN | Filesystem check fails | Quarantine, do not delete |
| Any state | ORPHANED | No valid lease + no live session | Preserve until manual review or policy-driven cleanup |

---

## 5.6 Review State Machine

```text
PENDING
  ↓
ASSIGNED
  ↓
IN_REVIEW
  ├────→ CHANGES_REQUIRED
  ├────→ REJECTED
  └────→ APPROVED
```

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| PENDING | ASSIGNED | ReviewAssign command | Reviewer agent_id ≠ producer agent_id |
| ASSIGNED | IN_REVIEW | Reviewer acknowledges + evidence loaded | Candidate SHA verified |
| IN_REVIEW | APPROVED | Structured verdict with evidence refs | Binds to candidate SHA |
| IN_REVIEW | CHANGES_REQUIRED | Feedback with specific change requests | Evidence-based, actionable |
| IN_REVIEW | REJECTED | Fundamental issue, not fixable by rework | Classified reason |
| APPROVED | PENDING | Candidate SHA changed after approval | Review invalidated, must re-review |

---

## 5.7 Merge State Machine

```text
QUEUED
  ↓
PRECHECK
  ↓
LAB_SIMULATION
  ├────→ CONFLICT
  ├────→ TEST_FAILED
  ↓
READY
  ↓
MERGING
  ├────→ FAILED
  ↓
COMPLETED
```

### Transition Rules
| From | To | Trigger | Preconditions |
|------|----|---------|---------------|
| QUEUED | PRECHECK | Dequeued (one per branch at a time) | Expected target SHA matches current HEAD |
| PRECHECK | LAB_SIMULATION | Target unchanged, candidate still applies | Disposable worktree created |
| PRECHECK | QUEUED | Target advanced since enqueue | Re-validate position in queue |
| LAB_SIMULATION | READY | Simulation CLEAN + tests pass | Evidence bound to exact tuple |
| LAB_SIMULATION | CONFLICT | Merge produces conflicts | Return to REVIEWING/NEEDS_CHANGES |
| LAB_SIMULATION | TEST_FAILED | Tests fail in simulated merge | Return to NEEDS_CHANGES with test report |
| READY | MERGING | Atomic merge operation begins | Journal entry written |
| MERGING | COMPLETED | Canonical branch updated successfully | Task transitions to DONE |
| MERGING | FAILED | Git operation error | Classified failure_reason, repo integrity verified |

---

## Enforcement

- All state machines implemented as Rust enums with exhaustive match.
- Transition functions return `Result<NewState, TransitionError>`.
- No direct field mutation of status columns; all changes go through transition services.
- Tests cover every valid transition and reject every invalid one.
- Runtime guard prevents instantiation of unlisted states.
- Related: [INVARIANTS.md](./INVARIANTS.md), [ARCHITECTURE.md](./ARCHITECTURE.md)