# Tópico 10 — UI, CLI e MCP Adapter (Prioridade Média)

> Este tópico define as interfaces de interação humana e programática com o Mega Brain V0. A UI é descartável; o CLI é a interface primária para automação; o MCP é um adapter sobre estado durável, nunca a fonte de verdade. Nenhum destes componentes pode manter estado crítico em memória ou substituir o Hub como autoridade.

## Referência no Blueprint
- Seção 2.5: Diri Synthesis
- Seção 2.6: CAS Synthesis
- Seção 2.9: VibeTree Synthesis
- Constitutional Principles 11, 14, 17, 18
- Appendix Q: Pre-Implementation Freeze Checklist (ADR-0008)

## Conteúdo Integral (sem resumo)

### Princípios Constitucionais Aplicados
11. **MCP is an adapter, not the core architecture.**
14. **The Hub owns all consequential state transitions.**
17. **The UI must be disposable.**
18. **No critical mutable orchestration state may exist only in RAM.**

### Diri Synthesis (Seção 2.5)
Adopt:
- desktop UI separate from headless engine;
- small holder process owns PTY master;
- sessions survive UI lifecycle;
- provider support can be declarative via manifests;
- provider-native adapters only where needed.

### CAS Synthesis (Seção 2.6)
Adopt:
- SQLite as shared durable coordination blackboard;
- task ledger;
- persistent project context/rules;
- MCP facade over durable state.

### VibeTree Synthesis (Seção 2.9)
Adopt:
- unified backend used by desktop/web clients;
- WebSocket for terminal/status transport;
- PTY scrollback replay;
- UI reconnect without spawning duplicate session;
- native IPC reserved for OS-only operations.

### ADR-0008: MCP as Adapter, Not Core
MCP servers exposed pelo Mega Brain são fachadas de leitura/escrita sobre o SQLite do Hub. Eles não mantêm estado próprio, não tomam decisões de scheduling, não executam merges e não validam workspaces. Qualquer lógica de negócio em um MCP server é um anti-pattern.

## Entregáveis deste Tópico
1. CLI Rust com comandos para todas as operações do Command Model (run create, task list, attempt steer, merge status, etc.).
2. Desktop UI (Tauri/Electron/native) que consome apenas eventos do Hub via WebSocket/IPC.
3. MCP server que expõe ferramentas de consulta e submissão de comandos, sem estado próprio.
4. WebSocket transport para streaming de eventos, PTY output e status updates.
5. PTY scrollback replay para reconexão de UI sem perda de contexto.
6. Native IPC apenas para operações específicas do SO (file picker, notification, credential prompt).
7. Testes que validam que UI fechada não interrompe sessões ativas.
8. Testes que validam que MCP server reiniciado não perde nem duplica estado.

## Critério de Conclusão
- CLI cobre 100% dos comandos do Command Model.
- UI é stateless; fecha e reabre sem perda de dados ou sessões.
- MCP server passa em testes de restart sem side effects.
- Nenhum componente fora do Hub mantém estado mutável de orquestração.
- PTY replay funciona após desconexão/reconexão.
- Documentação de API pública para CLI e MCP está completa.