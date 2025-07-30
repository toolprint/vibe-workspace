mod config;
pub mod config_validator;
mod discovery;
pub mod install;
pub mod manager;
mod operations;
pub mod repo_analyzer;
mod sync_operations;
pub mod templates;

pub use config::{
    AppIntegrations, ITerm2Integration, Repository, VSCodeIntegration, WezTermIntegration,
    WorkspaceConfig, WorkspaceInfo,
};
pub use manager::{AppSelection, WorkspaceManager};
pub use templates::TemplateManager;
