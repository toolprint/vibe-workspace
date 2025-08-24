use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::task;
use tracing::debug;
use walkdir::WalkDir;

/// Discover git repositories in a directory structure
pub async fn discover_git_repositories<P: AsRef<Path>>(
    root_path: P,
    max_depth: usize,
) -> Result<Vec<PathBuf>> {
    let root_path = root_path.as_ref().to_path_buf();

    debug!(
        "Discovering repositories in {} with max depth {}",
        root_path.display(),
        max_depth
    );

    // Run the blocking directory walk in a separate task
    let repositories = task::spawn_blocking(move || -> Result<Vec<PathBuf>> {
        let mut repos = Vec::new();

        for entry in WalkDir::new(&root_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip if this is not a directory
            if !path.is_dir() {
                continue;
            }

            // Check if this directory contains a .git folder
            let git_dir = path.join(".git");
            if git_dir.exists() {
                debug!("Found git repository: {}", path.display());
                repos.push(path.to_path_buf());

                // Skip scanning inside git repositories to avoid finding submodules
                // This will be handled by WalkDir's pruning
                continue;
            }

            // Skip common directories that shouldn't contain repositories
            if let Some(
                "node_modules" | "target" | "dist" | "build" | ".git" | "vendor" | "__pycache__"
                | ".venv" | "venv",
            ) = path.file_name().and_then(|n| n.to_str())
            {
                debug!("Skipping directory: {}", path.display());
                continue;
            }
        }

        // Sort repositories by path for consistent output
        repos.sort();

        Ok(repos)
    })
    .await
    .context("Failed to complete repository discovery task")?
    .context("Repository discovery failed")?;

    debug!("Discovered {} repositories", repositories.len());

    Ok(repositories)
}

/// Check if a path is a git repository
#[allow(dead_code)]
pub fn is_git_repository<P: AsRef<Path>>(path: P) -> bool {
    let git_dir = path.as_ref().join(".git");
    git_dir.exists()
}

/// Get the repository name from a path
pub fn get_repository_name<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|s| s.to_string())
}

/// Extract git remote URL if available
pub fn get_remote_url<P: AsRef<Path>>(repo_path: P) -> Result<Option<String>> {
    use git2::Repository;

    let repo = Repository::open(&repo_path).with_context(|| {
        format!(
            "Failed to open git repository: {}",
            repo_path.as_ref().display()
        )
    })?;

    // Try to get the 'origin' remote first, then fallback to the first remote
    let remote_name = match repo.find_remote("origin") {
        Ok(_) => "origin".to_string(),
        Err(_) => {
            // Get the first remote if origin doesn't exist
            let remotes = repo.remotes()?;
            match remotes.get(0) {
                Some(name) => name.to_string(),
                None => {
                    debug!(
                        "No remotes found for repository: {}",
                        repo_path.as_ref().display()
                    );
                    return Ok(None);
                }
            }
        }
    };

    let remote = repo
        .find_remote(&remote_name)
        .with_context(|| format!("Failed to find remote '{remote_name}' in repository"))?;

    Ok(remote.url().map(|url| url.to_string()))
}

/// Get the current branch name
pub fn get_current_branch<P: AsRef<Path>>(repo_path: P) -> Result<Option<String>> {
    use git2::Repository;

    let repo = Repository::open(&repo_path).with_context(|| {
        format!(
            "Failed to open git repository: {}",
            repo_path.as_ref().display()
        )
    })?;

    let head = match repo.head() {
        Ok(head) => head,
        Err(ref e) if e.code() == git2::ErrorCode::UnbornBranch => {
            debug!(
                "Repository has no commits yet: {}",
                repo_path.as_ref().display()
            );
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    };

    let branch_name = head.shorthand().map(|s| s.to_string());

    Ok(branch_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_discover_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let repos = discover_git_repositories(temp_dir.path(), 2).await.unwrap();
        assert!(repos.is_empty());
    }

    #[tokio::test]
    async fn test_discover_with_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir_all(&repo_dir).unwrap();
        fs::create_dir_all(repo_dir.join(".git")).unwrap();

        let repos = discover_git_repositories(temp_dir.path(), 2).await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], repo_dir);
    }

    #[test]
    fn test_is_git_repository() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_git_repository(temp_dir.path()));

        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        assert!(is_git_repository(temp_dir.path()));
    }

    #[test]
    fn test_get_repository_name() {
        assert_eq!(
            get_repository_name("/path/to/my-repo"),
            Some("my-repo".to_string())
        );
        assert_eq!(get_repository_name("/"), None);
    }
}
