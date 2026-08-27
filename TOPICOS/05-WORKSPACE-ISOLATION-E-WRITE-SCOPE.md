# Tópico 05 — Workspace Isolation e Write Scope (Prioridade Máxima)

> Este tópico define como os agentes recebem espaços de trabalho isolados e capacidades de escrita explícitas. Nenhum agente pode operar fora destes limites; violações são falhas de segurança, não erros de lógica.

## Referência no Blueprint
- Seção 12: Repository Identity
- Seção 13: Workspace Isolation
- Seção 14: Write Scope / Capability Engine
- Appendix F: Cleanup Semantics
- Appendix G: Artifact Integrity

## Conteúdo Integral (sem resumo)

### 12. Repository Identity
Resolve repository identity from Git:

```text
git rev-parse --git-common-dir
git rev-parse --show-toplevel
```

Store:

```text
git_common_dir
repository_fingerprint
```

Do not identify the same repository solely from workspace path.

### 13.1 Default Workspace Mode
```text
GIT_WORKTREE
```

Example:
```text
C:\MegaBrain\workspaces\
  tracked\
    TASK-142-ATT-01\
    TASK-143-ATT-01\
```

### 13.2 Fallback Workspace Mode
```text
LOCAL_COPY
```

Only when worktree is impossible or an execution backend requires it.

### 13.3 Canonical workspace
The original user project directory is the integration workspace.

Workers never receive it as their working directory.

If the user wants to code manually while a Run is active, create a dedicated human worktree.

### 14. Write Scope / Capability Engine
Workers receive an explicit write capability.

```json
{
  "capability_id": "WCAP-18",
  "task_id": "TASK-142",
  "attempt_id": "ATT-3",
  "fencing_token": 42,
  "allow": [
    "src/auth/**",
    "tests/auth/**"
  ],
  "deny": [
    ".git/**",
    ".megabrain/**",
    "src/finance/**"
  ],
  "expires_at": "..."
}
```

### 14.1 Path safety
Canonicalize and validate:

```text
realpath
junctions
symlinks
case normalization
Windows path semantics
```

Ambiguity fails closed.

### 14.2 Shell write seam
Adapters may inspect provider tool calls / shell commands and classify potential writes such as:

```text
cp
mv
rm
mkdir
sed -i
curl -o
wget -O
tar -x
unzip
git reset --hard
git clean -fd
Set-Content
Out-File
Copy-Item
Move-Item
Remove-Item
```

V0 does not require a perfect universal shell parser, but the policy hook must exist.

### 14.3 Scope drift
At submission, Git is authoritative.

Diff against base commit determines actual write scope exercised.

Scope violations at submission time fail the Attempt regardless of agent intent.

### Appendix F — Cleanup Semantics
Cleanup is never allowed to destroy the only copy of unintegrated work without explicit evidence/policy.

Before removing a Workspace:

```text
Task terminal or explicitly abandoned?
Candidate/result safely captured?
Git diff empty or archived?
No live Session owns workspace?
No active operation references workspace?
```

If uncertain:

```text
QUARANTINE / ORPHANED
```

rather than delete.

### Appendix G — Artifact Integrity
Artifacts should have:

```text
artifact_id
type
project_id/run_id/task_id/attempt_id
content path or inline payload
sha256
size
schema_version
created_at
producer
```

Large artifacts stay on disk/object storage abstraction; SQLite stores metadata and hashes.

Important evidence must be content-addressable or hash-verifiable so later review/recovery can prove it is inspecting the same object.

## Entregáveis deste Tópico
1. Módulo `workspace` com criação/remoção segura de Git worktrees via CLI.
2. Resolver de identidade de repositório baseado em `git-common-dir`.
3. Motor de Write Scope com validação de caminhos Windows/Linux/macOS.
4. Shell write seam hook para classificação de comandos mutativos.
5. Verificador de scope drift no momento de submissão (diff vs capability).
6. Política de cleanup com quarentena para workspaces órfãos.
7. Sistema de artefatos com hash SHA-256 e metadados no SQLite.

## Critério de Conclusão
- Todo Attempt recebe um Workspace isolado antes de iniciar.
- Nenhum agente consegue escrever fora dos paths listados em sua capability.
- Symlinks/junctions fora do escopo são bloqueados mesmo se apontarem para dentro.
- Submissões com diff fora do write_scope falham automaticamente.
- Workspaces só são removidos após verificação completa de segurança.
- Artefatos críticos são verificáveis por hash independente do produtor.