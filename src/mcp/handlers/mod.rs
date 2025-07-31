//! MCP tool handlers for vibe-workspace commands

pub mod apps;
pub mod config;
pub mod git;
pub mod repos;
pub mod validation;

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
pub use repos::{CloneAndOpenTool, LaunchRepoTool, OpenRepoTool};

// Git operation handlers
pub use git::{
    CloneRepoTool, ExecGitCommandTool, GitStatusTool, ResetGitConfigTool, ScanReposTool,
    SyncReposTool,
};

// Validation handler
pub use validation::ValidateMcpInterfaceTool;
