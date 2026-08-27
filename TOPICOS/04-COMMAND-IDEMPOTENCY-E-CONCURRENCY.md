# Tópico 04 — Command Model, Idempotency e Concurrency (Prioridade Máxima)

> Este tópico define como todas as mutações consequenciais são modeladas, entregues e protegidas contra duplicação, conflito de estado e autoridade obsoleta. Nenhum efeito colateral pode existir fora deste modelo.

## Referência no Blueprint
- Seção 8: Command Model
- Seção 9: Idempotency
- Seção 10: Optimistic Concurrency
- Seção 11: Leases + Fencing Tokens
- Appendix E: Cancellation Semantics

## Conteúdo Integral (sem resumo)

### 8. Command Model
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

### 9.1 Delivery model
```text
AT LEAST ONCE DELIVERY
+
IDEMPOTENT EFFECTS
```

Do not pretend DB + filesystem + Git + provider processes can give universal exactly-once semantics.

### 9.2 Command idempotency
Repeated `command_id` + identical payload hash:
```text
return previously committed result
```

Same ID + different payload hash:
```text
409 COMMAND_ID_PAYLOAD_MISMATCH
```

### 9.3 Side-effect idempotency
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

### 10. Optimistic Concurrency
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

### 11. Leases + Fencing Tokens
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

### Appendix E — Cancellation Semantics
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

## Entregáveis deste Tópico
1. Módulo Rust `command` com envelope tipado e validação de schema.
2. Middleware de idempotência que verifica `command_id` + `payload_hash` antes de executar.
3. Implementação de optimistic concurrency com `version` em todas as entidades mutáveis.
4. Serviço de leases com geração monotônica de fencing tokens e expiração automática.
5. State machine de cancelamento que separa `CANCEL_REQUESTED` de `CANCELLED` e `CANCEL_INDETERMINATE`.
6. Testes de concorrência que validam rejeição de comandos duplicados, conflitantes e obsoletos.

## Critério de Conclusão
- Todo efeito colateral passa por um Command registrado na tabela `commands`.
- Repetição de `command_id` com mesmo payload retorna resultado anterior sem re-executar.
- Repetição com payload diferente retorna 409 `COMMAND_ID_PAYLOAD_MISMATCH`.
- Updates concorrentes falham com 409 `STATE_CONFLICT` quando `version` não bate.
- Comandos com fencing token obsoleto são rejeitados com 409 `STALE_AUTHORITY`.
- Cancelamento só transiciona para `CANCELLED` após evidência observacional de terminação.