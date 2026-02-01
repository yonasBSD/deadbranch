//! Custom error types for deadbranch

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeadbranchError {
    #[error("Branch '{0}' has unmerged changes. Use --force to delete anyway")]
    UnmergedBranch(String),
}
