//! WorktreeManager - main coordinator for worktree operations

use anyhow::Result;
use std::path::PathBuf;

use crate::workspace::WorkspaceManager;
use crate::worktree::config::WorktreeConfig;
use crate::worktree::config_manager::{
    ConfigSummary, ConfigValidationError, WorktreeConfigManager,
};
use crate::worktree::operations::{CreateOptions, RemoveOptions, WorktreeOperations};
use crate::worktree::status::WorktreeInfo;

/// Main coordinator for all worktree operations
pub struct WorktreeManager {
    operations: WorktreeOperations,
    pub workspace_root: PathBuf,
    config: WorktreeConfig,
    config_manager: Option<WorktreeConfigManager>,
}

impl WorktreeManager {
    /// Create a new WorktreeManager
    pub async fn new(workspace_root: PathBuf, config: Option<WorktreeConfig>) -> Result<Self> {
        let config =
            config.unwrap_or_else(|| WorktreeConfig::load_with_overrides().unwrap_or_default());

        config
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid config: {}", e))?;

        // Extract repository name from workspace_root
        let repo_name = workspace_root
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let operations = if let Some(name) = repo_name {
            WorktreeOperations::new_with_repo_name(workspace_root.clone(), config.clone(), name)
        } else {
            WorktreeOperations::new(workspace_root.clone(), config.clone())
        };

        Ok(Self {
            operations,
            workspace_root,
            config,
            config_manager: None,
        })
    }

    /// Create a new WorktreeManager with workspace integration
    pub async fn new_with_workspace_manager(
        workspace_manager: &WorkspaceManager,
        repo_path: Option<PathBuf>,
    ) -> Result<Self> {
        let config_manager =
            WorktreeConfigManager::new(workspace_manager.get_config_path().clone());

        // Migrate legacy configuration if needed
        config_manager.migrate_legacy_config().await?;

        // Extract repository name if repo_path is provided
        let repo_name = repo_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Load configuration for specific repository or use global
        let config = if let Some(ref path) = repo_path {
            config_manager.load_config_for_repo(path).await?
        } else {
            WorktreeConfig::load_with_overrides()
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
        };

        // Use the appropriate repo root (repo_path if provided, otherwise workspace root)
        let repo_root = repo_path
            .as_ref()
            .unwrap_or(workspace_manager.get_workspace_root())
            .clone();

        let operations = if let Some(name) = repo_name {
            WorktreeOperations::new_with_repo_name(repo_root.clone(), config.clone(), name)
        } else {
            WorktreeOperations::new(repo_root.clone(), config.clone())
        };

        Ok(Self {
            operations,
            workspace_root: repo_root,
            config,
            config_manager: Some(config_manager),
        })
    }

    /// Create a new worktree
    pub async fn create_worktree(&self, task_id: String) -> Result<WorktreeInfo> {
        let options = CreateOptions {
            task_id,
            base_branch: None,
            force: false,
            custom_path: None,
        };

        self.operations.create_worktree(options).await
    }

    /// Create a worktree with custom options
    pub async fn create_worktree_with_options(
        &self,
        options: CreateOptions,
    ) -> Result<WorktreeInfo> {
        self.operations.create_worktree(options).await
    }

    /// Remove a worktree
    pub async fn remove_worktree(&self, branch_or_path: String, force: bool) -> Result<()> {
        let options = RemoveOptions {
            target: branch_or_path,
            force,
            delete_branch: false,
        };

        self.operations.remove_worktree(options).await
    }

    /// Remove a worktree with custom options
    pub async fn remove_worktree_with_options(&self, options: RemoveOptions) -> Result<()> {
        self.operations.remove_worktree(options).await
    }

    /// List all worktrees
    pub async fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        self.operations.list_worktrees().await
    }

    /// Get the git repository root
    pub async fn get_git_root(&self) -> Result<PathBuf> {
        self.operations.find_git_root().await
    }

    /// Get a reference to the config
    pub fn get_config(&self) -> &WorktreeConfig {
        &self.config
    }

    /// Resolve target (task ID, branch name, or path) to worktree info
    pub async fn resolve_worktree_target(&self, target: &str) -> Result<WorktreeInfo> {
        self.operations.resolve_worktree_target(target).await
    }

    /// Get a clone of the operations (for cleanup)
    pub fn get_operations(&self) -> WorktreeOperations {
        self.operations.clone()
    }

    /// Get configuration summary for diagnostics
    pub async fn get_config_summary(&self) -> Result<ConfigSummary> {
        if let Some(config_manager) = &self.config_manager {
            config_manager.get_config_summary().await
        } else {
            Err(anyhow::anyhow!("Config manager not available"))
        }
    }

    /// Validate current configuration
    pub async fn validate_configuration(&self) -> Result<Vec<ConfigValidationError>> {
        if let Some(config_manager) = &self.config_manager {
            config_manager.validate_all_configs().await
        } else {
            Ok(Vec::new())
        }
    }
}
