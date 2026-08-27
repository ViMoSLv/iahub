# Tópico 06 — Verificação, Review e Merge (Prioridade Máxima)

> Este tópico define os gates de qualidade independente que separam "agente diz que terminou" de "trabalho integrado com segurança". Nenhum candidato pode ser mergeado sem passar por verificação observacional e review independente.

## Referência no Blueprint
- Seção 5.2: Task State Machine (VERIFYING, REVIEWING, MERGE_READY, MERGING)
- Seção 5.6: Review State Machine
- Seção 5.7: Merge State Machine
- Appendix H: Review Evidence Rules
- Appendix I: Merge Evidence Rules
- Constitutional Principles 5, 6, 7

## Conteúdo Integral (sem resumo)

### Princípios Constitucionais Aplicados
5. **Reported state is not observed reality.**
6. **Workers cannot certify their own success.**
7. **Unknown state must remain unknown.**

### 5.2 Task States (trecho relevante)
```text
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

### 5.6 Review State Machine
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

### 5.7 Merge State Machine
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

### Appendix H — Review Evidence Rules
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

Do not "carry forward" approval to a modified candidate without an explicit review policy.

### Appendix I — Merge Evidence Rules
A Merge Laboratory result is valid only for the tuple:

```text
candidate_sha
target_sha
verification policy revision
repository identity
```

If any member changes, rerun relevant pre-merge analysis.

This prevents a clean simulation against yesterday's target from authorizing today's different merge.

## Entregáveis deste Tópico
1. Serviço de Verificação Observacional que executa testes, lints e checks contra o Workspace do Attempt.
2. Serviço de Review Independente que recebe evidências imutáveis e produz verdict estruturado.
3. Merge Laboratory que simula merge em worktree descartável antes de autorizar integração real.
4. Merge Queue serializada por target branch com estado durável no SQLite.
5. Validação de freshness: review/merge-analysis são invalidados se candidate_sha ou target_sha mudar.
6. Testes que validam que agente não pode aprovar seu próprio trabalho.
7. Testes que validam que merge só ocorre após LAB_SIMULATION bem-sucedida contra o target atual.

## Critério de Conclusão
- Todo Attempt SUBMITTED passa por VERIFYING antes de REVIEWING.
- Reviewer nunca é o mesmo agent_id do Attempt que produziu o candidato.
- Verdict de review referencia SHA imutável; mudança de SHA invalida aprovação.
- Merge Laboratory roda contra target_branch atual; mudança de target invalida resultado anterior.
- Merge Queue processa um item por vez por target branch.
- Nenhum merge direto na canonical workspace sem passar pela fila.
- Estados NEEDS_CHANGES retornam tarefa para novo Attempt com contexto preservado.