//! Configuration structures for worktree management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Worktree storage mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeMode {
    /// Store worktrees locally within each repository (default)
    Local,
    /// Store worktrees globally in a central location
    Global,
}

impl Default for WorktreeMode {
    fn default() -> Self {
        Self::Local
    }
}

/// Configuration for worktree management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Worktree storage mode (local or global)
    #[serde(default)]
    pub mode: WorktreeMode,

    /// Base directory for worktrees
    /// - Local mode: relative to repo root (e.g., ".worktrees")
    /// - Global mode: absolute path or relative to workspace root
    pub base_dir: PathBuf,

    /// Default branch prefix for managed worktrees
    pub prefix: String,

    /// Automatically manage .gitignore for worktree directories
    pub auto_gitignore: bool,

    /// Default editor command for opening worktrees
    pub default_editor: String,

    /// Cleanup configuration
    pub cleanup: WorktreeCleanupConfig,

    /// Merge detection configuration
    pub merge_detection: WorktreeMergeDetectionConfig,

    /// Status display configuration
    pub status: WorktreeStatusConfig,
}

/// Configuration for cleanup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeCleanupConfig {
    /// Minimum age (hours) before worktree can be cleaned
    pub age_threshold_hours: u64,

    /// Verify remote branch exists before cleanup
    pub verify_remote: bool,

    /// Automatically delete branch after worktree removal
    pub auto_delete_branch: bool,

    /// Require confirmation for bulk operations
    pub require_confirmation: bool,
}

/// Configuration for merge detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeMergeDetectionConfig {
    /// Enable GitHub CLI integration for PR status
    pub use_github_cli: bool,

    /// Methods to use for merge detection (in order of preference)
    pub methods: Vec<String>,

    /// Main branches to check merges against
    pub main_branches: Vec<String>,
}

/// Configuration for status display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatusConfig {
    /// Show file lists in status output
    pub show_files: bool,

    /// Maximum number of files to display
    pub max_files_shown: usize,

    /// Show commit messages for unpushed commits
    pub show_commit_messages: bool,

    /// Maximum number of commits to display
    pub max_commits_shown: usize,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            mode: WorktreeMode::default(),
            base_dir: PathBuf::from(".worktrees"),
            prefix: "vibe-ws/".to_string(),
            auto_gitignore: true,
            default_editor: "code".to_string(),
            cleanup: WorktreeCleanupConfig::default(),
            merge_detection: WorktreeMergeDetectionConfig::default(),
            status: WorktreeStatusConfig::default(),
        }
    }
}

impl Default for WorktreeCleanupConfig {
    fn default() -> Self {
        Self {
            age_threshold_hours: 24,
            verify_remote: true,
            auto_delete_branch: false,
            require_confirmation: true,
        }
    }
}

impl Default for WorktreeMergeDetectionConfig {
    fn default() -> Self {
        Self {
            use_github_cli: true,
            methods: vec![
                "standard".to_string(),
                "squash".to_string(),
                "github_pr".to_string(),
                "file_content".to_string(),
            ],
            main_branches: vec!["main".to_string(), "master".to_string()],
        }
    }
}

impl Default for WorktreeStatusConfig {
    fn default() -> Self {
        Self {
            show_files: true,
            max_files_shown: 10,
            show_commit_messages: true,
            max_commits_shown: 5,
        }
    }
}

impl WorktreeConfig {
    /// Get the resolved base directory based on mode and configuration
    pub fn get_resolved_base_dir(&self, repo_root: Option<&std::path::Path>) -> PathBuf {
        match self.mode {
            WorktreeMode::Local => {
                if self.base_dir.is_absolute() {
                    self.base_dir.clone()
                } else if let Some(root) = repo_root {
                    root.join(&self.base_dir)
                } else {
                    self.base_dir.clone() // Fallback to relative path
                }
            }
            WorktreeMode::Global => {
                if self.base_dir.is_absolute() {
                    self.base_dir.clone()
                } else {
                    // Resolve to global location (matching operations.rs logic)
                    if let Some(home) = dirs::home_dir() {
                        home.join(".toolprint").join("vibe-workspace").join("worktrees")
                    } else {
                        std::env::temp_dir().join("vibe-worktrees")
                    }
                }
            }
        }
    }

    /// Load configuration from environment variables, falling back to defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(mode) = std::env::var("VIBE_WORKTREE_MODE") {
            config.mode = match mode.to_lowercase().as_str() {
                "global" => WorktreeMode::Global,
                "local" => WorktreeMode::Local,
                _ => WorktreeMode::Local,
            };
        }

        if let Ok(base_dir) = std::env::var("VIBE_WORKTREE_BASE") {
            config.base_dir = PathBuf::from(base_dir);
        }

        if let Ok(prefix) = std::env::var("VIBE_WORKTREE_PREFIX") {
            config.prefix = prefix;
        }

        if let Ok(editor) = std::env::var("VIBE_WORKTREE_EDITOR") {
            config.default_editor = editor;
        }

        config
    }

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
        
        // Basic validation of base directory - just check it's not completely empty
        if self.base_dir.to_string_lossy().trim().is_empty() {
            return Err("Base directory cannot be empty or whitespace".to_string());
        }
        
        Ok(())
    }
    
    /// Get configuration documentation for help system
    pub fn get_help_text() -> &'static str {
        r#"Worktree Configuration Options:

Environment Variables:
  VIBE_WORKTREE_MODE              Storage mode: local or global (default: local)
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

/// Environment variable documentation
pub const WORKTREE_ENV_VARS: &[(&str, &str, &str)] = &[
    ("VIBE_WORKTREE_MODE", "local", "Worktree storage mode (local or global)"),
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

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_enhanced_validation() {
        let mut config = WorktreeConfig::default();
        
        // Valid configuration should pass
        let result = config.validate();
        if let Err(err) = &result {
            eprintln!("Default config validation failed: {}", err);
        }
        assert!(result.is_ok());
        
        // Test prefix validation
        config.prefix = "..".to_string();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("invalid characters"));
        
        config.prefix = "x".repeat(60); // Too long
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("too long"));
        
        // Reset to valid
        config.prefix = "test/".to_string();
        assert!(config.validate().is_ok());
        
        // Test empty methods
        config.merge_detection.methods.clear();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("merge detection method"));
        
        // Test empty main branches
        config.merge_detection.methods = vec!["standard".to_string()];
        config.merge_detection.main_branches.clear();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("main branch"));
        
        // Test file limits
        config.merge_detection.main_branches = vec!["main".to_string()];
        config.status.max_files_shown = 0;
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("Max files shown"));
        
        config.status.max_files_shown = 200;
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("Max files shown"));
    }
    
    #[test]
    fn test_load_with_overrides() {
        // Set environment variables with valid values
        env::set_var("VIBE_WORKTREE_PREFIX", "env-prefix/");
        env::set_var("VIBE_WORKTREE_BASE", "/tmp/worktrees");
        env::set_var("VIBE_WORKTREE_AGE_THRESHOLD", "48");
        env::set_var("VIBE_WORKTREE_AUTO_GITIGNORE", "false");
        env::set_var("VIBE_WORKTREE_MERGE_METHODS", "standard,custom");
        env::set_var("VIBE_WORKTREE_MAIN_BRANCHES", "main,dev");
        env::set_var("VIBE_WORKTREE_MAX_FILES_SHOWN", "20");
        
        let config = WorktreeConfig::load_with_overrides().unwrap();
        
        assert_eq!(config.prefix, "env-prefix/");
        assert_eq!(config.base_dir, PathBuf::from("/tmp/worktrees"));
        assert_eq!(config.cleanup.age_threshold_hours, 48);
        assert_eq!(config.auto_gitignore, false);
        assert_eq!(config.merge_detection.methods, vec!["standard", "custom"]);
        assert_eq!(config.merge_detection.main_branches, vec!["main", "dev"]);
        assert_eq!(config.status.max_files_shown, 20);
        
        // Clean up
        env::remove_var("VIBE_WORKTREE_PREFIX");
        env::remove_var("VIBE_WORKTREE_BASE");
        env::remove_var("VIBE_WORKTREE_AGE_THRESHOLD");
        env::remove_var("VIBE_WORKTREE_AUTO_GITIGNORE");
        env::remove_var("VIBE_WORKTREE_MERGE_METHODS");
        env::remove_var("VIBE_WORKTREE_MAIN_BRANCHES");
        env::remove_var("VIBE_WORKTREE_MAX_FILES_SHOWN");
    }
    
    #[test]
    fn test_sample_config_generation() {
        let yaml = WorktreeConfig::sample_config_yaml();
        assert!(!yaml.is_empty());
        assert!(yaml.contains("prefix"));
        assert!(yaml.contains("base_dir"));
    }
    
    #[test]
    fn test_help_text() {
        let help = WorktreeConfig::get_help_text();
        assert!(!help.is_empty());
        assert!(help.contains("Environment Variables"));
        assert!(help.contains("VIBE_WORKTREE_PREFIX"));
        assert!(help.contains("Configuration File"));
    }
    
    #[test]
    fn test_environment_variable_documentation() {
        // Test that all documented environment variables are valid
        for (env_var, default_value, description) in WORKTREE_ENV_VARS {
            assert!(!env_var.is_empty());
            assert!(!default_value.is_empty());
            assert!(!description.is_empty());
            assert!(env_var.starts_with("VIBE_WORKTREE_"));
        }
        
        assert!(WORKTREE_ENV_VARS.len() > 10); // Should have many env vars documented
    }
    
    #[test]
    fn test_worktree_mode() {
        // Test default mode
        let config = WorktreeConfig::default();
        assert_eq!(config.mode, WorktreeMode::Local);
        
        // Test mode serialization/deserialization
        let local_config = WorktreeConfig {
            mode: WorktreeMode::Local,
            ..Default::default()
        };
        
        let global_config = WorktreeConfig {
            mode: WorktreeMode::Global,
            ..Default::default()
        };
        
        // Test serialization
        let local_yaml = serde_yaml::to_string(&local_config).unwrap();
        let global_yaml = serde_yaml::to_string(&global_config).unwrap();
        
        assert!(local_yaml.contains("mode: local"));
        assert!(global_yaml.contains("mode: global"));
        
        // Test deserialization
        let deserialized_local: WorktreeConfig = serde_yaml::from_str(&local_yaml).unwrap();
        let deserialized_global: WorktreeConfig = serde_yaml::from_str(&global_yaml).unwrap();
        
        assert_eq!(deserialized_local.mode, WorktreeMode::Local);
        assert_eq!(deserialized_global.mode, WorktreeMode::Global);
    }
    
    #[test]
    fn test_environment_variable_mode_override() {
        use std::env;
        
        // Test local mode
        env::set_var("VIBE_WORKTREE_MODE", "local");
        let config = WorktreeConfig::from_env();
        assert_eq!(config.mode, WorktreeMode::Local);
        
        // Test global mode
        env::set_var("VIBE_WORKTREE_MODE", "global");
        let config = WorktreeConfig::from_env();
        assert_eq!(config.mode, WorktreeMode::Global);
        
        // Test invalid mode defaults to local
        env::set_var("VIBE_WORKTREE_MODE", "invalid");
        let config = WorktreeConfig::from_env();
        assert_eq!(config.mode, WorktreeMode::Local);
        
        // Clean up
        env::remove_var("VIBE_WORKTREE_MODE");
    }
}
