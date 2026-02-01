//! Branch struct and filtering logic

use chrono::{DateTime, Utc};

/// Represents a git branch with metadata
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch name (e.g., "feature/old-api" or "origin/feature/old-api")
    pub name: String,
    /// Days since last commit
    pub age_days: i64,
    /// Whether the branch is merged into the default branch
    pub is_merged: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// SHA of the last commit
    pub last_commit_sha: String,
    /// Date of the last commit
    pub last_commit_date: DateTime<Utc>,
}

impl Branch {
    /// Check if this branch matches any protected pattern
    pub fn is_protected(&self, protected_branches: &[String]) -> bool {
        let name = self.short_name();
        protected_branches.iter().any(|p| p == name)
    }

    /// Check if this branch matches any exclude pattern (glob-style)
    /// Supports: "wip/*", "*/draft", "feature/*/temp", etc.
    pub fn matches_exclude_pattern(&self, patterns: &[String]) -> bool {
        let name = self.short_name();
        patterns
            .iter()
            .any(|pattern| Self::glob_match(pattern, name))
    }

    /// Simple glob matching: supports * as wildcard
    fn glob_match(pattern: &str, text: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            // No wildcard, exact match
            return pattern == text;
        }

        let mut remaining = text;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part must be at the start
                if !remaining.starts_with(part) {
                    return false;
                }
                remaining = &remaining[part.len()..];
            } else if i == parts.len() - 1 {
                // Last part must be at the end
                if !remaining.ends_with(part) {
                    return false;
                }
                remaining = "";
            } else {
                // Middle parts can be anywhere
                if let Some(pos) = remaining.find(part) {
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Get the short name (without origin/ prefix for remote branches)
    pub fn short_name(&self) -> &str {
        if self.is_remote {
            self.name.strip_prefix("origin/").unwrap_or(&self.name)
        } else {
            &self.name
        }
    }

    /// Format age in a human-readable way
    pub fn format_age(&self) -> String {
        if self.age_days == 1 {
            "1 day".to_string()
        } else {
            format!("{} days", self.age_days)
        }
    }
}

/// Filter options for listing branches
#[derive(Debug, Clone, Default)]
pub struct BranchFilter {
    /// Minimum age in days
    pub min_age_days: u32,
    /// Only show local branches
    pub local_only: bool,
    /// Only show remote branches
    pub remote_only: bool,
    /// Only show merged branches
    pub merged_only: bool,
    /// Protected branch names to exclude
    pub protected_branches: Vec<String>,
    /// Glob patterns to exclude (e.g., "wip/*", "*/draft")
    pub exclude_patterns: Vec<String>,
}

impl BranchFilter {
    /// Check if a branch passes this filter
    pub fn matches(&self, branch: &Branch) -> bool {
        // Check age
        if branch.age_days < self.min_age_days as i64 {
            return false;
        }

        // Check local/remote filter
        if self.local_only && branch.is_remote {
            return false;
        }
        if self.remote_only && !branch.is_remote {
            return false;
        }

        // Check merged filter
        if self.merged_only && !branch.is_merged {
            return false;
        }

        // Exclude protected branches
        if branch.is_protected(&self.protected_branches) {
            return false;
        }

        // Exclude branches matching exclude patterns
        if branch.matches_exclude_pattern(&self.exclude_patterns) {
            return false;
        }

        true
    }
}

/// Sort branches: unmerged first, then by age (newest first)
pub fn sort_branches(branches: &mut [Branch]) {
    branches.sort_by(|a, b| {
        // First: unmerged before merged
        match (a.is_merged, b.is_merged) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            // Then: newest first (lower age_days first)
            _ => a.age_days.cmp(&b.age_days),
        }
    });
}
