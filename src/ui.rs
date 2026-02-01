//! UI utilities - output formatting, prompts, tables

use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, Table};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::branch::Branch;

/// Helper to pluralize "branch" correctly
pub fn pluralize_branch(count: usize) -> &'static str {
    if count == 1 {
        "branch"
    } else {
        "branches"
    }
}

/// Helper to pluralize "Branch" correctly (capitalized)
pub fn pluralize_branch_cap(count: usize) -> &'static str {
    if count == 1 {
        "Branch"
    } else {
        "Branches"
    }
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
            Cell::new(&branch.last_commit_date.format("%Y-%m-%d").to_string()).fg(Color::DarkGrey),
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
