mod config;
mod discovery;
mod manager;
mod operations;
pub mod templates;

pub use config::{
    AppIntegrations, ITerm2Integration, Repository, VSCodeIntegration, WezTermIntegration,
    WorkspaceConfig, WorkspaceInfo,
};
pub use manager::{AppSelection, WorkspaceManager};
pub use templates::TemplateManager;
