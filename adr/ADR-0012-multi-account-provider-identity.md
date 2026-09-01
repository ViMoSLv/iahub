# ADR-0012: Multi-Account Provider Identity and Authentication Isolation

## Status

**ACCEPTED** (2026-08-31)

## Context

The Mega Brain must support multiple independent authenticated accounts for the
same provider simultaneously. For example, two Claude accounts or two Antigravity
(Google) accounts may need to operate concurrently with isolated credentials,
sessions, quotas, and health state.

The existing architecture treats providers as flat string labels (`agents.provider`)
with no domain-level distinction between a *provider capability* and an
*authenticated identity within that provider*. This makes it architecturally
impossible to:

- Run concurrent sessions under different accounts of the same provider
- Track per-account concurrency, rate limits, or quota independently
- Guarantee authentication isolation between accounts
- Recover session-to-account bindings after restart without ambiguity

This ADR establishes the domain model foundation for multi-account support
before Topic 07 (Provider Adapters & Session Holder) implementation begins.

## Decision

### 1. Provider ≠ ProviderAccount

A **ProviderKind** represents an external capability/integration (e.g., `claude`,
`antigravity`, `codex`). It is data-driven (opaque string), not a closed enum,
so new providers can be added via manifests without recompiling the Core.

A **ProviderAccount** represents one independent authenticated identity within
a Provider. One ProviderKind may own zero or more ProviderAccounts.

```
ProviderKind("claude")
  ├── ProviderAccount("PA-CLAUDE-A") → auth profile A
  └── ProviderAccount("PA-CLAUDE-B") → auth profile B

ProviderKind("antigravity")
  ├── ProviderAccount("PA-ANTIGRAVITY-A") → Google account A
  └── ProviderAccount("PA-ANTIGRAVITY-B") → Google account B
```

### 2. Authentication Isolation Is a Hard Invariant

Two ProviderAccounts belonging to the same ProviderKind MUST NOT share mutable
authentication state, session state, quota state, health state, or concurrency
accounting. This is enforced architecturally through:

- Separate `auth_profile_id` per account (references external secret store)
- No shared credential stores at the domain level
- Per-account `ProviderAccountRuntimeState` for observations
- Session binding to exactly one `ProviderAccountId` (INV-042)

### 3. Credential References, Not Plaintext Secrets

The domain model never holds tokens, cookies, API keys, or passwords directly.
Credentials are referenced via `CredentialRef` / `AuthProfileId`, which are
opaque pointers resolved by the infrastructure layer at runtime. Logs and
events must not expose secrets.

### 4. Session Binds to Exactly One ProviderAccount

A Session's ownership chain is:

```
Session → ProviderAccount → ProviderKind
```

Not merely `Session → ProviderKind`. Once a Session is spawned under a
ProviderAccount, its account identity must not silently change. Recovery
must preserve this binding (INV-043).

### 5. Concurrency and Health Are Per-Account

`max_concurrent_sessions` lives on `ProviderAccount`, not on `ProviderKind`.
Runtime observations (`active_sessions`, `rate_limit_state`, `quota_state`,
`health`) are tracked in `ProviderAccountRuntimeState`, separate from the
aggregate to allow high-frequency updates without version bumps.

### 6. Scheduler May Select Among Compatible Accounts

The future scheduler (Topic 09) will select among eligible ProviderAccounts
based on capability match, health, authentication state, current concurrency,
and quota availability. This ADR ensures the data model supports that selection
without implementing the algorithm now.

### 7. Account Identity Is Stable and Opaque

`ProviderAccountId` is the stable identifier (e.g., `PA-CLAUDE-A`). Email or
username is optional metadata (`identity_hint`), never the primary key. Reasons:

- Account email can change
- Identity may not be email-based
- Some providers expose different identifiers
- Secrets must remain separate from domain IDs

### 8. Generic Schema, No Provider-Specific Columns

The `provider_accounts` table uses only generic columns. Provider-specific
configuration belongs in typed adapter config, not the core table. This
preserves the anti-pattern guard against `ProviderSpecificFieldsInCoreTask`.

### 9. Optimistic Concurrency Follows Existing Patterns

`ProviderAccount` mutations use the existing OCC model: `version INTEGER` with
`UPDATE ... WHERE id = ? AND version = ?`. Stale updates produce
`STATE_CONFLICT`. No last-write-wins.

### 10. Multi-Account ≠ Limit Evasion

This architecture supports legitimate multi-account usage. It does NOT implement
automatic rotation to evade provider restrictions. Rate/quota state informs
normal availability decisions; provider policies must be respected.

## Domain Types Introduced

| Type | Purpose |
|------|---------|
| `ProviderKind` | Data-driven provider identifier (opaque string) |
| `ProviderAccountId` | Stable, opaque account identifier |
| `AuthProfileId` | Reference to isolated auth profile in secret store |
| `CredentialRef` | Opaque credential pointer (never plaintext) |
| `ProviderAccountStatus` | Closed enum: Active, Unavailable, AuthenticationRequired, RateLimited, Disabled |
| `ProviderAccount` | Core aggregate: identity, auth ref, status, concurrency limit, version |
| `ProviderAccountRuntimeState` | Ephemeral observations: active sessions, rate limit, quota, health |

## Persistence

Migration v0005 adds `provider_accounts` table with:

- Generic columns only (no provider-specific fields)
- Indexes on `provider_kind` and `status` for future scheduler queries
- `version` column for optimistic concurrency
- CHECK constraints on `max_concurrent_sessions >= 0` and `version >= 1`

## Invariants Added

| ID | Statement | Coverage |
|----|-----------|----------|
| INV-042 | Every provider-backed Session is bound to exactly one ProviderAccount, and authentication/session state from one ProviderAccount must never be reused as another ProviderAccount. | PLANNED |
| INV-043 | ProviderAccount identity is durably preserved across Session recovery and reconciliation. | PLANNED |

Coverage is `PLANNED` because runtime enforcement requires Topic 07 (Session
Holder). Domain-level tests verify type-level isolation guarantees.

## Relationship to Existing Architecture

- **ADR-0011 (Delegation Model)**: Preserved. Capability-based provider selection
  remains unchanged. The chain extends: Capability → ProviderKind → ProviderAccount.
  ProviderAccount is an execution identity, not a capability.
- **Anti-pattern guard**: `ProviderSpecificFieldsInCoreTask` continues to apply.
  Provider-specific config stays in adapter layers.
- **Topics 01–04**: No impact. Command idempotency, leases, operation journal,
  and state machines are unaffected.

## Scope Boundary

This ADR covers domain model and persistence preparation ONLY.

**NOT implemented here:**

- Claude/Antigravity login automation
- Provider adapter runtime (Topic 07)
- Session Holder process management
- Scheduler balancing algorithm (Topic 09)
- Quota scraping or rate-limit detection
- Account rotation logic
- Secret store integration

## Consequences

### Positive

- Architecture correctly supports N accounts per provider before runtime work
- Authentication isolation is a type-enforced invariant, not a convention
- Per-account concurrency enables future scheduler optimization
- Generic schema avoids provider-specific coupling in core domain
- Recovery semantics are well-defined from the start

### Negative

- Two new invariants in PLANNED state increase tracking burden until Topic 07
- Runtime state separation adds a second persistence concern for accounts
- Data-driven ProviderKind loses compile-time exhaustiveness (mitigated by
  known constants and runtime validation at boundaries)

### Neutral

- ProviderAccount aggregate follows existing OCC patterns; no new concurrency
  model introduced
- Migration is forward-only; no historical migrations modified

## References

- INV-042: Session-to-account binding and auth isolation
- INV-043: Account identity preservation across recovery
- ADR-0011: Capability-based provider selection (preserved)
- ADR-0009: Provider manifests (future integration point)
- Topic 07: Provider Adapters & Session Holder (consumer of this model)
- Topic 09: Scheduler (consumer of per-account state)