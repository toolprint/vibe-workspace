pub mod claude_agents;
pub mod config;
pub mod config_validator;
pub mod constants;
mod discovery;
pub mod install;
pub mod manager;
pub mod operations;
pub mod repo_analyzer;
mod sync_operations;
pub mod templates;

pub use config::{Repository, WorkspaceConfig};

// Test-only exports - these are only used by app module tests
#[cfg(test)]
pub use config::{
    CursorIntegration, ITerm2Integration, VSCodeIntegration, WezTermIntegration,
    WindsurfIntegration, WorkspaceInfo,
};
pub use manager::{AppSelection, WorkspaceManager};
pub use templates::TemplateManager;
