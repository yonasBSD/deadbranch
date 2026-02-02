//! Integration tests for deadbranch backup commands
//!
//! These tests create real backups in ~/.deadbranch/backups/ and clean them up after.
//! Each test uses a unique temp directory name, so tests don't interfere with each other
//! or with real user backups.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Helper to create a test git repository with a commit
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Initialize git repo with explicit main branch
    StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Set git config (required for commits)
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create initial commit on main branch
    fs::write(temp_dir.path().join("README.md"), "# Test repo").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    temp_dir
}

/// Helper to create a branch with a commit
fn create_branch(repo_dir: &std::path::Path, branch_name: &str) {
    // Create and checkout branch
    StdCommand::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Make a commit
    fs::write(
        repo_dir.join("test.txt"),
        format!("Content for {}", branch_name),
    )
    .unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", &format!("Add {} content", branch_name)])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Go back to main
    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();
}

/// Helper to make a branch old by modifying git commit date
fn make_branch_old(repo_dir: &std::path::Path, branch_name: &str, days_old: u32) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (days_old as u64 * 86400);

    // Checkout the branch
    StdCommand::new("git")
        .args(["checkout", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Amend the commit with an old date
    let date = format!("@{}", old_timestamp);
    StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Go back to main
    StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();
}

/// Helper to merge a branch into main
fn merge_branch(repo_dir: &std::path::Path, branch_name: &str) {
    StdCommand::new("git")
        .args([
            "merge",
            branch_name,
            "--no-ff",
            "-m",
            &format!("Merge {}", branch_name),
        ])
        .current_dir(repo_dir)
        .output()
        .unwrap();
}

/// Get the repo name from the temp directory path (same logic as deadbranch uses)
fn get_repo_name(repo_path: &std::path::Path) -> String {
    repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Get the backup directory for a repo
fn get_backup_dir(repo_name: &str) -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home)
        .join(".deadbranch")
        .join("backups")
        .join(repo_name)
}

/// Clean up backups for a test repo
fn cleanup_backups(repo_name: &str) {
    let backup_dir = get_backup_dir(repo_name);
    if backup_dir.exists() {
        let _ = fs::remove_dir_all(&backup_dir);
    }
}

/// RAII guard to ensure backup cleanup even if test panics
struct BackupCleanupGuard {
    repo_name: String,
}

impl BackupCleanupGuard {
    fn new(repo_name: String) -> Self {
        // Clean up any existing backups first
        cleanup_backups(&repo_name);
        Self { repo_name }
    }
}

impl Drop for BackupCleanupGuard {
    fn drop(&mut self) {
        cleanup_backups(&self.repo_name);
    }
}

// ============================================================================
// Tests for `deadbranch backup list` (summary view)
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_backup_list_no_backups() {
    // Create a repo that has never had any branches cleaned
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name);

    // Running backup list should work but show no backups for this repo
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list"])
        .current_dir(&repo)
        .assert()
        .success();
}

#[test]
#[allow(deprecated)]
fn test_backup_list_shows_repository_after_clean() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create an old merged branch
    create_branch(repo.path(), "old-feature");
    make_branch_old(repo.path(), "old-feature", 45);
    merge_branch(repo.path(), "old-feature");

    // Clean it (creates a backup)
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Backup list should show this repository
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains(&repo_name));
}

// ============================================================================
// Tests for `deadbranch backup list --current`
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_backup_list_current_requires_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    // Running --current outside a git repo should fail
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current"])
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .code(1);
}

#[test]
#[allow(deprecated)]
fn test_backup_list_current_no_backups() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name);

    // No backups yet - should show appropriate message
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups found"));
}

#[test]
#[allow(deprecated)]
fn test_backup_list_current_shows_backups() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean an old merged branch
    create_branch(repo.path(), "feature-to-backup");
    make_branch_old(repo.path(), "feature-to-backup", 45);
    merge_branch(repo.path(), "feature-to-backup");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Backup list --current should show the backup
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("backup-"))
        .stdout(predicate::str::contains(".txt"));
}

#[test]
#[allow(deprecated)]
fn test_backup_list_current_shows_branch_count() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create multiple old merged branches
    for i in 1..=3 {
        let branch_name = format!("feature-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);
    }

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Should show branch count in the backup
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("3")); // 3 branches backed up
}

// ============================================================================
// Tests for `deadbranch backup list --repo <name>`
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_backup_list_repo_flag() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean a branch
    create_branch(repo.path(), "repo-flag-test");
    make_branch_old(repo.path(), "repo-flag-test", 45);
    merge_branch(repo.path(), "repo-flag-test");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Can query by repo name from anywhere (even outside the repo)
    let other_dir = TempDir::new().unwrap();
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--repo", &repo_name])
        .current_dir(&other_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("backup-"));
}

#[test]
#[allow(deprecated)]
fn test_backup_list_repo_not_found() {
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--repo", "nonexistent-repo-xyz123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups found"));
}

// ============================================================================
// Tests for flag validation
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_backup_list_mutual_exclusion() {
    let repo = create_test_repo();

    // --current and --repo should be mutually exclusive
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current", "--repo", "some-repo"])
        .current_dir(&repo)
        .assert()
        .failure();
}

// ============================================================================
// Tests for backup creation during clean
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_clean_creates_backup() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create an old merged branch
    create_branch(repo.path(), "branch-to-delete");
    make_branch_old(repo.path(), "branch-to-delete", 45);
    merge_branch(repo.path(), "branch-to-delete");

    // Verify no backups exist before
    let backup_dir = get_backup_dir(&repo_name);
    assert!(!backup_dir.exists() || fs::read_dir(&backup_dir).unwrap().count() == 0);

    // Run clean
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Verify backup was created
    assert!(backup_dir.exists());
    let backup_files: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(backup_files.len(), 1);
}

#[test]
#[allow(deprecated)]
fn test_backup_contains_branch_restore_command() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean a branch
    create_branch(repo.path(), "restorable-branch");
    make_branch_old(repo.path(), "restorable-branch", 45);
    merge_branch(repo.path(), "restorable-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Read the backup file and verify it contains the branch restore command
    let backup_dir = get_backup_dir(&repo_name);
    let backup_file = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .next()
        .unwrap()
        .path();

    let content = fs::read_to_string(&backup_file).unwrap();
    assert!(content.contains("git branch restorable-branch"));
    assert!(content.contains("# restorable-branch"));
}

#[test]
#[allow(deprecated)]
fn test_multiple_cleans_create_multiple_backups() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // First clean
    create_branch(repo.path(), "first-branch");
    make_branch_old(repo.path(), "first-branch", 45);
    merge_branch(repo.path(), "first-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Wait a tiny bit to ensure different timestamp
    std::thread::sleep(std::time::Duration::from_millis(1100));

    // Second clean
    create_branch(repo.path(), "second-branch");
    make_branch_old(repo.path(), "second-branch", 45);
    merge_branch(repo.path(), "second-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Should have 2 backup files
    let backup_dir = get_backup_dir(&repo_name);
    let backup_count = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert_eq!(backup_count, 2);

    // backup list should show both
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "list", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("2")); // Shows "2" somewhere (backup count or in table)
}
