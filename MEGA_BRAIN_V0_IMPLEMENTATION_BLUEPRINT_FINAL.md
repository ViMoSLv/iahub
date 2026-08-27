# MEGA BRAIN V0 — Implementation Blueprint FINAL

> **Status:** FROZEN FOR IMPLEMENTATION  
> **Scope:** V0 local-first multi-agent software engineering control plane  
> **Primary goal:** coordinate multiple real coding agents that directly edit project files without corrupting the repository, losing state, or depending on agent-to-agent chat.  
> **Target agents:** Claude Code, OpenAI Codex CLI/app-server, Cursor CLI/agent, Antigravity/agy, Gemini CLI, OpenCode, generic terminal-based coding agents.  
> **Primary operating system target:** Windows first-class, with Linux/macOS compatibility from the architecture.  
> **Design principle:** the system must remain correct when agents, UI, providers, sessions, or the Hub crash and restart.

---

# 0. Executive Summary

Mega Brain is not a chat room for AI agents.

It is a **durable software-engineering control plane** that treats coding agents as replaceable workers operating on isolated Git worktrees, while the system itself owns the authoritative state of work, permissions, verification, review, integration, and recovery.

The core idea is:

```text
OBJECTIVE
   ↓
RUN
   ↓
PLAN / TASK DAG
   ↓
SCHEDULER
   ↓
TASK ATTEMPT
   ↓
AGENT SESSION
   ↓
ISOLATED WORKSPACE / WORKTREE
   ↓
SUBMISSION
   ↓
OBSERVED REALITY
   ↓
VERIFICATION
   ↓
INDEPENDENT REVIEW
   ↓
MERGE LABORATORY
   ↓
SERIALIZED MERGE QUEUE
   ↓
CANONICAL TARGET
```

The system must continue to behave correctly if:

- an agent lies or is mistaken about completion;
- an agent loses context;
- an agent crashes;
- an old agent wakes after its lease expired;
- the Hub crashes during a DB transition;
- the Hub crashes after Git changed but before the DB recorded it;
- a provider returns duplicate responses;
- a mutating command is retried 100 times;
- a filesystem event is lost;
- a worktree is deleted manually;
- a branch is changed outside Mega Brain;
- two tasks edit the same logical file;
- a merge conflicts;
- tests fail after a candidate implementation;
- the desktop UI is closed;
- the PTY process outlives the UI;
- an external provider is temporarily unavailable.

The V0 is intentionally local-first and monolithic:

```text
1 Hub daemon
1 SQLite database
1 Git repository per project
N isolated worktrees
N agent sessions
1 merge queue per target branch
```

No Redis, Kafka, NATS, Kubernetes, Postgres, vector database, cloud scheduler, or multi-machine cluster is required for V0.

The V0 succeeds when **two independent coding agents can work on one real repository, in parallel, through isolated worktrees, with zero manual Git operations, deterministic recovery, independent verification, review, and safe integration.**

---

# 1. Architecture Constitution

These rules are not suggestions. They are product invariants.

## 1.1 Seven constitutional principles

1. **Agents are disposable. State is durable.**
2. **Agents do not share working trees.**
3. **Agents communicate through state, not conversation.**
4. **Every side effect must be replay-safe.**
5. **Reported state is not observed reality.**
6. **Workers cannot certify their own success.**
7. **Unknown state must remain unknown.**

## 1.2 Additional non-negotiable principles

8. **Git is the source of truth for code reality.**
9. **SQLite is the source of truth for orchestration state.**
10. **Filesystem watchers are hints, never authority.**
11. **MCP is an adapter, not the core architecture.**
12. **No agent may directly mutate the canonical integration workspace.**
13. **No agent may merge the target branch.**
14. **The Hub owns all consequential state transitions.**
15. **The same logical task survives retries, reviews, rework, and agent replacement.**
16. **A task is not complete because an agent says “done”.**
17. **The UI must be disposable.**
18. **No critical mutable orchestration state may exist only in RAM.**
19. **All external side effects must be journaled.**
20. **Failures must be classified, not flattened into generic `FAILED`.**

---

# 2. Open-Source Synthesis

Mega Brain should not clone a single existing project. The V0 combines proven ideas from several systems while hardening them around durable state, authority, and recovery.

## 2.1 Alethe

Adopt:

- real PTY sessions;
- multiple coding agents in parallel;
- Git worktrees;
- provider handoff;
- Claude hooks;
- supervisor concepts;
- MCP orchestration;
- merge simulation in disposable worktrees.

Harden with:

- persistent scheduler state;
- task attempts;
- leases + fencing;
- durable events;
- operation journal;
- crash reconciliation.

## 2.2 Groundcrew

Adopt:

- explicit separation of `Workspace`, `Run`, and `Session`;
- task → worktree → sandbox → agent;
- observed state vs reported state;
- startup reconcile instead of trusting graceful shutdown;
- black-box E2E boundaries;
- local vs remote freshness metadata.

## 2.3 FlowCrew

Adopt:

- planner emits an executable grammar, not prose only;
- dependency edges require reasons;
- explicit write capability;
- retry/repair edges belong to the graph;
- supervisor steers but does not execute;
- Reality Gate;
- honest outcomes such as `INCOMPLETE` and `OUTCOME_UNKNOWN`.

## 2.4 OpenCode Swarm

Adopt:

- write-scope enforcement;
- fail-closed path handling;
- symlink/junction guards;
- shell write detection seam;
- independent review/test gates;
- bounded parallelism only when scopes are compatible;
- workflow durability/WAL concepts;
- engineering invariants produced from historical bugs.

## 2.5 Diri

Adopt:

- desktop UI separate from headless engine;
- small holder process owns PTY master;
- sessions survive UI lifecycle;
- provider support can be declarative via manifests;
- provider-native adapters only where needed.

## 2.6 CAS

Adopt:

- SQLite as shared durable coordination blackboard;
- task ledger;
- persistent project context/rules;
- MCP facade over durable state.

## 2.7 dsh-agent-bus

Adopt:

- “real work items, not messages”;
- task identity survives rework;
- reviewer loop retains same logical task;
- DAG dispatch only after predecessors settle;
- structured handoffs instead of transcript archaeology.

## 2.8 Genie

Adopt:

- repository identity based on Git common directory;
- idempotent setup/transactions;
- reversible operations;
- shared repository coordination across linked worktrees.

## 2.9 VibeTree

Adopt:

- unified backend used by desktop/web clients;
- WebSocket for terminal/status transport;
- PTY scrollback replay;
- UI reconnect without spawning duplicate session;
- native IPC reserved for OS-only operations.

## 2.10 Agent Mesh

Adopt:

- per-provider circuit breaker;
- provider health-aware routing;
- pluggable transports;
- structured observability.

## 2.11 Railgun

Adopt:

- max runtime;
- max step runtime;
- concurrency caps;
- bounded retries;
- deterministic workflow progression instead of agent improvisation.

## 2.12 OpenHands

Adopt the architectural seam:

```text
ExecutionBackend
```

Ship only local worktree execution in V0; leave Docker/remote/VM for later.

---

# 3. Product Boundary

Mega Brain V0 is **not**:

- a general-purpose CrewAI replacement;
- a multi-agent debate engine;
- a cloud agent marketplace;
- a vector-memory research platform;
- a GitHub-only PR bot;
- a general enterprise workflow designer.

Mega Brain V0 is:

> **A local-first control plane for multiple coding agents that work directly on real project folders through isolated Git worktrees, with durable state, independent verification, review, recovery, and safe integration.**

---

# 4. Core Domain Model

## 4.1 Project

A Git repository registered in Mega Brain.

Fields:

```text
project_id
name
canonical_path
git_common_dir
repository_fingerprint
default_target_branch
created_at
version
```

A project is identified by repository identity, not merely by current folder path.

## 4.2 Run

A top-level execution of a user objective.

Example:

```text
RUN-202
Implement Meta OAuth inside TrackeD
```

A Run owns:

- objective;
- plan revisions;
- task DAG;
- budgets;
- run-level policy snapshot;
- run outcome.

## 4.3 Task

A stable logical unit of work.

```text
TASK-142
Implement OAuth callback endpoint
```

The Task survives retry, reassignment, reviewer rejection, and rework.

## 4.4 Attempt

One execution attempt of one Task.

```text
TASK-142
  ATT-1 → Claude → failed
  ATT-2 → Codex  → stale
  ATT-3 → Claude → submitted
```

Attempt owns execution authority.

## 4.5 Session

One live agent process/provider session attached to an Attempt.

A Session may be:

- visible PTY;
- headless RPC;
- provider-native resumable thread;
- generic CLI process.

## 4.6 Workspace

Physical filesystem allocated to an Attempt.

Default:

```text
GIT_WORKTREE
```

Fallback:

```text
LOCAL_COPY
```

## 4.7 Lease

Time-bounded authority over a Task Attempt/resource.

## 4.8 Fencing Token

Monotonically increasing authority number. An older token can never mutate newer authoritative state.

## 4.9 Artifact

Immutable or append-only evidence.

Types include:

```text
context-pack
handoff
diff
candidate-commit
test-report
review
security-report
terminal-output
plan
decision
merge-analysis
```

## 4.10 Review

Independent evaluation of a candidate produced by an Attempt.

## 4.11 Merge Item

Durable request to integrate one approved candidate into a target branch.

---

# 5. State Machines

All transitions go through explicit transition services. Arbitrary status mutations are forbidden.

## 5.1 Run

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

Terminal:

```text
SUCCEEDED
FAILED
INCOMPLETE
ESCALATED
CANCELLED
OUTCOME_UNKNOWN
```

## 5.2 Task

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

Additional:

```text
BLOCKED
PARKED
CANCELLED
ESCALATED
INCOMPLETE
```

## 5.3 Attempt

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

## 5.4 Session

Only observable states:

```text
CREATED
STARTING
CONNECTED
ACTIVE
IDLE
UNRESPONSIVE
EXITED
LOST
TERMINATED
```

Do not invent `THINKING`/`REASONING` unless the provider explicitly emits trustworthy telemetry for it.

## 5.5 Workspace

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

Failure states:

```text
BROKEN
ORPHANED
```

## 5.6 Review

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

## 5.7 Merge

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

---

# 6. Storage Architecture

## 6.1 SQLite

Use SQLite WAL for V0.

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;
PRAGMA busy_timeout = 5000;
```

The Hub daemon is the only logical writer.

UI, CLI, MCP adapters, provider adapters, and agents never open the coordination database directly.

## 6.2 Minimum tables

```text
projects
runs
run_plan_revisions

tasks
task_dependencies
task_attempts

agents
agent_capabilities
agent_sessions

workspaces

leases
resource_claims
write_scopes
file_touches

commands
events
outbox
operations

artifacts
context_packs
handoffs
decisions

reviews
review_findings

merge_queue
merge_attempts

provider_health
circuit_breakers

budgets
usage_records
```

---

# 7. Baseline SQL Semantics

The exact migrations may evolve. The semantics below are frozen.

## 7.1 projects

```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  canonical_path TEXT NOT NULL,
  git_common_dir TEXT NOT NULL,
  repository_fingerprint TEXT NOT NULL UNIQUE,
  default_target_branch TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1
);
```

## 7.2 runs

```sql
CREATE TABLE runs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  objective TEXT NOT NULL,
  status TEXT NOT NULL,
  plan_revision INTEGER NOT NULL DEFAULT 0,
  policy_snapshot_json TEXT NOT NULL,
  reported_outcome TEXT,
  verified_outcome TEXT,
  failure_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  finished_at TEXT,
  version INTEGER NOT NULL DEFAULT 1
);
```

## 7.3 tasks

```sql
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES runs(id),
  title TEXT NOT NULL,
  objective TEXT NOT NULL,
  status TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  acceptance_json TEXT NOT NULL,
  verification_json TEXT NOT NULL,
  write_scope_json TEXT NOT NULL,
  budget_json TEXT NOT NULL,
  current_attempt_id TEXT,
  candidate_commit_sha TEXT,
  base_commit_sha TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  finished_at TEXT,
  version INTEGER NOT NULL DEFAULT 1
);
```

## 7.4 task_dependencies

```sql
CREATE TABLE task_dependencies (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
  reason TEXT NOT NULL,
  dependency_type TEXT NOT NULL DEFAULT 'hard',
  PRIMARY KEY (task_id, depends_on_task_id)
);
```

## 7.5 task_attempts

```sql
CREATE TABLE task_attempts (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  attempt_no INTEGER NOT NULL,
  agent_id TEXT,
  session_id TEXT,
  workspace_id TEXT,
  status TEXT NOT NULL,
  lease_id TEXT,
  fencing_token INTEGER,
  started_at TEXT,
  submitted_at TEXT,
  ended_at TEXT,
  reported_outcome TEXT,
  failure_reason TEXT,
  UNIQUE(task_id, attempt_no)
);
```

Create a partial unique index enforcing one active Attempt per Task.

## 7.6 agents

```sql
CREATE TABLE agents (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider TEXT NOT NULL,
  adapter_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  max_parallel_tasks INTEGER NOT NULL DEFAULT 1,
  last_seen_at TEXT,
  manifest_json TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1
);
```

## 7.7 capabilities

```sql
CREATE TABLE agent_capabilities (
  agent_id TEXT NOT NULL REFERENCES agents(id),
  capability TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 1.0,
  source TEXT NOT NULL,
  PRIMARY KEY (agent_id, capability)
);
```

## 7.8 sessions

```sql
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL REFERENCES agents(id),
  task_attempt_id TEXT REFERENCES task_attempts(id),
  provider_session_id TEXT,
  holder_process_id TEXT,
  state TEXT NOT NULL,
  started_at TEXT,
  last_activity_at TEXT,
  ended_at TEXT,
  exit_code INTEGER,
  observed_json TEXT NOT NULL
);
```

## 7.9 workspaces

```sql
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_attempt_id TEXT REFERENCES task_attempts(id),
  mode TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  branch_name TEXT,
  base_commit_sha TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  sealed_at TEXT,
  removed_at TEXT,
  version INTEGER NOT NULL DEFAULT 1
);
```

## 7.10 leases

```sql
CREATE TABLE leases (
  id TEXT PRIMARY KEY,
  resource_type TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  owner_attempt_id TEXT,
  fencing_token INTEGER NOT NULL,
  expires_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL,
  status TEXT NOT NULL,
  UNIQUE(resource_type, resource_id, fencing_token)
);
```

## 7.11 commands

```sql
CREATE TABLE commands (
  command_id TEXT PRIMARY KEY,
  command_type TEXT NOT NULL,
  actor_type TEXT NOT NULL,
  actor_id TEXT,
  payload_hash TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  status TEXT NOT NULL,
  result_json TEXT,
  error_json TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);
```

## 7.12 events

```sql
CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id TEXT NOT NULL UNIQUE,
  aggregate_type TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

## 7.13 outbox

```sql
CREATE TABLE outbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id TEXT NOT NULL REFERENCES events(event_id),
  destination TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TEXT,
  published_at TEXT
);
```

## 7.14 operations

```sql
CREATE TABLE operations (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  project_id TEXT,
  task_id TEXT,
  attempt_id TEXT,
  state TEXT NOT NULL,
  preconditions_json TEXT NOT NULL,
  input_json TEXT NOT NULL,
  result_json TEXT,
  git_ref TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  committed_at TEXT,
  recovery_note TEXT
);
```

States:

```text
PREPARED
EXECUTING
SIDE_EFFECT_OBSERVED
COMMITTED
ROLLED_BACK
REQUIRES_RECONCILE
FAILED
```

## 7.15 reviews

```sql
CREATE TABLE reviews (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  candidate_commit_sha TEXT NOT NULL,
  reviewer_agent_id TEXT,
  status TEXT NOT NULL,
  verdict_json TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);
```

## 7.16 merge_queue

```sql
CREATE TABLE merge_queue (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  candidate_commit_sha TEXT NOT NULL,
  target_branch TEXT NOT NULL,
  expected_target_sha TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  failure_reason TEXT
);
```

---

# 8. Command Model

Every consequential mutation is a Command.

Examples:

```text
ProjectRegister
RunCreate
PlanFreeze
TaskClaim
TaskStart
TaskHeartbeat
TaskSubmit
TaskBlock
TaskCancel
ReviewAssign
ReviewSubmit
MergeEnqueue
MergeExecute
WorkspaceCreate
WorkspaceRelease
AgentRegister
AgentAttach
AgentSteer
```

Canonical envelope:

```json
{
  "command_id": "UUID",
  "actor": {
    "type": "agent|user|system|adapter",
    "id": "..."
  },
  "expected_version": 12,
  "attempt_id": "ATT-...",
  "lease_id": "LEASE-...",
  "fencing_token": 42,
  "payload": {}
}
```

Only include fields relevant to the specific command, but mutating Attempt commands require Attempt + lease authority.

---

# 9. Idempotency

## 9.1 Delivery model

```text
AT LEAST ONCE DELIVERY
+
IDEMPOTENT EFFECTS
```

Do not pretend DB + filesystem + Git + provider processes can give universal exactly-once semantics.

## 9.2 Command idempotency

Repeated `command_id` + identical payload hash:

```text
return previously committed result
```

Same ID + different payload hash:

```text
409 COMMAND_ID_PAYLOAD_MISMATCH
```

## 9.3 Side-effect idempotency

Every external side effect has:

```text
operation_id
preconditions
journal row
recovery evidence
```

Consequential operation types:

```text
CREATE_WORKTREE
REMOVE_WORKTREE
CREATE_CANDIDATE_COMMIT
CREATE_GIT_REF
SPAWN_AGENT
TERMINATE_AGENT
MERGE_SIMULATION
CANONICAL_MERGE
```

---

# 10. Optimistic Concurrency

Every mutable domain entity has integer `version`.

Example:

```sql
UPDATE tasks
SET status = ?, version = version + 1
WHERE id = ? AND version = ?;
```

Zero rows:

```text
409 STATE_CONFLICT
```

No silent last-write-wins.

---

# 11. Leases + Fencing Tokens

A lease says who currently owns authority.

A fencing token makes stale authority impossible to reuse.

Example:

```text
TASK-142
ATT-3
LEASE-88
fencing = 41
```

Lease expires.

```text
ATT-4
LEASE-89
fencing = 42
```

Old ATT-3 returns with token 41:

```text
409 STALE_AUTHORITY
```

Heartbeat should be emitted by Session Holder/sidecar, never rely on the LLM remembering to call it.

---

# 12. Repository Identity

Resolve repository identity from Git:

```text
git rev-parse --git-common-dir
git rev-parse --show-toplevel
```

Store:

```text
git_common_dir
repository_fingerprint
```

Do not identify the same repository solely from workspace path.

---

# 13. Workspace Isolation

## 13.1 Default

```text
GIT_WORKTREE
```

Example:

```text
C:\MegaBrain\workspaces\
  tracked\
    TASK-142-ATT-01\
    TASK-143-ATT-01\
```

## 13.2 Fallback

```text
LOCAL_COPY
```

Only when worktree is impossible or an execution backend requires it.

## 13.3 Canonical workspace

The original user project directory is the integration workspace.

Workers never receive it as their working directory.

If the user wants to code manually while a Run is active, create a dedicated human worktree.

---

# 14. Write Scope / Capability Engine

Workers receive an explicit write capability.

```json
{
  "capability_id": "WCAP-18",
  "task_id": "TASK-142",
  "attempt_id": "ATT-3",
  "fencing_token": 42,
  "allow": [
    "src/auth/**",
    "tests/auth/**"
  ],
  "deny": [
    ".git/**",
    ".megabrain/**",
    "src/finance/**"
  ],
  "expires_at": "..."
}
```

## 14.1 Path safety

Canonicalize and validate:

```text
realpath
junctions
symlinks
case normalization
Windows path semantics
```

Ambiguity fails closed.

## 14.2 Shell write seam

Adapters may inspect provider tool calls / shell commands and classify potential writes such as:

```text
cp
mv
rm
mkdir
sed -i
curl -o
wget -O
tar -x
unzip
git reset --hard
git clean -fd
Set-Content
Out-File
Copy-Item
Move-Item
Remove-Item
```

V0 does not require a perfect universal shell parser, but the policy hook must exist.

## 14.3 Scope drift

At submission, Git is authoritative.

Actual touched files outside write scope:

```text
SCOPE_DRIFT
```

V0 default:

```text
block verification
```

---

# 15. TaskSpec

Planner must output structured tasks.

```yaml
id: TASK-142
title: Implement Meta OAuth callback
objective: >
  Implement the server callback for Meta OAuth and persist tokens
  according to the project's existing credential model.

depends_on:
  - task: TASK-140
    reason: Requires the finalized token storage schema.

capabilities:
  - typescript
  - node
  - oauth

write_scope:
  - src/auth/**
  - tests/auth/**

acceptance:
  - Authorization code is exchanged successfully.
  - Refresh/access token persistence uses existing encryption.
  - Invalid state is rejected.
  - Existing auth APIs remain backward compatible.

verification:
  - npm run typecheck
  - npm test -- auth

budget:
  attempt_seconds: 1200
  total_seconds: 3600
  max_attempts: 3

review:
  required: true
  capability: backend-review
```

---

# 16. PlanSpec

Planner emits an executable plan, not prose only.

A Run Plan contains:

```text
plan_id
revision
objective
tasks[]
dependencies[]
parallel_groups[]
risk_notes[]
global_acceptance[]
global_verification[]
budget
policy
```

## 16.1 Plan validation

Reject before execution if:

- graph is cyclic;
- dependency target is missing;
- dependency has no reason;
- writable task has no explicit scope;
- scope is accidentally global;
- acceptance criteria are empty;
- verification policy is absent;
- budget is infinite/unbounded;
- capability requirements are invalid;
- planned parallel tasks have incompatible/unknown overlapping scopes;
- terminal tasks are unreachable;
- graph contains dead branches.

## 16.2 Immutable revisions

After freeze:

```text
PLAN REVISION N is immutable
```

Changes create revision N+1.

---

# 17. Scheduler

V0 Scheduler is deterministic. No LLM scheduler.

## 17.1 Task readiness

Task may be READY when:

- hard dependencies are satisfied;
- Run is active;
- required approval is resolved;
- retry budget remains;
- Task is not terminal.

## 17.2 Candidate agent filter

```text
online/available
capability match
circuit breaker != OPEN
workspace slot available
provider healthy
not quarantined
```

## 17.3 Deterministic score

Use simple weighted factors:

```text
capability confidence
least recently assigned
provider health
cost preference
```

## 17.4 Safe parallelism

Parallel only when:

- no dependency conflict;
- write scopes provably compatible;
- project concurrency budget permits;
- provider/session capacity permits.

Unknown/overlapping scope:

```text
serialize by default
```

---

# 18. Agent Adapter Contract

Core must not contain provider-specific conditional spaghetti.

Conceptual interface:

```ts
interface AgentAdapter {
  id(): string;

  detect(): Promise<DetectionResult>;
  capabilities(): Promise<AdapterCapabilities>;

  spawn(input: SpawnInput): Promise<SpawnResult>;
  resume(input: ResumeInput): Promise<ResumeResult>;
  send(input: SendInput): Promise<void>;

  interrupt(sessionId: string): Promise<void>;
  terminate(sessionId: string): Promise<void>;

  inspect(sessionId: string): Promise<ObservedSessionState>;

  subscribeEvents?(
    sessionId: string,
    sink: AgentEventSink
  ): Promise<Unsubscribe>;

  captureUsage?(
    sessionId: string
  ): Promise<UsageSnapshot>;
}
```

Adapters may use any mix of:

```text
PTY
CLI
MCP
provider hooks
JSON-RPC
app-server
process inspection
filesystem evidence
```

---

# 19. Declarative Provider Manifests

Prefer data before custom adapter code.

Example:

```json
{
  "provider": "claude-code",
  "executable": "claude",
  "spawnArgs": [],
  "resume": {
    "supported": true,
    "strategy": "cli"
  },
  "mcp": {
    "supported": true
  },
  "pty": {
    "required": true
  },
  "statusRules": [],
  "environment": {},
  "capabilities": [
    "coding",
    "terminal"
  ]
}
```

Native adapter code is justified for hooks, app-server/RPC, usage telemetry, steering, and provider session semantics.

---

# 20. Provider Strategy

## 20.1 Claude Code

Prefer:

```text
PTY
+
MCP
+
Claude hooks
+
provider-native resume
```

Useful events when available:

```text
PreToolUse
PostToolUse
SubagentStart
SubagentStop
TaskCreated
TaskCompleted
TeammateIdle
```

Provider event = evidence, never sole truth.

## 20.2 Codex

Prefer:

```text
app-server / JSON-RPC when stable
+
PTY fallback
+
CLI
```

Capture thread/session ID, turn state, diff, usage, and completion evidence where supported.

## 20.3 Cursor / Antigravity / Gemini / OpenCode / generic

Use strongest available path:

```text
native protocol
MCP
CLI
PTY
sidecar
```

Minimum universal contract for terminal-capable IDE agents:

```text
mb task current
mb task context
mb task submit
mb task block
```

---

# 21. Session Holder

Preferred runtime topology:

```text
Agent Process
    │
    ▼
Session Holder
    │
    ├── PTY master
    ├── bounded scrollback
    ├── process identity
    ├── heartbeat
    └── reconnect endpoint
         │
         ▼
       Hub
         │
         ▼
        UI
```

Goals:

- UI close does not kill agent;
- UI restart reconnects;
- provider process lifecycle does not depend on React/Tauri renderer;
- later Hub restart recovery can reuse the same seam.

Terminal text is diagnostic evidence, not orchestration truth.

---

# 22. Observed vs Reported State

## 22.1 Reported

Examples:

```text
agent says done
agent says tests passed
agent says blocked
reviewer says looks good
```

Store as reported evidence.

## 22.2 Observed

Examples:

```text
process alive
workspace exists
Git diff
Git status
commit SHA
test exit code
file hashes
merge conflict
provider hook
```

Consequential transitions use observed state.

---

# 23. Submission Pipeline

`TaskSubmit` means:

> I propose this Attempt for verification.

It does not mean completion.

Steps:

1. Validate command idempotency.
2. Validate Task state.
3. Validate Attempt ID.
4. Validate Lease.
5. Validate Fencing Token.
6. Validate Workspace ownership.
7. Recompute Git status.
8. Recompute touched files.
9. Validate write scope.
10. Capture authoritative diff.
11. Run deterministic validators.
12. Run verification commands.
13. Create candidate commit.
14. Persist artifacts/evidence.
15. Transition to REVIEWING.

---

# 24. Candidate Commit

Hub creates or certifies the candidate commit.

Suggested trailers:

```text
MegaBrain-Run: RUN-20
MegaBrain-Task: TASK-142
MegaBrain-Attempt: ATT-3
MegaBrain-Agent: claude-backend
MegaBrain-Base: a983f2
MegaBrain-Operation: OP-881
```

No Task reaches review without immutable candidate SHA.

---

# 25. Reality Gate

Engine-owned deterministic certification.

Checks include:

```text
scope valid
candidate exists
base known
required validators executed
required validators passed
required test evidence exists
required artifact exists
acceptance evidence present
```

A model may assist semantic evaluation, but deterministic claims require deterministic evidence.

---

# 26. Independent Review

Reviewer must not be the writing Attempt.

Minimum:

```text
reviewer_attempt != writer_attempt
```

Prefer different agent/provider when available.

Structured verdict:

```json
{
  "verdict": "CHANGES_REQUIRED",
  "findings": [
    {
      "severity": "high",
      "file": "src/auth/callback.ts",
      "reason": "state token is not validated before exchange",
      "evidence": "..."
    }
  ]
}
```

---

# 27. Rework

Reviewer rejection keeps Task identity.

```text
TASK-142
  ATT-1
  REVIEW-1 → CHANGES_REQUIRED
  ATT-2
  REVIEW-2 → APPROVED
```

Do not create a new Task just because implementation needs another Attempt.

---

# 28. Merge Laboratory

Before canonical integration:

```text
candidate commit
   ↓
disposable integration worktree
   ↓
trial merge
   ↓
build / typecheck / smoke tests
   ↓
conflict classification
```

Possible outcomes:

```text
CLEAN
CONFLICT
TEST_FAILED
INVALID_BASE
```

Conflict types:

```text
typescript
javascript
rust
css
json
lockfile
generated
binary
config
unknown
```

Conflict creates repair/reintegration work. V0 never silently auto-resolves.

---

# 29. Merge Queue

No worker merges target branch.

Queue is durable and serialized per target branch.

Preconditions:

- canonical workspace clean;
- candidate exists;
- review approved;
- latest Merge Laboratory passed;
- target HEAD matches expected SHA or candidate is revalidated;
- merge mutex available.

Every item stores:

```text
expected_target_sha
```

If actual target moved, return to precheck/lab. Never silently merge against an unexpected base.

---

# 30. Git Operation Journal

DB and Git are not one transaction.

Use Saga-style operation records.

```text
DB: OPERATION PREPARED
        ↓
Git side effect
        ↓
recovery marker
        ↓
DB: OPERATION COMMITTED
```

Useful Git marker:

```text
refs/megabrain/operations/OP-812
```

Crash after Git commit but before DB commit:

```text
Recovery finds operation ref
→ reconciles DB
→ no duplicate commit
```

---

# 31. Reconcile Engine

Reconcile is idempotent and runs:

```text
startup
periodic health sweep
manual doctor/repair
```

Compare expected state against:

```text
Git refs
branches
worktrees
workspace directories
process table
session holders
candidate commits
target branch
merge state
```

Rules:

- never guess success;
- never guess failure;
- insufficient proof → `OUTCOME_UNKNOWN`;
- live orphan process is surfaced loudly, not silently killed without safe ownership proof.

---

# 32. Recovery Engine

Startup:

```text
BOOT
 ↓
database integrity
 ↓
schema version
 ↓
unfinished commands
 ↓
unfinished operations
 ↓
outbox replay
 ↓
expired leases
 ↓
stale attempts
 ↓
session-holder inventory
 ↓
worktree inventory
 ↓
branch/ref inventory
 ↓
merge queue reconciliation
 ↓
canonical cleanliness
 ↓
READY
```

Expose `RECOVERING` until consequential reconciliation completes.

---

# 33. Event Store

Every accepted state mutation persists an event.

Examples:

```text
PROJECT_REGISTERED
RUN_CREATED
PLAN_FROZEN
TASK_READY
TASK_CLAIMED
ATTEMPT_STARTED
LEASE_RENEWED
SESSION_CONNECTED
WORKSPACE_CREATED
FILES_TOUCHED
TASK_SUBMITTED
VERIFICATION_PASSED
VERIFICATION_FAILED
REVIEW_REQUESTED
REVIEW_APPROVED
REVIEW_CHANGES_REQUIRED
MERGE_LAB_CLEAN
MERGE_LAB_CONFLICT
MERGE_QUEUED
MERGE_STARTED
MERGE_COMPLETED
RUN_SUCCEEDED
RUN_FAILED
RUN_INCOMPLETE
RUN_OUTCOME_UNKNOWN
```

---

# 34. Transactional Outbox

Never:

```text
update DB
then maybe send WebSocket
```

Instead:

```sql
BEGIN;
UPDATE domain_state ...;
INSERT INTO events ...;
INSERT INTO outbox ...;
COMMIT;
```

Publisher can retry later.

---

# 35. Budgets / Runaway Protection

Budgets are engine-owned.

Hierarchy:

```text
GLOBAL
PROJECT
RUN
TASK
ATTEMPT
PROVIDER
```

Dimensions:

```text
wall time
attempt count
concurrent sessions
token usage
estimated cost
provider calls
repair loops
review loops
```

Budget exhaustion normally means:

```text
INCOMPLETE
```

not generic `FAILED`.

---

# 36. Circuit Breakers

Per provider/agent:

```text
CLOSED
OPEN
HALF_OPEN
```

Open on repeated provider/infrastructure failures.

Do not treat user-code test failures as provider-health failures.

Separate categories:

```text
provider transient
provider permanent config/auth
agent logic
project verification
scope violation
```

---

# 37. Error Taxonomy

Minimum categories:

```text
STATE_CONFLICT
STALE_AUTHORITY
LEASE_EXPIRED
SCOPE_VIOLATION
SCOPE_DRIFT
PROVIDER_TRANSIENT
PROVIDER_UNAVAILABLE
PROVIDER_AUTH
PROCESS_LOST
WORKSPACE_BROKEN
GIT_PRECONDITION_FAILED
MERGE_CONFLICT
VERIFICATION_FAILED
REVIEW_REJECTED
BUDGET_EXHAUSTED
OPERATION_INDETERMINATE
INTERNAL_INVARIANT_VIOLATION
```

---

# 38. Context System

No vector database in V0.

## 38.1 Context Pack

Contains:

```text
task objective
acceptance criteria
dependency handoffs
architecture constraints
frozen decisions
base commit
relevant files
known blockers
verification commands
write scope
budget
```

## 38.2 Conversation Capsule

Optional. Include only information not already represented structurally.

## 38.3 Handoff

Downstream work receives:

```text
what changed
commit/artifact references
decisions
numbers
API contracts
caveats
unresolved issues
```

Full upstream transcript is not default context.

---

# 39. Context Provenance

Important facts carry provenance.

```json
{
  "fact": "financial_entries is immutable",
  "source_type": "decision",
  "source_id": "DECISION-041",
  "revision": 3,
  "status": "FROZEN",
  "scope": "finance"
}
```

---

# 40. Context Budget

Each Task has an explicit budget.

Example:

```text
structured context: 32 KB
conversation capsule: 16 KB
artifact inline max: 8 KB
large artifacts: reference only
```

---

# 41. Decision Registry

Fields:

```text
decision_id
project_id/run_id
title
decision
status
revision
supersedes
source
created_at
```

Statuses:

```text
PROPOSED
FROZEN
SUPERSEDED
REJECTED
```

---

# 42. Agent Communication

Do not create free-form all-to-all chat as the coordination primitive.

Three channels:

## 42.1 Note

Lightweight, non-critical.

## 42.2 Task

Durable work item.

## 42.3 Request Help / Review

Structured request becomes Task/Review.

```json
{
  "type": "technical_review",
  "capability": "postgres",
  "question": "Review concurrency semantics",
  "artifact": "ART-821"
}
```

---

# 43. Steering

Support:

```text
STEER
PAUSE
RESUME
CANCEL
```

Persist steering as event/artifact.

If provider supports live steering, inject it. Otherwise pause/update context/resume or create a new Attempt.

---

# 44. Human-in-the-Loop

Use explicit approval gates for high-consequence work:

- destructive operations;
- DB migrations;
- secrets;
- dependency upgrades;
- production/release transitions;
- ambiguous plan revisions.

A parked Run does not need a live worker process.

---

# 45. Execution Backend

Define from V0:

```ts
interface ExecutionBackend {
  provision(input: ProvisionInput): Promise<ProvisionedEnvironment>;
  start(...): Promise<ExecutionHandle>;
  inspect(...): Promise<ExecutionState>;
  terminate(...): Promise<void>;
  destroy(...): Promise<void>;
}
```

Ship:

```text
LocalWorktreeBackend
```

Future:

```text
DockerBackend
RemoteHostBackend
VMBackend
CloudBackend
```

---

# 46. Desktop Architecture

Preferred stack:

```text
Tauri 2
React
TypeScript
Vite
```

Desktop responsibilities:

```text
views
terminal rendering
notifications
dialogs
settings UI
launch external IDE
```

Desktop must not own:

```text
task scheduling
merge authority
lease authority
SQLite writes
recovery truth
```

---

# 47. Hub Daemon

Preferred language:

```text
Rust
```

Why:

```text
process lifecycle
PTY
filesystem
Git orchestration
cross-platform daemon
concurrency
Tauri ecosystem
```

Major components:

```text
api
commands
domain
persistence
events
scheduler
supervisor
sessions
workspaces
scope
git
merge
verification
review
context
artifacts
adapters
recovery
reconcile
policy
observability
```

---

# 48. CLI

First-class interface.

```text
mb doctor
mb project add .
mb project status

mb run create "Implement OAuth"
mb run status RUN-20

mb task list
mb task show TASK-142
mb task context TASK-142

mb agent list
mb agent attach claude-backend

mb workspace list
mb merge status

mb recovery status
mb events tail
```

Worker minimal surface:

```text
mb task current
mb task context
mb task submit
mb task block
mb request-help
mb artifact publish
```

---

# 49. MCP Surface

MCP is a replaceable adapter.

Suggested tools:

```text
mb_status
mb_task_current
mb_task_context
mb_task_submit
mb_task_block
mb_request_help
mb_artifact_publish
mb_artifact_get
mb_context_query
mb_agent_status
mb_steer
```

Orchestrator-only:

```text
mb_plan_create
mb_plan_freeze
mb_delegate
mb_run_status
```

Every mutating call requires `command_id`.

---

# 50. WebSocket

Use for:

```text
persisted event projections
PTY output
session status
dashboard updates
diff/review updates
```

UI must rebuild from durable read APIs after reconnect. WebSocket is not the state store.

---

# 51. Read Models

Provide explicit projections:

```text
RunOverview
TaskGraph
AgentFleet
WorkspaceStatus
MergeStatus
RecoveryStatus
ActivityTimeline
```

---

# 52. Minimal Dashboard

Final V0 information architecture:

```text
Runs
Task Graph
Agents
Changes
Context
Activity
```

First shippable dashboard may expose only:

```text
Overview
Tasks
Agents
Activity
```

---

# 53. Observability

Logs are diagnostics. DB + Git observed facts are authoritative.

Structured log shape:

```json
{
  "ts": "...",
  "level": "info",
  "module": "scheduler",
  "event": "task_dispatched",
  "project_id": "P-1",
  "run_id": "RUN-20",
  "task_id": "TASK-142",
  "attempt_id": "ATT-3",
  "session_id": "SES-9"
}
```

Correlation IDs:

```text
project_id
run_id
task_id
attempt_id
session_id
workspace_id
operation_id
command_id
```

---

# 54. Freshness Semantics

For remote/provider state store:

```text
captured_at
last_attempt_at
last_attempt_status
```

Failed refresh does not erase last-known-good data.

---

# 55. Security

## 55.1 Local API

Bind to loopback by default.

Use local auth token.

## 55.2 Workspace restriction

Worker starts inside allocated workspace. Do not inject canonical path unless an exceptional read-only workflow requires it.

## 55.3 Protected metadata

Workers cannot mutate:

```text
.git/**
.megabrain/**
Hub DB
operation refs
canonical integration metadata
```

## 55.4 Secrets

Do not store provider secrets in Task context. Prefer OS keychain/secret storage.

## 55.5 Dangerous operations

At minimum detect/surface:

```text
git reset --hard
git clean -fd
rm -rf
Remove-Item -Recurse
writes outside workspace
```

---

# 56. Invariants

```text
INV-001  Workers never receive canonical workspace as working directory.
INV-002  One active attempt per task.
INV-003  One active task per workspace.
INV-004  Expired fencing tokens cannot mutate authoritative state.
INV-005  Duplicate command IDs never duplicate effects.
INV-006  Same command ID with different payload is rejected.
INV-007  State transitions obey explicit state machines.
INV-008  Every accepted candidate has a commit SHA.
INV-009  Every consequential Git side effect has operation_id.
INV-010  Every accepted state mutation persists an event.
INV-011  Filesystem watcher output is never authoritative.
INV-012  Agent-reported completion never directly marks Task DONE.
INV-013  Failed verification cannot enter review.
INV-014  Unapproved review cannot enter merge queue.
INV-015  Merge simulation never mutates canonical target.
INV-016  Only Merge Engine may mutate target branch.
INV-017  Merge queue is serialized per target branch.
INV-018  Canonical workspace must be clean before canonical merge.
INV-019  Merge target SHA must satisfy expected precondition.
INV-020  Merge conflict cannot partially update target.
INV-021  Failed post-merge validation cannot silently succeed.
INV-022  Recovery reconciles unfinished Git operations.
INV-023  No critical orchestration state exists only in RAM.
INV-024  Frozen plans are immutable.
INV-025  Every dependency edge has a reason.
INV-026  Writable tasks have explicit write scope.
INV-027  Out-of-scope writes fail closed.
INV-028  Reviewer identity differs from writing attempt.
INV-029  Task identity survives rework.
INV-030  Unknown outcome remains unknown.
INV-031  Provider transient errors do not become project failures.
INV-032  Budget exhaustion is classified distinctly.
INV-033  Live orphan sessions are not silently killed without ownership proof.
INV-034  UI restart cannot alter task truth.
INV-035  PTY output cannot certify task success.
INV-036  Read models are reconstructible from durable state.
```

---

# 57. Invariant Lifecycle

Every serious bug:

```text
postmortem
→ invariant update/new invariant
→ regression test
```

Repository:

```text
docs/
  architecture/
  invariants/
  postmortems/
  adr/
```

---

# 58. Chaos / Fault Injection Matrix

## 58.1 Command layer

```text
same TaskSubmit ×100 concurrently
same WorkspaceCreate ×100
same MergeEnqueue ×100
same ReviewSubmit repeated
```

Expected:

```text
one logical effect
same replayed result
```

## 58.2 Hub crashes

Kill Hub:

```text
after command accepted before transaction commit
after DB commit before outbox publish
after OPERATION PREPARED before Git side effect
after Git commit before DB COMMITTED
during merge laboratory
during canonical merge
```

Expected:

```text
reconcile
no duplicate side effect
no corrupt target
```

## 58.3 Agent failures

```text
kill agent mid-edit
kill agent after submit
expire lease while agent continues
restart stale agent
provider session disappears
```

Expected:

```text
stale authority blocked
recoverable work preserved
new attempt can continue
```

## 58.4 Filesystem failures

```text
delete worktree manually
rename workspace
drop watcher events
create symlink escape
dirty canonical workspace
```

Expected:

```text
detected by reconcile/Git
no silent success
```

## 58.5 Git failures

```text
branch deleted
candidate ref deleted
target branch moved
merge conflict
lockfile conflict
invalid base commit
```

Expected:

```text
explicit classification
target intact
```

## 58.6 Provider failures

```text
429
503
timeout
auth failure
binary missing
adapter malformed output
```

Expected:

```text
correct error category
circuit breaker behavior
scheduler fallback when permitted
```

---

# 59. Acceptance E2E

## E2E-001 — coder + reviewer

```text
Run
→ Task
→ Claude worktree
→ code
→ submit
→ verify
→ candidate commit
→ Codex review
→ Merge Laboratory
→ merge
→ DONE
```

Requirement:

```text
zero manual Git operations
```

## E2E-002 — parallel independent tasks

Two disjoint write scopes execute concurrently, merge serially, both succeed.

## E2E-003 — logical file conflict

Two isolated tasks edit same file.

Expected:

```text
no filesystem corruption
potential conflict warning
first candidate can land
second is revalidated
Merge Lab detects conflict if present
canonical remains intact
```

## E2E-004 — crash after Git commit before DB update

Expected:

```text
restart
operation ref discovered
DB reconciled
no duplicate commit
```

## E2E-005 — stale worker returns

Expected:

```text
STALE_AUTHORITY
no mutation
```

## E2E-006 — review rejection

Expected:

```text
same Task ID
new Attempt
new candidate
new Review
```

## E2E-007 — provider outage

Circuit opens and scheduler uses another eligible provider if policy permits.

## E2E-008 — unknown crash outcome

Insufficient evidence:

```text
OUTCOME_UNKNOWN
```

never force `FAILED` or `SUCCEEDED`.

---

# 60. Repository Structure

```text
megabrain/
│
├── apps/
│   ├── desktop/
│   │   ├── src/
│   │   └── src-tauri/
│   └── cli/
│
├── crates/
│   ├── domain/
│   ├── protocol/
│   ├── persistence/
│   ├── command-engine/
│   ├── event-store/
│   ├── outbox/
│   ├── scheduler/
│   ├── supervisor/
│   ├── sessions/
│   ├── session-holder/
│   ├── workspaces/
│   ├── scope-engine/
│   ├── git-engine/
│   ├── merge-engine/
│   ├── verification/
│   ├── review/
│   ├── context/
│   ├── artifacts/
│   ├── adapters/
│   ├── adapter-claude/
│   ├── adapter-codex/
│   ├── execution/
│   ├── recovery/
│   ├── reconcile/
│   ├── policy/
│   └── observability/
│
├── mcp/
│   └── megabrain-mcp/
│
├── provider-manifests/
│   ├── claude-code.json
│   ├── codex.json
│   ├── cursor.json
│   ├── antigravity.json
│   ├── gemini.json
│   └── opencode.json
│
├── docs/
│   ├── ARCHITECTURE.md
│   ├── PROTOCOL.md
│   ├── STATE-MACHINES.md
│   ├── RECOVERY.md
│   ├── SECURITY.md
│   ├── invariants/
│   ├── postmortems/
│   └── adr/
│
└── tests/
    ├── unit/
    ├── integration/
    ├── e2e/
    ├── concurrency/
    ├── recovery/
    └── chaos/
```

---

# 61. Dependency Direction

```text
UI / CLI / MCP
      ↓
API
      ↓
Command Engine
      ↓
Domain Services
      ↓
Ports
      ↓
Infrastructure Adapters
```

Forbidden dependencies:

```text
UI → DB
MCP → DB
AgentAdapter → task table
GitEngine → Scheduler
WorkspaceEngine → Desktop
```

---

# 62. Implementation Milestones

Do not start with dashboard.

## V0.0 — Foundation

Build:

```text
monorepo
CI
format/lint
typed IDs
domain enums
error taxonomy
state-machine skeletons
```

Gate:

```text
status consequences are exhaustive
```

## V0.1 — SQLite

Build migrations, repositories, transactions, version columns, startup validation.

Gate:

```text
restart preserves authoritative entities
```

## V0.2 — Command Engine

Build command envelope, payload hash, idempotency, optimistic concurrency, replayed results.

Gate:

```text
duplicate mutation storm passes
```

## V0.3 — Event Store + Outbox

Build event persistence, outbox, publisher, WebSocket stub.

Gate:

```text
crash after DB commit before publish eventually delivers event
```

## V0.4 — Project / Repository Identity

Build project registration, Git common-dir identity, canonical path, repo inspector.

Gate:

```text
linked worktrees map to same Project
```

## V0.5 — Operation Journal

Build operation lifecycle and reconciliation metadata before complex Git operations.

Gate:

```text
PREPARED operations survive restart
```

## V0.6 — Worktree Engine

Build deterministic worktree path/branch, create/reuse/remove, LocalCopy seam.

Gate:

```text
10 isolated worktrees can coexist
```

## V0.7 — Workspace + Scope Engine

Build write capability, realpath/symlink validation, scope drift, file touches.

Gate:

```text
scope escape rejected
```

## V0.8 — Run / Task DAG

Build Run, Task, dependencies, PlanSpec, PlanValidator, readiness.

Gate:

```text
cycle rejected
missing dependency reason rejected
```

## V0.9 — Attempts / Leases / Fencing

Build Attempt lifecycle, lease issuance/heartbeat/expiry, monotonic fencing.

Gate:

```text
stale worker attack fails
```

## V0.10 — Scheduler

Build deterministic dispatch, capability matching, concurrency, scope compatibility, provider health hooks.

Gate:

```text
disjoint tasks parallelize
overlapping/unknown tasks serialize
```

## V0.11 — Session Holder + PTY

Build holder, reconnect, scrollback, process identity.

Gate:

```text
close UI → reopen → session still alive
```

## V0.12 — Generic Adapter

Support arbitrary terminal coding agent.

Gate:

```text
generic CLI can receive context and submit
```

## V0.13 — Claude Adapter

Add manifest, PTY, MCP, hooks where practical, resume, usage.

Gate:

```text
real Claude Code edits isolated worktree and submits
```

## V0.14 — Codex Adapter

Add manifest, PTY, app-server/RPC where stable, resume, usage.

Gate:

```text
real Codex can code/review through Hub
```

## V0.15 — Context / Handoff

Build context packs, frozen decisions, dependency handoffs, provenance, budgets.

Gate:

```text
downstream Task succeeds without upstream transcript
```

## V0.16 — Verification Engine

Build Git-authoritative diff, scope check, validators, tests, typecheck/build adapters.

Gate:

```text
agent cannot fake passing verification
```

## V0.17 — Candidate Commit

Build Hub commit, metadata, operation journal, Git operation refs.

Gate:

```text
crash after Git commit before DB record recovers exactly once
```

## V0.18 — Review Engine

Build independent reviewer assignment, structured findings, changes-required flow.

Gate:

```text
rejection creates new Attempt under same Task
```

## V0.19 — Merge Laboratory

Build disposable worktree, trial merge, conflict classification, smoke tests.

Gate:

```text
conflict cannot touch canonical target
```

## V0.20 — Merge Queue

Build durable queue, target mutex, expected target SHA, canonical cleanliness.

Gate:

```text
concurrent merge requests never corrupt target
```

## V0.21 — Reconcile Engine

Build DB↔Git↔process comparison, orphan detection, repair decisions, unknown handling.

Gate:

```text
manual worktree/branch drift is recovered or loudly classified
```

## V0.22 — Circuit Breakers + Budgets

Build provider health, retry categories, open/half-open/closed, runtime budgets.

Gate:

```text
provider outage cannot create infinite retry
```

## V0.23 — Minimal Dashboard

Build Runs/Tasks/Agents/Activity first.

Gate:

```text
UI restart loses zero authoritative orchestration state
```

## V0.24 — Chaos Suite

Run full matrix.

Release gate:

```text
NO DUPLICATE LOGICAL EFFECT
NO CORRUPTED TARGET
NO STALE AUTHORITY
NO LOST ACCEPTED RESULT
NO FALSE SUCCESS
```

---

# 63. Bootstrap / Dogfooding Strategy

Do not use unfinished Mega Brain to autonomously build its own critical control plane too early.

## Stage A

Human + one coding agent builds domain/persistence/tests.

## Stage B

Use Claude/Codex manually on isolated branches/worktrees.

## Stage C

After Generic Adapter is stable, dogfood Task execution.

## Stage D

After merge/recovery are stable, use Mega Brain to build secondary adapters, UI, and non-critical features.

---

# 64. First Real Dogfood

Use a real but bounded project such as TrackeD or RastreioHub.

Start away from payments/auth critical core.

Success criteria:

```text
2 agents
2 isolated worktrees
1 reviewer
1 merge queue
zero manual Git
```

---

# 65. V0 Release Gate

Release only when:

```text
[ ] Two different coding-agent providers integrated.
[ ] Two isolated worktrees execute concurrently.
[ ] DAG dependencies work.
[ ] Agent can die and be replaced.
[ ] Stale Attempt cannot submit.
[ ] Duplicate commands are idempotent.
[ ] Candidate commit crash recovery works.
[ ] Independent review works.
[ ] Merge Laboratory protects canonical target.
[ ] Merge queue serializes target updates.
[ ] Hub restart reconciles unfinished state.
[ ] UI restart loses no orchestration truth.
[ ] Provider outage trips circuit breaker.
[ ] Scope escape is blocked.
[ ] Unknown outcome stays unknown.
[ ] Chaos suite passes.
```

Golden flow:

```text
OBJECTIVE
→ PLAN
→ DAG
→ DISPATCH
→ WORKTREE
→ AGENT
→ SUBMIT
→ OBSERVE
→ VERIFY
→ REVIEW
→ MERGE LAB
→ MERGE QUEUE
→ TARGET
→ DONE
```

with:

```text
ZERO manual Git operations
```

---

# 66. Explicitly Excluded From V0

Do not implement yet:

```text
NATS
Kafka
Redis
Postgres cluster
Kubernetes
multi-machine scheduler
cloud execution marketplace
full A2A mesh
vector database
semantic long-term memory
autonomous architecture council
LLM scheduler
automatic merge-conflict resolution
plugin marketplace
enterprise RBAC
mobile app
billing
SaaS multi-tenancy
```

Leave seams, not implementation.

---

# 67. Future Directions

## V1

```text
more provider adapters
Docker backend
remote host backend
advanced cost routing
semantic context retrieval
policy improvements
GitHub/Linear/Jira sources
PR workflows
provider-native sandboxes
```

## V2

```text
multi-machine control plane
distributed event transport
Postgres
remote workers
cloud execution
teams/orgs
advanced A2A
knowledge graph
historical scheduler optimization
```

---

# 68. Vocabulary

Use one noun for one concept:

```text
Project
Run
Plan
Task
Attempt
Session
Workspace
Lease
Fencing Token
Command
Event
Operation
Artifact
Context Pack
Handoff
Decision
Review
Merge Item
Provider
Adapter
Execution Backend
```

Avoid ambiguous reuse of `job`, `worker`, `thread`, `state`, `session`, or `context` outside their defined meaning.

---

# 69. Error Philosophy

Safe automatic repair:

```text
replay outbox
reconcile known operation marker
expire old lease
recreate derived cache
```

Unsafe guessing:

```text
mark agent failed only because heartbeat vanished
mark Task done because candidate exists
auto-resolve merge conflict
assume recycled PID is same process
```

Insufficient proof:

```text
OUTCOME_UNKNOWN
OPERATION_INDETERMINATE
ORPHANED
```

---

# 70. Core README Statement

> **Mega Brain is a durable local control plane for AI software-engineering agents. It coordinates real coding agents through isolated workspaces, structured tasks, persistent state, independent verification, and safe Git integration. Agents may crash, restart, lose context, or be replaced; project state remains authoritative and recoverable.**

And:

> **Mega Brain does not orchestrate conversations. Mega Brain orchestrates authority, state, work, evidence, and integration.**

---

# 71. Final Architecture

```text
                              USER
                               │
                               ▼
                           OBJECTIVE
                               │
                               ▼
                              RUN
                               │
                               ▼
                        PLANNING ENGINE
                               │
                               ▼
                         PLAN VALIDATOR
                               │
                               ▼
                            TASK DAG
                               │
                               ▼
                            SCHEDULER
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
       Claude                Codex              Antigravity
          │                    │                    │
        Adapter              Adapter              Adapter
          │                    │                    │
      Session Holder      Session Holder      Session Holder
          │                    │                    │
          ▼                    ▼                    ▼
      Worktree A           Worktree B           Worktree C
          │                    │                    │
          └────────────────────┼────────────────────┘
                               ▼
                           SUBMISSION
                               │
                               ▼
                       OBSERVED REALITY
                        Git / tests / scope
                               │
                               ▼
                         REALITY GATE
                               │
                               ▼
                      INDEPENDENT REVIEW
                               │
                               ▼
                       MERGE LABORATORY
                               │
                     disposable worktree
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
                  CLEAN                CONFLICT
                    │                     │
                    ▼                     ▼
               MERGE QUEUE           REPAIR / REWORK
                    │
                    ▼
                 TARGET
                    │
                    ▼
                   DONE
```

Under everything:

```text
                      DURABLE CONTROL PLANE

SQLite
├── Projects
├── Runs
├── Plans
├── Tasks
├── Attempts
├── Sessions
├── Workspaces
├── Leases
├── Fencing Tokens
├── Commands
├── Events
├── Outbox
├── Operations
├── Artifacts
├── Context Packs
├── Handoffs
├── Decisions
├── Reviews
├── Merge Queue
├── Budgets
└── Provider Health
```

Cross-cutting engines:

```text
Command Engine
State Transition Engine
Scheduler
Scope Engine
Verification Engine
Reality Gate
Review Engine
Git Engine
Merge Engine
Reconcile Engine
Recovery Engine
Circuit Breaker Registry
Context Engine
Artifact Engine
Observability
```

---

# 72. Architecture Definition of Done

Architecture is frozen when:

```text
[✓] Every mutable concept has an owner.
[✓] Every consequential transition has an authority.
[✓] Every external side effect has a recovery model.
[✓] Every asynchronous command has idempotency semantics.
[✓] Every concurrent resource has fencing semantics.
[✓] Every agent output is independently verifiable.
[✓] Every code-writing Attempt is isolated.
[✓] Every target merge is serialized and prevalidated.
[✓] Every ambiguous crash outcome may remain unknown.
[✓] Every critical state survives process restart.
```

Further architecture research becomes implementation input, not a reason to indefinitely redesign V0.

---

# 73. Directive for Coding Agents

When an AI agent implements this blueprint:

1. Do not weaken invariants to simplify implementation.
2. Do not add distributed infrastructure outside V0.
3. Do not move orchestration truth into UI state.
4. Do not make MCP the core domain.
5. Do not leak provider-specific semantics into Run/Task domain.
6. Do not trust filesystem watchers for final state.
7. Do not mark Tasks complete from agent prose.
8. Do not perform canonical merges from worker sessions.
9. Do not silently resolve ambiguous recovery states.
10. Do not introduce a status without updating exhaustive semantics/tests.
11. Do not introduce side effects without operation/recovery strategy.
12. Do not add critical mutable global RAM state without durable counterpart.
13. Do not bypass optimistic concurrency.
14. Do not bypass lease/fencing validation.
15. Do not create alternate worktree naming rules outside Worktree Engine.
16. Do not create provider-specific code where a manifest is sufficient.
17. Every invariant-breaking bug gets a regression test.

---

# 74. Recommended First Implementation Run

```text
RUN: MB-BOOTSTRAP-001

Goal:
Create the Mega Brain domain + persistence foundation.

TASK-001
Create typed IDs and domain enums.

TASK-002
Implement Run/Task/Attempt state machines.

TASK-003
Create SQLite migration framework.

TASK-004
Create projects/runs/tasks/task_attempts tables.

TASK-005
Implement optimistic concurrency repositories.

TASK-006
Implement Command envelope and command idempotency.

TASK-007
Implement Event Store + Outbox transaction pattern.

TASK-008
Write concurrency and duplicate-command tests.

Acceptance:
- invalid state transitions fail;
- duplicate command execution is replay-safe;
- restart preserves authoritative state;
- no UI yet;
- no Git worktree functionality yet;
- tests green.
```

Only after this gate passes should Worktree execution begin.

---

# 75. Final Decision

**This document is the V0 architecture baseline.**

From this point:

```text
Research → implementation input
not
Research → endless redesign
```

Any architectural change requires an ADR answering:

```text
Which invariant changes?
Which failure mode improves?
What complexity is added?
Why is V0 blocked without it?
```

If there is no strong answer:

```text
DEFER.
```

---

# Final Motto

> **Agents are disposable. State is durable.**

> **Agents do not share working trees.**

> **Agents communicate through state, not conversation.**

> **Every side effect must be replay-safe.**

> **Reported state is not observed reality.**

> **Workers cannot certify their own success.**

> **Unknown state must remain unknown.**

> **Mega Brain does not orchestrate conversations. It orchestrates authority, state, work, evidence, and integration.**

---

# Appendix A — Database Constraints and Indexes

The database must encode important invariants whenever SQLite can enforce them.

Recommended examples:

```sql
CREATE UNIQUE INDEX uq_active_attempt_per_task
ON task_attempts(task_id)
WHERE status IN ('LEASED', 'STARTING', 'ACTIVE', 'SUBMITTED');
```

```sql
CREATE INDEX idx_tasks_run_status_priority
ON tasks(run_id, status, priority DESC);
```

```sql
CREATE INDEX idx_leases_expiry
ON leases(status, expires_at);
```

```sql
CREATE INDEX idx_events_aggregate
ON events(aggregate_type, aggregate_id, id);
```

```sql
CREATE INDEX idx_outbox_pending
ON outbox(status, next_attempt_at, id);
```

```sql
CREATE INDEX idx_operations_recovery
ON operations(state, updated_at);
```

```sql
CREATE INDEX idx_merge_queue_target
ON merge_queue(project_id, target_branch, status, priority DESC, created_at);
```

Where SQLite cannot fully encode a cross-table invariant, enforce it in one domain service and test it adversarially.

Do not scatter equivalent checks across API, CLI, MCP, and scheduler implementations.

---

# Appendix B — Atomic File Writes

Any Mega Brain-owned critical file outside SQLite should use atomic replacement semantics.

Pattern:

```text
write temp file in same filesystem
fsync temp when consequential
rename/replace atomically
fsync parent directory where platform semantics require it
```

Examples:

```text
provider manifest updates
session holder metadata
recovery markers
local config
artifact manifests
```

Never treat `writeFile()` followed by process exit as durable merely because the call returned.

SQLite remains preferred for mutable coordination state.

---

# Appendix C — Process Identity

A PID alone is not a stable process identity because operating systems recycle PIDs.

Session/process observation should store, where available:

```text
pid
process start identity/start timestamp
command/executable fingerprint
holder-generated session nonce
```

On platforms with stronger process start tokens, use them.

If process identity cannot be proven after restart:

```text
UNBOUND / UNKNOWN
```

Do not claim the process is dead merely because identity lookup failed.

---

# Appendix D — Schema Versioning

All persisted structured payloads should carry explicit schema version where they can outlive a process version.

Examples:

```text
PlanSpec
TaskSpec
ContextPack
Handoff
ReviewVerdict
ProviderManifest
Operation payload
Artifact manifest
```

Policy:

```text
known older schema → migrate explicitly
known current schema → accept
unknown newer schema → read-only where safe, block consequential mutation
malformed schema → explicit error
```

Never silently reinterpret an unknown future status/schema as a current default.

---

# Appendix E — Cancellation Semantics

Cancellation must distinguish request from observed termination.

```text
CANCEL_REQUESTED
      ↓
interrupt/terminate provider
      ↓
process/session observation
      ↓
CANCELLED
```

If termination cannot be proven:

```text
CANCEL_INDETERMINATE
```

Do not mark an Attempt safely cancelled while a potentially live process still has write authority. Revoke the lease/fencing authority first; then reconcile the process separately.

---

# Appendix F — Cleanup Semantics

Cleanup is never allowed to destroy the only copy of unintegrated work without explicit evidence/policy.

Before removing a Workspace:

```text
Task terminal or explicitly abandoned?
Candidate/result safely captured?
Git diff empty or archived?
No live Session owns workspace?
No active operation references workspace?
```

If uncertain:

```text
QUARANTINE / ORPHANED
```

rather than delete.

---

# Appendix G — Artifact Integrity

Artifacts should have:

```text
artifact_id
type
project_id/run_id/task_id/attempt_id
content path or inline payload
sha256
size
schema_version
created_at
producer
```

Large artifacts stay on disk/object storage abstraction; SQLite stores metadata and hashes.

Important evidence must be content-addressable or hash-verifiable so later review/recovery can prove it is inspecting the same object.

---

# Appendix H — Review Evidence Rules

A review verdict should reference immutable evidence:

```text
candidate commit SHA
diff hash
TaskSpec revision
acceptance revision
verification artifact IDs
```

If candidate SHA changes after review:

```text
review is stale
```

and cannot authorize merge.

Do not “carry forward” approval to a modified candidate without an explicit review policy.

---

# Appendix I — Merge Evidence Rules

A Merge Laboratory result is valid only for the tuple:

```text
candidate_sha
target_sha
verification policy revision
repository identity
```

If any member changes, rerun relevant pre-merge analysis.

This prevents a clean simulation against yesterday's target from authorizing today's different merge.

---

# Appendix J — Scheduler Fairness and Starvation

V0 scheduler can remain simple, but it should prevent permanent starvation.

Recommended ordering:

```text
priority DESC
ready_since ASC
least_recently_assigned agent preference
```

A continuously arriving stream of high-priority tasks may still starve lower work by policy; expose this rather than letting it happen invisibly.

Later versions may implement priority aging.

---

# Appendix K — Retry Policy

Retries must be category-aware.

Safe automatic retries:

```text
transient provider timeout
429 with backoff
503/temporary transport failure
outbox publication
read-only Git observation
```

Do not blindly retry:

```text
scope violation
invalid TaskSpec
merge conflict
review rejection
deterministic test failure
auth misconfiguration
internal invariant violation
```

A retry always consumes an explicit budget unless the operation is an idempotent infrastructure replay that did not start a new logical Attempt.

---

# Appendix L — Provider Health Model

Track separately:

```text
adapter health
binary availability
auth health
provider API health
session launch health
recent transient failures
recent permanent failures
```

Do not collapse all provider health into one boolean.

Scheduler may exclude an agent for one capability/provider path while leaving other execution paths available.

---

# Appendix M — Policy Snapshots

A Run should store the policy snapshot used when it started.

Examples:

```text
review requirements
scope strictness
parallelism limit
budget configuration
allowed providers
merge policy
human approval gates
```

Changing global configuration should not retroactively and silently change the semantics of an already-running Run.

Explicit policy migration creates an auditable Run event/revision.

---

# Appendix N — Test Layers

## Unit

Test pure state machines, parsers, hashes, path rules, scope relationships, score functions.

## Integration

Test SQLite transactions, Git operations, worktree lifecycle, operation recovery.

## Concurrency

Test claims, fencing, duplicate commands, merge queue, concurrent reads/writes.

## E2E

Use real built binaries and real Git repositories.

## Chaos

Kill processes and corrupt assumptions at controlled seams.

Rule:

> Any behavior whose correctness depends on restart must have a restart test.

---

# Appendix O — Architecture Anti-Patterns

Reject PRs that introduce:

```text
agent-to-agent chat as authoritative workflow state
shared mutable working tree between active coding agents
Task status controlled by terminal parsing
provider-specific fields in core Task state
UI-owned scheduler state
filesystem watcher as final truth
silent merge conflict resolution
unbounded autonomous retry loop
mutable frozen PlanSpec
PID-only process ownership
implicit global write scope
critical JSON state rewritten non-atomically
new side effect without recovery semantics
new status without exhaustive consequence mapping
```

---

# Appendix P — Source Projects Used as Architectural References

The V0 design was informed by public open-source projects including:

```text
Kc1t/alethe-agents
ClipboardHealth/groundcrew
cuibuaa/flow-crew
ZaxbyHub/opencode-swarm
cristicretu/diri
codingagentsystem/cas
MistyBridge/dsh-agent-bus
automagik-dev/genie
sahithvibudhi/vibe-tree
looptroop-ai/LoopTroop
reaatech/agent-mesh
ClawSecure/railgun
OpenHands/OpenHands
OpenHands/software-agent-sdk
ClickHouse/multiagent-terminal
catlog22/maestro-flow
Yeachan-Heo/oh-my-claudecode
```

This blueprint extracts architectural lessons; it is not a declaration that source code from those repositories is copied into Mega Brain.

Before copying literal code, templates, assets, or significant implementation fragments, inspect that repository's current license and obligations.

In particular, projects under copyleft licenses require deliberate legal/licensing review before reuse in a proprietary distribution.

---

# Appendix Q — Pre-Implementation Freeze Checklist

Before `MB-BOOTSTRAP-001` starts:

```text
[ ] Create ARCHITECTURE.md from this blueprint.
[ ] Create STATE-MACHINES.md with exhaustive transition tables.
[ ] Create INVARIANTS.md with INV-001..INV-036.
[ ] Create ADR-0001: Rust Hub + SQLite + Git CLI.
[ ] Create ADR-0002: isolated worktree per writing Attempt.
[ ] Create ADR-0003: Command/Event/Operation separation.
[ ] Create ADR-0004: leases + fencing tokens.
[ ] Create ADR-0005: observed vs reported state.
[ ] Create ADR-0006: independent verification/review.
[ ] Create ADR-0007: Merge Laboratory + serialized Merge Queue.
[ ] Create ADR-0008: MCP as adapter, not core.
[ ] Create ADR-0009: provider manifests + native adapters.
[ ] Create ADR-0010: Reconcile on startup.
[ ] Pin toolchain versions.
[ ] Define Windows CI lane from day one.
[ ] Define Linux CI lane from day one.
[ ] Add macOS lane when PTY/worktree layer begins.
```

Once these are committed, implementation should proceed milestone by milestone rather than reopening foundational architecture inside every coding session.
