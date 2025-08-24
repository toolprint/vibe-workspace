//! Git worktree management system
//!
//! This module provides comprehensive Git worktree management functionality
//! integrated with vibe-workspace, including creation, status tracking,
//! automated cleanup, and AI-assisted conflict resolution.

pub mod cache;
pub mod cleanup;
pub mod config;
pub mod config_manager;
pub mod manager;
pub mod merge_detection;
pub mod operations;
pub mod status;

// Re-export core types
pub use cache::{WorktreeStatusCache, CacheStats};
pub use cleanup::{CleanupOptions, CleanupReport, CleanupStrategy};
pub use config::WorktreeConfig;
pub use config_manager::{WorktreeConfigManager, ConfigValidationError, ConfigSummary};
pub use manager::WorktreeManager;
pub use merge_detection::{
    detect_worktree_merge_status, MergeDetectionMethod, MergeDetectionResult, 
    MergeDetector, MethodResult
};
pub use operations::{
    sanitize_branch_name, validate_branch_name, CreateOptions, RemoveOptions, WorktreeOperation,
    WorktreeOperations,
};
pub use status::{
    batch_update_worktree_status, check_worktree_activity, check_worktree_status, 
    check_worktree_status_with_config, get_branch_info, get_worktree_diff, update_worktree_info, 
    BranchInfo, CommitInfo, MergeInfo, StatusSeverity, WorktreeInfo, WorktreeStatus
};

use anyhow::Result;
use std::path::PathBuf;

/// Main entry point for worktree management
pub async fn create_worktree_manager(
    workspace_root: PathBuf,
    config: Option<WorktreeConfig>,
) -> Result<WorktreeManager> {
    WorktreeManager::new(workspace_root, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = WorktreeConfig::default();
        assert_eq!(config.prefix, "vibe-ws/");
        assert_eq!(config.base_dir, PathBuf::from(".worktrees"));
        assert!(config.auto_gitignore);
    }

    #[test]
    fn test_config_from_env() {
        env::set_var("VIBE_WORKTREE_PREFIX", "test/");
        env::set_var("VIBE_WORKTREE_BASE", "test-worktrees");

        let config = WorktreeConfig::from_env();
        assert_eq!(config.prefix, "test/");
        assert_eq!(config.base_dir, PathBuf::from("test-worktrees"));

        env::remove_var("VIBE_WORKTREE_PREFIX");
        env::remove_var("VIBE_WORKTREE_BASE");
    }

    #[test]
    fn test_config_validation() {
        let mut config = WorktreeConfig::default();
        assert!(config.validate().is_ok());

        config.prefix = String::new();
        assert!(config.validate().is_err());

        config.prefix = "test/".to_string();
        config.cleanup.age_threshold_hours = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_status_priority() {
        assert!(StatusSeverity::Warning.priority() < StatusSeverity::LightWarning.priority());
        assert!(StatusSeverity::LightWarning.priority() < StatusSeverity::Clean.priority());
    }

    #[test]
    fn test_status_description() {
        let mut status = WorktreeStatus::new();
        status.uncommitted_changes.push("file1.rs".to_string());
        status.untracked_files.push("file2.rs".to_string());

        let description = status.status_description();
        assert!(description.contains("1 uncommitted"));
        assert!(description.contains("1 untracked"));
    }

    #[test]
    fn test_status_icon() {
        let mut status = WorktreeStatus::new();
        status.severity = StatusSeverity::Clean;
        assert_eq!(status.status_icon(), "✅");

        status.severity = StatusSeverity::LightWarning;
        assert_eq!(status.status_icon(), "⚠️");

        status.severity = StatusSeverity::Warning;
        assert_eq!(status.status_icon(), "⚡");
    }

    #[test]
    fn test_cleanup_safe_detection() {
        let mut status = WorktreeStatus::new();
        assert!(!status.is_safe_to_cleanup());

        status.is_clean = true;
        status.uncommitted_changes.clear();
        status.untracked_files.clear();
        status.unpushed_commits.clear();
        assert!(status.is_safe_to_cleanup());

        // Test with unpushed commits but merged branch
        status.unpushed_commits.push(CommitInfo {
            id: "abc123".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: std::time::SystemTime::now(),
        });
        assert!(!status.is_safe_to_cleanup());

        status.merge_info = Some(MergeInfo {
            is_merged: true,
            detection_method: "standard".to_string(),
            details: None,
            confidence: 0.9,
        });
        assert!(status.is_safe_to_cleanup());
    }

    #[tokio::test]
    async fn test_worktree_manager_creation() {
        let workspace_root = PathBuf::from("/tmp/test-workspace");
        let manager = WorktreeManager::new(workspace_root.clone(), None).await;

        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.workspace_root, workspace_root);
    }

    #[tokio::test]
    async fn test_create_worktree_manager_helper() {
        let workspace_root = PathBuf::from("/tmp/test-workspace");
        let manager = create_worktree_manager(workspace_root.clone(), None).await;

        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.workspace_root, workspace_root);
    }

    #[test]
    fn test_worktree_operation_defaults() {
        let create_options = CreateOptions::default();
        assert!(!create_options.force);
        assert!(create_options.task_id.is_empty());
        assert!(create_options.base_branch.is_none());
        assert!(create_options.custom_path.is_none());

        let remove_options = RemoveOptions::default();
        assert!(!remove_options.force);
        assert!(!remove_options.delete_branch);
        assert!(remove_options.target.is_empty());
    }

    #[test]
    fn test_cleanup_defaults() {
        let cleanup_options = CleanupOptions::default();
        assert!(!cleanup_options.dry_run);
        assert_eq!(cleanup_options.min_age_hours, Some(24));
        assert!(!cleanup_options.force);
        assert!(matches!(
            cleanup_options.strategy,
            CleanupStrategy::Discard
        ));

        // CleanupReport doesn't have Default, so we create one manually for testing
        let cleanup_report = CleanupReport {
            total_evaluated: 0,
            cleaned_count: 0,
            skipped_count: 0,
            failed_count: 0,
            worktree_results: Vec::new(),
            strategy_used: CleanupStrategy::Discard,
            was_dry_run: false,
        };
        assert_eq!(cleanup_report.cleaned_count, 0);
        assert_eq!(cleanup_report.skipped_count, 0);
        assert_eq!(cleanup_report.failed_count, 0);
    }
}
