//! Backup management - list, restore, and clean backups

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;

/// Information about a backup file
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Path to the backup file
    pub path: PathBuf,
    /// Repository name
    pub repo_name: String,
    /// Timestamp when backup was created
    pub timestamp: DateTime<Utc>,
    /// Number of branches in the backup
    pub branch_count: usize,
}

impl BackupInfo {
    /// Parse a backup file and extract its info
    fn from_path(path: PathBuf, repo_name: &str) -> Result<Self> {
        let file = fs::File::open(&path)
            .with_context(|| format!("Failed to open backup file: {}", path.display()))?;
        let reader = std::io::BufReader::new(file);

        let mut timestamp: Option<DateTime<Utc>> = None;
        let mut branch_count = 0;

        for line in reader.lines() {
            let line = line?;

            // Parse header for timestamp
            if line.starts_with("# Created:") {
                if let Some(date_str) = line.strip_prefix("# Created:") {
                    let date_str = date_str.trim();
                    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
                        timestamp = Some(dt.with_timezone(&Utc));
                    }
                }
            }

            // Count branch entries (lines starting with "git branch")
            if line.starts_with("git branch") {
                branch_count += 1;
            }
        }

        // If no timestamp in file, try to parse from filename
        let timestamp = timestamp
            .unwrap_or_else(|| parse_timestamp_from_filename(&path).unwrap_or_else(Utc::now));

        Ok(BackupInfo {
            path,
            repo_name: repo_name.to_string(),
            timestamp,
            branch_count,
        })
    }

    /// Format the age of the backup as a human-readable string
    pub fn format_age(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.timestamp);

        let days = duration.num_days();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes();

        if days > 0 {
            format!("{} {} ago", days, if days == 1 { "day" } else { "days" })
        } else if hours > 0 {
            format!(
                "{} {} ago",
                hours,
                if hours == 1 { "hour" } else { "hours" }
            )
        } else if minutes > 0 {
            format!(
                "{} {} ago",
                minutes,
                if minutes == 1 { "minute" } else { "minutes" }
            )
        } else {
            "just now".to_string()
        }
    }

    /// Get just the filename without the full path
    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

/// Parse timestamp from backup filename (backup-YYYYMMDD-HHMMSS.txt)
fn parse_timestamp_from_filename(path: &PathBuf) -> Option<DateTime<Utc>> {
    let filename = path.file_stem()?.to_str()?;
    let timestamp_part = filename.strip_prefix("backup-")?;

    // Parse YYYYMMDD-HHMMSS format
    let parts: Vec<&str> = timestamp_part.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let date_str = parts[0]; // YYYYMMDD
    let time_str = parts[1]; // HHMMSS

    if date_str.len() != 8 || time_str.len() != 6 {
        return None;
    }

    let year: i32 = date_str[0..4].parse().ok()?;
    let month: u32 = date_str[4..6].parse().ok()?;
    let day: u32 = date_str[6..8].parse().ok()?;
    let hour: u32 = time_str[0..2].parse().ok()?;
    let min: u32 = time_str[2..4].parse().ok()?;
    let sec: u32 = time_str[4..6].parse().ok()?;

    chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|date| date.and_hms_opt(hour, min, sec))
        .map(|naive| DateTime::from_naive_utc_and_offset(naive, Utc))
}

/// List all backups grouped by repository
pub fn list_all_backups() -> Result<HashMap<String, Vec<BackupInfo>>> {
    let backups_dir = Config::backups_dir()?;

    let mut result: HashMap<String, Vec<BackupInfo>> = HashMap::new();

    if !backups_dir.exists() {
        return Ok(result);
    }

    // Each subdirectory is a repository
    let entries = fs::read_dir(&backups_dir).with_context(|| {
        format!(
            "Failed to read backups directory: {}",
            backups_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let repo_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let backups = list_repo_backups(&repo_name)?;
        if !backups.is_empty() {
            result.insert(repo_name, backups);
        }
    }

    Ok(result)
}

/// List backups for a specific repository
pub fn list_repo_backups(repo_name: &str) -> Result<Vec<BackupInfo>> {
    let repo_backup_dir = Config::repo_backup_dir(repo_name)?;

    let mut backups = Vec::new();

    if !repo_backup_dir.exists() {
        return Ok(backups);
    }

    let entries = fs::read_dir(&repo_backup_dir).with_context(|| {
        format!(
            "Failed to read backup directory: {}",
            repo_backup_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Only process .txt files that start with "backup-"
        if !path.is_file() {
            continue;
        }

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !filename.starts_with("backup-") || !filename.ends_with(".txt") {
            continue;
        }

        match BackupInfo::from_path(path, repo_name) {
            Ok(info) => backups.push(info),
            Err(e) => {
                // Log warning but continue with other files
                eprintln!("Warning: Could not parse backup file: {}", e);
            }
        }
    }

    // Sort by timestamp, newest first
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(backups)
}

/// Information about a branch entry in a backup file
#[derive(Debug, Clone)]
pub struct BackupBranchEntry {
    /// The branch name (as it would be restored)
    pub name: String,
    /// The commit SHA the branch pointed to
    pub commit_sha: String,
}

/// Information about a skipped/corrupted line in a backup file
#[derive(Debug, Clone)]
pub struct SkippedLine {
    /// Line number (1-based)
    pub line_number: usize,
    /// The content of the line
    pub content: String,
}

/// Result of parsing a backup file
#[derive(Debug)]
pub struct ParsedBackup {
    /// Successfully parsed branch entries
    pub entries: Vec<BackupBranchEntry>,
    /// Lines that were skipped due to corruption/malformation
    pub skipped_lines: Vec<SkippedLine>,
}

/// Result of a successful restore operation
#[derive(Debug)]
pub struct RestoreResult {
    /// The original branch name from the backup
    pub original_name: String,
    /// The name it was restored as (may differ if --as was used)
    pub restored_name: String,
    /// The commit SHA the branch now points to
    pub commit_sha: String,
    /// Whether an existing branch was overwritten
    pub overwrote_existing: bool,
}

/// Error type for restore failures
#[derive(Debug)]
pub enum RestoreError {
    /// Branch already exists and --force was not specified
    BranchExists { branch_name: String },
    /// The commit SHA no longer exists (garbage collected)
    CommitNotFound {
        branch_name: String,
        commit_sha: String,
    },
    /// Branch not found in the backup file
    BranchNotInBackup {
        branch_name: String,
        available_branches: Vec<BackupBranchEntry>,
        skipped_lines: Vec<SkippedLine>,
    },
    /// No backups exist for the repository
    NoBackupsFound { repo_name: String },
    /// Backup file is corrupted or invalid
    BackupCorrupted { message: String },
    /// Other git or IO errors
    Other(anyhow::Error),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreError::BranchExists { branch_name } => {
                write!(f, "Branch '{}' already exists", branch_name)
            }
            RestoreError::CommitNotFound {
                branch_name,
                commit_sha,
            } => {
                write!(
                    f,
                    "Cannot restore '{}': commit {} no longer exists",
                    branch_name, commit_sha
                )
            }
            RestoreError::BranchNotInBackup { branch_name, .. } => {
                write!(f, "Branch '{}' not found in backup", branch_name)
            }
            RestoreError::NoBackupsFound { repo_name } => {
                write!(f, "No backups found for repository '{}'", repo_name)
            }
            RestoreError::BackupCorrupted { message } => {
                write!(f, "Backup file is corrupted: {}", message)
            }
            RestoreError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RestoreError {}

/// Parse a backup file and extract branch entries
///
/// The backup format has lines like:
/// ```
/// # feature/old-api
/// git branch feature/old-api a1b2c3d4...
/// ```
///
/// Lines that don't match the expected format (but aren't comments/empty) are
/// tracked as skipped lines rather than causing a parse failure.
pub fn parse_backup_file(path: &Path) -> Result<ParsedBackup, RestoreError> {
    let file = fs::File::open(path).map_err(|e| RestoreError::Other(e.into()))?;
    let reader = std::io::BufReader::new(file);

    let mut entries = Vec::new();
    let mut skipped_lines = Vec::new();
    let mut found_header = false;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| RestoreError::Other(e.into()))?;

        // Check for valid header on first non-empty line
        if line_num == 0 {
            if !line.starts_with("# deadbranch backup") {
                return Err(RestoreError::BackupCorrupted {
                    message: format!(
                        "Invalid header at line 1. Expected '# deadbranch backup', found: '{}'",
                        line
                    ),
                });
            }
            found_header = true;
            continue;
        }

        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        // Parse "git branch <name> <sha>" lines
        if line.starts_with("git branch ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                // parts[0] = "git", parts[1] = "branch", parts[2] = name, parts[3] = sha
                entries.push(BackupBranchEntry {
                    name: parts[2].to_string(),
                    commit_sha: parts[3].to_string(),
                });
            } else {
                // Malformed "git branch" line - track as skipped
                skipped_lines.push(SkippedLine {
                    line_number: line_num + 1,
                    content: line,
                });
            }
        } else {
            // Line doesn't match expected format - track as skipped
            skipped_lines.push(SkippedLine {
                line_number: line_num + 1,
                content: line,
            });
        }
    }

    if !found_header {
        return Err(RestoreError::BackupCorrupted {
            message: "Empty or invalid backup file".to_string(),
        });
    }

    Ok(ParsedBackup {
        entries,
        skipped_lines,
    })
}

/// Restore a branch from a backup
///
/// # Arguments
/// * `branch_name` - The name of the branch to restore
/// * `backup_file` - Optional path to a specific backup file. If None, uses most recent backup.
/// * `target_name` - Optional alternate name for the restored branch (--as flag)
/// * `force` - Whether to overwrite an existing branch
///
/// # Returns
/// * `Ok(RestoreResult)` on success
/// * `Err(RestoreError)` on failure with detailed error information
pub fn restore_branch(
    branch_name: &str,
    backup_file: Option<&str>,
    target_name: Option<&str>,
    force: bool,
) -> Result<RestoreResult, RestoreError> {
    let repo_name = Config::get_repo_name();

    // Determine the final branch name
    let final_branch_name = target_name.unwrap_or(branch_name);

    // Check if branch already exists
    let branch_exists = check_branch_exists(final_branch_name);

    if branch_exists && !force {
        return Err(RestoreError::BranchExists {
            branch_name: final_branch_name.to_string(),
        });
    }

    // Determine which backup file to use
    let backup_path = if let Some(filename) = backup_file {
        // If it's just a filename, look in the repo's backup directory
        let path = PathBuf::from(filename);
        if path.is_absolute() || path.exists() {
            path
        } else {
            // Look in the repo's backup directory
            let backup_dir = Config::repo_backup_dir(&repo_name).map_err(RestoreError::Other)?;
            backup_dir.join(filename)
        }
    } else {
        // Use most recent backup
        let backups = list_repo_backups(&repo_name).map_err(RestoreError::Other)?;

        backups
            .into_iter()
            .next()
            .map(|info| info.path)
            .ok_or_else(|| RestoreError::NoBackupsFound {
                repo_name: repo_name.clone(),
            })?
    };

    // Parse the backup file
    let parsed = parse_backup_file(&backup_path)?;

    // Find the branch in the backup
    let entry = parsed
        .entries
        .iter()
        .find(|e| e.name == branch_name)
        .ok_or_else(|| RestoreError::BranchNotInBackup {
            branch_name: branch_name.to_string(),
            available_branches: parsed.entries.clone(),
            skipped_lines: parsed.skipped_lines.clone(),
        })?;

    // Check if the commit exists
    if !commit_exists(&entry.commit_sha) {
        return Err(RestoreError::CommitNotFound {
            branch_name: branch_name.to_string(),
            commit_sha: entry.commit_sha.clone(),
        });
    }

    // Create or update the branch
    create_branch(final_branch_name, &entry.commit_sha, force).map_err(RestoreError::Other)?;

    Ok(RestoreResult {
        original_name: branch_name.to_string(),
        restored_name: final_branch_name.to_string(),
        commit_sha: entry.commit_sha.clone(),
        overwrote_existing: branch_exists && force,
    })
}

/// Check if a local branch exists
fn check_branch_exists(branch_name: &str) -> bool {
    Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            &format!("refs/heads/{}", branch_name),
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if a commit exists in the repository
fn commit_exists(sha: &str) -> bool {
    Command::new("git")
        .args(["cat-file", "-t", sha])
        .output()
        .map(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "commit"
        })
        .unwrap_or(false)
}

/// Create a branch at a specific commit
fn create_branch(branch_name: &str, commit_sha: &str, force: bool) -> Result<()> {
    let mut args = vec!["branch"];
    if force {
        args.push("-f");
    }
    args.push(branch_name);
    args.push(commit_sha);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git branch command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to create branch '{}': {}",
            branch_name,
            stderr.trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_backup(dir: &std::path::Path, filename: &str, content: &str) -> PathBuf {
        let path = dir.join(filename);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_timestamp_from_filename() {
        let path = PathBuf::from("/some/path/backup-20260201-143022.txt");
        let ts = parse_timestamp_from_filename(&path).unwrap();

        assert_eq!(
            ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-02-01 14:30:22"
        );
    }

    #[test]
    fn test_parse_timestamp_invalid_filename() {
        let path = PathBuf::from("/some/path/not-a-backup.txt");
        assert!(parse_timestamp_from_filename(&path).is_none());

        let path = PathBuf::from("/some/path/backup-invalid.txt");
        assert!(parse_timestamp_from_filename(&path).is_none());
    }

    #[test]
    fn test_backup_info_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"# deadbranch backup
# Created: 2026-02-01T14:30:22Z
# Repository: test-repo

# feature/old-api
git branch feature/old-api a1b2c3d4

# bugfix/login
git branch bugfix/login e5f6g7h8
"#;
        let path = create_test_backup(temp_dir.path(), "backup-20260201-143022.txt", content);

        let info = BackupInfo::from_path(path, "test-repo").unwrap();

        assert_eq!(info.repo_name, "test-repo");
        assert_eq!(info.branch_count, 2);
        assert_eq!(info.timestamp.format("%Y-%m-%d").to_string(), "2026-02-01");
    }

    #[test]
    fn test_backup_info_format_age() {
        let info = BackupInfo {
            path: PathBuf::from("/test"),
            repo_name: "test".to_string(),
            timestamp: Utc::now() - chrono::Duration::hours(2),
            branch_count: 5,
        };

        let age = info.format_age();
        assert!(age.contains("hour"));
    }

    #[test]
    fn test_backup_info_filename() {
        let info = BackupInfo {
            path: PathBuf::from("/some/long/path/backup-20260201-143022.txt"),
            repo_name: "test".to_string(),
            timestamp: Utc::now(),
            branch_count: 5,
        };

        assert_eq!(info.filename(), "backup-20260201-143022.txt");
    }
}
