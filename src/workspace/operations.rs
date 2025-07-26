use anyhow::{Context, Result};
use colored::*;
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command as AsyncCommand;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub repository_name: String,
    pub path: String,
    pub branch: Option<String>,
    pub clean: bool,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub remote_url: Option<String>,
}

impl GitStatus {
    pub fn format_status_line(&self) -> String {
        let mut parts = Vec::new();

        // Repository name and path
        let name = format!("{}", self.repository_name.cyan().bold());
        let path = format!("({})", self.path.dimmed());
        parts.push(format!("{name} {path}"));

        // Branch information
        if let Some(ref branch) = self.branch {
            let branch_display = if self.ahead > 0 || self.behind > 0 {
                format!("{} [↑{} ↓{}]", branch, self.ahead, self.behind)
            } else {
                branch.clone()
            };
            parts.push(format!("on {}", branch_display.yellow()));
        }

        // Status indicators
        let mut status_parts = Vec::new();

        if self.clean {
            status_parts.push("✓".green().to_string());
        } else {
            if self.staged > 0 {
                status_parts.push(format!("{}S", self.staged).green().to_string());
            }
            if self.unstaged > 0 {
                status_parts.push(format!("{}M", self.unstaged).red().to_string());
            }
            if self.untracked > 0 {
                status_parts.push(format!("{}?", self.untracked).yellow().to_string());
            }
        }

        if !status_parts.is_empty() {
            parts.push(format!("[{}]", status_parts.join(" ")));
        }

        parts.join(" ")
    }

    pub fn is_dirty(&self) -> bool {
        !self.clean
    }
}

#[derive(Debug, Clone)]
pub enum GitOperation {
    Status,
    Pull,
    Push,
    Fetch,
    Custom(String),
}

impl GitOperation {
    pub async fn execute<P: AsRef<Path>>(&self, repo_path: P) -> Result<String> {
        let repo_path = repo_path.as_ref();

        match self {
            GitOperation::Status => get_git_status(repo_path)
                .await
                .map(|_| "Status checked".to_string()),
            GitOperation::Pull => execute_git_command(repo_path, &["pull"]).await,
            GitOperation::Push => execute_git_command(repo_path, &["push"]).await,
            GitOperation::Fetch => execute_git_command(repo_path, &["fetch"]).await,
            GitOperation::Custom(command) => {
                let args: Vec<&str> = command.split_whitespace().collect();
                execute_git_command(repo_path, &args).await
            }
        }
    }
}

/// Get comprehensive git status for a repository
pub async fn get_git_status<P: AsRef<Path>>(repo_path: P) -> Result<GitStatus> {
    let repo_path = repo_path.as_ref();
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    debug!("Getting git status for repository: {}", repo_path.display());

    // Open repository using git2
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open git repository: {}", repo_path.display()))?;

    // Get current branch
    let branch = get_current_branch_name(&repo)?;

    // Get remote URL
    let remote_url = get_remote_url(&repo)?;

    // Get working directory status
    let mut status_opts = StatusOptions::new();
    status_opts.include_untracked(true);
    status_opts.include_ignored(false);

    let statuses = repo
        .statuses(Some(&mut status_opts))
        .context("Failed to get repository status")?;

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let flags = entry.status();

        if flags.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            staged += 1;
        }

        if flags.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        ) {
            unstaged += 1;
        }

        if flags.contains(git2::Status::WT_NEW) {
            untracked += 1;
        }
    }

    // Get ahead/behind information
    let (ahead, behind) = get_ahead_behind_counts(&repo, branch.as_deref())?;

    let clean = staged == 0 && unstaged == 0 && untracked == 0;

    Ok(GitStatus {
        repository_name: repo_name,
        path: repo_path.display().to_string(),
        branch,
        clean,
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
        remote_url,
    })
}

/// Execute a git command in the specified repository
pub async fn execute_git_command<P: AsRef<Path>>(repo_path: P, args: &[&str]) -> Result<String> {
    let repo_path = repo_path.as_ref();

    debug!(
        "Executing git command in {}: git {}",
        repo_path.display(),
        args.join(" ")
    );

    let output = AsyncCommand::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .await
        .with_context(|| format!("Failed to execute git command: git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Git command failed: git {} (exit code: {})\n{}",
            args.join(" "),
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Get the current branch name
fn get_current_branch_name(repo: &Repository) -> Result<Option<String>> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(ref e) if e.code() == git2::ErrorCode::UnbornBranch => {
            debug!("Repository has no commits yet");
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };

    Ok(head.shorthand().map(|s| s.to_string()))
}

/// Get remote URL for the repository
fn get_remote_url(repo: &Repository) -> Result<Option<String>> {
    let remote_name = match repo.find_remote("origin") {
        Ok(_) => "origin".to_string(),
        Err(_) => {
            let remotes = repo.remotes()?;
            match remotes.get(0) {
                Some(name) => name.to_string(),
                None => return Ok(None),
            }
        }
    };

    let remote = repo.find_remote(&remote_name)?;
    Ok(remote.url().map(|url| url.to_string()))
}

/// Get ahead/behind counts compared to upstream
fn get_ahead_behind_counts(repo: &Repository, branch_name: Option<&str>) -> Result<(usize, usize)> {
    let branch_name = match branch_name {
        Some(name) => name,
        None => return Ok((0, 0)),
    };

    // Get local branch reference
    let local_ref = format!("refs/heads/{branch_name}");
    let local_oid = match repo.resolve_reference_from_short_name(&local_ref) {
        Ok(reference) => reference.target().unwrap_or_else(git2::Oid::zero),
        Err(_) => return Ok((0, 0)),
    };

    // Try to find upstream branch
    let upstream_ref = format!("refs/remotes/origin/{branch_name}");
    let upstream_oid = match repo.resolve_reference_from_short_name(&upstream_ref) {
        Ok(reference) => reference.target().unwrap_or_else(git2::Oid::zero),
        Err(_) => {
            debug!("No upstream branch found for {}", branch_name);
            return Ok((0, 0));
        }
    };

    // Calculate ahead/behind
    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((ahead, behind)) => Ok((ahead, behind)),
        Err(e) => {
            warn!(
                "Failed to calculate ahead/behind for {}: {}",
                branch_name, e
            );
            Ok((0, 0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_test_repo(path: &Path) -> Result<()> {
        Command::new("git")
            .args(&["init"])
            .current_dir(path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(path)
            .output()?;

        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_git_status_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let status = get_git_status(temp_dir.path()).await.unwrap();
        assert!(status.clean);
        assert_eq!(status.staged, 0);
        assert_eq!(status.unstaged, 0);
        assert_eq!(status.untracked, 0);
    }

    #[tokio::test]
    async fn test_git_status_with_untracked_file() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        // Create an untracked file
        fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let status = get_git_status(temp_dir.path()).await.unwrap();
        assert!(!status.clean);
        assert_eq!(status.untracked, 1);
    }

    #[tokio::test]
    async fn test_execute_git_command() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let result = execute_git_command(temp_dir.path(), &["status", "--porcelain"]).await;
        assert!(result.is_ok());
    }
}
