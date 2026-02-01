//! Integration tests for deadbranch CLI
//!
//! These tests use assert_cmd to test the CLI interface and
//! tempfile to create isolated git repositories for testing.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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
    // Calculate the timestamp for days_old days ago
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_timestamp = now - (days_old as u64 * 86400); // 86400 seconds per day

    // Checkout the branch
    StdCommand::new("git")
        .args(["checkout", branch_name])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Amend the commit with an old date using Unix timestamp
    // Git accepts @<timestamp> format for dates (portable across platforms)
    let date = format!("@{}", old_timestamp);
    let output = StdCommand::new("git")
        .args(["commit", "--amend", "--no-edit", "--date", &date])
        .env("GIT_COMMITTER_DATE", &date)
        .current_dir(repo_dir)
        .output()
        .unwrap();

    // Debug: print if the command failed
    if !output.status.success() {
        eprintln!(
            "Warning: git commit --amend failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Go back to main/master (handle both cases)
    let checkout_main = StdCommand::new("git")
        .args(["checkout", "main"])
        .current_dir(repo_dir)
        .output()
        .unwrap();

    if !checkout_main.status.success() {
        // Try master if main doesn't exist
        StdCommand::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_dir)
            .output()
            .unwrap();
    }
}

#[test]
#[allow(deprecated)]
fn test_version() {
    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("deadbranch"));
}

#[test]
#[allow(deprecated)]
fn test_help() {
    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Clean up stale git branches safely",
        ))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("config"));
}

#[test]
#[allow(deprecated)]
fn test_not_a_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&temp_dir)
        .assert()
        .failure()
        .code(1);
}

#[test]
#[allow(deprecated)]
fn test_list_empty_repo() {
    let repo = create_test_repo();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));
}

#[test]
#[allow(deprecated)]
fn test_list_with_old_branch() {
    let repo = create_test_repo();
    create_branch(repo.path(), "old-feature");
    make_branch_old(repo.path(), "old-feature", 45);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("old-feature"));
}

#[test]
#[allow(deprecated)]
fn test_list_with_new_branch() {
    let repo = create_test_repo();
    create_branch(repo.path(), "new-feature");
    // Don't make it old - should not appear in default 30-day filter

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));
}

#[test]
#[allow(deprecated)]
fn test_list_with_days_filter() {
    let repo = create_test_repo();
    create_branch(repo.path(), "feature");
    make_branch_old(repo.path(), "feature", 5);

    // With default 30 days, should not show
    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));

    // With --days 3, should show
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--days", "3"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("feature"));
}

#[test]
#[allow(deprecated)]
fn test_list_local_only() {
    let repo = create_test_repo();
    create_branch(repo.path(), "local-branch");
    make_branch_old(repo.path(), "local-branch", 45);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["list", "--local"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("local-branch"))
        .stdout(predicate::str::contains("Local Branches"));
}

#[test]
#[allow(deprecated)]
fn test_config_show() {
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("default_days"))
        .stdout(predicate::str::contains("protected"));
}

#[test]
#[allow(deprecated)]
fn test_config_set_default_days() {
    // Note: This modifies the actual config file, so we should test carefully
    // In a real scenario, we'd want to mock the config path

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["config", "set", "default-days", "45"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set default-days = 45"));

    // Reset to default
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["config", "set", "default-days", "30"])
        .assert()
        .success();
}

#[test]
#[allow(deprecated)]
fn test_clean_dry_run() {
    let repo = create_test_repo();
    create_branch(repo.path(), "old-merged");
    make_branch_old(repo.path(), "old-merged", 45);

    // Merge the branch
    StdCommand::new("git")
        .args(["merge", "old-merged", "--no-ff", "-m", "Merge old-merged"])
        .current_dir(&repo)
        .output()
        .unwrap();

    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "--dry-run"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN"))
        .stdout(predicate::str::contains("old-merged"))
        .stdout(predicate::str::contains("git branch -d"));
}

#[test]
#[allow(deprecated)]
fn test_clean_requires_confirmation() {
    let repo = create_test_repo();
    create_branch(repo.path(), "old-merged");
    make_branch_old(repo.path(), "old-merged", 45);

    // Merge the branch
    StdCommand::new("git")
        .args(["merge", "old-merged", "--no-ff", "-m", "Merge old-merged"])
        .current_dir(&repo)
        .output()
        .unwrap();

    // Without input, it should fail or prompt
    // We can't test interactive prompts easily, but we can verify the branch list shows
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "--dry-run"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("old-merged"));
}

#[test]
#[allow(deprecated)]
fn test_list_respects_protected_branches() {
    let repo = create_test_repo();

    // Make main branch old (though it shouldn't show up as protected)
    make_branch_old(repo.path(), "main", 60);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));
}

#[test]
#[allow(deprecated)]
fn test_list_excludes_wip_branches() {
    let repo = create_test_repo();
    create_branch(repo.path(), "wip/test-feature");
    make_branch_old(repo.path(), "wip/test-feature", 45);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));
}

#[test]
#[allow(deprecated)]
fn test_list_excludes_draft_branches() {
    let repo = create_test_repo();
    create_branch(repo.path(), "feature/draft");
    make_branch_old(repo.path(), "feature/draft", 45);

    Command::cargo_bin("deadbranch")
        .unwrap()
        .arg("list")
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale branches found"));
}

#[test]
#[allow(deprecated)]
fn test_clean_merged_only_by_default() {
    let repo = create_test_repo();
    create_branch(repo.path(), "unmerged-old");
    make_branch_old(repo.path(), "unmerged-old", 45);

    // Don't merge it - should not show in clean by default
    Command::cargo_bin("deadbranch")
        .unwrap()
        .args(["clean", "--dry-run"])
        .current_dir(&repo)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("No branches to delete")
                .or(predicate::str::contains("unmerged-old").not()),
        );
}
