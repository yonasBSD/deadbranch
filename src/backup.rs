//! Backup management - list, restore, and clean backups

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

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
