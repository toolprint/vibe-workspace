# Task 07: Configuration Integration

## Goal

Integrate worktree configuration with the existing vibe-workspace configuration system, ensuring seamless interoperability with existing workspace management, proper configuration persistence, and environment variable support. This task makes worktree management a first-class citizen within the vibe-workspace ecosystem.

## Scope

- Extend existing WorkspaceConfig to include worktree configuration
- Implement configuration loading, saving, and validation
- Add environment variable overrides for all worktree settings
- Integrate with existing workspace manager configuration patterns
- Provide configuration migration and upgrade paths
- Add configuration validation and error handling
- Support both global and repository-specific settings

## Implementation Details

### 1. Extend Existing Configuration Structure

Update `src/workspace/config.rs` to include worktree configuration:

```rust
// Add to the imports
use crate::worktree::config::{
    WorktreeConfig, WorktreeCleanupConfig, WorktreeMergeDetectionConfig, WorktreeStatusConfig
};

// Add to WorkspaceConfig struct (around line 17):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceInfo,
    pub repositories: Vec<Repository>,
    pub groups: Vec<RepositoryGroup>,
    pub apps: AppIntegrations,
    #[serde(default)]
    pub preferences: Option<Preferences>,
    #[serde(default)]
    pub claude_agents: Option<ClaudeAgentsIntegration>,
    #[serde(default)]
    pub worktree: WorktreeConfig,  // Add this field
}

// Update Repository struct to include per-repository worktree overrides:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub path: PathBuf,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub apps: HashMap<String, AppConfig>,
    #[serde(default)]
    pub worktree_config: Option<RepositoryWorktreeConfig>,  // Add this field
}

// Add repository-specific worktree configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryWorktreeConfig {
    /// Override global base directory for this repository
    pub base_dir: Option<PathBuf>,
    
    /// Override global prefix for this repository
    pub prefix: Option<String>,
    
    /// Repository-specific cleanup settings
    pub cleanup: Option<WorktreeCleanupConfig>,
    
    /// Repository-specific merge detection settings
    pub merge_detection: Option<WorktreeMergeDetectionConfig>,
    
    /// Disable worktree management for this repository
    pub disabled: Option<bool>,
}

impl RepositoryWorktreeConfig {
    /// Merge repository-specific config with global config
    pub fn merge_with_global(&self, global: &WorktreeConfig) -> WorktreeConfig {
        WorktreeConfig {
            base_dir: self.base_dir.clone().unwrap_or_else(|| global.base_dir.clone()),
            prefix: self.prefix.clone().unwrap_or_else(|| global.prefix.clone()),
            auto_gitignore: global.auto_gitignore, // Always use global setting
            default_editor: global.default_editor.clone(), // Always use global setting
            cleanup: self.cleanup.clone().unwrap_or_else(|| global.cleanup.clone()),
            merge_detection: self.merge_detection.clone()
                .unwrap_or_else(|| global.merge_detection.clone()),
            status: global.status.clone(), // Always use global status settings
        }
    }
    
    /// Check if worktree management is enabled for this repository
    pub fn is_enabled(&self) -> bool {
        !self.disabled.unwrap_or(false)
    }
}

impl Default for RepositoryWorktreeConfig {
    fn default() -> Self {
        Self {
            base_dir: None,
            prefix: None,
            cleanup: None,
            merge_detection: None,
            disabled: None,
        }
    }
}

impl WorkspaceConfig {
    /// Get effective worktree configuration for a specific repository
    pub fn get_worktree_config_for_repo(&self, repo_name: &str) -> WorktreeConfig {
        if let Some(repo) = self.repositories.iter().find(|r| r.name == repo_name) {
            if let Some(repo_config) = &repo.worktree_config {
                if repo_config.is_enabled() {
                    return repo_config.merge_with_global(&self.worktree);
                }
            }
        }
        
        // Return global config if no repository-specific overrides
        self.worktree.clone()
    }
    
    /// Check if worktree management is enabled for a repository
    pub fn is_worktree_enabled_for_repo(&self, repo_name: &str) -> bool {
        if let Some(repo) = self.repositories.iter().find(|r| r.name == repo_name) {
            if let Some(repo_config) = &repo.worktree_config {
                return repo_config.is_enabled();
            }
        }
        
        true // Enabled by default
    }
}
```

### 2. Enhanced Worktree Configuration

Update `src/worktree/config.rs` to support advanced configuration features:

```rust
// Add to existing WorktreeConfig implementation:
impl WorktreeConfig {
    /// Load configuration with environment variable overrides and validation
    pub fn load_with_overrides() -> Result<Self, String> {
        let mut config = Self::from_env();
        
        // Apply additional environment overrides
        if let Ok(auto_gitignore) = std::env::var("VIBE_WORKTREE_AUTO_GITIGNORE") {
            config.auto_gitignore = auto_gitignore.parse().unwrap_or(config.auto_gitignore);
        }
        
        // Override cleanup settings
        if let Ok(age_threshold) = std::env::var("VIBE_WORKTREE_AGE_THRESHOLD") {
            if let Ok(hours) = age_threshold.parse::<u64>() {
                config.cleanup.age_threshold_hours = hours;
            }
        }
        
        if let Ok(verify_remote) = std::env::var("VIBE_WORKTREE_VERIFY_REMOTE") {
            config.cleanup.verify_remote = verify_remote.parse().unwrap_or(config.cleanup.verify_remote);
        }
        
        if let Ok(auto_delete) = std::env::var("VIBE_WORKTREE_AUTO_DELETE_BRANCH") {
            config.cleanup.auto_delete_branch = auto_delete.parse().unwrap_or(config.cleanup.auto_delete_branch);
        }
        
        // Override merge detection settings
        if let Ok(use_github) = std::env::var("VIBE_WORKTREE_USE_GITHUB_CLI") {
            config.merge_detection.use_github_cli = use_github.parse().unwrap_or(config.merge_detection.use_github_cli);
        }
        
        if let Ok(methods) = std::env::var("VIBE_WORKTREE_MERGE_METHODS") {
            config.merge_detection.methods = methods
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
        
        if let Ok(main_branches) = std::env::var("VIBE_WORKTREE_MAIN_BRANCHES") {
            config.merge_detection.main_branches = main_branches
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
        
        // Override status settings
        if let Ok(show_files) = std::env::var("VIBE_WORKTREE_SHOW_FILES") {
            config.status.show_files = show_files.parse().unwrap_or(config.status.show_files);
        }
        
        if let Ok(max_files) = std::env::var("VIBE_WORKTREE_MAX_FILES_SHOWN") {
            if let Ok(count) = max_files.parse::<usize>() {
                config.status.max_files_shown = count;
            }
        }
        
        // Validate the final configuration
        config.validate()?;
        Ok(config)
    }
    
    /// Enhanced validation with more comprehensive checks
    pub fn validate(&self) -> Result<(), String> {
        // Basic validation (existing)
        if self.prefix.is_empty() {
            return Err("Worktree prefix cannot be empty".to_string());
        }
        
        if self.base_dir.to_string_lossy().is_empty() {
            return Err("Base directory cannot be empty".to_string());
        }
        
        if self.cleanup.age_threshold_hours == 0 {
            return Err("Age threshold must be greater than 0".to_string());
        }
        
        // Advanced validation
        if self.prefix.contains("..") || self.prefix.contains('\0') {
            return Err("Worktree prefix contains invalid characters".to_string());
        }
        
        if self.prefix.len() > 50 {
            return Err("Worktree prefix is too long (max 50 characters)".to_string());
        }
        
        if self.merge_detection.methods.is_empty() {
            return Err("At least one merge detection method must be configured".to_string());
        }
        
        if self.merge_detection.main_branches.is_empty() {
            return Err("At least one main branch must be configured".to_string());
        }
        
        if self.cleanup.age_threshold_hours > 24 * 365 {
            return Err("Age threshold is too high (max 1 year)".to_string());
        }
        
        if self.status.max_files_shown == 0 || self.status.max_files_shown > 100 {
            return Err("Max files shown must be between 1 and 100".to_string());
        }
        
        if self.status.max_commits_shown == 0 || self.status.max_commits_shown > 50 {
            return Err("Max commits shown must be between 1 and 50".to_string());
        }
        
        // Validate editor command
        if self.default_editor.is_empty() {
            return Err("Default editor cannot be empty".to_string());
        }
        
        // Validate base directory path
        if let Some(parent) = self.base_dir.parent() {
            if parent.to_string_lossy().is_empty() && !self.base_dir.is_absolute() {
                return Err("Relative base directory must have a parent".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Get configuration documentation for help system
    pub fn get_help_text() -> &'static str {
        r#"Worktree Configuration Options:

Environment Variables:
  VIBE_WORKTREE_BASE              Base directory for worktrees (default: .worktrees)
  VIBE_WORKTREE_PREFIX            Branch prefix for managed worktrees (default: vibe-ws/)
  VIBE_WORKTREE_EDITOR            Default editor command (default: code)
  VIBE_WORKTREE_AUTO_GITIGNORE    Auto-manage .gitignore (default: true)
  VIBE_WORKTREE_AGE_THRESHOLD     Minimum age in hours for cleanup (default: 24)
  VIBE_WORKTREE_VERIFY_REMOTE     Verify remote branch before cleanup (default: true)
  VIBE_WORKTREE_AUTO_DELETE_BRANCH Auto-delete branch after cleanup (default: false)
  VIBE_WORKTREE_USE_GITHUB_CLI    Use GitHub CLI for merge detection (default: true)
  VIBE_WORKTREE_MERGE_METHODS     Comma-separated merge detection methods
  VIBE_WORKTREE_MAIN_BRANCHES     Comma-separated main branch names
  VIBE_WORKTREE_SHOW_FILES        Show file lists in status (default: true)
  VIBE_WORKTREE_MAX_FILES_SHOWN   Max files to show in status (default: 10)

Configuration File:
  The worktree configuration is stored in ~/.toolprint/vibe-workspace/config.yaml
  under the 'worktree' section. Repository-specific overrides can be configured
  in the 'repositories[].worktree_config' section.
"#
    }
    
    /// Create a sample configuration for documentation
    pub fn sample_config_yaml() -> String {
        serde_yaml::to_string(&Self::default()).unwrap_or_else(|_| "# Error generating sample config".to_string())
    }
}

// Add environment variable documentation
pub const WORKTREE_ENV_VARS: &[(&str, &str, &str)] = &[
    ("VIBE_WORKTREE_BASE", ".worktrees", "Base directory for worktrees"),
    ("VIBE_WORKTREE_PREFIX", "vibe-ws/", "Branch prefix for managed worktrees"),
    ("VIBE_WORKTREE_EDITOR", "code", "Default editor command"),
    ("VIBE_WORKTREE_AUTO_GITIGNORE", "true", "Auto-manage .gitignore entries"),
    ("VIBE_WORKTREE_AGE_THRESHOLD", "24", "Minimum age in hours for cleanup eligibility"),
    ("VIBE_WORKTREE_VERIFY_REMOTE", "true", "Verify remote branch exists before cleanup"),
    ("VIBE_WORKTREE_AUTO_DELETE_BRANCH", "false", "Auto-delete branch after worktree removal"),
    ("VIBE_WORKTREE_USE_GITHUB_CLI", "true", "Use GitHub CLI for merge detection"),
    ("VIBE_WORKTREE_MERGE_METHODS", "standard,squash,github_pr", "Merge detection methods"),
    ("VIBE_WORKTREE_MAIN_BRANCHES", "main,master", "Main branches for merge detection"),
    ("VIBE_WORKTREE_SHOW_FILES", "true", "Show file lists in status output"),
    ("VIBE_WORKTREE_MAX_FILES_SHOWN", "10", "Maximum files to show in status"),
];
```

### 3. Configuration Management Integration

Create `src/worktree/config_manager.rs`:

```rust
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
        
        let repo_overrides = workspace_config.repositories
            .iter()
            .filter(|r| r.worktree_config.is_some())
            .map(|r| (r.name.clone(), r.worktree_config.as_ref().unwrap().clone()))
            .collect();
        
        Ok(ConfigSummary {
            global_config: workspace_config.worktree,
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

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            workspace: crate::workspace::config::WorkspaceInfo {
                name: "Default Workspace".to_string(),
                root: PathBuf::from("."),
                auto_discover: true,
            },
            repositories: Vec::new(),
            groups: Vec::new(),
            apps: crate::workspace::config::AppIntegrations {
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
            worktree: WorktreeConfig::default(), // Add default worktree config
        }
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
    pub repo_overrides: Vec<(String, RepositoryWorktreeConfig)>,
    pub total_repositories: usize,
    pub enabled_repositories: usize,
}

impl ConfigSummary {
    /// Generate a human-readable summary
    pub fn format_summary(&self) -> String {
        let mut summary = String::new();
        
        summary.push_str("Worktree Configuration Summary:\n");
        summary.push_str(&format!("  Global prefix: {}\n", self.global_config.prefix));
        summary.push_str(&format!("  Global base directory: {}\n", self.global_config.base_dir.display()));
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
```

### 4. Update WorktreeManager Integration

Update `src/worktree/manager.rs` to use configuration system:

```rust
// Add imports
use crate::worktree::config_manager::{WorktreeConfigManager, ConfigSummary};
use crate::workspace::WorkspaceManager;

// Update WorktreeManager
impl WorktreeManager {
    /// Create a new WorktreeManager with workspace integration
    pub async fn new_with_workspace_manager(
        workspace_manager: &WorkspaceManager,
        repo_path: Option<PathBuf>,
    ) -> Result<Self> {
        let config_manager = WorktreeConfigManager::new(
            workspace_manager.get_config_path().clone()
        );
        
        // Migrate legacy configuration if needed
        config_manager.migrate_legacy_config().await?;
        
        // Load configuration for specific repository or use global
        let config = if let Some(path) = repo_path {
            config_manager.load_config_for_repo(&path).await?
        } else {
            WorktreeConfig::load_with_overrides()?
        };
        
        let operations = WorktreeOperations::new(
            workspace_manager.get_workspace_root().clone(), 
            config.clone()
        );
        
        Ok(Self {
            operations,
            workspace_root: workspace_manager.get_workspace_root().clone(),
            config,
            config_manager: Some(config_manager),
        })
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
    pub async fn validate_configuration(&self) -> Result<Vec<crate::worktree::config_manager::ConfigValidationError>> {
        if let Some(config_manager) = &self.config_manager {
            config_manager.validate_all_configs().await
        } else {
            Ok(Vec::new())
        }
    }
}

// Add to WorktreeManager struct
pub struct WorktreeManager {
    operations: WorktreeOperations,
    workspace_root: PathBuf,
    config: WorktreeConfig,
    config_manager: Option<WorktreeConfigManager>,
}
```

### 5. Add Configuration CLI Commands

Add configuration management commands to the CLI:

```rust
// Add to WorktreeCommands enum in main.rs:
#[derive(Subcommand)]
enum WorktreeCommands {
    // ... existing commands ...
    
    /// Manage worktree configuration
    Config {
        #[command(subcommand)]
        action: WorktreeConfigCommands,
    },
}

#[derive(Subcommand)]
enum WorktreeConfigCommands {
    /// Show current configuration
    Show {
        /// Show configuration for specific repository
        #[arg(short, long)]
        repository: Option<String>,
        
        /// Output format: yaml, json, summary
        #[arg(short, long, default_value = "summary")]
        format: String,
    },
    
    /// Set configuration values
    Set {
        /// Configuration key (e.g., prefix, base_dir, cleanup.age_threshold_hours)
        key: String,
        
        /// Configuration value
        value: String,
        
        /// Apply to specific repository only
        #[arg(short, long)]
        repository: Option<String>,
    },
    
    /// Reset configuration to defaults
    Reset {
        /// Reset specific key only
        #[arg(short, long)]
        key: Option<String>,
        
        /// Reset configuration for specific repository
        #[arg(short, long)]
        repository: Option<String>,
    },
    
    /// Validate configuration
    Validate,
    
    /// Show configuration help and environment variables
    Help,
}
```

## Integration Points

### With Workspace Manager
- **Configuration Loading**: Uses existing workspace config file and patterns
- **Repository Management**: Integrates with existing repository discovery and management
- **Path Resolution**: Uses workspace root and repository paths from workspace manager

### With Environment Variables
- **Override System**: Comprehensive environment variable support for all settings
- **Development Workflow**: Easy configuration changes without editing files
- **CI/CD Integration**: Environment-based configuration for different environments

### With Existing Configuration
- **Backward Compatibility**: Supports migration from any existing worktree configurations
- **Validation**: Comprehensive validation with helpful error messages
- **Documentation**: Built-in help and documentation generation

## Success Criteria

### Configuration Loading
- [ ] Configuration loads from workspace config file correctly
- [ ] Environment variables override file-based settings
- [ ] Repository-specific overrides work as expected
- [ ] Default configuration is sensible and functional
- [ ] Configuration validation catches common errors

### Configuration Persistence
- [ ] Changes to configuration are saved correctly
- [ ] Repository-specific settings are preserved
- [ ] Configuration file format is human-readable
- [ ] Backup and migration functionality works

### Integration Requirements
- [ ] Seamless integration with existing workspace management
- [ ] No breaking changes to existing configuration
- [ ] Configuration changes are picked up without restart
- [ ] Error messages guide users to fix configuration issues

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod config_tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let config_manager = WorktreeConfigManager::new(config_path);
        let config = config_manager.load_config_for_repo(&temp_dir.path().join("test-repo")).await.unwrap();
        
        assert!(!config.prefix.is_empty());
        assert!(!config.base_dir.to_string_lossy().is_empty());
    }
    
    #[tokio::test]
    async fn test_environment_overrides() {
        std::env::set_var("VIBE_WORKTREE_PREFIX", "test-prefix/");
        std::env::set_var("VIBE_WORKTREE_AGE_THRESHOLD", "48");
        
        let config = WorktreeConfig::load_with_overrides().unwrap();
        
        assert_eq!(config.prefix, "test-prefix/");
        assert_eq!(config.cleanup.age_threshold_hours, 48);
        
        std::env::remove_var("VIBE_WORKTREE_PREFIX");
        std::env::remove_var("VIBE_WORKTREE_AGE_THRESHOLD");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = WorktreeConfig::default();
        
        // Valid configuration should pass
        assert!(config.validate().is_ok());
        
        // Invalid configuration should fail
        config.prefix = String::new();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_repository_config_merge() {
        let global = WorktreeConfig::default();
        let repo_config = RepositoryWorktreeConfig {
            prefix: Some("custom-prefix/".to_string()),
            base_dir: Some(PathBuf::from("/custom/path")),
            ..Default::default()
        };
        
        let merged = repo_config.merge_with_global(&global);
        
        assert_eq!(merged.prefix, "custom-prefix/");
        assert_eq!(merged.base_dir, PathBuf::from("/custom/path"));
        // Other settings should come from global
        assert_eq!(merged.auto_gitignore, global.auto_gitignore);
    }
}
```

### Integration Tests

Test configuration loading and saving with real workspace configurations.

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
serde_yaml = "0.9"
```

## Notes

- Configuration system maintains backward compatibility with existing setups
- Environment variables provide flexible override mechanism for development
- Repository-specific configuration allows fine-grained control
- Validation prevents common configuration errors
- Migration system handles upgrades gracefully

## Future Enhancements

- Configuration templates for common setups
- Dynamic configuration reloading without restart
- Configuration diffing and change tracking
- Integration with external configuration management systems
- Per-user configuration profiles

## Next Task

After completing this task, proceed to **Task 08: MCP Tools Integration** to expose worktree management functionality through the Model Context Protocol for AI-assisted development workflows.