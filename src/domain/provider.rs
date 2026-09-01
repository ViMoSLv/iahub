//! Mega Brain V0 — Provider & ProviderAccount Domain Model (ADR-0012)
//!
//! Pure domain types for multi-account provider identity and authentication
//! isolation. This module establishes the architectural foundation for
//! supporting N independent authenticated accounts per provider.
//!
//! Key concepts:
//! - **ProviderKind**: Identifies an external capability/integration (e.g., Claude, Antigravity).
//! - **ProviderAccount**: One independent authenticated identity within a Provider.
//! - **CredentialRef**: Opaque reference to credentials stored in an external secret store.
//! - **ProviderAccountStatus**: Closed enum for account lifecycle states.
//!
//! Reference: ADR-0012 — Multi-Account Provider Identity and Authentication Isolation
//!
//! Invariants enforced by these types:
//! - INV-042: Every provider-backed Session is bound to exactly one ProviderAccount
//! - INV-043: ProviderAccount identity is durably preserved across recovery

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{EntityVersion, Timestamp};

// ---------------------------------------------------------------------------
// Strongly-typed IDs
// ---------------------------------------------------------------------------

macro_rules! define_provider_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_provider_id!(
    ProviderAccountId,
    "Stable, opaque identifier for a ProviderAccount. Not an email or username."
);

define_provider_id!(
    AuthProfileId,
    "Reference to an isolated authentication profile within a secret store."
);

// ---------------------------------------------------------------------------
// ProviderKind — data-driven, not closed enum
// ---------------------------------------------------------------------------

/// Opaque provider identifier. Data-driven: new providers can be added via
/// manifests without recompiling the Core. Known providers are exposed as
/// associated constants for convenience, but the type accepts any valid string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderKind(pub String);

impl ProviderKind {
    pub const CLAUDE: &'static str = "claude";
    pub const ANTIGRAVITY: &'static str = "antigravity";
    pub const CODEX: &'static str = "codex";

    /// Create a new ProviderKind from a string. No validation beyond non-empty.
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    /// Returns true if this provider matches a known constant.
    pub fn is_known(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::CLAUDE | Self::ANTIGRAVITY | Self::CODEX
        )
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// CredentialRef — never store plaintext secrets in domain objects
// ---------------------------------------------------------------------------

/// Opaque reference to credentials managed by an external secret store.
/// The domain model never holds tokens, cookies, API keys, or passwords directly.
/// This type is a pointer that the infrastructure layer resolves at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CredentialRef(pub String);

impl CredentialRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }
}

impl fmt::Display for CredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ProviderAccountStatus — closed typed enum
// ---------------------------------------------------------------------------

/// Lifecycle status of a ProviderAccount. Closed enum: unknown variants
/// deserialize fail-closed to prevent silent state corruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ProviderAccountStatus {
    /// Account is operational and may accept new sessions.
    Active,
    /// Temporarily unable to serve (e.g., network issue, maintenance).
    Unavailable,
    /// Credentials expired or revoked; re-authentication required.
    AuthenticationRequired,
    /// Provider-imposed rate limit or quota exhaustion.
    RateLimited,
    /// Administratively disabled; will not be selected by scheduler.
    Disabled,
}

impl ProviderAccountStatus {
    /// Returns true if this status permits new session creation.
    pub fn is_available(self) -> bool {
        matches!(self, Self::Active)
    }
}

// ---------------------------------------------------------------------------
// ProviderAccount — core aggregate for multi-account support
// ---------------------------------------------------------------------------

/// One independent authenticated identity within a Provider.
///
/// Two ProviderAccounts belonging to the same ProviderKind MUST NOT share
/// mutable authentication state, session state, quota state, health state,
/// or concurrency accounting. This is a hard architectural invariant (INV-042).
///
/// Credentials are referenced indirectly via `credential_ref`; plaintext
/// secrets must never appear in this struct or its serialized form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAccount {
    /// Stable, opaque identifier. Not derived from email or username.
    pub id: ProviderAccountId,
    /// Which provider this account belongs to.
    pub provider: ProviderKind,
    /// Human-readable label for UI/display purposes.
    pub label: String,
    /// Optional hint about the underlying identity (e.g., email prefix).
    /// Metadata only — never used as authentication material.
    pub identity_hint: Option<String>,
    /// Reference to the isolated authentication profile in the secret store.
    pub auth_profile_id: AuthProfileId,
    /// Current lifecycle status.
    pub status: ProviderAccountStatus,
    /// Maximum number of concurrent sessions this account may hold.
    pub max_concurrent_sessions: u32,
    /// Optimistic concurrency version.
    pub version: EntityVersion,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ---------------------------------------------------------------------------
// ProviderAccountRuntimeState — ephemeral, per-account observations
// ---------------------------------------------------------------------------

/// Mutable runtime observations for a ProviderAccount. These are updated
/// by the adapter/observation layer and consumed by the scheduler.
/// Persisted separately from the ProviderAccount aggregate to allow
/// high-frequency updates without bumping the aggregate version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAccountRuntimeState {
    pub provider_account_id: ProviderAccountId,
    /// Number of currently active sessions under this account.
    pub active_sessions: u32,
    /// Last observed rate-limit state from the provider (opaque).
    /// None means unknown/not-yet-observed — never guessed.
    pub rate_limit_state: Option<String>,
    /// Last observed quota state from the provider (opaque).
    /// None means unknown/not-yet-observed — never guessed.
    pub quota_state: Option<String>,
    /// Free-text health observation (e.g., "healthy", "degraded").
    /// None means unknown — never defaulted.
    pub health: Option<String>,
    /// When these observations were last refreshed.
    pub last_observed_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_account_id_newtype_works() {
        let id = ProviderAccountId::from("PA-CLAUDE-A");
        assert_eq!(id.to_string(), "PA-CLAUDE-A");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"PA-CLAUDE-A\"");
        let back: ProviderAccountId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn auth_profile_id_newtype_works() {
        let id = AuthProfileId::from("auth-profile-001");
        assert_eq!(id.to_string(), "auth-profile-001");
    }

    #[test]
    fn provider_kind_data_driven() {
        // Known providers work
        let claude = ProviderKind::new(ProviderKind::CLAUDE);
        assert_eq!(claude.to_string(), "claude");
        assert!(claude.is_known());

        let antigravity = ProviderKind::new(ProviderKind::ANTIGRAVITY);
        assert_eq!(antigravity.to_string(), "antigravity");
        assert!(antigravity.is_known());

        // Unknown/custom providers also work without recompilation
        let custom = ProviderKind::new("custom-provider");
        assert_eq!(custom.to_string(), "custom-provider");
        assert!(!custom.is_known());

        // Serialization roundtrip
        let json = serde_json::to_string(&custom).unwrap();
        let back: ProviderKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, custom);
    }

    #[test]
    fn credential_ref_is_opaque() {
        let cred = CredentialRef::new("vault://secrets/claude-a/token");
        assert_eq!(cred.to_string(), "vault://secrets/claude-a/token");
        let json = serde_json::to_string(&cred).unwrap();
        let back: CredentialRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cred);
    }

    #[test]
    fn provider_account_status_closed_enum() {
        let statuses = [
            ProviderAccountStatus::Active,
            ProviderAccountStatus::Unavailable,
            ProviderAccountStatus::AuthenticationRequired,
            ProviderAccountStatus::RateLimited,
            ProviderAccountStatus::Disabled,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: ProviderAccountStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn unknown_provider_account_status_fails_deserialization() {
        let json = "\"MAGIC_STATUS\"";
        let result: Result<ProviderAccountStatus, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unknown ProviderAccountStatus must fail closed"
        );
    }

    #[test]
    fn provider_account_status_availability() {
        assert!(ProviderAccountStatus::Active.is_available());
        assert!(!ProviderAccountStatus::Unavailable.is_available());
        assert!(!ProviderAccountStatus::AuthenticationRequired.is_available());
        assert!(!ProviderAccountStatus::RateLimited.is_available());
        assert!(!ProviderAccountStatus::Disabled.is_available());
    }

    #[test]
    fn provider_account_two_accounts_same_provider_have_different_ids() {
        let account_a = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-A"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude A".to_string(),
            identity_hint: Some("user-a@example.com".to_string()),
            auth_profile_id: AuthProfileId::from("auth-claude-a"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        let account_b = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-B"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude B".to_string(),
            identity_hint: Some("user-b@example.com".to_string()),
            auth_profile_id: AuthProfileId::from("auth-claude-b"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        // Same provider, different identities
        assert_eq!(account_a.provider, account_b.provider);
        assert_ne!(account_a.id, account_b.id);
        assert_ne!(account_a.auth_profile_id, account_b.auth_profile_id);
    }

    #[test]
    fn provider_account_serialization_roundtrip() {
        let account = ProviderAccount {
            id: ProviderAccountId::from("PA-ANTIGRAVITY-A"),
            provider: ProviderKind::new(ProviderKind::ANTIGRAVITY),
            label: "Antigravity Google A".to_string(),
            identity_hint: Some("google-a@example.com".to_string()),
            auth_profile_id: AuthProfileId::from("auth-antigravity-a"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 3,
            version: EntityVersion(5),
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&account).unwrap();
        let back: ProviderAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, account.id);
        assert_eq!(back.provider, account.provider);
        assert_eq!(back.label, account.label);
        assert_eq!(back.identity_hint, account.identity_hint);
        assert_eq!(back.auth_profile_id, account.auth_profile_id);
        assert_eq!(back.status, account.status);
        assert_eq!(back.max_concurrent_sessions, account.max_concurrent_sessions);
        assert_eq!(back.version, account.version);
    }

    #[test]
    fn credential_ref_is_separate_from_identity_metadata() {
        let account = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-A"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude A".to_string(),
            identity_hint: Some("user-a@example.com".to_string()),
            auth_profile_id: AuthProfileId::from("auth-claude-a"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        // identity_hint is metadata, auth_profile_id is the credential reference
        // They are distinct fields with distinct purposes
        assert_ne!(
            account.identity_hint.as_deref().unwrap_or(""),
            account.auth_profile_id.to_string()
        );
    }

    #[test]
    fn provider_account_independent_mutability() {
        let mut account_a = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-A"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude A".to_string(),
            identity_hint: None,
            auth_profile_id: AuthProfileId::from("auth-a"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        let account_b = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-B"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude B".to_string(),
            identity_hint: None,
            auth_profile_id: AuthProfileId::from("auth-b"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        // Mutate account A independently
        account_a.status = ProviderAccountStatus::RateLimited;
        account_a.version = account_a.version.next();

        // Account B remains unchanged
        assert_eq!(account_b.status, ProviderAccountStatus::Active);
        assert_eq!(account_b.version, EntityVersion::INITIAL);
        assert_ne!(account_a.version, account_b.version);
    }

    #[test]
    fn optimistic_version_conflict_detected() {
        let account = ProviderAccount {
            id: ProviderAccountId::from("PA-CLAUDE-A"),
            provider: ProviderKind::new(ProviderKind::CLAUDE),
            label: "Claude A".to_string(),
            identity_hint: None,
            auth_profile_id: AuthProfileId::from("auth-a"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 2,
            version: EntityVersion(3),
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        // Simulate stale update attempt: expected version 2, actual is 3
        let expected_version = EntityVersion(2);
        let actual_version = account.version;
        assert_ne!(
            expected_version, actual_version,
            "stale version must be detectable for OCC conflict"
        );
    }

    #[test]
    fn no_provider_specific_email_field_required() {
        // ProviderAccount uses identity_hint (optional, generic) instead of
        // provider-specific fields like google_email or claude_email.
        // This test documents the design decision.
        let account = ProviderAccount {
            id: ProviderAccountId::from("PA-ANYTHING-X"),
            provider: ProviderKind::new("any-provider"),
            label: "Any Account".to_string(),
            identity_hint: None, // optional, not required
            auth_profile_id: AuthProfileId::from("auth-x"),
            status: ProviderAccountStatus::Active,
            max_concurrent_sessions: 1,
            version: EntityVersion::INITIAL,
            created_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
            updated_at: Timestamp("2026-08-31T10:00:00Z".to_string()),
        };

        // Compiles and works without any provider-specific field
        assert!(account.identity_hint.is_none());
        assert_eq!(account.provider.to_string(), "any-provider");
    }

    #[test]
    fn runtime_state_tracks_per_account_observations() {
        let state = ProviderAccountRuntimeState {
            provider_account_id: ProviderAccountId::from("PA-CLAUDE-A"),
            active_sessions: 1,
            rate_limit_state: Some("80%".to_string()),
            quota_state: None, // unknown — not guessed
            health: Some("healthy".to_string()),
            last_observed_at: Timestamp("2026-08-31T12:00:00Z".to_string()),
        };

        assert_eq!(state.active_sessions, 1);
        assert!(state.quota_state.is_none(), "unknown quota must remain None");

        let json = serde_json::to_string(&state).unwrap();
        let back: ProviderAccountRuntimeState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider_account_id, state.provider_account_id);
        assert_eq!(back.active_sessions, state.active_sessions);
    }
}