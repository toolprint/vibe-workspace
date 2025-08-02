mod config;
pub mod config_validator;
pub mod constants;
mod discovery;
pub mod install;
pub mod manager;
pub mod operations;
pub mod repo_analyzer;
mod sync_operations;
pub mod templates;

pub use config::{
    AppIntegrations, CursorIntegration, ITerm2Integration, Repository, VSCodeIntegration,
    WezTermIntegration, WindsurfIntegration, WorkspaceConfig, WorkspaceInfo,
};
pub use manager::{AppSelection, WorkspaceManager};
pub use templates::TemplateManager;
