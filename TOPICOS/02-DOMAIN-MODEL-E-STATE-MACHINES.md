# Tópico 02 — Domain Model e State Machines (Prioridade Máxima)

> Este tópico define o modelo de domínio central e todas as máquinas de estado que governam o ciclo de vida das entidades do Mega Brain V0. A implementação deve seguir rigorosamente estes estados e transições; estados não listados aqui são proibidos.

## Referência no Blueprint
- Seção 4: Core Domain Model
- Seção 5: State Machines
- Appendix D: Schema Versioning

## Conteúdo Integral (sem resumo)

### 4.1 Project
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

### 4.2 Run
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

### 4.3 Task
A stable logical unit of work.

```text
TASK-142
Implement OAuth callback endpoint
```

The Task survives retry, reassignment, reviewer rejection, and rework.

### 4.4 Attempt
One execution attempt of one Task.

```text
TASK-142
  ATT-1 → Claude → failed
  ATT-2 → Codex  → stale
  ATT-3 → Claude → submitted
```

Attempt owns execution authority.

### 4.5 Session
One live agent process/provider session attached to an Attempt.

A Session may be:
- visible PTY;
- headless RPC;
- provider-native resumable thread;
- generic CLI process.

### 4.6 Workspace
Physical filesystem allocated to an Attempt.

Default:
```text
GIT_WORKTREE
```

Fallback:
```text
LOCAL_COPY
```

### 4.7 Lease
Time-bounded authority over a Task Attempt/resource.

### 4.8 Fencing Token
Monotonically increasing authority number. An older token can never mutate newer authoritative state.

### 4.9 Artifact
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

### 4.10 Review
Independent evaluation of a candidate produced by an Attempt.

### 4.11 Merge Item
Durable request to integrate one approved candidate into a target branch.

---

## State Machines (Seção 5 — Transições Explícitas)

All transitions go through explicit transition services. Arbitrary status mutations are forbidden.

### 5.1 Run
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

### 5.2 Task
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

### 5.3 Attempt
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

### 5.4 Session
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

### 5.5 Workspace
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

### 5.6 Review
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

### 5.7 Merge
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

## Entregáveis deste Tópico
1. `STATE-MACHINES.md` — tabelas exaustivas de transição com pré/pós-condições.
2. Implementação em Rust das máquinas de estado como tipos enum + funções de transição pura.
3. Testes unitários cobrindo todas as transições válidas e rejeitando transições inválidas.
4. Validação de que nenhum estado não-documentado pode ser instanciado.

## Critério de Conclusão
- Todas as entidades do domain model têm struct/tipo correspondente em código.
- Todas as transições listadas acima passam em testes.
- Nenhuma transição não-listada é permitida pelo compilador ou por runtime guard.
- STATE-MACHINES.md está completo e referenciado nos ADRs.