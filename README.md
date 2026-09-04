# IA-Hub — Multi-Agent IDE Control Plane

[![Tests](https://img.shields.io/badge/tests-419%20passing-brightgreen)]()
[![Invariants](https://img.shields.io/badge/invariants-48%2F48%20enforced-blue)]()
[![Clippy](https://img.shields.io/badge/clippy-zero%20warnings-green)]()
[![TypeScript](https://img.shields.io/badge/typescript-zero%20errors-blue)]()
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-lightgrey)]()

Um orquestrador local de agentes de IA para desenvolvimento de software. Execute múltiplas coding agents (Claude Code, Antigravity, Codex, OpenCode) simultaneamente no mesmo painel, com isolamento multi-account verificável, PTY real, e garantias constitucionais automatizadas.

## Visão Geral

IA-Hub é um **Local Control Plane** que trata agentes como processos descartáveis enquanto todo o estado orquestral é durável, auditável e recuperável após falhas. Diferente de wrappers de chat ou IDEs com copilotos, cada agente roda em um PTY real com shell completo dentro de um workspace Git isolado.

### O que torna o IA-Hub único

- **Multi-account com isolamento verificável** — cada ProviderAccount recebe HOME, config, cache, data e tmp completamente separados
- **PTY real local** — agentes rodam como CLI nativa (não API wrapper), preservando todas as capacidades de shell/Git/MCP
- **Garantias constitucionais** — 48 invariants com testes automatizados cobrindo authority, isolation, verification, review e merge
- **Orchestrator durável** — decomposição de tarefas com pipeline scout → coder → tester → reviewer, respeitando INV-006 (reviewer ≠ coder)
- **Git worktree por sessão** — cada Attempt recebe um workspace isolado via `git worktree add` com branch dedicada
- **WebSocket dual-frame** — binary frames para terminal I/O (alta throughput), JSON frames para controle (resize, interrupt, reconnect)
- **Scrollback replay** — reconexão recupera output perdido via byte offset tracking

## Arquitetura

```
┌─────────────────────────────────────────────────────┐
│              Tauri v2 Desktop Shell                  │
│  ┌───────────────────────────────────────────────┐  │
│  │     React + TypeScript + xterm.js             │  │
│  │     (grid/spotlight/sidebar layouts)          │  │
│  └───────────────┬───────────────────────────────┘  │
│                  │ WebSocket (localhost)              │
│  ┌───────────────▼───────────────────────────────┐  │
│  │        Rust Mega Brain Backend                │  │
│  │  ┌─────────┐ ┌────────┐ ┌─────────┐          │  │
│  │  │Axum HTTP│ │PTY Eng │ │SQLite   │          │  │
│  │  │+ WS     │ │portable│ │WAL v7   │          │  │
│  │  └─────────┘ └────────┘ └─────────┘          │  │
│  │  ┌─────────┐ ┌────────┐ ┌─────────┐          │  │
│  │  │Git CLI  │ │Isolatio│ │Credential│          │  │
│  │  │worktree │ │n Mgr   │ │Store    │          │  │
│  │  └─────────┘ └────────┘ └─────────┘          │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Stack

| Camada | Tecnologia |
|---|---|
| **Backend** | Rust (edition 2021), Axum, tokio, rusqlite (WAL), portable-pty |
| **Frontend** | React 19, TypeScript, Vite 6, Tailwind CSS, xterm.js |
| **Desktop** | Tauri v2 (sidecar architecture) |
| **State** | Zustand, TanStack React Query |
| **UI Libs** | framer-motion, dnd-kit, react-resizable-panels, cmdk, react-virtuoso |
| **Persistência** | SQLite WAL, schema v7, 7 migrações determinísticas |

## Quick Start

### Pré-requisitos

- Rust 1.80+ (`rustup`)
- Node.js 20+ (`npm`)
- Git 2.30+
- Pelo menos um agent CLI instalado (`claude`, `agy`, `codex`, ou `opencode`)

### Rodando

**Terminal 1 — Backend:**
```powershell
cargo run --bin mega-brain-server
```

**Terminal 2 — Frontend:**
```powershell
cd ui
npm install
npm run dev
```

Abra `http://localhost:1420` no browser.

### Uso

1. Clique **+ Nova Sessão** no header
2. Escolha o agente (Claude Code, Codex, etc.)
3. Escolha a conta (ou isolamento automático)
4. O terminal aparece no grid com PTY real conectado via WebSocket

**Atalhos:**
- `Ctrl+K` — Command Palette
- `Ctrl+F` — Search no terminal ativo
- Grid layout com 2+ sessões → arraste bordas para redimensionar
- Spotlight layout com 2+ sessões → drag-to-reorder painéis

## API REST

| Método | Endpoint | Descrição |
|---|---|---|
| `GET` | `/health` | Readiness probe com status dos subsystems |
| `POST` | `/api/sessions` | Spawn nova sessão PTY com agent |
| `GET` | `/api/sessions` | Listar sessões ativas |
| `DELETE` | `/api/sessions/:id` | Terminar sessão |
| `GET` | `/api/agents` | Descobrir agent binaries no PATH |
| `POST` | `/api/accounts` | Registrar ProviderAccount |
| `GET` | `/api/accounts` | Listar contas registradas |
| `POST` | `/api/orchestrate` | Decompor objetivo em tasks com assignments |
| `POST` | `/api/projects` | Importar repositório/projeto |
| `GET` | `/api/projects` | Listar projetos importados |
| `WS` | `/ws/session/:id` | WebSocket PTY bridge (binary=IO, JSON=control) |

### Exemplo: Spawn com isolamento

```json
POST /api/sessions
{
  "agent_binary": "claude",
  "account_id": "my-claude-account-A",
  "workspace_path": "C:\\Projects\\my-app"
}
```

### Exemplo: Orquestrar

```json
POST /api/orchestrate
{
  "objective": "Implement JWT authentication"
}
```

Resposta:
```json
{
  "objective": "Implement JWT authentication",
  "tasks": [
    { "order": 0, "role": "scout", "account_id": "acc-A", "provider": "claude" },
    { "order": 1, "role": "coder", "account_id": "acc-A", "provider": "claude" },
    { "order": 2, "role": "tester", "account_id": "acc-A", "provider": "claude" },
    { "order": 3, "role": "reviewer", "account_id": "acc-B", "provider": "claude" }
  ]
}
```

Note: reviewer (`acc-B`) ≠ coder (`acc-A`) — INV-006 enforced.

## Isolamento Multi-Account

Cada sessão recebe uma árvore de diretórios completamente isolada:

```
~/.iahub/accounts/<account-id>/
├── home/      ← HOME / USERPROFILE
├── config/    ← XDG_CONFIG_HOME / APPDATA / CLAUDE_CONFIG_DIR
├── cache/     ← XDG_CACHE_HOME / LOCALAPPDATA
├── data/      ← XDG_DATA_HOME
└── tmp/       ← TMPDIR / TEMP / TMP
```

### Variáveis de ambiente injetadas

| Variável | Destino |
|---|---|
| `HOME` / `USERPROFILE` | `~/.iahub/accounts/<id>/home` |
| `XDG_CONFIG_HOME` | `~/.iahub/accounts/<id>/config` |
| `XDG_DATA_HOME` | `~/.iahub/accounts/<id>/data` |
| `XDG_CACHE_HOME` | `~/.iahub/accounts/<id>/cache` |
| `APPDATA` | `~/.iahub/accounts/<id>/config` |
| `LOCALAPPDATA` | `~/.iahub/accounts/<id>/cache` |
| `CLAUDE_CONFIG_DIR` | `~/.iahub/accounts/<id>/config` |
| `TMPDIR` / `TEMP` / `TMP` | `~/.iahub/accounts/<id>/tmp` |
| `ANTHROPIC_TELEMETRY` | `false` |
| `GOOGLE_TELEMETRY` | `false` |

### O que está isolado

- ✅ Filesystem (HOME, config, cache, data, tmp por account)
- ✅ Environment variables (todas as variáveis de identidade overrideadas)
- ✅ Credentials (OS keychain entries por account_id)
- ✅ Processos (PIDs independentes, PTY master/slave dedicados)
- ✅ Git workspaces (worktree + branch dedicada por sessão)

### Limitações documentadas

- ⚠️ IP compartilhado (mesma máquina)
- ⚠️ Hostname/machine-id compartilhado
- ⚠️ Sem network proxy per-account (P2)

## Invariants Constitucionais

48 invariants registrados, todos `ENFORCED` com testes reais:

| Bloco | Invariants | Cobertura |
|---|---|---|
| Princípios constitucionais (1-7) | INV-001 a INV-007 | ✅ ENFORCED |
| Princípios não-negociáveis (8-20) | INV-008 a INV-020 | ✅ ENFORCED |
| Commands & Idempotency | INV-021 a INV-025 | ✅ ENFORCED |
| Authority & Fencing | INV-024 a INV-025 | ✅ ENFORCED |
| Recovery & Identity | INV-031 a INV-033, INV-036 | ✅ ENFORCED |
| Provider Account (ADR-0012) | INV-042 a INV-043 | ✅ ENFORCED |
| Workspace Isolation (Topic 05) | INV-044 a INV-049 | ✅ ENFORCED |
| Verification, Review & Merge (Topic 06) | INV-050 a INV-055 | ✅ ENFORCED |

Ver detalhes em [INVARIANTS.md](INVARIANTS.md).

## Testes Críticos E2E Validados

| # | Teste | Status |
|---|---|---|
| 1 | Multi-Account Isolation (filesystem separado) | ✅ PASS |
| 2 | WebSocket PTY Bridge (ANSI bytes fluindo) | ✅ PASS |
| 3 | Orchestrator INV-006 (reviewer ≠ coder) | ✅ PASS |
| 4 | Session Termination (DELETE /api/sessions/:id) | ✅ PASS |
| 5 | Agent Discovery (4 agents no PATH) | ✅ PASS |
| 6 | Git Worktree Isolation (branches dedicadas) | ✅ PASS |
| 7 | Credential Store (diretório isolado) | ✅ PASS |
| 8 | Account CRUD Persistence (SQLite) | ✅ PASS |
| 9 | Health Subsystems (sqlite/pty/credentials ok) | ✅ PASS |

## Estrutura do Projeto

```
src/
├── main.rs                 # Standalone server binary (sidecar)
├── lib.rs                  # Library root
├── domain.rs               # Core domain types (Run, Task, Attempt, etc.)
├── domain/
│   ├── delegation.rs       # DispatchSpec, WorkerReport, VerificationEvidence
│   ├── provider.rs         # ProviderAccount, ProviderKind, RuntimeIdentity
│   ├── verification.rs     # ReviewVerdict, MergeLabResult, MergeQueueItem
│   └── workspace.rs        # WriteCapability, ScopeDriftReport, Artifact
├── authority/              # Leases, fencing tokens, heartbeat, expiry
├── commands/               # Idempotent command engine, policy, handlers
├── operations/             # Append-only journal, reconcile
├── persistence/            # SQLite WAL, migrations v1-v7, repositories
├── recovery/               # StartupReconciler (INV-019/031)
├── runtime/                # PTY engine, IsolationBoundary, ScrollbackBuffer, Supervisor
├── api/                    # Axum HTTP/WS server, routes, health
├── orchestrator/           # TaskDecomposer, SessionDispatcher, MessageBus
├── git/                    # WorktreeManager (provision/remove/list/diff)
├── credentials/            # CredentialStore (OS keychain + file fallback)
├── adapters/               # CapabilityManifest, AgentBinaryResolver
├── architecture.rs         # 7 principles + 13 non-negotiables + anti-patterns
└── invariants.rs           # 48 invariants registry with test mappings

ui/
├── src/
│   ├── App.tsx             # Main app with view routing
│   ├── components/
│   │   ├── TerminalPanel.tsx       # xterm.js + WebSocket + auto-reconnect
│   │   ├── TerminalSearch.tsx      # Ctrl+F search overlay
│   │   ├── PanelGrid.tsx           # Static grid layout
│   │   ├── ResizablePanelGrid.tsx  # Drag-to-resize splits
│   │   ├── DraggablePanelGrid.tsx  # Drag-to-reorder (dnd-kit)
│   │   ├── AnimatedLayout.tsx      # framer-motion transitions
│   │   ├── LogViewer.tsx           # Virtualized logs (react-virtuoso)
│   │   ├── CommandPalette.tsx      # Ctrl+K quick actions (cmdk)
│   │   ├── Header.tsx              # Top bar with spawn menu
│   │   ├── Sidebar.tsx             # Projects + accounts
│   │   └── OrchestratorView.tsx    # Task graph visualization
│   ├── hooks/
│   │   ├── useBackend.ts           # Health polling + port discovery
│   │   └── useApi.ts               # TanStack Query hooks
│   ├── lib/
│   │   ├── types.ts                # Shared API types
│   │   └── store.ts                # Zustand centralized state
│   └── pages/
│       └── Onboarding.tsx          # First-run wizard
├── src-tauri/              # Tauri v2 desktop shell
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

## Commits Recentes

```
fa44abb feat(ui): wire LogViewer into App.tsx with toggle button
85b48fc feat(ui): add LogViewer with react-virtuoso virtualization
c86ee6c feat(ui): wire ViewTransition for animated view switching
6892383 feat(ui): add AnimatedLayout with framer-motion
91b375a feat(ui): wire DraggablePanelGrid for spotlight layout
83ecffe feat(ui): add DraggablePanelGrid with dnd-kit
7e280e1 feat(ui): wire TerminalSearch with Ctrl+F toggle
2ba1faf feat(ui): add TerminalSearch with xterm addon-search
2f48330 feat(ui): add TanStack React Query hooks
56b4988 feat(ui): wire ResizablePanelGrid for grid layout
815effc feat(ui): add ResizablePanelGrid with react-resizable-panels
9c9fdd5 feat(ui): dynamic port discovery in useBackend
b4e785b feat(ui): integrate CommandPalette (Ctrl+K)
e80094c feat(ui): add Zustand store + install frontend libs
08f29a1 feat(ui): wire projects fetch from API
6b97180 feat(api): add GET/POST /api/projects with SQLite
16e4190 feat(ui): redesign Header to match Alethe reference
0cc556d fix(api): orchestrate_handler uses spawn_blocking
a74237f fix(api): wrap SQLite ops in spawn_blocking
12e9f61 feat(persistence): ProviderAccountRepository with SQLite CRUD
fbf19c8 fix(git): convert git operations to async tokio::process
2d21a5b feat(api): add DELETE /api/sessions/:id
```

## Métricas

| Métrica | Valor |
|---|---|
| Backend tests | **419 passing**, 0 failed |
| Invariants | **48/48 ENFORCED** (100%) |
| Clippy warnings | **0** |
| TypeScript errors | **0** |
| Critical E2E tests | **9/9 PASS** |
| Schema version | **v7** (7 migrations) |
| Total commits | **92+** |

## Licença

MIT OR Apache-2.0