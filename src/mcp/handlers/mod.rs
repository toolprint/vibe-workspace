//! MCP tool handlers for vibe-workspace commands

pub mod git;

// Re-export all handlers for easy access
pub use git::GitStatusTool;
