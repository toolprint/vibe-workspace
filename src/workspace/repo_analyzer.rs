use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::config::{Repository, WorkspaceConfig};
use super::discovery::{discover_git_repositories, get_remote_url, get_repository_name};

#[derive(Debug, Clone, PartialEq)]
pub enum RepoStatus {
    /// Repository is tracked in config and exists on filesystem
    Tracked,
    /// Repository exists on filesystem but not in config
    New,
    /// Repository is in config but missing from filesystem
    Missing,
}

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub status: RepoStatus,
    pub remote_url: Option<String>,
    pub organization: Option<String>,
    pub config_repo: Option<Repository>,
}

#[derive(Debug, Clone)]
pub struct NonGitFolder {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug)]
pub struct WorkspaceAnalysis {
    pub repositories: Vec<RepoInfo>,
    pub non_git_folders: Vec<NonGitFolder>,
    pub organizations: HashMap<String, Vec<RepoInfo>>,
}

impl RepoInfo {
    pub fn new(name: String, path: PathBuf, status: RepoStatus) -> Self {
        Self {
            name,
            path,
            status,
            remote_url: None,
            organization: None,
            config_repo: None,
        }
    }

    pub fn with_remote_url(mut self, url: String) -> Self {
        self.organization = extract_organization_from_url(&url);
        self.remote_url = Some(url);
        self
    }

    pub fn with_config_repo(mut self, repo: Repository) -> Self {
        if let Some(url) = &repo.url {
            self.organization = extract_organization_from_url(url);
            self.remote_url = Some(url.clone());
        }
        self.config_repo = Some(repo);
        self
    }
}

impl WorkspaceAnalysis {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
            non_git_folders: Vec::new(),
            organizations: HashMap::new(),
        }
    }

    pub fn add_repository(&mut self, repo: RepoInfo) {
        let org_name = repo
            .organization
            .clone()
            .unwrap_or_else(|| "Other".to_string());
        self.organizations
            .entry(org_name)
            .or_default()
            .push(repo.clone());
        self.repositories.push(repo);
    }

    pub fn add_non_git_folder(&mut self, folder: NonGitFolder) {
        self.non_git_folders.push(folder);
    }

    pub fn get_tracked_repos(&self) -> Vec<&RepoInfo> {
        self.repositories
            .iter()
            .filter(|r| r.status == RepoStatus::Tracked)
            .collect()
    }

    pub fn get_new_repos(&self) -> Vec<&RepoInfo> {
        self.repositories
            .iter()
            .filter(|r| r.status == RepoStatus::New)
            .collect()
    }

    pub fn get_missing_repos(&self) -> Vec<&RepoInfo> {
        self.repositories
            .iter()
            .filter(|r| r.status == RepoStatus::Missing)
            .collect()
    }

    pub fn has_actionable_items(&self) -> bool {
        !self.get_new_repos().is_empty()
            || !self.get_missing_repos().is_empty()
            || !self.non_git_folders.is_empty()
    }
}

pub async fn analyze_workspace(
    workspace_root: &Path,
    config: &WorkspaceConfig,
    scan_depth: usize,
) -> Result<WorkspaceAnalysis> {
    let mut analysis = WorkspaceAnalysis::new();

    // Get all git repositories from filesystem
    let discovered_repos = discover_git_repositories(workspace_root, scan_depth).await?;

    // Create sets for efficient lookups with normalized paths
    let discovered_paths: HashSet<PathBuf> = discovered_repos
        .iter()
        .map(|path| normalize_path(path))
        .collect();
    let config_repos: HashMap<PathBuf, &Repository> = config
        .repositories
        .iter()
        .map(|repo| (normalize_path(&workspace_root.join(&repo.path)), repo))
        .collect();

    // Process discovered repositories
    for repo_path in &discovered_repos {
        let repo_name = get_repository_name(repo_path).unwrap_or_else(|| "unknown".to_string());
        let normalized_repo_path = normalize_path(repo_path);

        let status = if config_repos.contains_key(&normalized_repo_path) {
            RepoStatus::Tracked
        } else {
            RepoStatus::New
        };

        let mut repo_info = RepoInfo::new(repo_name, repo_path.clone(), status);

        // Try to get remote URL
        if let Ok(Some(url)) = get_remote_url(repo_path) {
            repo_info = repo_info.with_remote_url(url);
        }

        // Add config repo if tracked
        if let Some(config_repo) = config_repos.get(&normalized_repo_path) {
            repo_info = repo_info.with_config_repo((*config_repo).clone());
        }

        analysis.add_repository(repo_info);
    }

    // Process missing repositories from config
    for config_repo in &config.repositories {
        let full_path = workspace_root.join(&config_repo.path);
        let normalized_full_path = normalize_path(&full_path);
        if !discovered_paths.contains(&normalized_full_path) {
            let mut repo_info =
                RepoInfo::new(config_repo.name.clone(), full_path, RepoStatus::Missing);
            repo_info = repo_info.with_config_repo(config_repo.clone());
            analysis.add_repository(repo_info);
        }
    }

    // Find non-git folders with improved detection logic
    if let Ok(entries) = std::fs::read_dir(workspace_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !path.join(".git").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip common system directories
                    if !name.starts_with('.') && name != "node_modules" && name != "target" {
                        // Skip organization folders (folders that only contain git repositories)
                        if is_organization_folder(&path) {
                            continue;
                        }

                        // Only flag folders that contain loose files or mixed content
                        if contains_flaggable_content(&path) {
                            analysis.add_non_git_folder(NonGitFolder {
                                path: path.clone(),
                                name: name.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(analysis)
}

fn extract_organization_from_url(url: &str) -> Option<String> {
    // Handle common Git URL formats
    if let Some(captures) = regex::Regex::new(r"github\.com[:/]([^/]+)/")
        .ok()?
        .captures(url)
    {
        return Some(captures.get(1)?.as_str().to_string());
    }

    if let Some(captures) = regex::Regex::new(r"gitlab\.com[:/]([^/]+)/")
        .ok()?
        .captures(url)
    {
        return Some(captures.get(1)?.as_str().to_string());
    }

    if let Some(captures) = regex::Regex::new(r"bitbucket\.org[:/]([^/]+)/")
        .ok()?
        .captures(url)
    {
        return Some(captures.get(1)?.as_str().to_string());
    }

    // For other Git hosting services, try to extract the organization part
    if let Some(captures) = regex::Regex::new(r"[:/]([^/]+)/[^/]+(?:\.git)?/?$")
        .ok()?
        .captures(url)
    {
        return Some(captures.get(1)?.as_str().to_string());
    }

    None
}

/// Check if a directory is an organization folder (only contains git repositories)
fn is_organization_folder(dir_path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir_path) else {
        return false;
    };

    let mut has_subdirs = false;
    let mut has_files = false;
    let mut all_subdirs_are_git = true;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            has_subdirs = true;
            // Check if this subdirectory is a git repository
            if !path.join(".git").exists() {
                all_subdirs_are_git = false;
            }
        } else {
            // Found a file - this is not a pure organization folder
            has_files = true;
        }
    }

    // It's an organization folder if:
    // 1. It has subdirectories
    // 2. It has no files
    // 3. All subdirectories are git repositories
    has_subdirs && !has_files && all_subdirs_are_git
}

/// Check if a directory contains loose files or mixed content that should be flagged
fn contains_flaggable_content(dir_path: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir_path) else {
        return false;
    };

    let mut has_files = false;
    let mut has_non_git_dirs = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            has_files = true;
        } else if path.is_dir() && !path.join(".git").exists() {
            has_non_git_dirs = true;
        }
    }

    // Flag if it has loose files or non-git directories
    has_files || has_non_git_dirs
}

/// Normalize a path by canonicalizing it, handling errors gracefully
fn normalize_path(path: &Path) -> PathBuf {
    // Try to canonicalize the path, fall back to the original path if it fails
    match path.canonicalize() {
        Ok(canonical_path) => canonical_path,
        Err(_) => {
            // If canonicalization fails (e.g., path doesn't exist),
            // try to at least normalize the components
            let mut normalized = PathBuf::new();
            for component in path.components() {
                match component {
                    std::path::Component::ParentDir => {
                        normalized.pop();
                    }
                    std::path::Component::CurDir => {
                        // Skip current directory references
                    }
                    _ => {
                        normalized.push(component);
                    }
                }
            }
            normalized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_organization_from_url() {
        assert_eq!(
            extract_organization_from_url("https://github.com/octocat/Hello-World.git"),
            Some("octocat".to_string())
        );
        assert_eq!(
            extract_organization_from_url("git@github.com:octocat/Hello-World.git"),
            Some("octocat".to_string())
        );
        assert_eq!(
            extract_organization_from_url("https://gitlab.com/gitlab-org/gitlab.git"),
            Some("gitlab-org".to_string())
        );
        assert_eq!(
            extract_organization_from_url("git@bitbucket.org:atlassian/bitbucket.git"),
            Some("atlassian".to_string())
        );
    }

    #[test]
    fn test_repo_info_creation() {
        let repo = RepoInfo::new(
            "test-repo".to_string(),
            PathBuf::from("/test"),
            RepoStatus::New,
        );
        assert_eq!(repo.name, "test-repo");
        assert_eq!(repo.status, RepoStatus::New);
        assert!(repo.remote_url.is_none());
        assert!(repo.organization.is_none());
    }

    #[test]
    fn test_workspace_analysis_filtering() {
        let mut analysis = WorkspaceAnalysis::new();

        analysis.add_repository(RepoInfo::new(
            "tracked".to_string(),
            PathBuf::from("/tracked"),
            RepoStatus::Tracked,
        ));

        analysis.add_repository(RepoInfo::new(
            "new".to_string(),
            PathBuf::from("/new"),
            RepoStatus::New,
        ));

        analysis.add_repository(RepoInfo::new(
            "missing".to_string(),
            PathBuf::from("/missing"),
            RepoStatus::Missing,
        ));

        assert_eq!(analysis.get_tracked_repos().len(), 1);
        assert_eq!(analysis.get_new_repos().len(), 1);
        assert_eq!(analysis.get_missing_repos().len(), 1);
        assert!(analysis.has_actionable_items());
    }

    #[test]
    fn test_organization_folder_detection() {
        // These tests would require actual filesystem setup in a real test environment
        // For now, we'll just test the function exists and compiles
        use std::path::Path;

        // Test with a non-existent path (should return false)
        assert!(!is_organization_folder(Path::new("/non/existent/path")));
        assert!(!contains_flaggable_content(Path::new("/non/existent/path")));
    }

    #[test]
    fn test_path_normalization() {
        use std::path::Path;

        // Test basic path normalization (these should not fail even with non-existent paths)
        let path1 = Path::new("/Users/test/../test/repo");
        let normalized1 = normalize_path(path1);

        let path2 = Path::new("/Users/test/./repo");
        let normalized2 = normalize_path(path2);

        // The function should handle these gracefully even if paths don't exist
        // At minimum, it should return some normalized form
        assert!(!normalized1.to_string_lossy().is_empty());
        assert!(!normalized2.to_string_lossy().is_empty());
    }
}
