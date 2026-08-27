# Tópico 08 — Recovery, Reconcile e Observability (Prioridade Alta)

> Este tópico define como o Mega Brain V0 sobrevive a crashes, reinícios, processos órfãos e estado inconsistente. O sistema deve ser correto após qualquer falha, não apenas durante operação normal. Observabilidade é parte da recuperação, não um add-on.

## Referência no Blueprint
- Seção 5: State Machines (todos os estados de falha)
- Seção 9.3: Side-effect idempotency (operations table)
- Seção 11: Leases + Fencing Tokens
- Appendix C: Process Identity
- Appendix E: Cancellation Semantics
- Appendix F: Cleanup Semantics
- Appendix N: Test Layers (Chaos)
- Constitutional Principles 4, 7, 10, 18, 19, 20

## Conteúdo Integral (sem resumo)

### Princípios Constitucionais Aplicados
4. **Every side effect must be replay-safe.**
7. **Unknown state must remain unknown.**
10. **Filesystem watchers are hints, never authority.**
18. **No critical mutable orchestration state may exist only in RAM.**
19. **All external side effects must be journaled.**
20. **Failures must be classified, not flattened into generic `FAILED`.**

### Operations Table (Seção 7.14)
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

Every consequential operation type:
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

### Startup Reconcile (Synthesis from Groundcrew, Alethe)
On startup, the Hub must:
1. Scan all leases for expiry → revoke stale authority.
2. Scan all operations in non-terminal states → classify as recoverable or failed.
3. Scan all workspaces → verify filesystem existence matches DB state.
4. Scan all sessions → verify process identity (Appendix C).
5. Scan all tasks/attempts → detect orphaned or stuck states.
6. Emit reconcile events for every correction made.

Do not trust graceful shutdown. Assume the previous process died mid-write.

### Lease Expiry and Stale Authority
When a lease expires:
```text
1. Mark lease EXPIRED
2. Increment fencing token for resource
3. Transition owning Attempt to STALE or LOST
4. Emit event for scheduler to reassign
5. Do NOT assume workspace is clean — quarantine if uncertain
```

### Process Death Detection
Combine multiple signals:
```text
- PID no longer exists
- Process start timestamp mismatch
- Session nonce mismatch
- Heartbeat timeout exceeded
- PTY read returns EOF/error
- Exit code observed
```

If any signal contradicts another:
```text
UNRESPONSIVE / UNKNOWN
```
not `TERMINATED`.

### Failure Classification (Principle 20)
Failures must be categorized:
```text
TRANSIENT_PROVIDER     → retry with backoff
AUTH_EXPIRED           → refresh or escalate
BINARY_MISSING         → mark agent unavailable
SCOPE_VIOLATION        → fail attempt, do not retry
MERGE_CONFLICT         → return to planning/review
TEST_FAILURE           → return to rework
INVARIANT_VIOLATION    → escalate, never auto-recover
WORKSPACE_CORRUPT      → quarantine + new attempt
LEASE_STALE            → revoke + reassign
OPERATION_PARTIAL      → reconcile or rollback
UNKNOWN                → preserve evidence, escalate
```

Never store bare `FAILED` without a `failure_reason` tag.

### Observability Requirements
All state transitions emit events to the `events` table.
All external side effects journal to the `operations` table.
All provider interactions log to structured telemetry.
All reconcile actions produce audit entries.

UI, CLI, and MCP read from these tables — they do not maintain parallel state.

### Chaos Testing Requirement (Appendix N)
> Any behavior whose correctness depends on restart must have a restart test.

Required chaos scenarios for V0:
```text
- Kill Hub during CREATE_WORKTREE → verify workspace reconciled
- Kill Hub during MERGE_SIMULATION → verify no partial merge
- Kill agent process mid-task → verify attempt marked LOST
- Corrupt SQLite WAL → verify busy_timeout + recovery
- Delete worktree manually → verify BROKEN/ORPHANED detection
- Restart Hub with expired leases → verify revocation
- Duplicate command delivery → verify idempotent result
- Concurrent claim of same task → verify optimistic conflict
```

## Entregáveis deste Tópico
1. Serviço `Reconciler` executado no startup do Hub com passos 1-6 acima.
2. Módulo `LeaseManager` com expiração automática e fencing token monotônico.
3. Detector de morte de processo combinando múltiplos sinais.
4. Classificador de falhas com enum tipado e mapeamento para ações.
5. Event emitter integrado a todas as transições de estado.
6. Operation journal com recovery semantics para cada tipo consequencial.
7. Suite de testes de chaos cobrindo todos os cenários listados.
8. Documentação de runbook para recuperação manual quando automatizada falhar.

## Critério de Conclusão
- Hub reinicia e recupera estado consistente em < 30 segundos para 100 tasks.
- Nenhum workspace órfão permanece sem classificação após reconcile.
- Nenhuma operação parcial permanece em EXECUTING após reconcile.
- Todos os eventos de reconcile são auditáveis na tabela `events`.
- Testes de chaos passam em CI para Windows e Linux.
- Falhas sempre têm `failure_reason` classificado, nunca genérico.
- Processos reciclados pelo SO não são confundidos com sessões vivas.