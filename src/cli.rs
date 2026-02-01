//! CLI argument definitions using clap

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "deadbranch")]
#[command(author, version, about = "Clean up stale git branches safely", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List stale branches
    List {
        /// Only show branches older than N days (default: from config or 30)
        #[arg(short, long)]
        days: Option<u32>,

        /// Only show local branches
        #[arg(long)]
        local: bool,

        /// Only show remote branches
        #[arg(long, conflicts_with = "local")]
        remote: bool,

        /// Only show merged branches
        #[arg(long)]
        merged: bool,
    },

    /// Delete stale branches (merged only by default, use --force for unmerged)
    Clean {
        /// Only delete branches older than N days (default: from config or 30)
        #[arg(short, long)]
        days: Option<u32>,

        /// Only delete merged branches (this is the default behavior)
        #[arg(long)]
        merged: bool,

        /// Force delete unmerged branches (dangerous!)
        #[arg(long)]
        force: bool,

        /// Show what would be deleted without doing it
        #[arg(long)]
        dry_run: bool,

        /// Only delete local branches
        #[arg(long)]
        local: bool,

        /// Only delete remote branches
        #[arg(long, conflicts_with = "local")]
        remote: bool,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., default-days, protected-branches, default-branch, exclude-patterns)
        key: String,

        /// Configuration value(s) - use multiple arguments for lists
        /// Example: config set exclude-patterns "wip/*" "draft/*" "temp/*"
        #[arg(required = true, num_args = 1..)]
        values: Vec<String>,
    },

    /// Show current configuration
    Show,

    /// Open config file in $EDITOR
    Edit,

    /// Reset configuration to defaults
    Reset,
}
