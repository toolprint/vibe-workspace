# Task 01: Core Module Structure and Types

## Goal

Create the foundational module structure and core types for the Git worktree management system. This task establishes the basic architecture, type definitions, and module organization that all subsequent tasks will build upon.

## Scope

- Create the module structure under `src/worktree/`
- Define core data types and structures
- Set up basic configuration structures
- Create module exports and public interface
- Ensure integration with existing vibe-workspace architecture

## Implementation Details

### 1. Create Module Structure

Create the following files under `src/worktree/`:

```
src/worktree/
├── mod.rs                  # Module exports and public interface
├── manager.rs              # WorktreeManager - main coordinator (stub)
├── operations.rs           # Core git worktree operations (stub)
├── status.rs               # Status checking and reporting (stub)
├── merge_detection.rs      # Advanced merge detection algorithms (stub)
├── cleanup.rs              # Cleanup strategies and safety mechanisms (stub)
└── config.rs               # Worktree-specific configuration
```

### 2. Core Type Definitions

#### In `src/worktree/mod.rs`:

```rust
//! Git worktree management system
//!
//! This module provides comprehensive Git worktree management functionality
//! integrated with vibe-workspace, including creation, status tracking,
//! automated cleanup, and AI-assisted conflict resolution.

pub mod config;
pub mod manager;
pub mod operations;
pub mod status;
pub mod merge_detection;
pub mod cleanup;

// Re-export core types
pub use config::{WorktreeConfig, WorktreeCleanupConfig, WorktreeMergeDetectionConfig, WorktreeStatusConfig};
pub use manager::WorktreeManager;
pub use status::{WorktreeInfo, WorktreeStatus, StatusSeverity, CommitInfo, RemoteStatus, MergeInfo};
pub use operations::{WorktreeOperation, CreateOptions, RemoveOptions};
pub use cleanup::{CleanupOptions, CleanupReport, CleanupStrategy};

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

/// Main entry point for worktree management
pub async fn create_worktree_manager(
    workspace_root: PathBuf,
    config: Option<WorktreeConfig>,
) -> Result<WorktreeManager> {
    WorktreeManager::new(workspace_root, config).await
}
```

#### In `src/worktree/status.rs`:

```rust
//! Worktree status tracking and reporting

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Comprehensive information about a Git worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    /// Path to the worktree directory
    pub path: PathBuf,
    
    /// Branch name associated with this worktree
    pub branch: String,
    
    /// Current HEAD commit SHA
    pub head: String,
    
    /// Detailed status information
    pub status: WorktreeStatus,
    
    /// Age of the worktree directory
    pub age: Duration,
    
    /// Whether this worktree is detached HEAD
    pub is_detached: bool,
}

/// Detailed status information for a worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatus {
    /// Overall cleanliness of the worktree
    pub is_clean: bool,
    
    /// Severity level for UI display
    pub severity: StatusSeverity,
    
    /// List of uncommitted changed files
    pub uncommitted_changes: Vec<String>,
    
    /// List of untracked files
    pub untracked_files: Vec<String>,
    
    /// List of unpushed commits
    pub unpushed_commits: Vec<CommitInfo>,
    
    /// Remote branch tracking status
    pub remote_status: RemoteStatus,
    
    /// Merge detection information
    pub merge_info: Option<MergeInfo>,
    
    /// Number of commits ahead of remote
    pub ahead_count: usize,
    
    /// Number of commits behind remote
    pub behind_count: usize,
}

/// Status severity levels for different types of issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusSeverity {
    /// ✅ No issues - clean worktree with everything synced
    Clean,
    
    /// ⚠️ Light warning - worktree issues (uncommitted/unsynced)
    LightWarning,
    
    /// ⚡ Warning - feature branch issues (stale, conflicts, etc.)
    Warning,
}

/// Information about a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit SHA (short form)
    pub id: String,
    
    /// Commit message (first line)
    pub message: String,
    
    /// Author name
    pub author: String,
    
    /// Commit timestamp
    pub timestamp: SystemTime,
}

/// Remote branch tracking status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteStatus {
    /// No remote tracking branch configured
    NoRemote,
    
    /// Remote branch exists and is up to date
    UpToDate,
    
    /// Local is ahead of remote
    Ahead(usize),
    
    /// Local is behind remote
    Behind(usize),
    
    /// Both ahead and behind (diverged)
    Diverged { ahead: usize, behind: usize },
    
    /// Remote branch was deleted
    RemoteDeleted,
}

/// Information about merge status detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeInfo {
    /// Whether the branch appears to be merged
    pub is_merged: bool,
    
    /// Method used to detect the merge
    pub detection_method: String,
    
    /// Additional information about the merge
    pub details: Option<String>,
    
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
}

impl WorktreeStatus {
    /// Create a new empty status
    pub fn new() -> Self {
        Self {
            is_clean: false,
            severity: StatusSeverity::Warning,
            uncommitted_changes: Vec::new(),
            untracked_files: Vec::new(),
            unpushed_commits: Vec::new(),
            remote_status: RemoteStatus::NoRemote,
            merge_info: None,
            ahead_count: 0,
            behind_count: 0,
        }
    }
    
    /// Check if this worktree is safe to clean up
    pub fn is_safe_to_cleanup(&self) -> bool {
        self.is_clean && 
        self.uncommitted_changes.is_empty() && 
        self.untracked_files.is_empty() &&
        (self.unpushed_commits.is_empty() || 
         self.merge_info.as_ref().map_or(false, |info| info.is_merged))
    }
    
    /// Get a user-friendly status description
    pub fn status_description(&self) -> String {
        if self.is_clean {
            match &self.merge_info {
                Some(info) if info.is_merged => format!("Clean ({})", info.detection_method),
                _ => "Clean".to_string(),
            }
        } else {
            let mut issues = Vec::new();
            
            if !self.uncommitted_changes.is_empty() {
                issues.push(format!("{} uncommitted", self.uncommitted_changes.len()));
            }
            
            if !self.untracked_files.is_empty() {
                issues.push(format!("{} untracked", self.untracked_files.len()));
            }
            
            if !self.unpushed_commits.is_empty() {
                issues.push(format!("{} unpushed", self.unpushed_commits.len()));
            }
            
            match &self.remote_status {
                RemoteStatus::NoRemote => issues.push("no remote".to_string()),
                RemoteStatus::Behind(count) => issues.push(format!("{} behind", count)),
                RemoteStatus::Diverged { ahead, behind } => {
                    issues.push(format!("{} ahead, {} behind", ahead, behind));
                }
                RemoteStatus::RemoteDeleted => issues.push("remote deleted".to_string()),
                _ => {}
            }
            
            if issues.is_empty() {
                "Unknown issue".to_string()
            } else {
                issues.join(", ")
            }
        }
    }
    
    /// Get the appropriate status icon
    pub fn status_icon(&self) -> &'static str {
        match self.severity {
            StatusSeverity::Clean => "✅",
            StatusSeverity::LightWarning => "⚠️",
            StatusSeverity::Warning => "⚡",
        }
    }
}

impl Default for WorktreeStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusSeverity {
    /// Get numeric priority for sorting (lower is more severe)
    pub fn priority(&self) -> u8 {
        match self {
            StatusSeverity::Warning => 0,
            StatusSeverity::LightWarning => 1,
            StatusSeverity::Clean => 2,
        }
    }
}
```

#### In `src/worktree/config.rs`:

```rust
//! Configuration structures for worktree management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for worktree management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Base directory for all worktrees (relative to repo root)
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
    /// Load configuration from environment variables, falling back to defaults
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
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
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.prefix.is_empty() {
            return Err("Worktree prefix cannot be empty".to_string());
        }
        
        if self.base_dir.to_string_lossy().is_empty() {
            return Err("Base directory cannot be empty".to_string());
        }
        
        if self.cleanup.age_threshold_hours == 0 {
            return Err("Age threshold must be greater than 0".to_string());
        }
        
        Ok(())
    }
}
```

#### Create stub files for other modules:

**`src/worktree/manager.rs`:**
```rust
//! WorktreeManager - main coordinator for worktree operations

use anyhow::Result;
use std::path::PathBuf;
use crate::worktree::config::WorktreeConfig;

/// Main coordinator for all worktree operations
pub struct WorktreeManager {
    pub workspace_root: PathBuf,
    pub config: WorktreeConfig,
}

impl WorktreeManager {
    /// Create a new WorktreeManager
    pub async fn new(
        workspace_root: PathBuf,
        config: Option<WorktreeConfig>,
    ) -> Result<Self> {
        let config = config.unwrap_or_default();
        config.validate().map_err(|e| anyhow::anyhow!("Invalid config: {}", e))?;
        
        Ok(Self {
            workspace_root,
            config,
        })
    }
}
```

**Create empty stub files for:**
- `src/worktree/operations.rs`
- `src/worktree/merge_detection.rs`
- `src/worktree/cleanup.rs`

Each should contain:
```rust
//! [Module description]

// TODO: Implementation in subsequent tasks
```

### 3. Integration with Main Module

Add to `src/lib.rs`:
```rust
pub mod worktree;
```

Or if using a different module structure, ensure the worktree module is properly exported.

### 4. Dependencies

Add any required dependencies to `Cargo.toml` if not already present:
```toml
[dependencies]
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Integration Points

### With Existing Architecture
- **WorkspaceManager**: The WorktreeManager should integrate with the existing WorkspaceManager for repository discovery and git operations
- **GitConfig**: Reuse existing GitConfig type from `src/git/mod.rs`
- **Configuration**: Worktree config should be embeddable within the main WorkspaceConfig structure
- **CLI**: Prepare for CLI integration by ensuring clean public interfaces

### Error Handling
- Use `anyhow::Result` consistently with existing codebase patterns
- Provide meaningful error messages with context
- Follow existing error handling conventions

## Success Criteria

### Functional Requirements
- [ ] All core types compile without errors
- [ ] Module structure is properly organized and exported
- [ ] Configuration types have sensible defaults
- [ ] Basic WorktreeManager can be instantiated
- [ ] Integration with existing module system works

### Code Quality
- [ ] All types implement appropriate traits (Debug, Clone, Serialize, Deserialize where needed)
- [ ] Documentation comments are comprehensive
- [ ] Code follows existing project conventions
- [ ] No compiler warnings

### Testing
- [ ] Unit tests for configuration validation
- [ ] Unit tests for status description generation
- [ ] Unit tests for severity priority ordering
- [ ] Default implementations work as expected

## Testing Requirements

Create `src/worktree/mod.rs` tests:

```rust
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
    fn test_status_priority() {
        assert!(StatusSeverity::Warning.priority() < StatusSeverity::LightWarning.priority());
        assert!(StatusSeverity::LightWarning.priority() < StatusSeverity::Clean.priority());
    }
    
    #[tokio::test]
    async fn test_worktree_manager_creation() {
        let workspace_root = PathBuf::from("/tmp/test-workspace");
        let manager = WorktreeManager::new(workspace_root.clone(), None).await;
        
        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.workspace_root, workspace_root);
    }
}
```

## Notes

- This task focuses on establishing the foundation - all operations will be implemented as stubs
- The design prioritizes type safety and clear interfaces
- Configuration is designed to be flexible and environment-aware
- Status types are rich enough to support the three-tier severity model
- All types are serializable to support MCP integration later

## Next Task

After completing this task, proceed to **Task 02: Git Operations Foundation** which will implement the actual git worktree operations using these type definitions.