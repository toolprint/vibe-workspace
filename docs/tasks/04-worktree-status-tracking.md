# Task 04: Status Tracking System

## Goal

Implement comprehensive status tracking for worktrees that provides detailed information about uncommitted changes, unpushed commits, remote synchronization status, and file modifications. This system implements the three-tier severity model and provides rich status information for users and AI systems.

## Scope

- Implement detailed git status checking for worktrees
- Track uncommitted changes, untracked files, and staged changes
- Monitor unpushed commits and remote branch status  
- Implement the three-tier severity classification system
- Provide file-level change tracking and commit information
- Calculate worktree age and activity metrics
- Integrate with existing git status patterns in the codebase

## Implementation Details

### 1. Implement `src/worktree/status.rs` Core Functions

Add these implementations to the existing status.rs file:

```rust
use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, warn};

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
    
    // Determine overall cleanliness
    status.is_clean = status.uncommitted_changes.is_empty() 
        && status.untracked_files.is_empty()
        && (status.ahead_count == 0 || status.remote_status == RemoteStatus::NoRemote);
    
    // Classify severity
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

/// Classify the overall severity of a worktree's status
fn classify_status_severity(status: &WorktreeStatus) -> StatusSeverity {
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
    
    // Get first commit on this branch (diverged from main)
    let first_commit_output = Command::new("git")
        .args(&["log", "--reverse", "--oneline", "main..HEAD"])
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
        .args(&["rev-list", "--count", "main..HEAD"])
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
```

### 2. Enhanced Status Display Functions

Add to the CLI integration in `src/main.rs`:

```rust
/// Enhanced status table printing with detailed information
fn print_detailed_status_table(
    worktrees: &[WorktreeInfo],
    show_files: bool,
) {
    use colored::*;
    
    for (i, worktree) in worktrees.iter().enumerate() {
        if i > 0 {
            println!();
        }
        
        // Header
        println!("{} {}", 
                 worktree.status.status_icon().bold(),
                 worktree.branch.cyan().bold());
        println!("Path: {}", worktree.path.display().to_string().blue());
        
        if !worktree.head.is_empty() {
            let short_head = if worktree.head.len() > 7 { 
                &worktree.head[..7] 
            } else { 
                &worktree.head 
            };
            println!("HEAD: {}", short_head.dimmed());
        }
        
        println!("Age: {}", format_age(worktree.age).dimmed());
        
        // Remote status
        match &worktree.status.remote_status {
            RemoteStatus::NoRemote => {
                println!("Remote: {}", "No remote tracking".yellow());
            }
            RemoteStatus::UpToDate => {
                println!("Remote: {}", "Up to date".green());
            }
            RemoteStatus::Ahead(count) => {
                println!("Remote: {} {} ahead", "↑".green(), count);
            }
            RemoteStatus::Behind(count) => {
                println!("Remote: {} {} behind", "↓".red(), count);
            }
            RemoteStatus::Diverged { ahead, behind } => {
                println!("Remote: {} {} ahead, {} {} behind", 
                         "↑".green(), ahead, "↓".red(), behind);
            }
            RemoteStatus::RemoteDeleted => {
                println!("Remote: {}", "Remote branch deleted".red());
            }
        }
        
        // Changes summary
        let changes = vec![
            (!worktree.status.uncommitted_changes.is_empty(), 
             format!("{} uncommitted", worktree.status.uncommitted_changes.len())),
            (!worktree.status.untracked_files.is_empty(),
             format!("{} untracked", worktree.status.untracked_files.len())),
            (!worktree.status.unpushed_commits.is_empty(),
             format!("{} unpushed", worktree.status.unpushed_commits.len())),
        ].into_iter()
         .filter(|(has, _)| *has)
         .map(|(_, desc)| desc)
         .collect::<Vec<_>>();
        
        if !changes.is_empty() {
            println!("Changes: {}", changes.join(", ").yellow());
        }
        
        // Show files if requested and present
        if show_files {
            if !worktree.status.uncommitted_changes.is_empty() {
                println!("  {} Uncommitted changes:", "📝".dimmed());
                for file in &worktree.status.uncommitted_changes {
                    println!("    {}", file);
                }
            }
            
            if !worktree.status.untracked_files.is_empty() {
                println!("  {} Untracked files:", "❓".dimmed());
                for file in worktree.status.untracked_files.iter().take(5) {
                    println!("    {}", file);
                }
                if worktree.status.untracked_files.len() > 5 {
                    println!("    {} ... and {} more", 
                             "⋯".dimmed(),
                             worktree.status.untracked_files.len() - 5);
                }
            }
            
            if !worktree.status.unpushed_commits.is_empty() {
                println!("  {} Unpushed commits:", "↑".dimmed());
                for commit in worktree.status.unpushed_commits.iter().take(3) {
                    println!("    {} {} ({})", 
                             commit.id.yellow(),
                             commit.message,
                             commit.author.dimmed());
                }
                if worktree.status.unpushed_commits.len() > 3 {
                    println!("    {} ... and {} more commits",
                             "⋯".dimmed(),
                             worktree.status.unpushed_commits.len() - 3);
                }
            }
        }
    }
}
```

### 3. Add Caching for Performance

Create `src/worktree/cache.rs`:

```rust
//! Caching layer for worktree status to improve performance

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::worktree::status::WorktreeInfo;

const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatusCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    worktree_info: WorktreeInfo,
    last_updated: SystemTime,
    file_mtime: SystemTime,
}

impl WorktreeStatusCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
    
    /// Get cached worktree info if still valid
    pub fn get(&self, path: &Path) -> Option<&WorktreeInfo> {
        if let Some(entry) = self.entries.get(path) {
            // Check if cache is still valid
            if self.is_entry_valid(entry, path).unwrap_or(false) {
                return Some(&entry.worktree_info);
            }
        }
        
        None
    }
    
    /// Store worktree info in cache
    pub fn insert(&mut self, path: PathBuf, info: WorktreeInfo) -> Result<()> {
        let file_mtime = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());
        
        let entry = CacheEntry {
            worktree_info: info,
            last_updated: SystemTime::now(),
            file_mtime,
        };
        
        self.entries.insert(path, entry);
        Ok(())
    }
    
    /// Remove stale entries from cache
    pub fn cleanup_stale_entries(&mut self) {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(CACHE_TTL_SECONDS);
        
        self.entries.retain(|path, entry| {
            // Remove if too old or if path no longer exists
            if let Ok(age) = now.duration_since(entry.last_updated) {
                age < ttl && path.exists()
            } else {
                false
            }
        });
    }
    
    /// Check if a cache entry is still valid
    fn is_entry_valid(&self, entry: &CacheEntry, path: &Path) -> Result<bool> {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(CACHE_TTL_SECONDS);
        
        // Check age
        if now.duration_since(entry.last_updated)? > ttl {
            return Ok(false);
        }
        
        // Check if directory was modified
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(current_mtime) = metadata.modified() {
                if current_mtime > entry.file_mtime {
                    return Ok(false);
                }
            }
        }
        
        Ok(true)
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let now = SystemTime::now();
        
        let valid_entries = self.entries.values()
            .filter(|entry| {
                now.duration_since(entry.last_updated)
                    .map(|age| age.as_secs() < CACHE_TTL_SECONDS)
                    .unwrap_or(false)
            })
            .count();
        
        CacheStats {
            total_entries,
            valid_entries,
            hit_ratio: if total_entries > 0 { 
                valid_entries as f64 / total_entries as f64 
            } else { 
                0.0 
            },
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub hit_ratio: f64,
}

impl Default for WorktreeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4. Update Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
futures-util = "0.3"
```

## Integration Points

### With Existing Git Status
- **Reuse Patterns**: Uses similar git command patterns as existing `get_git_status` function
- **Error Handling**: Follows same `anyhow::Result` error propagation
- **Command Execution**: Uses `tokio::process::Command` consistently

### With CLI System
- **Output Formatting**: Integrates with existing table/json/compact formatting
- **Color Coding**: Uses `colored` crate consistently with existing commands
- **Status Icons**: Follows existing visual indicator patterns

### With Caching
- **Performance**: Leverages caching to avoid expensive git operations
- **Cache Integration**: Can integrate with existing workspace cache systems
- **TTL Management**: Smart cache invalidation based on file modification times

## Success Criteria

### Functional Requirements
- [ ] Accurately detects uncommitted changes (staged, unstaged, untracked)
- [ ] Correctly identifies ahead/behind status with remote branches
- [ ] Tracks unpushed commits with detailed information
- [ ] Implements three-tier severity classification correctly
- [ ] Provides file-level change information
- [ ] Handles edge cases (no remote, deleted remote, etc.)
- [ ] Batch processing for multiple worktrees is efficient

### Performance Requirements
- [ ] Status checks complete in < 1 second for typical worktrees
- [ ] Batch updates handle 10+ worktrees efficiently
- [ ] Caching reduces repeated git command execution
- [ ] Memory usage remains reasonable for large numbers of worktrees

### Accuracy Requirements
- [ ] Status information matches `git status` output
- [ ] Remote status correctly reflects actual git state
- [ ] Severity classification is intuitive and useful
- [ ] File change detection is comprehensive
- [ ] Commit information is accurate and complete

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod status_tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;
    
    async fn setup_test_worktree() -> Result<(TempDir, PathBuf)> {
        // Similar setup to operations tests
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();
        
        // Initialize git repo and create test scenario
        // ... setup code
        
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
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_uncommitted_changes_detection() -> Result<()> {
        let (_temp, path) = setup_test_worktree().await?;
        
        // Create a modified file
        std::fs::write(path.join("test.txt"), "modified content")?;
        
        let status = check_worktree_status(&path).await?;
        
        assert!(!status.is_clean);
        assert_eq!(status.severity, StatusSeverity::LightWarning);
        assert!(!status.uncommitted_changes.is_empty());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_severity_classification() -> Result<()> {
        // Test different scenarios for each severity level
        // ... comprehensive severity testing
        
        Ok(())
    }
    
    #[test]
    fn test_porcelain_status_parsing() {
        let sample_output = b"M  modified.txt\0?? untracked.txt\0A  added.txt\0";
        let status = parse_porcelain_status(sample_output).unwrap();
        
        assert_eq!(status.changed_files.len(), 2); // M and A
        assert_eq!(status.untracked_files.len(), 1); // ??
        assert!(status.untracked_files.contains(&"untracked.txt".to_string()));
    }
    
    #[test]
    fn test_cache_functionality() {
        let mut cache = WorktreeStatusCache::new();
        let path = PathBuf::from("/tmp/test");
        let info = WorktreeInfo {
            path: path.clone(),
            branch: "test".to_string(),
            head: "abc123".to_string(),
            status: WorktreeStatus::new(),
            age: Duration::from_secs(0),
            is_detached: false,
        };
        
        // Test cache miss
        assert!(cache.get(&path).is_none());
        
        // Test cache hit after insert
        cache.insert(path.clone(), info).unwrap();
        assert!(cache.get(&path).is_some());
    }
}
```

### Integration Tests

Test integration with the CLI system and verify output formatting works correctly.

## Dependencies

Add any additional dependencies needed:
```toml
[dependencies]
futures-util = "0.3"
```

## Notes

- Git commands use porcelain format for reliable parsing
- Status checking is designed to be extensible for future enhancements
- Caching layer improves performance for frequent status checks
- Error handling gracefully degrades when git operations fail
- The three-tier severity model provides intuitive status classification

## Future Enhancements

- Git hooks integration for real-time status updates
- Conflict detection and resolution assistance
- Integration with external code quality tools
- Custom status rules and filters
- Background status monitoring

## Next Task

After completing this task, proceed to **Task 05: Merge Detection** to implement advanced algorithms for detecting merged branches and cleanup candidates.