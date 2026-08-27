# Tópico 03 — Storage e SQLite Schema (Prioridade Máxima)

> Este tópico define a arquitetura de armazenamento durável do Mega Brain V0. SQLite é a fonte única de verdade para estado de orquestração; nenhum outro banco, fila ou cache é permitido no V0.

## Referência no Blueprint
- Seção 6: Storage Architecture
- Seção 7: Baseline SQL Semantics
- Appendix A: Database Constraints and Indexes
- Appendix B: Atomic File Writes
- Appendix D: Schema Versioning

## Conteúdo Integral (sem resumo)

### 6.1 SQLite
Use SQLite WAL for V0.

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;
PRAGMA busy_timeout = 5000;
```

The Hub daemon is the only logical writer.

UI, CLI, MCP adapters, provider adapters, and agents never open the coordination database directly.

### 6.2 Minimum tables
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

### 7.1 projects
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

### 7.2 runs
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

### 7.3 tasks
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

### 7.4 task_dependencies
```sql
CREATE TABLE task_dependencies (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
  reason TEXT NOT NULL,
  dependency_type TEXT NOT NULL DEFAULT 'hard',
  PRIMARY KEY (task_id, depends_on_task_id)
);
```

### 7.5 task_attempts
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

### 7.6 agents
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

### 7.7 capabilities
```sql
CREATE TABLE agent_capabilities (
  agent_id TEXT NOT NULL REFERENCES agents(id),
  capability TEXT NOT NULL,
  confidence REAL NOT NULL DEFAULT 1.0,
  source TEXT NOT NULL,
  PRIMARY KEY (agent_id, capability)
);
```

### 7.8 sessions
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

### 7.9 workspaces
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

### 7.10 leases
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

### 7.11 commands
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

### 7.12 events
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

### 7.13 outbox
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

### 7.14 operations
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

### 7.15 reviews
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

### 7.16 merge_queue
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

### Appendix A — Database Constraints and Indexes
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

### Appendix B — Atomic File Writes
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

## Entregáveis deste Tópico
1. Migrations SQL completas e versionadas para todas as tabelas listadas.
2. Seed/test fixtures que exercitam constraints e índices.
3. Módulo Rust `storage` com connection pool WAL + busy_timeout configurado.
4. Testes de integração que validam foreign keys, unique constraints e partial indexes.
5. Documentação de schema versioning para payloads JSON persistidos.

## Critério de Conclusão
- Todas as tabelas da seção 6.2 existem no schema migrado.
- Todos os CREATE TABLE da seção 7 passam em testes de integração.
- Índices recomendados no Appendix A estão aplicados.
- Nenhum acesso direto ao SQLite fora do Hub daemon.
- Atomic file write pattern implementado e testado para arquivos críticos não-SQLite.