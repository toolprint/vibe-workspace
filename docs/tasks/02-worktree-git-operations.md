# Task 02: Git Operations Foundation

## Goal

Implement the core Git worktree operations including creation, removal, and listing of worktrees. This task provides the fundamental git operations layer that all higher-level functionality will build upon, with proper path management, branch validation, and .gitignore handling.

## Scope

- Implement basic git worktree commands (create, remove, list)
- Branch name validation and sanitization
- Worktree path resolution and management
- Automatic .gitignore management for worktrees within repositories
- Integration with existing git command execution patterns
- Error handling and safety validation

## Implementation Details

### 1. Implement `src/worktree/operations.rs`

```rust
//! Core Git worktree operations

use anyhow::{Context, Result, bail};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::worktree::config::WorktreeConfig;
use crate::worktree::status::WorktreeInfo;

/// Options for creating a new worktree
#[derive(Debug, Clone)]
pub struct CreateOptions {
    /// Task identifier to generate branch name
    pub task_id: String,
    
    /// Base branch to create worktree from (default: current branch)
    pub base_branch: Option<String>,
    
    /// Force creation even if branch exists
    pub force: bool,
    
    /// Custom worktree path (overrides default path calculation)
    pub custom_path: Option<PathBuf>,
}

/// Options for removing a worktree
#[derive(Debug, Clone)]
pub struct RemoveOptions {
    /// Branch name or worktree path to remove
    pub target: String,
    
    /// Force removal even with uncommitted changes
    pub force: bool,
    
    /// Also delete the branch after removing worktree
    pub delete_branch: bool,
}

/// Git worktree operation types
#[derive(Debug, Clone)]
pub enum WorktreeOperation {
    Create(CreateOptions),
    Remove(RemoveOptions),
    List,
    Status(String),
}

/// Core worktree operations implementation
pub struct WorktreeOperations {
    repo_root: PathBuf,
    config: WorktreeConfig,
}

impl WorktreeOperations {
    /// Create new operations instance
    pub fn new(repo_root: PathBuf, config: WorktreeConfig) -> Self {
        Self { repo_root, config }
    }
    
    /// Create a new git worktree
    pub async fn create_worktree(&self, options: CreateOptions) -> Result<WorktreeInfo> {
        // Validate and sanitize the task ID
        let sanitized_task_id = sanitize_branch_name(&options.task_id)?;
        let branch_name = format!("{}{}", self.config.prefix, sanitized_task_id);
        
        // Validate branch name
        validate_branch_name(&branch_name)?;
        
        // Calculate worktree path
        let worktree_path = match options.custom_path {
            Some(custom) => custom,
            None => self.calculate_worktree_path(&sanitized_task_id)?,
        };
        
        // Ensure base directory exists
        self.ensure_base_directory_exists().await?;
        
        // Update .gitignore if needed
        if self.config.auto_gitignore {
            self.update_gitignore().await?;
        }
        
        // Check if branch already exists
        let branch_exists = self.branch_exists(&branch_name).await?;
        if branch_exists && !options.force {
            bail!("Branch '{}' already exists. Use --force to recreate.", branch_name);
        }
        
        // Create the worktree
        let result = if branch_exists && options.force {
            // Remove existing worktree first if it exists
            if let Ok(existing_path) = self.find_worktree_path(&branch_name).await {
                warn!("Removing existing worktree at: {}", existing_path.display());
                self.execute_git_command(&["worktree", "remove", "--force", &existing_path.to_string_lossy()]).await?;
            }
            
            // Remove and recreate branch
            self.execute_git_command(&["branch", "-D", &branch_name]).await.ok(); // Ignore errors
            self.create_branch_and_worktree(&branch_name, &worktree_path, options.base_branch.as_deref()).await?
        } else {
            self.create_branch_and_worktree(&branch_name, &worktree_path, options.base_branch.as_deref()).await?
        };
        
        debug!("Created worktree: {} -> {}", branch_name, worktree_path.display());
        
        // Return worktree info
        Ok(WorktreeInfo {
            path: worktree_path,
            branch: branch_name,
            head: result.head,
            status: Default::default(), // Will be filled by status tracking
            age: std::time::Duration::from_secs(0),
            is_detached: false,
        })
    }
    
    /// Remove a git worktree
    pub async fn remove_worktree(&self, options: RemoveOptions) -> Result<()> {
        // Find the worktree path
        let worktree_path = if options.target.contains('/') {
            // Treat as path
            PathBuf::from(&options.target)
        } else {
            // Treat as branch name, find corresponding path
            self.find_worktree_path(&options.target).await?
        };
        
        // Validate worktree exists
        if !worktree_path.exists() {
            bail!("Worktree path does not exist: {}", worktree_path.display());
        }
        
        // Safety check: ensure it's actually a worktree
        if !self.is_valid_worktree(&worktree_path).await? {
            bail!("Path is not a valid git worktree: {}", worktree_path.display());
        }
        
        // Remove the worktree
        let mut args = vec!["worktree", "remove"];
        if options.force {
            args.push("--force");
        }
        args.push(&worktree_path.to_string_lossy());
        
        self.execute_git_command(&args).await?;
        
        // Delete branch if requested
        if options.delete_branch {
            // Extract branch name from target or find it
            let branch_name = if options.target.contains('/') {
                self.get_worktree_branch(&worktree_path).await?
            } else {
                options.target.clone()
            };
            
            self.execute_git_command(&["branch", "-D", &branch_name]).await?;
            debug!("Deleted branch: {}", branch_name);
        }
        
        debug!("Removed worktree: {}", worktree_path.display());
        Ok(())
    }
    
    /// List all git worktrees
    pub async fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self.execute_git_command(&["worktree", "list", "--porcelain"]).await?;
        self.parse_worktree_list(&output).await
    }
    
    /// Find git repository root
    pub async fn find_git_root(&self) -> Result<PathBuf> {
        let output = self.execute_git_command(&["rev-parse", "--show-toplevel"]).await?;
        Ok(PathBuf::from(output.trim()))
    }
    
    // Private implementation methods
    
    async fn create_branch_and_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        base_branch: Option<&str>,
    ) -> Result<CreateResult> {
        let base = base_branch.unwrap_or("HEAD");
        
        let output = self.execute_git_command(&[
            "worktree", "add", "-b", branch_name,
            &worktree_path.to_string_lossy(),
            base,
        ]).await?;
        
        // Get the HEAD commit
        let head = self.execute_git_command(&["rev-parse", "HEAD"]).await?
            .trim().to_string();
        
        Ok(CreateResult { head })
    }
    
    async fn calculate_worktree_path(&self, task_id: &str) -> Result<PathBuf> {
        let base_path = if self.config.base_dir.is_absolute() {
            self.config.base_dir.clone()
        } else {
            self.repo_root.join(&self.config.base_dir)
        };
        
        // Handle task IDs with slashes (e.g., "feat/new-ui" -> "feat/new-ui")
        let path_segments: Vec<&str> = task_id.split('/').collect();
        let mut worktree_path = base_path;
        
        for segment in path_segments {
            worktree_path = worktree_path.join(segment);
        }
        
        // Add timestamp suffix to ensure uniqueness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let final_name = format!("{}_{:x}", 
            worktree_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("worktree"),
            timestamp);
        
        Ok(worktree_path.parent().unwrap_or(&worktree_path).join(final_name))
    }
    
    async fn ensure_base_directory_exists(&self) -> Result<()> {
        let base_path = if self.config.base_dir.is_absolute() {
            self.config.base_dir.clone()
        } else {
            self.repo_root.join(&self.config.base_dir)
        };
        
        if !base_path.exists() {
            fs::create_dir_all(&base_path)
                .with_context(|| format!("Failed to create base directory: {}", base_path.display()))?;
        }
        
        Ok(())
    }
    
    async fn update_gitignore(&self) -> Result<()> {
        // Only update .gitignore if worktrees are within the repository
        let base_path = if self.config.base_dir.is_absolute() {
            return Ok(()); // External worktrees don't need .gitignore
        } else {
            self.config.base_dir.clone()
        };
        
        let gitignore_path = self.repo_root.join(".gitignore");
        let ignore_pattern = format!("{}/", base_path.display());
        
        // Check if pattern already exists
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            if content.lines().any(|line| line.trim() == ignore_pattern.trim()) {
                return Ok(()); // Already present
            }
        }
        
        // Append the ignore pattern
        let mut content = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };
        
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        
        content.push_str(&format!("# Vibe worktree directories\n{}\n", ignore_pattern));
        
        fs::write(&gitignore_path, content)
            .with_context(|| format!("Failed to update .gitignore at: {}", gitignore_path.display()))?;
        
        debug!("Updated .gitignore with pattern: {}", ignore_pattern);
        Ok(())
    }
    
    async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let result = self.execute_git_command(&["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch_name)]).await;
        Ok(result.is_ok())
    }
    
    async fn find_worktree_path(&self, branch_name: &str) -> Result<PathBuf> {
        let worktrees = self.list_worktrees().await?;
        for worktree in worktrees {
            if worktree.branch == branch_name {
                return Ok(worktree.path);
            }
        }
        bail!("No worktree found for branch: {}", branch_name);
    }
    
    async fn is_valid_worktree(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }
        
        // Check if git recognizes this as a worktree
        let result = Command::new("git")
            .args(&["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()
            .await?;
            
        Ok(result.status.success())
    }
    
    async fn get_worktree_branch(&self, worktree_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if !output.status.success() {
            bail!("Failed to get branch name for worktree: {}", worktree_path.display());
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
    
    async fn parse_worktree_list(&self, output: &str) -> Result<Vec<WorktreeInfo>> {
        let mut worktrees = Vec::new();
        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            if line.starts_with("worktree ") {
                let path = PathBuf::from(line.strip_prefix("worktree ").unwrap());
                let mut branch = String::new();
                let mut head = String::new();
                let mut is_detached = false;
                
                i += 1;
                while i < lines.len() && !lines[i].starts_with("worktree ") {
                    let info_line = lines[i];
                    if info_line.starts_with("HEAD ") {
                        head = info_line.strip_prefix("HEAD ").unwrap().to_string();
                    } else if info_line.starts_with("branch ") {
                        let branch_ref = info_line.strip_prefix("branch ").unwrap();
                        branch = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref).to_string();
                    } else if info_line == "detached" {
                        is_detached = true;
                        branch = "(detached)".to_string();
                    } else if info_line == "bare" {
                        branch = "(bare)".to_string();
                    }
                    i += 1;
                }
                
                // Calculate age
                let age = if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(created) = metadata.created() {
                        std::time::SystemTime::now().duration_since(created).unwrap_or_default()
                    } else {
                        std::time::Duration::from_secs(0)
                    }
                } else {
                    std::time::Duration::from_secs(0)
                };
                
                worktrees.push(WorktreeInfo {
                    path,
                    branch,
                    head,
                    status: Default::default(),
                    age,
                    is_detached,
                });
            } else {
                i += 1;
            }
        }
        
        Ok(worktrees)
    }
    
    async fn execute_git_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .await
            .with_context(|| format!("Failed to execute git command: git {}", args.join(" ")))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Git command failed: git {}\nError: {}", args.join(" "), stderr);
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[derive(Debug)]
struct CreateResult {
    head: String,
}

/// Validate a Git branch name for security and compatibility
pub fn validate_branch_name(branch_name: &str) -> Result<()> {
    if branch_name.is_empty() {
        bail!("Branch name cannot be empty");
    }
    
    // Security: Check for dangerous characters that could lead to command injection
    let dangerous_chars = ['$', '`', '(', ')', '{', '}', '|', '&', ';', '<', '>', '\n', '\r', '\0', '"', '\'', '\\'];
    if branch_name.chars().any(|c| dangerous_chars.contains(&c)) {
        bail!("Branch name contains invalid characters");
    }
    
    // Git branch name validation
    if branch_name.starts_with('.') || branch_name.ends_with('.') {
        bail!("Branch name cannot start or end with a dot");
    }
    
    if branch_name.starts_with('/') || branch_name.ends_with('/') {
        bail!("Branch name cannot start or end with a slash");
    }
    
    if branch_name.contains("..") {
        bail!("Branch name cannot contain consecutive dots");
    }
    
    if branch_name.contains("@{") {
        bail!("Branch name cannot contain '@{{' sequence");
    }
    
    // Length validation
    if branch_name.len() > 255 {
        bail!("Branch name too long (max 255 characters)");
    }
    
    Ok(())
}

/// Sanitize a task ID to create a valid Git branch name
pub fn sanitize_branch_name(name: &str) -> Result<String> {
    if name.is_empty() {
        bail!("Task ID cannot be empty");
    }
    
    // Replace invalid characters with hyphens
    let re = Regex::new(r"[^a-zA-Z0-9\-_/]")?;
    let sanitized = re.replace_all(name, "-").to_string();
    
    // Remove multiple consecutive hyphens
    let re = Regex::new(r"-+")?;
    let sanitized = re.replace_all(&sanitized, "-").to_string();
    
    // Trim leading/trailing hyphens and slashes
    let sanitized = sanitized.trim_matches('-').trim_matches('/');
    
    if sanitized.is_empty() {
        bail!("Task ID '{}' cannot be sanitized to a valid branch name", name);
    }
    
    Ok(sanitized.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_branch_name() {
        // Valid names
        assert!(validate_branch_name("feature/new-ui").is_ok());
        assert!(validate_branch_name("vibe-ws/task-123").is_ok());
        assert!(validate_branch_name("main").is_ok());
        
        // Invalid names
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name(".hidden").is_err());
        assert!(validate_branch_name("branch.").is_err());
        assert!(validate_branch_name("/branch").is_err());
        assert!(validate_branch_name("branch/").is_err());
        assert!(validate_branch_name("branch..name").is_err());
        assert!(validate_branch_name("branch@{upstream}").is_err());
        assert!(validate_branch_name("branch$injection").is_err());
        assert!(validate_branch_name("branch`command`").is_err());
    }
    
    #[test]
    fn test_sanitize_branch_name() {
        // Basic sanitization
        assert_eq!(sanitize_branch_name("Task 123").unwrap(), "Task-123");
        assert_eq!(sanitize_branch_name("feat/new-ui").unwrap(), "feat/new-ui");
        assert_eq!(sanitize_branch_name("Fix: issue #456").unwrap(), "Fix-issue-456");
        
        // Multiple consecutive characters
        assert_eq!(sanitize_branch_name("task   with   spaces").unwrap(), "task-with-spaces");
        assert_eq!(sanitize_branch_name("task---dashes").unwrap(), "task-dashes");
        
        // Edge cases
        assert!(sanitize_branch_name("").is_err());
        assert!(sanitize_branch_name("!!!").is_err());
        assert!(sanitize_branch_name("---").is_err());
    }
}
```

### 2. Update `src/worktree/manager.rs`

```rust
//! WorktreeManager - main coordinator for worktree operations

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use crate::worktree::config::WorktreeConfig;
use crate::worktree::operations::{WorktreeOperations, CreateOptions, RemoveOptions};
use crate::worktree::status::WorktreeInfo;

/// Main coordinator for all worktree operations
pub struct WorktreeManager {
    operations: WorktreeOperations,
    workspace_root: PathBuf,
}

impl WorktreeManager {
    /// Create a new WorktreeManager
    pub async fn new(
        workspace_root: PathBuf,
        config: Option<WorktreeConfig>,
    ) -> Result<Self> {
        let config = config.unwrap_or_default();
        config.validate().map_err(|e| anyhow::anyhow!("Invalid config: {}", e))?;
        
        let operations = WorktreeOperations::new(workspace_root.clone(), config);
        
        Ok(Self {
            operations,
            workspace_root,
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
    pub async fn create_worktree_with_options(&self, options: CreateOptions) -> Result<WorktreeInfo> {
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
}
```

### 3. Update `src/worktree/mod.rs`

Add the new operations types to the re-exports:

```rust
pub use operations::{WorktreeOperations, CreateOptions, RemoveOptions, validate_branch_name, sanitize_branch_name};
```

## Integration Points

### With Existing Git Operations
- **Command Execution**: Uses the same pattern as existing git operations with `tokio::process::Command`
- **Error Handling**: Follows `anyhow::Result` patterns used throughout the codebase
- **Working Directory**: Respects repository root context like existing operations

### With Workspace Manager
- **Repository Discovery**: Can be integrated with existing repository discovery
- **Path Management**: Works with existing path resolution patterns
- **Configuration**: Ready to be embedded in main workspace configuration

### Security Considerations
- **Input Validation**: Comprehensive branch name validation prevents injection attacks
- **Path Safety**: Validates worktree paths to prevent directory traversal
- **Git Command Safety**: All git commands are constructed safely with validated inputs

## Success Criteria

### Functional Requirements
- [ ] Can create new worktrees with proper branch creation
- [ ] Can remove worktrees safely with validation
- [ ] Can list all existing worktrees with metadata
- [ ] Properly handles worktree paths with subdirectories (feat/new-ui)
- [ ] Automatically manages .gitignore for worktrees within repository
- [ ] Validates and sanitizes branch names securely
- [ ] Integrates with existing git command execution patterns

### Safety Requirements
- [ ] Never creates worktrees with invalid git branch names
- [ ] Prevents command injection via task IDs
- [ ] Validates worktree paths before operations
- [ ] Provides clear error messages for failure cases
- [ ] Handles edge cases (missing directories, permission issues)

### Performance Requirements
- [ ] Operations complete in reasonable time
- [ ] Handles concurrent operations safely
- [ ] Efficient parsing of git worktree list output
- [ ] Minimal filesystem operations for path resolution

## Testing Requirements

### Unit Tests
Create comprehensive tests in `src/worktree/operations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;
    
    async fn setup_test_repo() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();
        
        // Initialize git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .status()
            .await?;
            
        // Set up git config
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .status()
            .await?;
            
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .status()
            .await?;
            
        // Create initial commit
        Command::new("git")
            .args(&["commit", "--allow-empty", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .status()
            .await?;
        
        Ok((temp_dir, repo_path))
    }
    
    #[tokio::test]
    async fn test_create_worktree() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await?;
        let config = WorktreeConfig::default();
        let ops = WorktreeOperations::new(repo_path, config);
        
        let options = CreateOptions {
            task_id: "test-feature".to_string(),
            base_branch: None,
            force: false,
            custom_path: None,
        };
        
        let worktree_info = ops.create_worktree(options).await?;
        
        assert!(worktree_info.path.exists());
        assert!(worktree_info.branch.starts_with("vibe-ws/"));
        assert!(worktree_info.branch.contains("test-feature"));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_list_worktrees() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await?;
        let config = WorktreeConfig::default();
        let ops = WorktreeOperations::new(repo_path, config);
        
        // Should have at least the main worktree
        let worktrees = ops.list_worktrees().await?;
        assert!(!worktrees.is_empty());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_remove_worktree() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await?;
        let config = WorktreeConfig::default();
        let ops = WorktreeOperations::new(repo_path, config);
        
        // Create a worktree first
        let create_options = CreateOptions {
            task_id: "test-remove".to_string(),
            base_branch: None,
            force: false,
            custom_path: None,
        };
        
        let worktree_info = ops.create_worktree(create_options).await?;
        assert!(worktree_info.path.exists());
        
        // Remove it
        let remove_options = RemoveOptions {
            target: worktree_info.branch.clone(),
            force: false,
            delete_branch: true,
        };
        
        ops.remove_worktree(remove_options).await?;
        assert!(!worktree_info.path.exists());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_path_with_slashes() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await?;
        let config = WorktreeConfig::default();
        let ops = WorktreeOperations::new(repo_path, config);
        
        let options = CreateOptions {
            task_id: "feat/new-ui".to_string(),
            base_branch: None,
            force: false,
            custom_path: None,
        };
        
        let worktree_info = ops.create_worktree(options).await?;
        
        // Should create subdirectory structure
        assert!(worktree_info.path.exists());
        assert!(worktree_info.path.to_string_lossy().contains("feat"));
        
        Ok(())
    }
}
```

### Integration Tests
Create tests that verify integration with .gitignore management and existing workspace patterns.

## Dependencies

Add to `Cargo.toml` if not present:
```toml
[dependencies]
regex = "1.0"
tempfile = { version = "3.0", optional = true }

[dev-dependencies]
tempfile = "3.0"
```

## Notes

- All git operations use async/await for consistency with existing codebase
- Error messages provide actionable information for users
- Path handling supports both relative and absolute worktree base directories
- Branch name validation prevents security issues while allowing flexible naming
- The implementation follows the same patterns as existing git operations in the codebase

## Next Task

After completing this task, proceed to **Task 03: CLI Integration** to expose these operations through the command-line interface.