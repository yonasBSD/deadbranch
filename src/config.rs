//! Configuration handling for deadbranch

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Default number of days before a branch is considered stale
const DEFAULT_DAYS: u32 = 30;

/// Default protected branches
const DEFAULT_PROTECTED: &[&str] = &["main", "master", "develop", "staging", "production"];

/// Configuration for deadbranch
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_days")]
    pub default_days: u32,

    #[serde(default = "default_protected_branches")]
    pub protected_branches: Vec<String>,

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

impl Default for Config {
    fn default() -> Self {
        Self {
            default_days: default_days(),
            protected_branches: default_protected_branches(),
            default_branch: None,
        }
    }
}

impl Config {
    /// Get the path to the config file
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("deadbranch");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from file, or return defaults if file doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        } else {
            Ok(Config::default())
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
            _ => {
                anyhow::bail!("Unknown config key: {}. Valid keys: default-days, protected-branches, default-branch", key);
            }
        }
        Ok(())
    }
}
