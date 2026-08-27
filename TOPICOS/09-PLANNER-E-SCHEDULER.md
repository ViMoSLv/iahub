# Tópico 09 — Planner e Scheduler (Prioridade Alta)

> Este tópico define como objetivos são decompostos em DAGs executáveis e como tarefas são despachadas para agentes. O planner produz gramática executável, não prosa; o scheduler é determinístico e justo, nunca baseado em heurísticas de LLM.

## Referência no Blueprint
- Seção 2.3: FlowCrew Synthesis
- Seção 2.11: Railgun Synthesis
- Seção 5.1: Run State Machine
- Appendix J: Scheduler Fairness and Starvation
- Appendix M: Policy Snapshots
- Constitutional Principles 3, 14, 15, 16

## Conteúdo Integral (sem resumo)

### Princípios Constitucionais Aplicados
3. **Agents communicate through state, not conversation.**
14. **The Hub owns all consequential state transitions.**
15. **The same logical task survives retries, reviews, rework, and agent replacement.**
16. **A task is not complete because an agent says "done".**

### FlowCrew Synthesis (Seção 2.3)
Adopt:
- planner emits an executable grammar, not prose only;
- dependency edges require reasons;
- explicit write capability;
- retry/repair edges belong to the graph;
- supervisor steers but does not execute;
- Reality Gate;
- honest outcomes such as `INCOMPLETE` and `OUTCOME_UNKNOWN`.

### Railgun Synthesis (Seção 2.11)
Adopt:
- max runtime;
- max step runtime;
- concurrency caps;
- bounded retries;
- deterministic workflow progression instead of agent improvisation.

### 5.1 Run State Machine
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

### Appendix J — Scheduler Fairness and Starvation
V0 scheduler can remain simple, but it should prevent permanent starvation.

Recommended ordering:
```text
priority DESC
ready_since ASC
least_recently_assigned agent preference
```

A continuously arriving stream of high-priority tasks may still starve lower work by policy; expose this rather than letting it happen invisibly.

Later versions may implement priority aging.

### Appendix M — Policy Snapshots
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

## Entregáveis deste Tópico
1. Serviço `Planner` que recebe objetivo e produz PlanSpec com DAG tipado.
2. Validador de PlanSpec que verifica dependências cíclicas, razões de aresta e write scopes.
3. Scheduler determinístico com ordenação priority DESC + ready_since ASC + least_recently_assigned.
4. Concurrency cap por projeto e por provider.
5. Bounded retry policy integrado ao scheduler (não ao agente).
6. Policy snapshot imutável por Run.
7. Reality Gate que valida pré-condições antes de transicionar Run para RUNNING.
8. Testes que validam que starvation é detectável e reportada.

## Critério de Conclusão
- Todo PlanSpec tem estrutura validada antes de ser aceito.
- Nenhuma tarefa é despachada sem satisfazer todas as dependências hard.
- Scheduler produz ordem determinística dada a mesma entrada.
- Policy snapshot da Run nunca muda implicitamente durante execução.
- Concurrency caps são respeitados mesmo sob carga.
- Retries esgotam budget e transicionam para FAILED/ESCALATED, não loop infinito.
- Reality Gate bloqueia Run se pré-condições não forem satisfeitas.