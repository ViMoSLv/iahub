# ADR-0008: MCP as Adapter, Not Core

## Status
ACCEPTED

## Context
Model Context Protocol (MCP) provides a standardized interface for LLMs to interact with external tools and data sources. There is a risk that MCP servers become the de facto orchestration layer, accumulating business logic, state management, and decision-making that should belong to the Hub. This creates a shadow architecture that bypasses invariants, evades audit trails, and makes recovery impossible.

Key constraints:
- The Hub owns all consequential state transitions (Principle 14).
- No critical mutable orchestration state may exist only in RAM (Principle 18).
- MCP is an adapter, not the core architecture (Principle 11).
- All external side effects must be journaled (Principle 19).
- UI and CLI are also adapters over the same durable state.

## Decision
MCP servers exposed by Mega Brain are **read/write facades** over the Hub's SQLite database. They contain **zero business logic**, **zero scheduling decisions**, **zero merge authorization**, and **zero workspace validation**.

### Allowed in MCP Servers
- Read queries against events, tasks, attempts, artifacts tables.
- Command submission (delegated entirely to Hub command handler).
- Streaming event subscriptions via SSE/WebSocket.
- Schema discovery and tool listing.

### Forbidden in MCP Servers
- State machine transition logic.
- Lease or fencing token management.
- Workspace path validation or creation.
- Merge simulation or queue management.
- Agent session spawning or steering.
- Caching of orchestration state.
- Any mutation not routed through Hub command handlers.

### Architectural Position
```text
LLM ↔ MCP Server ↔ SQLite ↔ Hub Daemon ↔ Git/Filesystem/Agents
         ↑                    ↑
      Facade only        Authority + Side Effects
```

All three client interfaces (CLI, Desktop UI, MCP) consume the same event stream and submit commands through the same idempotent command handler. None maintains independent state.

## Consequences
### Positive
- Single source of truth preserved; no shadow state in MCP layer.
- Invariants enforced uniformly regardless of client interface.
- MCP server restart has zero impact on orchestration state.
- Audit trail complete: all mutations visible in commands/events tables.
- Easier to add new client interfaces without duplicating logic.
- Security boundary clear: MCP cannot bypass Hub authority.

### Negative
- MCP latency increased by Hub round-trip for every operation.
- MCP server cannot function independently of Hub daemon.
- Tool implementations are thinner; more boilerplate per tool.
- Developers accustomed to "smart" MCP servers must adjust expectations.
- Testing requires Hub integration, not just MCP unit tests.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Developer adds business logic to MCP for convenience | Code review gate; lint rule forbidding state mutations outside command handlers; architecture decision record reference in PR template |
| MCP caching creates stale reads | No caching allowed; read-through to SQLite with WAL concurrency |
| MCP server becomes bottleneck under load | Horizontal scaling of stateless MCP instances; Hub remains single writer |
| Client expects MCP to handle retries/idempotency | Document that idempotency is command_id-based at Hub level; MCP passes through unchanged |
| New MCP tool inadvertently violates invariant | Mandatory invariant checklist for new tools; automated test verifying no direct state mutation |

## Related
- INV-011: MCP is adapter
- INV-014: Hub owns transitions
- INV-018: No RAM-only state
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/10-UI-CLI-E-MCP-ADAPTER.md](../TOPICOS/10-UI-CLI-E-MCP-ADAPTER.md)