//! MCP tool handlers for vibe-workspace commands

pub mod apps;
pub mod config;
pub mod git;
pub mod repos;
pub mod validation;
pub mod worktree;
pub mod worktree_help;

// Re-export all handlers for easy access

// Config handlers
pub use config::{
    BackupConfigTool, InitConfigTool, InitWorkspaceTool, ResetConfigTool, RestoreConfigTool,
    ShowConfigTool, ValidateConfigTool,
};

// App management handlers
pub use apps::{
    ConfigureAppTool, CreateAppTemplateTool, DeleteAppTemplateTool, ListAppTemplatesTool,
    ShowAppsTool, UpdateDefaultTemplatesTool,
};

// Repository operation handlers
pub use repos::{CloneTool, CreateRepositoryTool, LaunchRepoTool, OpenRepoTool};

// Git operation handlers
pub use git::{
    CloneRepoTool, ExecGitCommandTool, GitStatusTool, ResetGitConfigTool, ScanReposTool,
    SyncReposTool,
};

// Validation handler
pub use validation::ValidateMcpInterfaceTool;

// Worktree handlers
pub use worktree::{
    AnalyzeConflictsTool, CreateWorktreeTool, ExecuteCleanupTool, ListWorktreesTool,
    RecommendCleanupTool,
};

// Worktree help handler
pub use worktree_help::WorktreeHelpTool;
