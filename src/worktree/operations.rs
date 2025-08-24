//! Core Git worktree operations

use anyhow::{bail, Context, Result};
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

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            base_branch: None,
            force: false,
            custom_path: None,
        }
    }
}

impl Default for RemoveOptions {
    fn default() -> Self {
        Self {
            target: String::new(),
            force: false,
            delete_branch: false,
        }
    }
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
#[derive(Clone)]
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
            bail!(
                "Branch '{}' already exists. Use --force to recreate.",
                branch_name
            );
        }

        // Create the worktree
        let result = if branch_exists && options.force {
            // Remove existing worktree first if it exists
            if let Ok(existing_path) = self.find_worktree_path(&branch_name).await {
                warn!("Removing existing worktree at: {}", existing_path.display());
                self.execute_git_command(&[
                    "worktree",
                    "remove",
                    "--force",
                    &existing_path.to_string_lossy(),
                ])
                .await?;
            }

            // Remove and recreate branch
            self.execute_git_command(&["branch", "-D", &branch_name])
                .await
                .ok(); // Ignore errors
            self.create_branch_and_worktree(
                &branch_name,
                &worktree_path,
                options.base_branch.as_deref(),
            )
            .await?
        } else {
            self.create_branch_and_worktree(
                &branch_name,
                &worktree_path,
                options.base_branch.as_deref(),
            )
            .await?
        };

        debug!(
            "Created worktree: {} -> {}",
            branch_name,
            worktree_path.display()
        );

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
        // Check if it's a path (contains directory separators and exists) or a branch name
        let worktree_path = if Path::new(&options.target).exists() {
            // Treat as existing path
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
            bail!(
                "Path is not a valid git worktree: {}",
                worktree_path.display()
            );
        }

        // Extract branch name BEFORE removing the worktree if needed for deletion
        let branch_name_for_deletion = if options.delete_branch {
            if options.target.contains('/') {
                Some(self.get_worktree_branch(&worktree_path).await?)
            } else {
                Some(options.target.clone())
            }
        } else {
            None
        };

        // Remove the worktree
        let mut args = vec!["worktree", "remove"];
        if options.force {
            args.push("--force");
        }
        let path_str = worktree_path.to_string_lossy();
        args.push(&path_str);

        self.execute_git_command(&args).await?;

        // Delete branch if requested (after worktree removal)
        if let Some(branch_name) = branch_name_for_deletion {
            self.execute_git_command(&["branch", "-D", &branch_name])
                .await?;
            debug!("Deleted branch: {}", branch_name);
        }

        debug!("Removed worktree: {}", worktree_path.display());
        Ok(())
    }

    /// List all git worktrees
    pub async fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = self
            .execute_git_command(&["worktree", "list", "--porcelain"])
            .await?;
        self.parse_worktree_list(&output).await
    }

    /// Find git repository root
    pub async fn find_git_root(&self) -> Result<PathBuf> {
        let output = self
            .execute_git_command(&["rev-parse", "--show-toplevel"])
            .await?;
        Ok(PathBuf::from(output.trim()))
    }

    /// Get a reference to the config
    pub fn get_config(&self) -> &WorktreeConfig {
        &self.config
    }

    // Private implementation methods

    async fn create_branch_and_worktree(
        &self,
        branch_name: &str,
        worktree_path: &Path,
        base_branch: Option<&str>,
    ) -> Result<CreateResult> {
        let base = base_branch.unwrap_or("HEAD");

        let _output = self
            .execute_git_command(&[
                "worktree",
                "add",
                "-b",
                branch_name,
                &worktree_path.to_string_lossy(),
                base,
            ])
            .await?;

        // Get the HEAD commit
        let head = self
            .execute_git_command(&["rev-parse", "HEAD"])
            .await?
            .trim()
            .to_string();

        Ok(CreateResult { head })
    }

    fn calculate_worktree_path(&self, task_id: &str) -> Result<PathBuf> {
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

        let final_name = format!(
            "{}__{:x}",
            worktree_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("worktree"),
            timestamp
        );

        Ok(worktree_path
            .parent()
            .unwrap_or(&worktree_path)
            .join(final_name))
    }

    async fn ensure_base_directory_exists(&self) -> Result<()> {
        let base_path = if self.config.base_dir.is_absolute() {
            self.config.base_dir.clone()
        } else {
            self.repo_root.join(&self.config.base_dir)
        };

        if !base_path.exists() {
            fs::create_dir_all(&base_path).with_context(|| {
                format!("Failed to create base directory: {}", base_path.display())
            })?;
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
            if content
                .lines()
                .any(|line| line.trim() == ignore_pattern.trim())
            {
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

        content.push_str(&format!(
            "# Vibe worktree directories\n{}\n",
            ignore_pattern
        ));

        fs::write(&gitignore_path, content).with_context(|| {
            format!(
                "Failed to update .gitignore at: {}",
                gitignore_path.display()
            )
        })?;

        debug!("Updated .gitignore with pattern: {}", ignore_pattern);
        Ok(())
    }

    async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let result = self
            .execute_git_command(&[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch_name),
            ])
            .await;
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
            bail!(
                "Failed to get branch name for worktree: {}",
                worktree_path.display()
            );
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
                        branch = branch_ref
                            .strip_prefix("refs/heads/")
                            .unwrap_or(branch_ref)
                            .to_string();
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
                        std::time::SystemTime::now()
                            .duration_since(created)
                            .unwrap_or_default()
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
            bail!(
                "Git command failed: git {}\nError: {}",
                args.join(" "),
                stderr
            );
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
    let dangerous_chars = [
        '$', '`', '(', ')', '{', '}', '|', '&', ';', '<', '>', '\n', '\r', '\0', '"', '\'', '\\',
    ];
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
        bail!(
            "Task ID '{}' cannot be sanitized to a valid branch name",
            name
        );
    }

    Ok(sanitized.to_string())
}

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
        assert_eq!(
            sanitize_branch_name("Fix: issue #456").unwrap(),
            "Fix-issue-456"
        );

        // Multiple consecutive characters
        assert_eq!(
            sanitize_branch_name("task   with   spaces").unwrap(),
            "task-with-spaces"
        );
        assert_eq!(
            sanitize_branch_name("task---dashes").unwrap(),
            "task-dashes"
        );

        // Edge cases
        assert!(sanitize_branch_name("").is_err());
        assert!(sanitize_branch_name("!!!").is_err());
        assert!(sanitize_branch_name("---").is_err());
    }
}
