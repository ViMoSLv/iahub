# Mega Brain V0 — Tópicos de Implementação

> Este diretório contém a decomposição do blueprint `MEGA_BRAIN_V0_IMPLEMENTATION_BLUEPRINT_FINAL.md` em tópicos de implementação priorizados. Cada arquivo é autocontido, não resume nem exclui conteúdo do plano original, e lista entregáveis + critérios de conclusão verificáveis.

## Ordem de Prioridade

### Prioridade Máxima (Fundação — bloqueia tudo)
1. [01-ARQUITETURA-E-INVARIANTES.md](./01-ARQUITETURA-E-INVARIANTES.md) — Constituição, ADRs, anti-patterns, freeze checklist
2. [02-DOMAIN-MODEL-E-STATE-MACHINES.md](./02-DOMAIN-MODEL-E-STATE-MACHINES.md) — Entidades, estados, transições válidas
3. [03-STORAGE-E-SQLITE-SCHEMA.md](./03-STORAGE-E-SQLITE-SCHEMA.md) — SQLite WAL, tabelas, índices, atomic writes
4. [04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md](./04-COMMAND-IDEMPOTENCY-E-CONCURRENCY.md) — Commands, idempotência, leases, fencing tokens
5. [05-WORKSPACE-ISOLATION-E-WRITE-SCOPE.md](./05-WORKSPACE-ISOLATION-E-WRITE-SCOPE.md) — Worktrees, write scope, path safety, cleanup
6. [06-VERIFICACAO-REVIEW-E-MERGE.md](./06-VERIFICACAO-REVIEW-E-MERGE.md) — Gates independentes, merge lab, fila serializada

### Prioridade Alta (Execução — habilita agentes reais)
7. [07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md](./07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md) — Adapters, session holder, process identity, health
8. [08-RECOVERY-RECONCILE-E-OBSERVABILITY.md](./08-RECOVERY-RECONCILE-E-OBSERVABILITY.md) — Startup reconcile, chaos testing, failure classification
9. [09-PLANNER-E-SCHEDULER.md](./09-PLANNER-E-SCHEDULER.md) — PlanSpec, DAG, scheduler determinístico, policy snapshots

### Prioridade Média (Interface — habilita uso humano/externo)
10. [10-UI-CLI-E-MCP-ADAPTER.md](./10-UI-CLI-E-MCP-ADAPTER.md) — CLI, desktop UI, MCP facade, WebSocket transport

## Regras de Uso

- **Nenhum tópico pode ser iniciado sem que seus predecessores de prioridade máxima estejam concluídos.**
- Cada tópico deve ser implementado até atingir todos os critérios de conclusão listados antes de marcar como DONE.
- Anti-patterns do Appendix O são gates de PR; violações bloqueiam merge independentemente de funcionalidade.
- Mudanças arquiteturais exigem ADR novo respondendo às quatro perguntas do Section 75.
- Conteúdo destes arquivos é derivativo do blueprint congelado; conflitos são resolvidos em favor do blueprint.

## Status Global

| # | Tópico | Status | Bloqueado por |
|---|--------|--------|---------------|
| 01 | Arquitetura e Invariantes | 🔲 Não iniciado | — |
| 02 | Domain Model e State Machines | 🔲 Não iniciado | 01 |
| 03 | Storage e SQLite Schema | 🔲 Não iniciado | 01, 02 |
| 04 | Command, Idempotency, Concurrency | 🔲 Não iniciado | 02, 03 |
| 05 | Workspace Isolation e Write Scope | 🔲 Não iniciado | 03, 04 |
| 06 | Verificação, Review e Merge | 🔲 Não iniciado | 04, 05 |
| 07 | Provider Adapters e Session Holder | 🔲 Não iniciado | 04, 05 |
| 08 | Recovery, Reconcile e Observability | 🔲 Não iniciado | 03, 04, 05 |
| 09 | Planner e Scheduler | 🔲 Não iniciado | 02, 04 |
| 10 | UI, CLI e MCP Adapter | 🔲 Não iniciado | 04, 06, 07 |

Legenda: 🔲 Não iniciado | 🔨 Em andamento | ✅ Concluído | ⛔ Bloqueado