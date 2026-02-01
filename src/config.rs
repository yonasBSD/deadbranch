//! Configuration handling for deadbranch

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default number of days before a branch is considered stale
const DEFAULT_DAYS: u32 = 30;

/// Default protected branches
const DEFAULT_PROTECTED: &[&str] = &["main", "master", "develop", "staging", "production"];

/// Default exclude patterns (WIP/draft branches)
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &["wip/*", "draft/*", "*/wip", "*/draft"];

/// Configuration for deadbranch
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_days")]
    pub default_days: u32,

    #[serde(default = "default_protected_branches")]
    pub protected_branches: Vec<String>,

    /// Branch name patterns to exclude (glob-style: wip/*, */draft, etc.)
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,

    /// The default branch to check merges against (auto-detected if not set)
    #[serde(default)]
    pub default_branch: Option<String>,
}

fn default_days() -> u32 {
    DEFAULT_DAYS
}

fn default_protected_branches() -> Vec<String> {
    DEFAULT_PROTECTED.iter().map(|s| s.to_string()).collect()
}

fn default_exclude_patterns() -> Vec<String> {
    DEFAULT_EXCLUDE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_days: default_days(),
            protected_branches: default_protected_branches(),
            exclude_patterns: default_exclude_patterns(),
            default_branch: None,
        }
    }
}

impl Config {
    /// Get the main deadbranch directory (~/.deadbranch)
    pub fn deadbranch_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".deadbranch"))
    }

    /// Get the path to the config file (~/.deadbranch/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::deadbranch_dir()?.join("config.toml"))
    }

    /// Get the backups directory (~/.deadbranch/backups)
    pub fn backups_dir() -> Result<PathBuf> {
        Ok(Self::deadbranch_dir()?.join("backups"))
    }

    /// Get the backup directory for a specific repository
    pub fn repo_backup_dir(repo_name: &str) -> Result<PathBuf> {
        Ok(Self::backups_dir()?.join(repo_name))
    }

    /// Get the current repository name (uses directory name)
    pub fn get_repo_name() -> String {
        std::env::current_dir()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown-repo".to_string())
    }

    /// Load config from file, or create default config if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        } else {
            // Auto-create config file with defaults on first use
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Set a configuration value by key
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "default-days" | "days" => {
                self.default_days = value
                    .parse()
                    .with_context(|| format!("Invalid number: {}", value))?;
            }
            "protected-branches" => {
                self.protected_branches = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "default-branch" => {
                self.default_branch = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "exclude-patterns" => {
                self.exclude_patterns = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {
                anyhow::bail!("Unknown config key: {}. Valid keys: default-days, protected-branches, default-branch, exclude-patterns", key);
            }
        }
        Ok(())
    }
}
