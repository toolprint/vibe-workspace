use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Check if git is available on the system
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if GitHub CLI is available on the system
pub fn is_github_cli_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get git version string
pub fn get_git_version() -> Result<String> {
    let output = Command::new("git").arg("--version").output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get git version");
    }

    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.trim().to_string())
}

/// Validate git repository path
pub fn validate_git_repository<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", path.display());
    }

    let git_dir = path.join(".git");
    if !git_dir.exists() {
        anyhow::bail!("Not a git repository: {}", path.display());
    }

    Ok(())
}

/// Extract repository name from URL
pub fn extract_repo_name_from_url(url: &str) -> Option<String> {
    // Handle both SSH and HTTPS URLs
    let url = url.trim_end_matches(".git");

    if let Some(last_part) = url.split('/').next_back() {
        if !last_part.is_empty() {
            return Some(last_part.to_string());
        }
    }

    None
}

/// Normalize git URL to HTTPS format
pub fn normalize_git_url(url: &str) -> String {
    if url.starts_with("git@github.com:") {
        // Convert SSH to HTTPS
        let repo_path = url.strip_prefix("git@github.com:").unwrap_or(url);
        let repo_path = repo_path.strip_suffix(".git").unwrap_or(repo_path);
        format!("https://github.com/{repo_path}")
    } else if url.starts_with("https://github.com/") {
        // Already HTTPS, just ensure no .git suffix
        url.strip_suffix(".git").unwrap_or(url).to_string()
    } else {
        // Return as-is for other hosts
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_from_url() {
        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/repo.git"),
            Some("repo".to_string())
        );

        assert_eq!(
            extract_repo_name_from_url("git@github.com:user/repo.git"),
            Some("repo".to_string())
        );

        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/repo"),
            Some("repo".to_string())
        );

        assert_eq!(
            extract_repo_name_from_url("invalid-url"),
            Some("invalid-url".to_string())
        );
    }

    #[test]
    fn test_normalize_git_url() {
        assert_eq!(
            normalize_git_url("git@github.com:user/repo.git"),
            "https://github.com/user/repo"
        );

        assert_eq!(
            normalize_git_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );

        assert_eq!(
            normalize_git_url("https://github.com/user/repo"),
            "https://github.com/user/repo"
        );

        assert_eq!(
            normalize_git_url("https://gitlab.com/user/repo.git"),
            "https://gitlab.com/user/repo.git"
        );
    }
}
