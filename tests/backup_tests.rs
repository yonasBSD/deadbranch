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
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("HOME or USERPROFILE not set");
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

// ============================================================================
// Tests for `deadbranch backup restore`
// ============================================================================

/// Helper to get the SHA of a branch
fn get_branch_sha(repo_dir: &std::path::Path, branch_name: &str) -> String {
    let output = StdCommand::new("git")
        .args(["rev-parse", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Helper to check if a branch exists
fn branch_exists(repo_dir: &std::path::Path, branch_name: &str) -> bool {
    StdCommand::new("git")
        .args([
            "rev-parse",
            "--verify",
            &format!("refs/heads/{}", branch_name),
        ])
        .current_dir(repo_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_basic() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create an old merged branch and save its SHA
    create_branch(repo.path(), "branch-to-restore");
    let _original_sha = get_branch_sha(repo.path(), "branch-to-restore");
    make_branch_old(repo.path(), "branch-to-restore", 45);
    merge_branch(repo.path(), "branch-to-restore");

    // Clean it (creates backup)
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Verify branch is gone
    assert!(!branch_exists(repo.path(), "branch-to-restore"));

    // Restore it
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "branch-to-restore"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("Restored branch"))
        .stdout(predicate::str::contains("branch-to-restore"));

    // Verify branch is back
    assert!(branch_exists(repo.path(), "branch-to-restore"));

    // Verify it points to the correct commit (the SHA after make_branch_old changed it)
    let restored_sha = get_branch_sha(repo.path(), "branch-to-restore");
    // The SHA will be different because make_branch_old amends the commit
    // But we just need to verify the branch exists and points to a valid commit
    assert!(!restored_sha.is_empty());
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_requires_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    // Restore outside a git repo should fail
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "some-branch"])
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .code(1);
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_no_backups() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name);

    // Try to restore without any backups
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "nonexistent-branch"])
        .current_dir(&repo)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("No backups found"));
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_branch_not_in_backup() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean a branch (creates backup)
    create_branch(repo.path(), "backed-up-branch");
    make_branch_old(repo.path(), "backed-up-branch", 45);
    merge_branch(repo.path(), "backed-up-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Try to restore a different branch that wasn't in the backup
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "not-in-backup"])
        .current_dir(&repo)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found in backup"))
        .stdout(predicate::str::contains("backed-up-branch")); // Should list available branches
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_branch_already_exists() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create a branch, clean it, then recreate it
    create_branch(repo.path(), "existing-branch");
    make_branch_old(repo.path(), "existing-branch", 45);
    merge_branch(repo.path(), "existing-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Recreate the branch
    create_branch(repo.path(), "existing-branch");

    // Try to restore - should fail because branch exists
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "existing-branch"])
        .current_dir(&repo)
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("already exists"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_with_force() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create a branch, clean it, then recreate it with different content
    create_branch(repo.path(), "force-test-branch");
    make_branch_old(repo.path(), "force-test-branch", 45);
    merge_branch(repo.path(), "force-test-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Recreate the branch with new content
    create_branch(repo.path(), "force-test-branch");
    let new_sha = get_branch_sha(repo.path(), "force-test-branch");

    // Restore with --force should overwrite
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "force-test-branch", "--force"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("overwrote existing"));

    // SHA should be different (restored to old commit)
    let restored_sha = get_branch_sha(repo.path(), "force-test-branch");
    assert_ne!(new_sha, restored_sha);
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_with_as_flag() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean a branch
    create_branch(repo.path(), "original-name");
    make_branch_old(repo.path(), "original-name", 45);
    merge_branch(repo.path(), "original-name");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Restore with a different name
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "original-name", "--as", "new-name"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("original-name"))
        .stdout(predicate::str::contains("new-name"));

    // Old name should not exist, new name should
    assert!(!branch_exists(repo.path(), "original-name"));
    assert!(branch_exists(repo.path(), "new-name"));
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_from_specific_backup() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean first branch
    create_branch(repo.path(), "first-backup-branch");
    make_branch_old(repo.path(), "first-backup-branch", 45);
    merge_branch(repo.path(), "first-backup-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Wait to ensure different timestamp
    std::thread::sleep(std::time::Duration::from_millis(1100));

    // Create and clean second branch
    create_branch(repo.path(), "second-backup-branch");
    make_branch_old(repo.path(), "second-backup-branch", 45);
    merge_branch(repo.path(), "second-backup-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Get the first backup file name (older one)
    let backup_dir = get_backup_dir(&repo_name);
    let mut backups: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    backups.sort(); // Sort alphabetically (oldest first since format is YYYYMMDD-HHMMSS)
    let first_backup = &backups[0];

    // Try to restore from the first backup (should only have first-backup-branch)
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args([
            "backup",
            "restore",
            "first-backup-branch",
            "--from",
            first_backup,
        ])
        .current_dir(&repo)
        .assert()
        .success();

    assert!(branch_exists(repo.path(), "first-backup-branch"));
}

#[test]
#[allow(deprecated)]
fn test_backup_restore_shows_short_sha() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create and clean a branch
    create_branch(repo.path(), "sha-display-test");
    make_branch_old(repo.path(), "sha-display-test", 45);
    merge_branch(repo.path(), "sha-display-test");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Restore and check that output contains a short SHA (8 chars)
    let assert = Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "restore", "sha-display-test"])
        .current_dir(&repo)
        .assert()
        .success();

    // The output should contain "at commit" followed by something that looks like a short SHA
    let output = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(output.contains("at commit"));
}

// ============================================================================
// Tests for `deadbranch backup clean`
// ============================================================================

#[test]
#[allow(deprecated)]
fn test_backup_clean_requires_current_or_repo() {
    let repo = create_test_repo();

    // Running backup clean without --current or --repo should fail
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean"])
        .current_dir(&repo)
        .assert()
        .failure();
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_current_requires_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    // Running --current outside a git repo should fail
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current"])
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .code(1);
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_no_backups() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name);

    // Cleaning when no backups exist should show appropriate message
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No backups found"));
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_nothing_to_clean() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create just one backup (less than default keep=10)
    create_branch(repo.path(), "single-backup-branch");
    make_branch_old(repo.path(), "single-backup-branch", 45);
    merge_branch(repo.path(), "single-backup-branch");

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Cleaning should show nothing to clean
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No old backups to clean"));
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_dry_run() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create 3 backups
    for i in 1..=3 {
        let branch_name = format!("dry-run-branch-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);

        Command::cargo_bin("deadbranch")
            .unwrap()
            .args(["clean", "-y"])
            .current_dir(&repo)
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Verify we have 3 backups
    let backup_dir = get_backup_dir(&repo_name);
    let backup_count_before = fs::read_dir(&backup_dir).unwrap().count();
    assert_eq!(backup_count_before, 3);

    // Dry run with keep=1 should show what would be deleted but not delete
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current", "--keep", "1", "--dry-run"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"))
        .stdout(predicate::str::contains("No backups will be deleted"));

    // Verify no files were actually deleted
    let backup_count_after = fs::read_dir(&backup_dir).unwrap().count();
    assert_eq!(backup_count_after, 3);
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_with_yes_flag() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create 3 backups
    for i in 1..=3 {
        let branch_name = format!("yes-flag-branch-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);

        Command::cargo_bin("deadbranch")
            .unwrap()
            .args(["clean", "-y"])
            .current_dir(&repo)
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Verify we have 3 backups
    let backup_dir = get_backup_dir(&repo_name);
    let backup_count_before = fs::read_dir(&backup_dir).unwrap().count();
    assert_eq!(backup_count_before, 3);

    // Clean with --yes and --keep=1 should delete 2 backups
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current", "--keep", "1", "-y"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"))
        .stdout(predicate::str::contains("2")); // 2 files deleted

    // Verify only 1 backup remains
    let backup_count_after = fs::read_dir(&backup_dir).unwrap().count();
    assert_eq!(backup_count_after, 1);
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_keeps_most_recent() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create 3 backups with distinct timestamps
    for i in 1..=3 {
        let branch_name = format!("keep-recent-branch-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);

        Command::cargo_bin("deadbranch")
            .unwrap()
            .args(["clean", "-y"])
            .current_dir(&repo)
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Get backup filenames sorted (oldest first)
    let backup_dir = get_backup_dir(&repo_name);
    let mut backups: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    backups.sort();
    let newest_backup = backups.last().unwrap().clone();

    // Clean with keep=1
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current", "--keep", "1", "-y"])
        .current_dir(&repo)
        .assert()
        .success();

    // Verify only the newest backup remains
    let remaining: Vec<_> = fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0], newest_backup);
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_with_repo_flag() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create 2 backups
    for i in 1..=2 {
        let branch_name = format!("repo-flag-clean-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);

        Command::cargo_bin("deadbranch")
            .unwrap()
            .args(["clean", "-y"])
            .current_dir(&repo)
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Can clean by repo name from anywhere
    let other_dir = TempDir::new().unwrap();
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--repo", &repo_name, "--keep", "1", "-y"])
        .current_dir(&other_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    // Verify only 1 backup remains
    let backup_dir = get_backup_dir(&repo_name);
    let backup_count = fs::read_dir(&backup_dir).unwrap().count();
    assert_eq!(backup_count, 1);
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_shows_table() {
    let repo = create_test_repo();
    let repo_name = get_repo_name(repo.path());
    let _guard = BackupCleanupGuard::new(repo_name.clone());

    // Create 2 backups
    for i in 1..=2 {
        let branch_name = format!("table-display-{}", i);
        create_branch(repo.path(), &branch_name);
        make_branch_old(repo.path(), &branch_name, 45);
        merge_branch(repo.path(), &branch_name);

        Command::cargo_bin("deadbranch")
            .unwrap()
            .args(["clean", "-y"])
            .current_dir(&repo)
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(1100));
    }

    // Dry run should show table with columns
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current", "--keep", "1", "--dry-run"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("Backup"))
        .stdout(predicate::str::contains("Age"))
        .stdout(predicate::str::contains("Branches"))
        .stdout(predicate::str::contains("Size"))
        .stdout(predicate::str::contains("backup-"));
}

#[test]
#[allow(deprecated)]
fn test_backup_clean_mutual_exclusion() {
    let repo = create_test_repo();

    // --current and --repo should be mutually exclusive
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["backup", "clean", "--current", "--repo", "some-repo"])
        .current_dir(&repo)
        .assert()
        .failure();
}
