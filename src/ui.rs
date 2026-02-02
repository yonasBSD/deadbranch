//! UI utilities - output formatting, prompts, tables

use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, Table};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

use crate::backup::BackupInfo;
use crate::branch::Branch;

/// Generic pluralization helper
pub fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

/// Helper to pluralize "branch" correctly
pub fn pluralize_branch(count: usize) -> &'static str {
    pluralize(count, "branch", "branches")
}

/// Helper to pluralize "Branch" correctly (capitalized)
pub fn pluralize_branch_cap(count: usize) -> &'static str {
    pluralize(count, "Branch", "Branches")
}

/// Create a spinner with a message
pub fn spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

/// Finish spinner with success
pub fn spinner_success(spinner: &ProgressBar, message: &str) {
    spinner.finish_and_clear();
    println!("{} {}", style("✓").green(), message);
}

/// Finish spinner with warning
pub fn spinner_warn(spinner: &ProgressBar, message: &str) {
    spinner.finish_and_clear();
    println!("{} {}", style("!").yellow(), message);
}

/// Display a list of branches in a table
pub fn display_branches(branches: &[Branch], title: &str) {
    if branches.is_empty() {
        println!("{}", style("No stale branches found.").dim());
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Last Commit").add_attribute(Attribute::Bold),
    ]);

    for branch in branches {
        let status = if branch.is_merged {
            Cell::new("merged").fg(Color::Green)
        } else {
            Cell::new("unmerged").fg(Color::Yellow)
        };

        let branch_type = if branch.is_remote {
            Cell::new("remote").fg(Color::Blue)
        } else {
            Cell::new("local").fg(Color::Cyan)
        };

        table.add_row(vec![
            Cell::new(&branch.name),
            Cell::new(branch.format_age()),
            status,
            branch_type,
            Cell::new(branch.last_commit_date.format("%Y-%m-%d").to_string()).fg(Color::DarkGrey),
        ]);
    }

    println!("\n{}", style(title).bold());
    println!("{table}\n");
}

/// Ask for confirmation with nice themed UI
pub fn confirm(prompt: &str, default: bool) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .wait_for_newline(true)
        .interact()
        .unwrap_or(false)
}

/// Ask for confirmation to delete local branches with visual summary
pub fn confirm_local_deletion(branches: &[Branch]) -> bool {
    let total = branches.len();
    let merged_count = branches.iter().filter(|b| b.is_merged).count();
    let unmerged_count = total - merged_count;
    let branch_word = pluralize_branch(total);

    // Build a descriptive prompt
    let summary = if unmerged_count > 0 {
        format!(
            "{} {} local {} ({} merged, {} unmerged)?",
            style("Delete").red().bold(),
            style(total).yellow().bold(),
            branch_word,
            style(merged_count).green(),
            style(unmerged_count).yellow()
        )
    } else {
        format!(
            "{} {} local {} (all merged)?",
            style("Delete").yellow().bold(),
            style(total).cyan().bold(),
            branch_word
        )
    };

    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(summary)
        .default(false)
        .wait_for_newline(true)
        .interact()
        .unwrap_or(false)
}

/// Display success message
pub fn success(message: &str) {
    println!("{} {}", style("✓").green().bold(), message);
}

/// Display warning message
pub fn warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), message);
}

/// Display error message
pub fn error(message: &str) {
    eprintln!("{} {}", style("✗").red().bold(), message);
}

/// Display info message
pub fn info(message: &str) {
    println!("{} {}", style("ℹ").blue().bold(), message);
}

/// Print dry-run header
pub fn print_dry_run_header() {
    println!(
        "\n{}\n",
        style("[DRY RUN] No branches will be deleted.")
            .yellow()
            .bold()
    );
    println!("Commands that would run:");
}

/// Print dry-run command
pub fn print_dry_run_command(cmd: &str) {
    println!("  {}", style(cmd).dim());
}

/// Print dry-run footer
pub fn print_dry_run_footer() {
    println!();
    info("No branches were actually deleted.");
}

/// Display remote deletion warning and get confirmation
/// Returns true if user confirms, false otherwise
pub fn confirm_remote_deletion(branches: &[Branch]) -> bool {
    let count = branches.len();
    let branch_word = pluralize_branch(count);

    println!();
    println!(
        "{}",
        style(format!(
            "⚠  WARNING: You are about to delete remote {}!",
            branch_word
        ))
        .yellow()
        .bold()
    );
    println!();
    println!("This action:");
    println!("  • {} easily", style("Cannot be undone").red());
    println!("  • Will {} all team members", style("affect").red());
    println!(
        "  • Removes {} from origin {}",
        branch_word,
        style("permanently").red()
    );
    println!();

    // Simple confirmation text with just the count
    let expected = format!("delete {} remote {}", count, branch_word);
    println!(
        "To confirm, type exactly: {}",
        style(format!("\"{}\"", expected)).yellow()
    );
    println!();

    let term = console::Term::stdout();
    let _ = term.show_cursor();

    let input: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Type confirmation")
        .allow_empty(true)
        .interact_on(&term)
        .unwrap_or_default();

    input.trim() == expected
}

/// Display configuration in a table
pub fn display_config(
    default_days: u32,
    protected_branches: &[String],
    exclude_patterns: &[String],
    default_branch: Option<&str>,
    config_path: &str,
) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("Section").add_attribute(Attribute::Bold),
        Cell::new("Setting").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);

    // General section
    table.add_row(vec![
        Cell::new("general").fg(Color::Yellow),
        Cell::new("default_days"),
        Cell::new(default_days.to_string()).fg(Color::Cyan),
    ]);

    // Branches section
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("default_branch"),
        Cell::new(default_branch.unwrap_or("(auto-detect)")).fg(Color::Cyan),
    ]);

    let protected_display = if protected_branches.is_empty() {
        "(none)".to_string()
    } else {
        protected_branches.join(", ")
    };
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("protected"),
        Cell::new(protected_display).fg(Color::Cyan),
    ]);

    let exclude_display = if exclude_patterns.is_empty() {
        "(none)".to_string()
    } else {
        exclude_patterns.join(", ")
    };
    table.add_row(vec![
        Cell::new("branches").fg(Color::Yellow),
        Cell::new("exclude_patterns"),
        Cell::new(exclude_display).fg(Color::Cyan),
    ]);

    println!("\n{}", style("Configuration:").bold());
    println!("{table}");
    println!(
        "{} {}",
        style("Config file:").dim(),
        style(config_path).dim()
    );
    println!();
}

/// Display backups for a single repository
pub fn display_repo_backups(repo_name: &str, backups: &[BackupInfo]) {
    if backups.is_empty() {
        println!(
            "{}",
            style(format!("No backups found for '{}'.", repo_name)).dim()
        );
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Backup").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Branches").add_attribute(Attribute::Bold),
    ]);

    for (i, backup) in backups.iter().enumerate() {
        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(backup.filename()),
            Cell::new(backup.format_age()).fg(Color::Cyan),
            Cell::new(backup.branch_count.to_string()).fg(Color::Yellow),
        ]);
    }

    println!(
        "\n{}",
        style(format!("Backups for '{}':", repo_name)).bold()
    );
    println!("{table}");

    // Show restore hint
    println!();
    println!("{}", style("To restore a branch:").dim());
    println!(
        "  {}",
        style("deadbranch backup restore <branch-name>").dim()
    );
    println!(
        "  {}",
        style("deadbranch backup restore <branch-name> --from <backup-file>").dim()
    );
    println!();
}

/// Display all backups as a summary grouped by repository
pub fn display_all_backups(all_backups: &HashMap<String, Vec<BackupInfo>>) {
    if all_backups.is_empty() {
        println!("{}", style("No backups found.").dim());
        return;
    }

    // Sort repositories alphabetically
    let mut repos: Vec<_> = all_backups.keys().collect();
    repos.sort();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Repository").add_attribute(Attribute::Bold),
        Cell::new("Backups").add_attribute(Attribute::Bold),
        Cell::new("Latest").add_attribute(Attribute::Bold),
        Cell::new("Oldest").add_attribute(Attribute::Bold),
    ]);

    let mut total_backups = 0;

    for (i, repo_name) in repos.iter().enumerate() {
        let backups = &all_backups[*repo_name];
        total_backups += backups.len();

        // Backups are already sorted newest first
        let latest_age = backups.first().map(|b| b.format_age()).unwrap_or_default();
        let oldest_age = backups.last().map(|b| b.format_age()).unwrap_or_default();

        table.add_row(vec![
            Cell::new((i + 1).to_string()).fg(Color::DarkGrey),
            Cell::new(repo_name.as_str()).fg(Color::Yellow),
            Cell::new(backups.len().to_string()).fg(Color::Yellow),
            Cell::new(latest_age).fg(Color::Cyan),
            Cell::new(oldest_age).fg(Color::DarkGrey),
        ]);
    }

    println!("\n{}", style("All backups:").bold());
    println!("{table}");

    // Summary
    println!(
        "\n{} {} {} across {} {}",
        style("Total:").dim(),
        style(total_backups).cyan(),
        pluralize(total_backups, "backup", "backups"),
        style(repos.len()).cyan(),
        pluralize(repos.len(), "repository", "repositories")
    );

    // Hint
    println!();
    println!("{}", style("To see details for a repository:").dim());
    println!("  {}", style("deadbranch backup list --repo <name>").dim());
    println!(
        "  {}",
        style("deadbranch backup list --current  (for current repo)").dim()
    );
    println!();
}
