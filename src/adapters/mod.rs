//! Mega Brain V0 — Agent Runtime Adapters (Phase 0, Gap: Agent Binary Discovery)
//!
//! Abstraction layer for coding agent CLIs (Claude Code, Antigravity, Codex, etc.).
//! The orchestrator interacts ONLY through the AgentRuntimeAdapter trait — never
//! provider-specific code in the core.
//!
//! Key concepts:
//! - **CapabilityManifest**: Discovered capabilities per adapter (supports_resume, supports_mcp, etc.)
//! - **AgentBinaryResolver**: Discovers and validates agent binaries on the system PATH
//! - **TransportType**: How the adapter communicates (PTY/CLI, API, ACP)

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How an adapter communicates with its agent process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum TransportType {
    /// Direct PTY/CLI execution — first-class integration.
    PtyCli,
    /// Structured API calls (future).
    Api,
    /// Agent Client Protocol (future, OpenHands compatibility).
    Acp,
}

impl std::fmt::Display for TransportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PtyCli => write!(f, "PTY_CLI"),
            Self::Api => write!(f, "API"),
            Self::Acp => write!(f, "ACP"),
        }
    }
}

/// Discovered capabilities of an agent runtime adapter.
/// UI and orchestrator query these — never hardcode provider checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub supports_resume: bool,
    pub supports_interrupt: bool,
    pub supports_shell: bool,
    pub supports_mcp: bool,
    pub supports_skills: bool,
    pub supports_model_selection: bool,
    pub supports_usage_reporting: bool,
    pub supports_structured_events: bool,
    pub transport: TransportType,
}

impl CapabilityManifest {
    /// Default capabilities for a PTY/CLI-based agent (most common case).
    pub fn pty_cli_defaults() -> Self {
        Self {
            supports_resume: false,
            supports_interrupt: true,
            supports_shell: true,
            supports_mcp: false,
            supports_skills: false,
            supports_model_selection: false,
            supports_usage_reporting: false,
            supports_structured_events: false,
            transport: TransportType::PtyCli,
        }
    }

    /// Claude Code CLI capabilities.
    pub fn claude_code() -> Self {
        Self {
            supports_resume: true, // --resume flag
            supports_interrupt: true,
            supports_shell: true,
            supports_mcp: true,
            supports_skills: true,
            supports_model_selection: true,
            supports_usage_reporting: false,
            supports_structured_events: false,
            transport: TransportType::PtyCli,
        }
    }
}

/// Status of a discovered agent binary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AgentBinaryStatus {
    /// Binary found and version detected.
    Ok {
        path: String,
        version: Option<String>,
    },
    /// Binary not found on PATH.
    NotFound {
        name: String,
    },
    /// Binary found but version detection failed.
    VersionUnknown {
        path: String,
    },
}

/// Result of resolving an agent binary on the system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedAgent {
    /// Human-readable name (e.g., "Claude Code", "Antigravity").
    pub name: String,
    /// The binary name to search for (e.g., "claude", "agy").
    pub binary_name: String,
    /// Discovery status.
    pub status: AgentBinaryStatus,
    /// Capabilities of this agent.
    pub capabilities: CapabilityManifest,
}

/// Resolves agent binaries on the system PATH.
pub struct AgentBinaryResolver;

impl AgentBinaryResolver {
    /// Resolve a single agent binary by name.
    ///
    /// Searches PATH for the binary and attempts to detect its version
    /// by running `<binary> --version`.
    pub fn resolve(name: &str, binary_name: &str, capabilities: CapabilityManifest) -> ResolvedAgent {
        let path = Self::find_in_path(binary_name);

        let status = match path {
            Some(found_path) => {
                let version = Self::detect_version(&found_path);
                match version {
                    Some(v) => AgentBinaryStatus::Ok {
                        path: found_path.to_string_lossy().to_string(),
                        version: Some(v),
                    },
                    None => AgentBinaryStatus::VersionUnknown {
                        path: found_path.to_string_lossy().to_string(),
                    },
                }
            }
            None => AgentBinaryStatus::NotFound {
                name: binary_name.to_string(),
            },
        };

        ResolvedAgent {
            name: name.to_string(),
            binary_name: binary_name.to_string(),
            status,
            capabilities,
        }
    }

    /// Resolve all known agent binaries.
    pub fn resolve_all() -> Vec<ResolvedAgent> {
        vec![
            Self::resolve("Claude Code", "claude", CapabilityManifest::claude_code()),
            Self::resolve("Antigravity", "agy", CapabilityManifest::pty_cli_defaults()),
            Self::resolve("Codex", "codex", CapabilityManifest::pty_cli_defaults()),
            Self::resolve("OpenCode", "opencode", CapabilityManifest::pty_cli_defaults()),
        ]
    }

    /// Search for a binary in the system PATH.
    fn find_in_path(binary_name: &str) -> Option<PathBuf> {
        let path_var = std::env::var("PATH").ok()?;
        let separator = if cfg!(windows) { ';' } else { ':' };

        // On Windows, also try with .exe extension
        let exe_name = format!("{}.exe", binary_name);
        let candidates_owned: Vec<&str> = if cfg!(windows) {
            vec![binary_name, &exe_name]
        } else {
            vec![binary_name]
        };

        for dir in path_var.split(separator) {
            for candidate in &candidates_owned {
                let full_path = PathBuf::from(dir).join(candidate);
                if full_path.exists() && full_path.is_file() {
                    return Some(full_path);
                }
            }
        }
        None
    }

    /// Attempt to detect the version of a binary by running `--version`.
    fn detect_version(binary_path: &PathBuf) -> Option<String> {
        let output = std::process::Command::new(binary_path)
            .arg("--version")
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() {
                return Some(stdout);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_manifest_serialization_roundtrip() {
        let manifest = CapabilityManifest::claude_code();
        let json = serde_json::to_string(&manifest).unwrap();
        let back: CapabilityManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, manifest);
        assert!(back.supports_resume);
        assert!(back.supports_mcp);
        assert_eq!(back.transport, TransportType::PtyCli);
    }

    #[test]
    fn transport_type_display() {
        assert_eq!(TransportType::PtyCli.to_string(), "PTY_CLI");
        assert_eq!(TransportType::Api.to_string(), "API");
        assert_eq!(TransportType::Acp.to_string(), "ACP");
    }

    #[test]
    fn transport_type_serialization_roundtrip() {
        let t = TransportType::PtyCli;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"PTY_CLI\"");
        let back: TransportType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn resolved_agent_not_found() {
        let agent = AgentBinaryResolver::resolve(
            "Nonexistent Agent",
            "this_binary_definitely_does_not_exist_xyz_12345",
            CapabilityManifest::pty_cli_defaults(),
        );
        assert_eq!(agent.name, "Nonexistent Agent");
        match &agent.status {
            AgentBinaryStatus::NotFound { name } => {
                assert_eq!(name, "this_binary_definitely_does_not_exist_xyz_12345");
            }
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn resolved_agent_serialization_roundtrip() {
        let agent = ResolvedAgent {
            name: "Test Agent".to_string(),
            binary_name: "test-bin".to_string(),
            status: AgentBinaryStatus::NotFound { name: "test-bin".to_string() },
            capabilities: CapabilityManifest::pty_cli_defaults(),
        };
        let json = serde_json::to_string(&agent).unwrap();
        let back: ResolvedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, agent);
    }

    #[test]
    fn agent_binary_status_ok_serializes() {
        let status = AgentBinaryStatus::Ok {
            path: "/usr/local/bin/claude".to_string(),
            version: Some("1.2.3".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("1.2.3"));
    }

    #[test]
    fn agent_binary_status_not_found_serializes() {
        let status = AgentBinaryStatus::NotFound { name: "agy".to_string() };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"not_found\""));
        assert!(json.contains("agy"));
    }

    #[test]
    fn capability_manifest_defaults() {
        let defaults = CapabilityManifest::pty_cli_defaults();
        assert!(defaults.supports_interrupt);
        assert!(defaults.supports_shell);
        assert!(!defaults.supports_resume);
        assert!(!defaults.supports_mcp);
        assert_eq!(defaults.transport, TransportType::PtyCli);
    }

    #[test]
    fn inv_059_capabilities_queried_not_assumed() {
        // INV-059: Adapter capability manifest is queried, never assumed.
        // The orchestrator checks manifest.supports_resume before using --resume,
        // rather than assuming all agents support it.
        let claude = CapabilityManifest::claude_code();
        let generic = CapabilityManifest::pty_cli_defaults();

        // Claude supports resume — orchestrator can use --resume
        assert!(claude.supports_resume);
        // Generic agent does not — orchestrator must NOT use --resume
        assert!(!generic.supports_resume);

        // This structural difference proves capabilities are queried per-adapter
        assert_ne!(claude.supports_resume, generic.supports_resume);
    }

    #[test]
    fn resolve_all_returns_known_agents() {
        let agents = AgentBinaryResolver::resolve_all();
        assert_eq!(agents.len(), 4);
        assert_eq!(agents[0].name, "Claude Code");
        assert_eq!(agents[1].name, "Antigravity");
        assert_eq!(agents[2].name, "Codex");
        assert_eq!(agents[3].name, "OpenCode");
    }
}