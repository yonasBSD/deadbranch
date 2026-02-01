//! Custom error types for deadbranch

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeadbranchError {
    #[error("Not a git repository (or any parent up to mount point)")]
    NotAGitRepository,

    #[error("Git command failed: {0}")]
    GitCommandFailed(String),

    #[error("Branch '{0}' is protected and cannot be deleted")]
    ProtectedBranch(String),

    #[error("Branch '{0}' has unmerged changes. Use --force to delete anyway")]
    UnmergedBranch(String),

    #[error("Branch '{0}' not found")]
    BranchNotFound(String),

    #[error("Failed to read config: {0}")]
    ConfigRead(String),

    #[error("Failed to write config: {0}")]
    ConfigWrite(String),

    #[error("Operation cancelled by user")]
    UserCancelled,
}
