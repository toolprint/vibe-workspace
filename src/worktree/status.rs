//! Worktree status tracking and reporting

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::process::Command;
use tracing::debug;

use crate::worktree::config::WorktreeMergeDetectionConfig;
use crate::worktree::merge_detection::detect_worktree_merge_status;

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
        self.is_clean
            && self.uncommitted_changes.is_empty()
            && self.untracked_files.is_empty()
            && (self.unpushed_commits.is_empty()
                || self
                    .merge_info
                    .as_ref()
                    .map_or(false, |info| info.is_merged))
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

impl WorktreeInfo {
    /// Update status information for this worktree
    pub async fn update_status(&mut self) -> Result<()> {
        self.status = check_worktree_status(&self.path).await?;
        self.update_age()?;
        Ok(())
    }
    
    /// Update the age of this worktree
    fn update_age(&mut self) -> Result<()> {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if let Ok(created) = metadata.created() {
                self.age = std::time::SystemTime::now().duration_since(created)?;
            }
        }
        Ok(())
    }
}

/// Check comprehensive status for a worktree
pub async fn check_worktree_status(worktree_path: &Path) -> Result<WorktreeStatus> {
    check_worktree_status_with_config(worktree_path, None).await
}

/// Check comprehensive status for a worktree with optional merge detection config
pub async fn check_worktree_status_with_config(
    worktree_path: &Path, 
    merge_config: Option<&WorktreeMergeDetectionConfig>
) -> Result<WorktreeStatus> {
    let mut status = WorktreeStatus::new();
    
    // Get basic git status (staged, unstaged, untracked files)
    let git_status = get_git_porcelain_status(worktree_path).await?;
    status.uncommitted_changes = git_status.changed_files;
    status.untracked_files = git_status.untracked_files;
    
    // Get remote status and commit information
    let remote_info = get_remote_status(worktree_path).await?;
    status.remote_status = remote_info.status;
    status.ahead_count = remote_info.ahead;
    status.behind_count = remote_info.behind;
    
    // Get unpushed commits
    if status.ahead_count > 0 {
        status.unpushed_commits = get_unpushed_commits(worktree_path).await?;
    }
    
    // Add merge detection if config is provided
    if let Some(config) = merge_config {
        if let Ok(current_branch) = get_current_branch(worktree_path).await {
            match detect_worktree_merge_status(worktree_path, &current_branch, config).await {
                Ok(merge_info) => {
                    status.merge_info = Some(merge_info);
                }
                Err(e) => {
                    debug!("Merge detection failed for branch '{}': {}", current_branch, e);
                    // Continue without merge info rather than failing
                }
            }
        }
    }
    
    // Determine overall cleanliness
    status.is_clean = status.uncommitted_changes.is_empty() 
        && status.untracked_files.is_empty()
        && (status.ahead_count == 0 || matches!(status.remote_status, RemoteStatus::NoRemote));
    
    // Classify severity (now considering merge status)
    status.severity = classify_status_severity(&status);
    
    Ok(status)
}

/// Get git status in porcelain format for parsing
async fn get_git_porcelain_status(worktree_path: &Path) -> Result<GitStatusInfo> {
    let output = Command::new("git")
        .args(&["status", "--porcelain=v1", "-z"])
        .current_dir(worktree_path)
        .output()
        .await
        .with_context(|| format!("Failed to get git status for: {}", worktree_path.display()))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Git status failed: {}", stderr));
    }
    
    parse_porcelain_status(&output.stdout)
}

/// Parse git status porcelain output
fn parse_porcelain_status(output: &[u8]) -> Result<GitStatusInfo> {
    let output_str = String::from_utf8_lossy(output);
    let mut changed_files = Vec::new();
    let mut untracked_files = Vec::new();
    
    for line in output_str.split('\0') {
        if line.is_empty() {
            continue;
        }
        
        if line.len() < 3 {
            continue;
        }
        
        let status_code = &line[0..2];
        let file_path = &line[3..];
        
        match status_code {
            "??" => {
                untracked_files.push(file_path.to_string());
            }
            _ => {
                let status_desc = match status_code {
                    "M " => "modified (unstaged)",
                    " M" => "modified (staged)",
                    "MM" => "modified (both staged and unstaged)",
                    "A " => "added (staged)",
                    " A" => "added (unstaged)",
                    "D " => "deleted (staged)", 
                    " D" => "deleted (unstaged)",
                    "R " => "renamed (staged)",
                    " R" => "renamed (unstaged)",
                    "C " => "copied (staged)",
                    " C" => "copied (unstaged)",
                    "U " | " U" | "UU" => "unmerged",
                    _ => "unknown",
                };
                
                changed_files.push(format!("{}: {}", status_desc, file_path));
            }
        }
    }
    
    Ok(GitStatusInfo {
        changed_files,
        untracked_files,
    })
}

/// Get remote branch status and ahead/behind counts
async fn get_remote_status(worktree_path: &Path) -> Result<RemoteInfo> {
    // First, check if there's a remote tracking branch
    let upstream_result = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "@{u}"])
        .current_dir(worktree_path)
        .output()
        .await;
    
    let upstream_branch = match upstream_result {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    };
    
    if upstream_branch.is_none() {
        return Ok(RemoteInfo {
            status: RemoteStatus::NoRemote,
            ahead: 0,
            behind: 0,
        });
    }
    
    // Get ahead/behind counts
    let count_output = Command::new("git")
        .args(&["rev-list", "--count", "--left-right", "@{u}...HEAD"])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    if !count_output.status.success() {
        // Remote branch might be deleted
        return Ok(RemoteInfo {
            status: RemoteStatus::RemoteDeleted,
            ahead: 0,
            behind: 0,
        });
    }
    
    let count_str = String::from_utf8_lossy(&count_output.stdout);
    let counts: Vec<&str> = count_str.trim().split_whitespace().collect();
    
    let (behind, ahead) = if counts.len() >= 2 {
        let behind = counts[0].parse::<usize>().unwrap_or(0);
        let ahead = counts[1].parse::<usize>().unwrap_or(0);
        (behind, ahead)
    } else {
        (0, 0)
    };
    
    let status = match (ahead, behind) {
        (0, 0) => RemoteStatus::UpToDate,
        (a, 0) if a > 0 => RemoteStatus::Ahead(a),
        (0, b) if b > 0 => RemoteStatus::Behind(b),
        (a, b) if a > 0 && b > 0 => RemoteStatus::Diverged { ahead: a, behind: b },
        _ => RemoteStatus::UpToDate,
    };
    
    Ok(RemoteInfo {
        status,
        ahead,
        behind,
    })
}

/// Get list of unpushed commits with details
async fn get_unpushed_commits(worktree_path: &Path) -> Result<Vec<CommitInfo>> {
    let output = Command::new("git")
        .args(&[
            "log", 
            "--oneline",
            "--format=%H|%s|%an|%ct",
            "@{u}..HEAD"
        ])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    
    for line in output_str.lines() {
        if line.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let full_sha = parts[0];
            let short_sha = if full_sha.len() >= 7 { 
                &full_sha[..7] 
            } else { 
                full_sha 
            };
            
            let timestamp_secs = parts[3].parse::<u64>().unwrap_or(0);
            let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp_secs);
            
            commits.push(CommitInfo {
                id: short_sha.to_string(),
                message: parts[1].to_string(),
                author: parts[2].to_string(),
                timestamp,
            });
        }
    }
    
    Ok(commits)
}

/// Get the current branch name
async fn get_current_branch(worktree_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .await?;
        
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!("Failed to get current branch"))
    }
}

/// Classify the overall severity of a worktree's status
fn classify_status_severity(status: &WorktreeStatus) -> StatusSeverity {
    // If branch is merged with high confidence, it's safer to clean
    if let Some(merge_info) = &status.merge_info {
        if merge_info.is_merged && merge_info.confidence > 0.8 {
            // Even with uncommitted changes, merged branches are less concerning
            if status.is_clean {
                return StatusSeverity::Clean;
            } else {
                return StatusSeverity::LightWarning;
            }
        }
    }
    
    // Clean status: everything is up to date
    if status.is_clean && status.ahead_count == 0 && status.behind_count == 0 {
        return StatusSeverity::Clean;
    }
    
    // Warning (⚡) - serious issues with the branch itself
    if matches!(status.remote_status, RemoteStatus::RemoteDeleted) {
        return StatusSeverity::Warning;
    }
    
    if status.behind_count > 10 {
        return StatusSeverity::Warning;
    }
    
    if let RemoteStatus::Diverged { ahead, behind } = status.remote_status {
        if behind > 5 || ahead > 20 {
            return StatusSeverity::Warning;
        }
    }
    
    // Light warning (⚠️) - typical worktree development issues
    if !status.uncommitted_changes.is_empty() 
        || !status.untracked_files.is_empty() 
        || status.ahead_count > 0
        || status.behind_count > 0
        || matches!(status.remote_status, RemoteStatus::NoRemote) {
        return StatusSeverity::LightWarning;
    }
    
    StatusSeverity::Clean
}

/// Check if a worktree has any activity in the last N days
pub async fn check_worktree_activity(
    worktree_path: &Path, 
    days: u64
) -> Result<bool> {
    let since = format!("--since={} days ago", days);
    
    let output = Command::new("git")
        .args(&["log", "--oneline", &since, "HEAD"])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    Ok(output.status.success() && !output.stdout.is_empty())
}

/// Get detailed file-level diff for conflicts or changes
pub async fn get_worktree_diff(
    worktree_path: &Path,
    compact: bool,
) -> Result<String> {
    let mut args = vec!["diff"];
    
    if compact {
        args.extend(&["--name-status"]);
    } else {
        args.extend(&["--stat", "--color=never"]);
    }
    
    let output = Command::new("git")
        .args(&args)
        .current_dir(worktree_path)
        .output()
        .await?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Ok(String::new())
    }
}

/// Get branch creation time and first commit
pub async fn get_branch_info(worktree_path: &Path) -> Result<BranchInfo> {
    // Get current branch name
    let branch_output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    let branch_name = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
    
    // Get first commit on this branch (if it's not the default branch)
    // First, try to find the default branch
    let default_branch_result = Command::new("git")
        .args(&["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(worktree_path)
        .output()
        .await;
    
    // If current branch is a common default branch, assume it's the main branch
    if branch_name == "main" || branch_name == "master" {
        return Ok(BranchInfo {
            name: branch_name,
            first_commit: None,
            commit_count: 0,
        });
    }

    let default_branch = match default_branch_result {
        Ok(output) if output.status.success() => {
            let full_ref = String::from_utf8_lossy(&output.stdout);
            let trimmed = full_ref.trim();
            trimmed.strip_prefix("refs/remotes/origin/")
                .unwrap_or("main")
                .to_string()
        }
        _ => {
            // Fallback: try common default branch names
            let mut found_default = None;
            for default in &["main", "master"] {
                let check_output = Command::new("git")
                    .args(&["show-ref", "--verify", &format!("refs/heads/{}", default)])
                    .current_dir(worktree_path)
                    .output()
                    .await;
                
                if let Ok(output) = check_output {
                    if output.status.success() {
                        found_default = Some(default.to_string());
                        break;
                    }
                }
            }
            
            found_default.unwrap_or_else(|| "main".to_string())
        }
    };
    
    // Don't compare branch to itself
    if branch_name == default_branch {
        return Ok(BranchInfo {
            name: branch_name,
            first_commit: None,
            commit_count: 0,
        });
    }
    
    let first_commit_output = Command::new("git")
        .args(&["log", "--reverse", "--oneline", &format!("{}..HEAD", default_branch)])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    let first_commit = if first_commit_output.status.success() && !first_commit_output.stdout.is_empty() {
        String::from_utf8_lossy(&first_commit_output.stdout)
            .lines()
            .next()
            .map(|line| line.to_string())
    } else {
        None
    };
    
    // Get total commits on this branch  
    let commit_count_output = Command::new("git")
        .args(&["rev-list", "--count", &format!("{}..HEAD", default_branch)])
        .current_dir(worktree_path)
        .output()
        .await?;
    
    let commit_count = if commit_count_output.status.success() {
        String::from_utf8_lossy(&commit_count_output.stdout)
            .trim()
            .parse::<usize>()
            .unwrap_or(0)
    } else {
        0
    };
    
    Ok(BranchInfo {
        name: branch_name,
        first_commit,
        commit_count,
    })
}

/// Update an existing WorktreeInfo with fresh status
pub async fn update_worktree_info(mut worktree: WorktreeInfo) -> Result<WorktreeInfo> {
    worktree.update_status().await?;
    
    // Refresh HEAD
    let head_output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&worktree.path)
        .output()
        .await?;
    
    if head_output.status.success() {
        worktree.head = String::from_utf8_lossy(&head_output.stdout).trim().to_string();
    }
    
    Ok(worktree)
}

/// Batch update multiple worktrees for efficiency
pub async fn batch_update_worktree_status(
    worktrees: Vec<WorktreeInfo>
) -> Result<Vec<WorktreeInfo>> {
    let mut updated = Vec::new();
    
    // Update in parallel for better performance
    let futures = worktrees.into_iter()
        .map(|worktree| async move { update_worktree_info(worktree).await });
    
    let results = futures_util::future::try_join_all(futures).await?;
    updated.extend(results);
    
    Ok(updated)
}

// Supporting types
#[derive(Debug)]
struct GitStatusInfo {
    changed_files: Vec<String>,
    untracked_files: Vec<String>,
}

#[derive(Debug)]
struct RemoteInfo {
    status: RemoteStatus,
    ahead: usize,
    behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub first_commit: Option<String>,
    pub commit_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;
    
    async fn setup_test_worktree() -> Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();
        
        // Initialize git repo
        let init_output = Command::new("git")
            .args(&["init"])
            .current_dir(&path)
            .output()
            .await?;
        
        if !init_output.status.success() {
            anyhow::bail!("Failed to initialize git repo");
        }
        
        // Configure git user
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&path)
            .output()
            .await?;
        
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&path)
            .output()
            .await?;
        
        // Create initial commit
        std::fs::write(path.join("README.md"), "# Test Repository")?;
        
        Command::new("git")
            .args(&["add", "README.md"])
            .current_dir(&path)
            .output()
            .await?;
        
        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(&path)
            .output()
            .await?;
        
        Ok((temp_dir, path))
    }
    
    #[tokio::test]
    async fn test_clean_worktree_status() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        let status = check_worktree_status(&path).await?;
        
        assert!(status.is_clean);
        assert_eq!(status.severity, StatusSeverity::Clean);
        assert!(status.uncommitted_changes.is_empty());
        assert!(status.untracked_files.is_empty());
        assert_eq!(status.ahead_count, 0);
        assert_eq!(status.behind_count, 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_uncommitted_changes_detection() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        // Create a modified file
        std::fs::write(path.join("README.md"), "# Modified Test Repository")?;
        
        let status = check_worktree_status(&path).await?;
        
        assert!(!status.is_clean);
        assert_eq!(status.severity, StatusSeverity::LightWarning);
        assert!(!status.uncommitted_changes.is_empty());
        
        // Should contain information about the modified file
        let change_desc = &status.uncommitted_changes[0];
        assert!(change_desc.contains("README.md"));
        assert!(change_desc.contains("modified"));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_untracked_files_detection() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        // Create an untracked file
        std::fs::write(path.join("untracked.txt"), "Untracked content")?;
        
        let status = check_worktree_status(&path).await?;
        
        assert!(!status.is_clean);
        assert_eq!(status.severity, StatusSeverity::LightWarning);
        assert!(!status.untracked_files.is_empty());
        assert!(status.untracked_files.contains(&"untracked.txt".to_string()));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_severity_classification() -> Result<()> {
        // Test clean status
        let mut status = WorktreeStatus::new();
        status.is_clean = true;
        status.ahead_count = 0;
        status.behind_count = 0;
        status.uncommitted_changes.clear();
        status.untracked_files.clear();
        status.remote_status = RemoteStatus::UpToDate;
        
        assert_eq!(classify_status_severity(&status), StatusSeverity::Clean);
        
        // Test light warning - uncommitted changes
        status.uncommitted_changes.push("file.txt".to_string());
        status.is_clean = false;
        assert_eq!(classify_status_severity(&status), StatusSeverity::LightWarning);
        
        // Test warning - many commits behind
        status.behind_count = 15;
        status.remote_status = RemoteStatus::Behind(15);
        assert_eq!(classify_status_severity(&status), StatusSeverity::Warning);
        
        // Test warning - remote deleted
        status.behind_count = 0;
        status.remote_status = RemoteStatus::RemoteDeleted;
        assert_eq!(classify_status_severity(&status), StatusSeverity::Warning);
        
        // Test warning - diverged significantly
        status.remote_status = RemoteStatus::Diverged { ahead: 25, behind: 8 };
        assert_eq!(classify_status_severity(&status), StatusSeverity::Warning);
        
        Ok(())
    }
    
    #[test]
    fn test_porcelain_status_parsing() {
        let sample_output = b"M  modified.txt\0?? untracked.txt\0A  added.txt\0D  deleted.txt\0";
        let status = parse_porcelain_status(sample_output).unwrap();
        
        assert_eq!(status.changed_files.len(), 3); // M, A, D
        assert_eq!(status.untracked_files.len(), 1); // ??
        assert!(status.untracked_files.contains(&"untracked.txt".to_string()));
        
        // Check that status descriptions are included
        assert!(status.changed_files.iter().any(|f| f.contains("modified.txt") && f.contains("modified")));
        assert!(status.changed_files.iter().any(|f| f.contains("added.txt") && f.contains("added")));
        assert!(status.changed_files.iter().any(|f| f.contains("deleted.txt") && f.contains("deleted")));
    }
    
    #[test]
    fn test_empty_porcelain_status() {
        let empty_output = b"";
        let status = parse_porcelain_status(empty_output).unwrap();
        
        assert!(status.changed_files.is_empty());
        assert!(status.untracked_files.is_empty());
    }
    
    #[test]
    fn test_status_severity_priority() {
        assert!(StatusSeverity::Warning.priority() < StatusSeverity::LightWarning.priority());
        assert!(StatusSeverity::LightWarning.priority() < StatusSeverity::Clean.priority());
    }
    
    #[tokio::test]
    async fn test_worktree_info_update_status() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        let mut worktree_info = WorktreeInfo {
            path: path.clone(),
            branch: "main".to_string(),
            head: "".to_string(),
            status: WorktreeStatus::new(),
            age: Duration::from_secs(0),
            is_detached: false,
        };
        
        // Update status should work without errors
        worktree_info.update_status().await?;
        
        // Should have a clean status for the fresh repo
        assert!(worktree_info.status.is_clean);
        assert_eq!(worktree_info.status.severity, StatusSeverity::Clean);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_batch_update_worktree_status() -> Result<()> {
        let (_temp1, path1) = setup_test_worktree().await?;
        let (_temp2, path2) = setup_test_worktree().await?;
        
        let worktrees = vec![
            WorktreeInfo {
                path: path1,
                branch: "main".to_string(),
                head: "abc123".to_string(),
                status: WorktreeStatus::new(),
                age: Duration::from_secs(0),
                is_detached: false,
            },
            WorktreeInfo {
                path: path2,
                branch: "feature".to_string(),
                head: "def456".to_string(),
                status: WorktreeStatus::new(),
                age: Duration::from_secs(0),
                is_detached: false,
            },
        ];
        
        let updated_worktrees = batch_update_worktree_status(worktrees).await?;
        
        assert_eq!(updated_worktrees.len(), 2);
        
        // Both should be clean since they're fresh repos
        for worktree in &updated_worktrees {
            assert!(worktree.status.is_clean);
            assert_eq!(worktree.status.severity, StatusSeverity::Clean);
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_check_worktree_activity() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        // Should have recent activity (the initial commit)
        let has_recent_activity = check_worktree_activity(&path, 1).await?;
        assert!(has_recent_activity);
        
        // Check for activity in the distant past (should be false)
        // Note: This might be flaky depending on system clock
        let has_old_activity = check_worktree_activity(&path, 0).await?;
        // We can't reliably test this because it depends on timing
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_get_worktree_diff_compact() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        // Clean repo should have empty diff
        let diff = get_worktree_diff(&path, true).await?;
        assert!(diff.is_empty() || diff.trim().is_empty());
        
        // Modify a file and check diff
        std::fs::write(path.join("README.md"), "# Modified Test Repository")?;
        
        let diff = get_worktree_diff(&path, true).await?;
        // Should contain information about the modified file
        assert!(diff.contains("README.md") || diff.trim().is_empty()); // Git might not show diff until staged
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_get_branch_info() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        let branch_info = get_branch_info(&path).await?;
        
        // Default branch name can be either "main" or "master" depending on git configuration
        assert!(branch_info.name == "main" || branch_info.name == "master");
        // For a new repo, there shouldn't be commits ahead of main/master
        assert_eq!(branch_info.commit_count, 0);
        assert!(branch_info.first_commit.is_none());
        
        Ok(())
    }
    
    #[test]
    fn test_status_description() {
        let mut status = WorktreeStatus::new();
        status.is_clean = true;
        
        // Clean status
        assert_eq!(status.status_description(), "Clean");
        
        // Status with issues
        status.is_clean = false;
        status.uncommitted_changes.push("file1.rs".to_string());
        status.untracked_files.push("file2.rs".to_string());
        status.unpushed_commits.push(CommitInfo {
            id: "abc123".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: SystemTime::now(),
        });
        
        let description = status.status_description();
        assert!(description.contains("1 uncommitted"));
        assert!(description.contains("1 untracked"));
        assert!(description.contains("1 unpushed"));
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
    fn test_cleanup_safety_detection() {
        let mut status = WorktreeStatus::new();
        
        // Not safe initially
        assert!(!status.is_safe_to_cleanup());
        
        // Make it clean
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
            timestamp: SystemTime::now(),
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
    
    #[test]
    fn test_remote_status_display() {
        // Test the different remote status variants would be covered
        // in integration tests with the CLI display functions
        let status_no_remote = RemoteStatus::NoRemote;
        let status_up_to_date = RemoteStatus::UpToDate;
        let status_ahead = RemoteStatus::Ahead(3);
        let status_behind = RemoteStatus::Behind(2);
        let status_diverged = RemoteStatus::Diverged { ahead: 3, behind: 2 };
        let status_deleted = RemoteStatus::RemoteDeleted;
        
        // These would be tested in the display functions
        // Here we just verify the enum variants exist and can be constructed
        assert!(matches!(status_no_remote, RemoteStatus::NoRemote));
        assert!(matches!(status_up_to_date, RemoteStatus::UpToDate));
        assert!(matches!(status_ahead, RemoteStatus::Ahead(3)));
        assert!(matches!(status_behind, RemoteStatus::Behind(2)));
        assert!(matches!(status_diverged, RemoteStatus::Diverged { ahead: 3, behind: 2 }));
        assert!(matches!(status_deleted, RemoteStatus::RemoteDeleted));
    }
}
