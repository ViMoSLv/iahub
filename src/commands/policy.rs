//! Mega Brain V0 — Command Authority Policy
//!
//! Typed declaration of what authority a command requires before its handler
//! may execute. The Command Engine checks this policy BEFORE invoking the
//! handler, ensuring no protected mutation can bypass authority validation.
//!
//! This replaces scattered `if command_type == "..."` checks with a single
//! typed abstraction that each command declares explicitly.

use crate::authority::model::ResourceId;
use crate::domain::{AttemptId, FencingToken, LeaseId};

/// What authority a command requires before execution.
/// Each command type declares its requirement; the engine enforces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityRequirement {
    /// No authority check needed (e.g., CreateProject, system commands).
    None,

    /// Requires optimistic concurrency version check only.
    EntityVersion,

    /// Requires valid lease authority over a specific resource.
    /// The engine validates attempt_id + lease_id + fencing_token against
    /// the LeaseService before executing the handler.
    Lease {
        /// How to resolve the resource from the command payload.
        /// For V0, this is a static resource type; the resource_id comes
        /// from the envelope's attempt_id or payload.
        resource_type: &'static str,
    },
}

/// Resolved authority context extracted from the envelope for validation.
#[derive(Debug, Clone)]
pub struct ResolvedAuthority {
    pub attempt_id: AttemptId,
    pub lease_id: LeaseId,
    pub fencing_token: FencingToken,
    pub resource: ResourceId,
}

impl AuthorityRequirement {
    /// Extract and validate authority fields from a command envelope.
    /// Returns `ResolvedAuthority` if all required fields are present,
    /// or `None` if the requirement is not met (missing fields).
    pub fn resolve<C>(
        &self,
        envelope: &super::types::CommandEnvelope<C>,
    ) -> Option<ResolvedAuthority> {
        match self {
            AuthorityRequirement::None | AuthorityRequirement::EntityVersion => None,
            AuthorityRequirement::Lease { resource_type } => {
                let attempt_id = envelope.attempt_id.clone()?;
                let lease_id = envelope.lease_id.clone()?;
                let fencing_token = envelope.fencing_token?;

                // Resource ID is derived from the attempt_id for task-scoped authority.
                // In future topics, this could be resolved from the payload.
                let resource = ResourceId::new(*resource_type, attempt_id.0.clone());

                Some(ResolvedAuthority {
                    attempt_id,
                    lease_id,
                    fencing_token,
                    resource,
                })
            }
        }
    }

    /// Returns true if this requirement mandates lease authority validation.
    pub fn requires_lease(&self) -> bool {
        matches!(self, AuthorityRequirement::Lease { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::types::{Actor, ActorType, CommandEnvelope, CorrelationId};
    use crate::domain::{AttemptId, CommandId, FencingToken, LeaseId, Timestamp};

    fn make_envelope_with_authority(
        attempt: Option<&str>,
        lease: Option<&str>,
        token: Option<i64>,
    ) -> CommandEnvelope<()> {
        CommandEnvelope {
            command_id: CommandId::from("cmd-test"),
            actor: Actor {
                r#type: ActorType::System,
                id: "test".into(),
            },
            correlation_id: CorrelationId::from("corr"),
            causation_id: None,
            expected_version: None,
            attempt_id: attempt.map(|s| AttemptId(s.into())),
            lease_id: lease.map(|s| LeaseId(s.into())),
            fencing_token: token.map(FencingToken),
            issued_at: Timestamp("1000".into()),
            payload: (),
        }
    }

    #[test]
    fn none_requirement_resolves_to_none() {
        let req = AuthorityRequirement::None;
        let env = make_envelope_with_authority(Some("att-1"), Some("l-1"), Some(1));
        assert!(req.resolve(&env).is_none());
    }

    #[test]
    fn lease_requirement_resolves_all_fields() {
        let req = AuthorityRequirement::Lease {
            resource_type: "task",
        };
        let env = make_envelope_with_authority(Some("att-1"), Some("l-1"), Some(42));
        let resolved = req.resolve(&env).expect("must resolve");
        assert_eq!(resolved.attempt_id.0, "att-1");
        assert_eq!(resolved.lease_id.0, "l-1");
        assert_eq!(resolved.fencing_token, FencingToken(42));
        assert_eq!(resolved.resource.resource_type, "task");
        assert_eq!(resolved.resource.resource_id, "att-1");
    }

    #[test]
    fn lease_requirement_fails_on_missing_attempt() {
        let req = AuthorityRequirement::Lease {
            resource_type: "task",
        };
        let env = make_envelope_with_authority(None, Some("l-1"), Some(42));
        assert!(req.resolve(&env).is_none());
    }

    #[test]
    fn lease_requirement_fails_on_missing_lease() {
        let req = AuthorityRequirement::Lease {
            resource_type: "task",
        };
        let env = make_envelope_with_authority(Some("att-1"), None, Some(42));
        assert!(req.resolve(&env).is_none());
    }

    #[test]
    fn lease_requirement_fails_on_missing_token() {
        let req = AuthorityRequirement::Lease {
            resource_type: "task",
        };
        let env = make_envelope_with_authority(Some("att-1"), Some("l-1"), None);
        assert!(req.resolve(&env).is_none());
    }

    #[test]
    fn requires_lease_returns_true_for_lease_variant() {
        assert!(AuthorityRequirement::Lease {
            resource_type: "task"
        }
        .requires_lease());
        assert!(!AuthorityRequirement::None.requires_lease());
        assert!(!AuthorityRequirement::EntityVersion.requires_lease());
    }
}
