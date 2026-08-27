# Tópico 01 — Arquitetura e Invariantes (Prioridade Máxima)

> Este tópico consolida a base constitucional do Mega Brain V0. Nenhuma implementação deve prosseguir sem que estes princípios estejam codificados, testados e documentados como invariantes executáveis.

## Referência no Blueprint
- Seção 1: Architecture Constitution
- Seção 75: Final Decision
- Appendix O: Architecture Anti-Patterns
- Appendix Q: Pre-Implementation Freeze Checklist

## Conteúdo Integral (sem resumo)

### 1.1 Seven constitutional principles
1. **Agents are disposable. State is durable.**
2. **Agents do not share working trees.**
3. **Agents communicate through state, not conversation.**
4. **Every side effect must be replay-safe.**
5. **Reported state is not observed reality.**
6. **Workers cannot certify their own success.**
7. **Unknown state must remain unknown.**

### 1.2 Additional non-negotiable principles
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

### Anti-Patterns (Appendix O — Reject PRs that introduce)
- agent-to-agent chat as authoritative workflow state
- shared mutable working tree between active coding agents
- Task status controlled by terminal parsing
- provider-specific fields in core Task state
- UI-owned scheduler state
- filesystem watcher as final truth
- silent merge conflict resolution
- unbounded autonomous retry loop
- mutable frozen PlanSpec
- PID-only process ownership
- implicit global write scope
- critical JSON state rewritten non-atomically
- new side effect without recovery semantics
- new status without exhaustive consequence mapping

### Freeze Checklist (Appendix Q — Before MB-BOOTSTRAP-001)
- [ ] Create ARCHITECTURE.md from this blueprint.
- [ ] Create STATE-MACHINES.md with exhaustive transition tables.
- [ ] Create INVARIANTS.md with INV-001..INV-036.
- [ ] Create ADR-0001: Rust Hub + SQLite + Git CLI.
- [ ] Create ADR-0002: isolated worktree per writing Attempt.
- [ ] Create ADR-0003: Command/Event/Operation separation.
- [ ] Create ADR-0004: leases + fencing tokens.
- [ ] Create ADR-0005: observed vs reported state.
- [ ] Create ADR-0006: independent verification/review.
- [ ] Create ADR-0007: Merge Laboratory + serialized Merge Queue.
- [ ] Create ADR-0008: MCP as adapter, not core.
- [ ] Create ADR-0009: provider manifests + native adapters.
- [ ] Create ADR-0010: Reconcile on startup.
- [ ] Pin toolchain versions.
- [ ] Define Windows CI lane from day one.
- [ ] Define Linux CI lane from day one.
- [ ] Add macOS lane when PTY/worktree layer begins.

## Entregáveis deste Tópico
1. `ARCHITECTURE.md` — versão condensada e navegável da constituição.
2. `INVARIANTS.md` — lista enumerada INV-001 a INV-036 com testes de violação.
3. `ADR-0001.md` a `ADR-0010.md` — decisões arquiteturais congeladas.
4. Testes unitários que validam cada invariante como asserção executável.
5. CI gate que rejeita PRs violando anti-patterns documentados.

## Critério de Conclusão
- Todos os ADRs listados acima existem e estão commitados.
- INVARIANTS.md contém 36 invariantes com referência cruzada ao blueprint.
- Pelo menos um teste automatizado por invariante crítico.
- Anti-pattern checker integrado ao pipeline de PR.