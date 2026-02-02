//! deadbranch - Clean up stale git branches safely

mod backup;
mod branch;
mod cli;
mod config;
mod error;
mod git;
mod ui;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use std::fs;
use std::io::Write;

use branch::BranchFilter;
use cli::{BackupAction, Cli, Commands, ConfigAction};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check if we're in a git repository (except for config and backup commands)
    if !matches!(
        cli.command,
        Commands::Config { .. } | Commands::Backup { .. }
    ) && !git::is_git_repository()
    {
        ui::error("Not a git repository (or any parent up to mount point)");
        std::process::exit(1);
    }

    match cli.command {
        Commands::List {
            days,
            local,
            remote,
            merged,
        } => cmd_list(days, local, remote, merged),

        Commands::Clean {
            days,
            merged,
            force,
            dry_run,
            local,
            remote,
            yes,
        } => cmd_clean(days, merged, force, dry_run, local, remote, yes),

        Commands::Config { action } => cmd_config(action),

        Commands::Backup { action } => cmd_backup(action),
    }
}

/// List stale branches
fn cmd_list(
    days: Option<u32>,
    local_only: bool,
    remote_only: bool,
    merged_only: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Use CLI value if provided, otherwise use config default
    let min_age = days.unwrap_or(config.general.default_days);

    // Get default branch for merge detection
    let default_branch = config
        .branches
        .default_branch
        .clone()
        .unwrap_or_else(|| git::get_default_branch().unwrap_or_else(|_| "main".to_string()));

    ui::info(&format!(
        "Using '{}' as the default branch for merge detection",
        default_branch
    ));

    // List all branches
    let spinner = ui::spinner("Loading branches...");
    let all_branches = git::list_branches(&default_branch)?;
    spinner.finish_and_clear();

    // Filter branches
    let filter = BranchFilter {
        min_age_days: min_age,
        local_only,
        remote_only,
        merged_only,
        protected_branches: config.branches.protected,
        exclude_patterns: config.branches.exclude_patterns,
    };

    let mut branches: Vec<_> = all_branches
        .into_iter()
        .filter(|b| filter.matches(b))
        .collect();

    // Sort: unmerged first, then by age (oldest first)
    branch::sort_branches(&mut branches);

    // Separate local and remote for grouped display
    let mut local: Vec<_> = branches.iter().filter(|b| !b.is_remote).cloned().collect();
    let mut remote: Vec<_> = branches.iter().filter(|b| b.is_remote).cloned().collect();

    // Sort each group separately
    branch::sort_branches(&mut local);
    branch::sort_branches(&mut remote);

    // Display in table format
    if !local.is_empty() {
        ui::display_branches(&local, "Local Branches:");
    }
    if !remote.is_empty() {
        ui::display_branches(&remote, "Remote Branches:");
    }
    if local.is_empty() && remote.is_empty() {
        ui::info("No stale branches found.");
    }

    Ok(())
}

/// Clean (delete) stale branches
fn cmd_clean(
    days: Option<u32>,
    merged: bool,
    force: bool,
    dry_run: bool,
    local_only: bool,
    remote_only: bool,
    skip_confirm: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Use CLI value if provided, otherwise use config default
    let min_age = days.unwrap_or(config.general.default_days);

    // Get default branch for merge detection
    let default_branch = config
        .branches
        .default_branch
        .clone()
        .unwrap_or_else(|| git::get_default_branch().unwrap_or_else(|_| "main".to_string()));

    // By default, only delete merged branches unless --force is used
    let merged_only = merged || !force;

    // Create filter - by default, show both local and remote branches
    // Use --local or --remote to filter to only one type
    let filter = BranchFilter {
        min_age_days: min_age,
        local_only,
        remote_only,
        merged_only,
        protected_branches: config.branches.protected.clone(),
        exclude_patterns: config.branches.exclude_patterns,
    };

    // List all branches
    let spinner = ui::spinner("Loading branches...");
    let all_branches = git::list_branches(&default_branch)?;
    spinner.finish_and_clear();

    // Filter branches
    let mut branches: Vec<_> = all_branches
        .into_iter()
        .filter(|b| filter.matches(b))
        .collect();

    // Sort: unmerged first, then by age (oldest first)
    branch::sort_branches(&mut branches);

    if branches.is_empty() {
        ui::info("No branches to delete.");
        return Ok(());
    }

    // Separate local and remote
    let mut local_branches: Vec<_> = branches.iter().filter(|b| !b.is_remote).cloned().collect();
    let mut remote_branches: Vec<_> = branches.iter().filter(|b| b.is_remote).cloned().collect();

    // Sort each group separately
    branch::sort_branches(&mut local_branches);
    branch::sort_branches(&mut remote_branches);

    if dry_run {
        // For dry-run, show all tables upfront
        if !local_branches.is_empty() {
            let title = format!(
                "Local {} to Delete:",
                ui::pluralize_branch_cap(local_branches.len())
            );
            ui::display_branches(&local_branches, &title);
        }
        if !remote_branches.is_empty() {
            let title = format!(
                "Remote {} to Delete:",
                ui::pluralize_branch_cap(remote_branches.len())
            );
            ui::display_branches(&remote_branches, &title);
        }

        ui::print_dry_run_header();

        for branch in &local_branches {
            let flag = if force || branch.is_merged {
                "-d"
            } else {
                "-D"
            };
            ui::print_dry_run_command(&format!("git branch {} {}", flag, branch.name));
        }

        for branch in &remote_branches {
            let name = branch.name.strip_prefix("origin/").unwrap_or(&branch.name);
            ui::print_dry_run_command(&format!("git push origin --delete {}", name));
        }

        ui::print_dry_run_footer();
        return Ok(());
    }

    // Handle local branches - show table right before confirmation
    if !local_branches.is_empty() {
        let title = format!(
            "Local {} to Delete:",
            ui::pluralize_branch_cap(local_branches.len())
        );
        ui::display_branches(&local_branches, &title);

        if skip_confirm || ui::confirm_local_deletion(&local_branches) {
            delete_branches_with_backup(&local_branches, force)?;
        } else {
            println!();
            ui::info("Skipped local branch deletion.");
        }
    }

    // Handle remote branches - show table as part of the warning
    if !remote_branches.is_empty() {
        // Add visual separation if we just handled local branches
        if !local_branches.is_empty() {
            println!();
            println!("{}", console::style("─".repeat(50)).dim());
            println!();
        }

        // First, fetch and prune to ensure we have accurate data
        let spinner = ui::spinner("Fetching remote to ensure data is up to date...");
        match git::fetch_and_prune() {
            Ok(()) => ui::spinner_success(&spinner, "Remote data is up to date"),
            Err(e) => {
                ui::spinner_warn(&spinner, "Could not fetch remote");
                ui::warning(&format!("  {}", e));
                ui::warning("  Remote branch data may be stale.");
            }
        }

        // Show table and get confirmation
        let title = format!(
            "Remote {} to Delete:",
            ui::pluralize_branch_cap(remote_branches.len())
        );
        ui::display_branches(&remote_branches, &title);

        if skip_confirm || ui::confirm_remote_deletion(&remote_branches) {
            delete_remote_branches_with_backup(&remote_branches)?;
        } else {
            println!();
            ui::info("Skipped remote branch deletion.");
        }
    }

    Ok(())
}

/// Delete local branches and create backup file
fn delete_branches_with_backup(branches: &[branch::Branch], force: bool) -> Result<()> {
    let backup = create_backup_file(branches)?;
    let branch_word = ui::pluralize_branch(branches.len());

    // Visual separation after confirmation
    println!();
    println!("Deleting local {}...", branch_word);

    let mut deleted = 0;
    let mut failed = 0;

    for branch in branches {
        match git::delete_local_branch(&branch.name, force) {
            Ok(()) => {
                println!("  {} {}", console::style("✓").green(), branch.name);
                deleted += 1;
            }
            Err(e) => {
                println!("  {} {} ({})", console::style("✗").red(), branch.name, e);
                failed += 1;
            }
        }
    }

    // Summary footer
    println!();
    let branch_word = ui::pluralize_branch(deleted);
    if failed == 0 {
        ui::success(&format!("Deleted {} local {}", deleted, branch_word));
    } else {
        ui::warning(&format!(
            "Deleted {} local {}, {} failed",
            deleted, branch_word, failed
        ));
    }
    println!(
        "  {} Backup: {}",
        console::style("↪").dim(),
        console::style(&backup).dim()
    );

    Ok(())
}

/// Delete remote branches and create backup file
fn delete_remote_branches_with_backup(branches: &[branch::Branch]) -> Result<()> {
    let backup = create_backup_file(branches)?;
    let branch_word = ui::pluralize_branch(branches.len());

    // Visual separation after confirmation
    println!();
    println!("Deleting remote {}...", branch_word);

    let mut deleted = 0;
    let mut failed = 0;

    for branch in branches {
        match git::delete_remote_branch(&branch.name) {
            Ok(()) => {
                println!("  {} {}", console::style("✓").green(), branch.name);
                deleted += 1;
            }
            Err(e) => {
                println!("  {} {} ({})", console::style("✗").red(), branch.name, e);
                failed += 1;
            }
        }
    }

    // Summary footer
    println!();
    let branch_word = ui::pluralize_branch(deleted);
    if failed == 0 {
        ui::success(&format!("Deleted {} remote {}", deleted, branch_word));
    } else {
        ui::warning(&format!(
            "Deleted {} remote {}, {} failed",
            deleted, branch_word, failed
        ));
    }
    println!(
        "  {} Backup: {}",
        console::style("↪").dim(),
        console::style(&backup).dim()
    );

    Ok(())
}

/// Create a backup file with branch SHAs for potential restoration
/// Saves to ~/.deadbranch/backups/<repo-name>/backup-<timestamp>.txt
fn create_backup_file(branches: &[branch::Branch]) -> Result<String> {
    let repo_name = Config::get_repo_name();
    let backup_dir = Config::repo_backup_dir(&repo_name)?;

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&backup_dir)?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("backup-{}.txt", timestamp);
    let backup_path = backup_dir.join(&filename);

    let mut file = fs::File::create(&backup_path)?;

    writeln!(file, "# deadbranch backup")?;
    writeln!(file, "# Created: {}", Utc::now().to_rfc3339())?;
    writeln!(file, "# Repository: {}", repo_name)?;
    writeln!(
        file,
        "# Working directory: {}",
        std::env::current_dir()?.display()
    )?;
    writeln!(file, "#")?;
    writeln!(file, "# To restore a branch, run the git command shown")?;
    writeln!(file, "#")?;
    writeln!(file)?;

    for branch in branches {
        let sha =
            git::get_branch_sha(&branch.name).unwrap_or_else(|_| branch.last_commit_sha.clone());
        let restore_name = if branch.is_remote {
            branch.name.strip_prefix("origin/").unwrap_or(&branch.name)
        } else {
            &branch.name
        };
        writeln!(file, "# {}", branch.name)?;
        writeln!(file, "git branch {} {}", restore_name, sha)?;
        writeln!(file)?;
    }

    Ok(backup_path.display().to_string())
}

/// Handle config subcommands
fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            let config_path = Config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(unknown)".to_string());

            ui::display_config(
                config.general.default_days,
                &config.branches.protected,
                &config.branches.exclude_patterns,
                config.branches.default_branch.as_deref(),
                &config_path,
            );
        }

        ConfigAction::Set { key, values } => {
            let mut config = Config::load()?;
            config.set(&key, &values)?;
            config.save()?;

            // Format display based on single value or list
            let display_value = if values.len() == 1 {
                values[0].clone()
            } else {
                values.join(", ")
            };
            ui::success(&format!("Set {} = {}", key, display_value));
        }

        ConfigAction::Edit => {
            // Ensure config file exists
            let _ = Config::load()?;
            let config_path = Config::config_path()?;

            // Get editor from $EDITOR or $VISUAL, fallback to common editors
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    // Try common editors
                    if which::which("nano").is_ok() {
                        "nano".to_string()
                    } else if which::which("vim").is_ok() {
                        "vim".to_string()
                    } else if which::which("vi").is_ok() {
                        "vi".to_string()
                    } else {
                        "nano".to_string() // Default fallback
                    }
                });

            ui::info(&format!(
                "Opening {} in {}...",
                config_path.display(),
                editor
            ));

            let status = std::process::Command::new(&editor)
                .arg(&config_path)
                .status()
                .with_context(|| format!("Failed to open editor: {}", editor))?;

            if status.success() {
                ui::success("Config file saved");
            } else {
                ui::warning("Editor exited with non-zero status");
            }
        }

        ConfigAction::Reset => {
            if ui::confirm("Reset configuration to defaults?", false) {
                let config = Config::default();
                config.save()?;
                ui::success("Configuration reset to defaults");
            } else {
                ui::info("Cancelled");
            }
        }
    }

    Ok(())
}

/// Handle backup subcommands
fn cmd_backup(action: BackupAction) -> Result<()> {
    match action {
        BackupAction::List { current, repo } => {
            // Determine which repo to show (if any specific one)
            let target_repo = if current {
                // Check if we're in a git repo for --current
                if !git::is_git_repository() {
                    ui::error("Not a git repository (or any parent up to mount point)");
                    ui::info("Use 'deadbranch backup list' without --current to see all backups.");
                    std::process::exit(1);
                }
                Some(Config::get_repo_name())
            } else {
                repo
            };

            if let Some(repo_name) = target_repo {
                // Show detailed view for specific repo
                let backups = backup::list_repo_backups(&repo_name)?;

                if backups.is_empty() {
                    ui::info(&format!("No backups found for repository '{}'", repo_name));
                    println!();
                    println!(
                        "  {} Backups are created automatically when running 'deadbranch clean'.",
                        console::style("↪").dim()
                    );
                } else {
                    ui::display_repo_backups(&repo_name, &backups);
                }
            } else {
                // Show summary of all repos
                let all_backups = backup::list_all_backups()?;

                if all_backups.is_empty() {
                    ui::info("No backups found.");
                    println!();
                    println!(
                        "  {} Backups are created automatically when running 'deadbranch clean'.",
                        console::style("↪").dim()
                    );
                } else {
                    ui::display_all_backups(&all_backups);
                }
            }
        }
    }

    Ok(())
}
