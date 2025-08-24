//! vibe-workspace library
//! 
//! Extremely lightweight workspace management for multiple git repositories

pub mod apps;
pub mod cache;
pub mod git;
pub mod mcp;
pub mod output;
pub mod repository;
pub mod ui;
pub mod uri;
pub mod utils;
pub mod workspace;
pub mod worktree;

// Re-export commonly used types
pub use workspace::{WorkspaceManager, WorkspaceConfig};
pub use worktree::{
    WorktreeManager, WorktreeConfig, WorktreeStatus, WorktreeInfo, MergeInfo,
    detect_worktree_merge_status, check_worktree_status, check_worktree_status_with_config
};