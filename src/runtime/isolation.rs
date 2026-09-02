//! Mega Brain V0 — Runtime Isolation Manager (Gap: Multi-Account Isolation)
//!
//! Creates per-ProviderAccount isolation boundaries using environment variable
//! injection and dedicated directory trees. Each account gets its own HOME,
//! config, cache, and temp directories to prevent credential/config bleed
//! between concurrent sessions.
//!
//! Isolation matrix (from plan Section 17):
//! - HOME / USERPROFILE → per-account home dir
//! - XDG_CONFIG_HOME → per-account config dir
//! - XDG_DATA_HOME → per-account data dir
//! - XDG_CACHE_HOME → per-account cache dir
//! - APPDATA / LOCALAPPDATA → per-account (Windows)
//! - CLAUDE_CONFIG_DIR → per-account (Claude-specific)
//! - ANTHROPIC_TELEMETRY=false, GOOGLE_TELEMETRY=false

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// An isolation boundary for a single ProviderAccount.
/// All filesystem paths and environment variables are scoped to this boundary.
#[derive(Debug, Clone)]
pub struct IsolationBoundary {
    /// The ProviderAccount ID this boundary belongs to.
    pub account_id: String,
    /// Base data directory (e.g., ~/.iahub/accounts/)
    pub base_data_dir: PathBuf,
}

impl IsolationBoundary {
    /// Create a new isolation boundary for the given account.
    pub fn new(account_id: &str, base_data_dir: &Path) -> Self {
        Self {
            account_id: account_id.to_string(),
            base_data_dir: base_data_dir.to_path_buf(),
        }
    }

    /// Provision the physical directory tree for this isolation boundary.
    /// Creates home, config, cache, data, and tmp directories if they don't exist.
    pub fn provision(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.home_dir())?;
        std::fs::create_dir_all(self.config_dir())?;
        std::fs::create_dir_all(self.cache_dir())?;
        std::fs::create_dir_all(self.data_dir())?;
        std::fs::create_dir_all(self.tmp_dir())?;
        Ok(())
    }

    /// Per-account home directory — replaces HOME / USERPROFILE.
    pub fn home_dir(&self) -> PathBuf {
        self.base_data_dir.join(&self.account_id).join("home")
    }

    /// Per-account config directory — replaces XDG_CONFIG_HOME, APPDATA.
    pub fn config_dir(&self) -> PathBuf {
        self.base_data_dir.join(&self.account_id).join("config")
    }

    /// Per-account cache directory — replaces XDG_CACHE_HOME, LOCALAPPDATA.
    pub fn cache_dir(&self) -> PathBuf {
        self.base_data_dir.join(&self.account_id).join("cache")
    }

    /// Per-account data directory — replaces XDG_DATA_HOME.
    pub fn data_dir(&self) -> PathBuf {
        self.base_data_dir.join(&self.account_id).join("data")
    }

    /// Per-account temp directory — replaces TMPDIR / TEMP.
    pub fn tmp_dir(&self) -> PathBuf {
        self.base_data_dir.join(&self.account_id).join("tmp")
    }

    /// Generate the strict environment variable map that obliterates identity
    /// leakage between accounts. Every variable that could carry cross-account
    /// state is overridden to point at this boundary's directories.
    pub fn generate_env_map(&self) -> HashMap<String, String> {
        let mut envs = HashMap::new();
        let home_str = self.home_dir().to_string_lossy().to_string();
        let config_str = self.config_dir().to_string_lossy().to_string();
        let cache_str = self.cache_dir().to_string_lossy().to_string();
        let data_str = self.data_dir().to_string_lossy().to_string();
        let tmp_str = self.tmp_dir().to_string_lossy().to_string();

        // OS-level home isolation (Unix + Windows)
        envs.insert("HOME".to_string(), home_str.clone());
        envs.insert("USERPROFILE".to_string(), home_str.clone());

        // XDG specification redirects (Linux/macOS, harmless on Windows)
        envs.insert("XDG_CONFIG_HOME".to_string(), config_str.clone());
        envs.insert("XDG_DATA_HOME".to_string(), data_str.clone());
        envs.insert("XDG_CACHE_HOME".to_string(), cache_str.clone());

        // Windows AppData overrides
        envs.insert("APPDATA".to_string(), config_str.clone());
        envs.insert("LOCALAPPDATA".to_string(), cache_str.clone());

        // Temp directory isolation
        envs.insert("TMPDIR".to_string(), tmp_str.clone());
        envs.insert("TEMP".to_string(), tmp_str.clone());
        envs.insert("TMP".to_string(), tmp_str);

        // Agent-specific config isolation
        envs.insert("CLAUDE_CONFIG_DIR".to_string(), config_str.clone());

        // Telemetry suppression — prevent behavioral fingerprinting
        envs.insert("ANTHROPIC_TELEMETRY".to_string(), "false".to_string());
        envs.insert("GOOGLE_TELEMETRY".to_string(), "false".to_string());

        envs
    }

    /// Verify that this boundary's directories are properly isolated from
    /// another boundary. Returns true if no paths overlap.
    pub fn is_isolated_from(&self, other: &IsolationBoundary) -> bool {
        self.account_id != other.account_id
            && !self.home_dir().starts_with(other.home_dir())
            && !other.home_dir().starts_with(self.home_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provision_creates_directory_tree() {
        let dir = tempfile::tempdir().unwrap();
        let boundary = IsolationBoundary::new("test-account", dir.path());
        boundary.provision().unwrap();

        assert!(boundary.home_dir().exists());
        assert!(boundary.config_dir().exists());
        assert!(boundary.cache_dir().exists());
        assert!(boundary.data_dir().exists());
        assert!(boundary.tmp_dir().exists());
    }

    #[test]
    fn env_map_contains_all_required_variables() {
        let dir = tempfile::tempdir().unwrap();
        let boundary = IsolationBoundary::new("acc-1", dir.path());
        let envs = boundary.generate_env_map();

        // OS-level
        assert!(envs.contains_key("HOME"));
        assert!(envs.contains_key("USERPROFILE"));

        // XDG
        assert!(envs.contains_key("XDG_CONFIG_HOME"));
        assert!(envs.contains_key("XDG_DATA_HOME"));
        assert!(envs.contains_key("XDG_CACHE_HOME"));

        // Windows
        assert!(envs.contains_key("APPDATA"));
        assert!(envs.contains_key("LOCALAPPDATA"));

        // Temp
        assert!(envs.contains_key("TMPDIR"));
        assert!(envs.contains_key("TEMP"));
        assert!(envs.contains_key("TMP"));

        // Agent-specific
        assert!(envs.contains_key("CLAUDE_CONFIG_DIR"));

        // Telemetry suppression
        assert_eq!(envs.get("ANTHROPIC_TELEMETRY").unwrap(), "false");
        assert_eq!(envs.get("GOOGLE_TELEMETRY").unwrap(), "false");
    }

    #[test]
    fn env_map_points_to_account_specific_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let boundary = IsolationBoundary::new("my-account", dir.path());
        let envs = boundary.generate_env_map();

        let home = envs.get("HOME").unwrap();
        assert!(home.contains("my-account"));
        assert!(home.ends_with("home"));

        let config = envs.get("XDG_CONFIG_HOME").unwrap();
        assert!(config.contains("my-account"));
        assert!(config.ends_with("config"));
    }

    #[test]
    fn two_accounts_have_different_env_maps() {
        let dir = tempfile::tempdir().unwrap();
        let boundary_a = IsolationBoundary::new("account-a", dir.path());
        let boundary_b = IsolationBoundary::new("account-b", dir.path());

        let envs_a = boundary_a.generate_env_map();
        let envs_b = boundary_b.generate_env_map();

        assert_ne!(envs_a.get("HOME"), envs_b.get("HOME"));
        assert_ne!(envs_a.get("XDG_CONFIG_HOME"), envs_b.get("XDG_CONFIG_HOME"));
        assert_ne!(envs_a.get("CLAUDE_CONFIG_DIR"), envs_b.get("CLAUDE_CONFIG_DIR"));
    }

    #[test]
    fn isolation_check_detects_same_account() {
        let dir = tempfile::tempdir().unwrap();
        let b1 = IsolationBoundary::new("same", dir.path());
        let b2 = IsolationBoundary::new("same", dir.path());
        assert!(!b1.is_isolated_from(&b2));
    }

    #[test]
    fn isolation_check_confirms_different_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let b1 = IsolationBoundary::new("acc-1", dir.path());
        let b2 = IsolationBoundary::new("acc-2", dir.path());
        assert!(b1.is_isolated_from(&b2));
    }

    #[test]
    fn inv_057_each_account_gets_unique_home() {
        let dir = tempfile::tempdir().unwrap();
        let b1 = IsolationBoundary::new("claude-a", dir.path());
        let b2 = IsolationBoundary::new("claude-b", dir.path());
        let envs1 = b1.generate_env_map();
        let envs2 = b2.generate_env_map();
        assert_ne!(
            envs1["HOME"], envs2["HOME"],
            "INV-057: each ProviderAccount must get unique HOME"
        );
    }
}