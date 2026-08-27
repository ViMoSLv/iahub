# Tópico 07 — Provider Adapters e Session Holder (Prioridade Alta)

> Este tópico define como o Mega Brain V0 se conecta a agentes reais, gerencia sessões duráveis e observa processos sem confiar em auto-relato. O Hub nunca fala diretamente com um LLM; ele fala com um Adapter que implementa um contrato observacional.

## Referência no Blueprint
- Seção 2: Open-Source Synthesis (Alethe, Diri, Agent Mesh, Railgun)
- Seção 5.4: Session State Machine
- Appendix C: Process Identity
- Appendix L: Provider Health Model
- Constitutional Principles 1, 5, 6, 19

## Conteúdo Integral (sem resumo)

### Princípios Constitucionais Aplicados
1. **Agents are disposable. State is durable.**
5. **Reported state is not observed reality.**
6. **Workers cannot certify their own success.**
19. **All external side effects must be journaled.**

### 5.4 Session State Machine
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

### Appendix C — Process Identity
A PID alone is not a stable process identity because operating systems recycle PIDs.

Session/process observation should store, where available:
```text
pid
process start identity/start timestamp
command/executable fingerprint
holder-generated session nonce
```

On platforms with stronger process start tokens, use them.

If process identity cannot be proven after restart:
```text
UNBOUND / UNKNOWN
```

Do not claim the process is dead merely because identity lookup failed.

### Appendix L — Provider Health Model
Track separately:
```text
adapter health
binary availability
auth health
provider API health
session launch health
recent transient failures
recent permanent failures
```

Do not collapse all provider health into one boolean.

Scheduler may exclude an agent for one capability/provider path while leaving other execution paths available.

### Síntese de Projetos de Referência (Seção 2)
- **Alethe:** real PTY sessions; multiple coding agents in parallel; provider handoff; Claude hooks.
- **Diri:** desktop UI separate from headless engine; small holder process owns PTY master; sessions survive UI lifecycle; provider support can be declarative via manifests.
- **Agent Mesh:** per-provider circuit breaker; provider health-aware routing; pluggable transports; structured observability.
- **Railgun:** max runtime; max step runtime; concurrency caps; bounded retries; deterministic workflow progression instead of agent improvisation.

## Entregáveis deste Tópico
1. Trait/interface `ProviderAdapter` com métodos para spawn, steer, heartbeat, terminate e observe.
2. Implementação de Session Holder como processo separado que sobrevive ao ciclo de vida da UI.
3. Módulo de Process Identity com validação de PID + start timestamp + nonce.
4. Circuit breaker por provider com estados independentes para auth, API, binary e session launch.
5. Telemetria estruturada que alimenta a tabela `provider_health` e `circuit_breakers`.
6. Adapters iniciais: Claude Code CLI, OpenAI Codex CLI, Cursor CLI, Gemini CLI, generic terminal.
7. Testes que validam que sessão perdida é detectada mesmo quando agente não reporta falha.

## Critério de Conclusão
- Todo adapter implementa o contrato observacional completo.
- Session Holder mantém sessões vivas após fechamento da UI.
- Process Identity distingue processos reciclados pelo SO.
- Provider health é granular; falha de auth não bloqueia execução local.
- Nenhum estado de sessão é inferido apenas por parsing de terminal.
- Heartbeat é emitido pelo Holder/sidecar, nunca pelo LLM.
- Adapters são testados contra falhas simuladas de rede, auth e binário.