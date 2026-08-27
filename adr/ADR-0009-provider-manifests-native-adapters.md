# ADR-0009: Provider Manifests + Native Adapters

## Status
ACCEPTED

## Context
Mega Brain V0 must support multiple coding agents (Claude Code, OpenAI Codex CLI, Cursor CLI, Gemini CLI, generic terminal agents) without coupling core orchestration logic to any single provider. Each provider has distinct capabilities, authentication models, session semantics, and failure modes. Hardcoding provider-specific behavior into the scheduler or command handler violates Constitutional Principle 11 (MCP is adapter, not core) and makes adding new providers a breaking change.

Key constraints:
- Agents are disposable; providers are replaceable (Principle 1).
- MCP is an adapter, not the core architecture (Principle 11).
- Provider health must be granular, not a single boolean (Appendix L).
- New providers must be addable without modifying Hub core logic.
- Windows, Linux, and macOS may require different binary paths or session mechanics per provider.
- Authentication, API availability, binary presence, and session launch are independent failure domains.

## Decision
We use **declarative provider manifests** to describe agent capabilities and configuration, paired with **native adapter implementations** that fulfill a common `ProviderAdapter` trait.

### Provider Manifest
A JSON/YAML file declaring:
```json
{
  "provider_id": "claude-code-cli",
  "display_name": "Claude Code CLI",
  "adapter_kind": "native-cli",
  "binary": {
    "windows": "claude.exe",
    "linux": "claude",
    "macos": "claude"
  },
  "auth": {
    "type": "env-var",
    "variable": "ANTHROPIC_API_KEY"
  },
  "capabilities": [
    "code-edit",
    "terminal-exec",
    "file-read",
    "git-ops"
  ],
  "session": {
    "type": "pty",
    "heartbeat_interval_ms": 5000,
    "max_idle_ms": 30000,
    "supports_resume": false
  },
  "limits": {
    "max_parallel_tasks": 3,
    "max_step_runtime_ms": 300000,
    "max_total_runtime_ms": 1800000
  },
  "health_checks": {
    "binary": "which claude",
    "auth": "claude --version",
    "api": "curl -s https://api.anthropic.com/v1/messages"
  }
}
```

Manifests are loaded at startup and validated against schema. Invalid manifests fail closed with classified error.

### Native Adapter Trait
```rust
trait ProviderAdapter {
    fn spawn(&self, config: SpawnConfig) -> Result<SessionHandle>;
    fn steer(&self, session: &SessionHandle, instruction: SteerInstruction) -> Result<()>;
    fn heartbeat(&self, session: &SessionHandle) -> Result<HeartbeatResult>;
    fn terminate(&self, session: &SessionHandle) -> Result<TerminationEvidence>;
    fn observe(&self, session: &SessionHandle) -> Result<ObservedState>;
    fn health_check(&self, check_type: HealthCheckType) -> Result<HealthStatus>;
}
```

Each provider implements this trait. The Hub never calls provider-specific APIs directly.

### Health Model Granularity
Per Appendix L, track separately:
- `adapter_health` — can the adapter code run?
- `binary_availability` — is the executable present and executable?
- `auth_health` — are credentials valid and unexpired?
- `provider_api_health` — is the upstream API reachable?
- `session_launch_health` — can new sessions be spawned?
- `recent_transient_failures` — count in sliding window
- `recent_permanent_failures` — count since last success

Scheduler excludes agents for specific capability/provider paths while leaving others available. Auth failure does not block local-only operations.

### Adding a New Provider
1. Write manifest file.
2. Implement `ProviderAdapter` trait.
3. Register adapter in Hub configuration.
4. Add integration tests for spawn/steer/terminate/observe.
5. No changes to scheduler, command handler, or state machines required.

## Consequences
### Positive
- New providers added without touching core orchestration.
- Capabilities drive scheduling decisions, not hardcoded provider names.
- Health granularity enables partial availability (e.g., auth expired but binary works for local tasks).
- Manifest validation catches misconfiguration before runtime.
- Cross-platform binary resolution handled declaratively.
- Testing can mock adapters without mocking entire Hub.

### Negative
- Manifest schema must evolve carefully to avoid breaking existing providers.
- Native adapter implementation effort per provider.
- Health check definitions may become stale as providers update CLIs.
- Capability confidence scoring requires calibration over time.
- Manifest parsing adds startup complexity.

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Manifest schema drift breaks providers | Versioned schema; backward-compatible defaults; migration tooling |
| Adapter implementation diverges from contract | Integration test suite per adapter; contract tests in CI |
| Health checks give false positives/negatives | Multiple signal combination; grace periods; manual override path |
| Capability list incomplete causes scheduling failures | Conservative default (no capability = not scheduled); learning from observed behavior in future versions |
| Binary path changes after OS/provider update | Auto-detection fallback; manifest hot-reload; alert on binary missing |
| Auth token rotation not detected | Periodic auth health check; proactive refresh where supported; escalate on expiry |

## Related
- INV-001: Agents disposable
- INV-011: MCP is adapter
- INV-032: Process identity
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [TOPICOS/07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md](../TOPICOS/07-PROVIDER-ADAPTERS-E-SESSION-HOLDER.md)