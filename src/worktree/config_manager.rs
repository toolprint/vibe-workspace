//! Configuration management integration for worktree system

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::workspace::config::{WorkspaceConfig, RepositoryWorktreeConfig};
use crate::worktree::config::WorktreeConfig;

/// Configuration manager for worktree settings
pub struct WorktreeConfigManager {
    workspace_config_path: PathBuf,
}

impl WorktreeConfigManager {
    pub fn new(workspace_config_path: PathBuf) -> Self {
        Self { workspace_config_path }
    }
    
    /// Load worktree configuration for a specific repository
    pub async fn load_config_for_repo(&self, repo_path: &Path) -> Result<WorktreeConfig> {
        // Try to load workspace configuration
        let workspace_config = self.load_workspace_config().await?;
        
        // Find the repository in the workspace config
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid repository path")?;
        
        // Get repository-specific config with fallback to global
        let config = workspace_config.get_worktree_config_for_repo(repo_name);
        
        debug!("Loaded worktree config for {}: base_dir={}, prefix={}", 
               repo_name, config.base_dir.display(), config.prefix);
        
        Ok(config)
    }
    
    /// Save worktree configuration changes
    pub async fn save_worktree_config(
        &self, 
        global_config: Option<WorktreeConfig>,
        repo_configs: Vec<(String, RepositoryWorktreeConfig)>,
    ) -> Result<()> {
        let mut workspace_config = self.load_workspace_config().await?;
        
        // Update global worktree config if provided
        if let Some(global) = global_config {
            workspace_config.worktree = global;
        }
        
        // Update repository-specific configs
        for (repo_name, repo_config) in repo_configs {
            if let Some(repo) = workspace_config.repositories.iter_mut().find(|r| r.name == repo_name) {
                repo.worktree_config = Some(repo_config);
            } else {
                warn!("Repository '{}' not found in workspace config", repo_name);
            }
        }
        
        // Save the updated configuration
        self.save_workspace_config(&workspace_config).await?;
        
        Ok(())
    }
    
    /// Initialize worktree configuration for a new repository
    pub async fn initialize_repo_config(
        &self,
        repo_name: &str,
        repo_config: Option<RepositoryWorktreeConfig>,
    ) -> Result<()> {
        let mut workspace_config = self.load_workspace_config().await?;
        
        // Find or create repository entry
        if let Some(repo) = workspace_config.repositories.iter_mut().find(|r| r.name == repo_name) {
            repo.worktree_config = repo_config;
        } else {
            warn!("Repository '{}' not found for worktree initialization", repo_name);
            return Ok(());
        }
        
        self.save_workspace_config(&workspace_config).await?;
        Ok(())
    }
    
    /// Migrate old configuration format to new format
    pub async fn migrate_legacy_config(&self) -> Result<bool> {
        // Check if there's an old worktree configuration file
        let legacy_config_path = self.workspace_config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("worktree-config.yaml");
        
        if !legacy_config_path.exists() {
            return Ok(false); // No legacy config to migrate
        }
        
        debug!("Found legacy worktree config, migrating...");
        
        // Load legacy configuration
        let legacy_content = tokio::fs::read_to_string(&legacy_config_path).await?;
        let legacy_config: WorktreeConfig = serde_yaml::from_str(&legacy_content)?;
        
        // Load current workspace config
        let mut workspace_config = self.load_workspace_config().await?;
        
        // Merge legacy config into workspace config
        workspace_config.worktree = legacy_config;
        
        // Save updated workspace config
        self.save_workspace_config(&workspace_config).await?;
        
        // Archive the legacy config file
        let archived_path = legacy_config_path.with_extension("yaml.migrated");
        tokio::fs::rename(&legacy_config_path, &archived_path).await?;
        
        debug!("Migrated legacy worktree config and archived original");
        Ok(true)
    }
    
    /// Validate configuration across all repositories
    pub async fn validate_all_configs(&self) -> Result<Vec<ConfigValidationError>> {
        let workspace_config = self.load_workspace_config().await?;
        let mut errors = Vec::new();
        
        // Validate global configuration
        if let Err(error) = workspace_config.worktree.validate() {
            errors.push(ConfigValidationError {
                repository: None,
                error: error,
            });
        }
        
        // Validate repository-specific configurations
        for repo in &workspace_config.repositories {
            if let Some(repo_config) = &repo.worktree_config {
                let effective_config = repo_config.merge_with_global(&workspace_config.worktree);
                if let Err(error) = effective_config.validate() {
                    errors.push(ConfigValidationError {
                        repository: Some(repo.name.clone()),
                        error,
                    });
                }
            }
        }
        
        Ok(errors)
    }
    
    /// Get configuration summary for diagnostics
    pub async fn get_config_summary(&self) -> Result<ConfigSummary> {
        let workspace_config = self.load_workspace_config().await?;
        
        // Apply environment variable overrides to the global config
        let mut global_config = workspace_config.worktree.clone();
        
        // Apply environment variable overrides
        if let Ok(mode) = std::env::var("VIBE_WORKTREE_MODE") {
            global_config.mode = match mode.to_lowercase().as_str() {
                "global" => crate::worktree::config::WorktreeMode::Global,
                "local" => crate::worktree::config::WorktreeMode::Local,
                _ => global_config.mode, // Keep existing if invalid
            };
        }
        
        if let Ok(base_dir) = std::env::var("VIBE_WORKTREE_BASE") {
            global_config.base_dir = PathBuf::from(base_dir);
        }
        
        if let Ok(prefix) = std::env::var("VIBE_WORKTREE_PREFIX") {
            global_config.prefix = prefix;
        }
        
        let repo_overrides = workspace_config.repositories
            .iter()
            .filter(|r| r.worktree_config.is_some())
            .map(|r| (r.name.clone(), r.worktree_config.as_ref().unwrap().clone()))
            .collect();
        
        // Calculate resolved base directory (pass None for repo_root since this is global config)
        let resolved_base_dir = global_config.get_resolved_base_dir(None);
        
        Ok(ConfigSummary {
            global_config,
            resolved_base_dir,
            repo_overrides,
            total_repositories: workspace_config.repositories.len(),
            enabled_repositories: workspace_config.repositories
                .iter()
                .filter(|r| workspace_config.is_worktree_enabled_for_repo(&r.name))
                .count(),
        })
    }
    
    // Private helper methods
    
    async fn load_workspace_config(&self) -> Result<WorkspaceConfig> {
        if !self.workspace_config_path.exists() {
            // Create default configuration if it doesn't exist
            let default_config = WorkspaceConfig::default();
            self.save_workspace_config(&default_config).await?;
            return Ok(default_config);
        }
        
        let content = tokio::fs::read_to_string(&self.workspace_config_path).await
            .with_context(|| format!("Failed to read config from {}", self.workspace_config_path.display()))?;
            
        let config: WorkspaceConfig = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse workspace configuration")?;
            
        Ok(config)
    }
    
    async fn save_workspace_config(&self, config: &WorkspaceConfig) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.workspace_config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Serialize configuration
        let content = serde_yaml::to_string(config)
            .with_context(|| "Failed to serialize workspace configuration")?;
        
        // Write to file
        tokio::fs::write(&self.workspace_config_path, content).await
            .with_context(|| format!("Failed to write config to {}", self.workspace_config_path.display()))?;
        
        debug!("Saved workspace configuration to {}", self.workspace_config_path.display());
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfigValidationError {
    pub repository: Option<String>,
    pub error: String,
}

#[derive(Debug)]
pub struct ConfigSummary {
    pub global_config: WorktreeConfig,
    pub resolved_base_dir: PathBuf,
    pub repo_overrides: Vec<(String, RepositoryWorktreeConfig)>,
    pub total_repositories: usize,
    pub enabled_repositories: usize,
}

impl ConfigSummary {
    /// Generate a human-readable summary
    pub fn format_summary(&self) -> String {
        let mut summary = String::new();
        
        summary.push_str("Worktree Configuration Summary:\n");
        summary.push_str(&format!("  Mode: {:?}\n", self.global_config.mode));
        summary.push_str(&format!("  Global prefix: {}\n", self.global_config.prefix));
        summary.push_str(&format!("  Base directory (configured): {}\n", self.global_config.base_dir.display()));
        
        // Show resolved path if different from configured path
        if self.resolved_base_dir != self.global_config.base_dir {
            summary.push_str(&format!("  Base directory (resolved): {}\n", self.resolved_base_dir.display()));
        }
        
        summary.push_str(&format!("  Total repositories: {}\n", self.total_repositories));
        summary.push_str(&format!("  Enabled repositories: {}\n", self.enabled_repositories));
        
        if !self.repo_overrides.is_empty() {
            summary.push_str(&format!("  Repository overrides: {}\n", self.repo_overrides.len()));
            for (repo_name, _) in &self.repo_overrides {
                summary.push_str(&format!("    - {}\n", repo_name));
            }
        }
        
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::workspace::config::{WorkspaceInfo, AppIntegrations, Repository};

    #[tokio::test]
    async fn test_config_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let config_manager = WorktreeConfigManager::new(config_path);
        
        // Should be able to create config manager with any path
        assert!(!config_manager.workspace_config_path.to_string_lossy().is_empty());
    }

    #[tokio::test]
    async fn test_config_loading_for_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        // Create a minimal workspace config
        let workspace_config = WorkspaceConfig {
            workspace: WorkspaceInfo {
                name: "test".to_string(),
                root: temp_dir.path().to_path_buf(),
                auto_discover: true,
            },
            repositories: vec![
                Repository {
                    name: "test-repo".to_string(),
                    path: temp_dir.path().join("test-repo"),
                    url: None,
                    branch: None,
                    apps: std::collections::HashMap::new(),
                    worktree_config: Some(RepositoryWorktreeConfig {
                        mode: None,
                        prefix: Some("custom-prefix/".to_string()),
                        base_dir: Some(PathBuf::from("/custom/path")),
                        cleanup: None,
                        merge_detection: None,
                        disabled: Some(false),
                    }),
                }
            ],
            groups: Vec::new(),
            apps: AppIntegrations {
                github: None,
                warp: None,
                iterm2: None,
                vscode: None,
                wezterm: None,
                cursor: None,
                windsurf: None,
            },
            preferences: None,
            claude_agents: None,
            worktree: WorktreeConfig::default(),
        };

        // Save the config
        workspace_config.save_to_file(&config_path).await.unwrap();

        let config_manager = WorktreeConfigManager::new(config_path);
        let repo_path = temp_dir.path().join("test-repo");
        
        let config = config_manager.load_config_for_repo(&repo_path).await.unwrap();
        
        // Should have repository-specific overrides
        assert_eq!(config.prefix, "custom-prefix/");
        assert_eq!(config.base_dir, PathBuf::from("/custom/path"));
    }

    #[tokio::test]
    async fn test_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let config_manager = WorktreeConfigManager::new(config_path);
        
        // Should return empty errors for default config
        let errors = config_manager.validate_all_configs().await.unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_summary() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let config_manager = WorktreeConfigManager::new(config_path);
        
        let summary = config_manager.get_config_summary().await.unwrap();
        
        // Should have basic summary information
        assert!(!summary.global_config.prefix.is_empty());
        assert_eq!(summary.total_repositories, 0);
        assert_eq!(summary.enabled_repositories, 0);
    }

    #[test]
    fn test_repository_config_merge() {
        let global = WorktreeConfig::default();
        let repo_config = RepositoryWorktreeConfig {
            mode: None,
            prefix: Some("custom-prefix/".to_string()),
            base_dir: Some(PathBuf::from("/custom/path")),
            cleanup: None,
            merge_detection: None,
            disabled: Some(false),
        };
        
        let merged = repo_config.merge_with_global(&global);
        
        assert_eq!(merged.prefix, "custom-prefix/");
        assert_eq!(merged.base_dir, PathBuf::from("/custom/path"));
        // Other settings should come from global
        assert_eq!(merged.auto_gitignore, global.auto_gitignore);
        assert_eq!(merged.default_editor, global.default_editor);
    }

    #[test]
    fn test_repository_config_enabled() {
        let mut repo_config = RepositoryWorktreeConfig::default();
        assert!(repo_config.is_enabled()); // Enabled by default
        
        repo_config.disabled = Some(true);
        assert!(!repo_config.is_enabled());
        
        repo_config.disabled = Some(false);
        assert!(repo_config.is_enabled());
    }
}