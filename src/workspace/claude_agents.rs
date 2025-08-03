//! Claude Code subagents integration for vibe-workspace
//!
//! This module handles symlink management for Claude Code subagents,
//! allowing users to configure subagents from their workspace.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

use super::config::{ClaudeAgentsIntegration, WorkspaceConfig};

/// Status of the Claude agents symlink
#[derive(Debug, Clone, PartialEq)]
pub enum SymlinkStatus {
    /// Symlink exists and points to the correct source
    Valid,
    /// Symlink exists but points to a different source
    InvalidTarget(PathBuf),
    /// Symlink does not exist
    Missing,
    /// Target exists but is not a symlink
    InvalidType,
    /// Target parent directory does not exist
    ParentMissing,
}

/// Create or update the Claude agents symlink
pub async fn create_symlink(config: &ClaudeAgentsIntegration, workspace_root: &Path) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let source_path = resolve_source_path(&config.source_path, workspace_root);
    let target_path = expand_home_path(&config.target_path)?;

    // Validate source path exists
    if !source_path.exists() {
        anyhow::bail!("Source path does not exist: {}", source_path.display());
    }

    // Ensure target parent directory exists
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    // Remove existing target if it exists
    if target_path.exists() {
        if target_path.is_symlink() {
            fs::remove_file(&target_path).await.with_context(|| {
                format!(
                    "Failed to remove existing symlink: {}",
                    target_path.display()
                )
            })?;
        } else if target_path.is_dir() {
            fs::remove_dir_all(&target_path).await.with_context(|| {
                format!(
                    "Failed to remove existing directory: {}",
                    target_path.display()
                )
            })?;
        } else {
            fs::remove_file(&target_path).await.with_context(|| {
                format!("Failed to remove existing file: {}", target_path.display())
            })?;
        }
    }

    // Create the symlink
    #[cfg(unix)]
    {
        fs::symlink(&source_path, &target_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to create symlink from {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
    }

    #[cfg(windows)]
    {
        if source_path.is_dir() {
            fs::symlink_dir(&source_path, &target_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create directory symlink from {} to {}",
                        source_path.display(),
                        target_path.display()
                    )
                })?;
        } else {
            fs::symlink_file(&source_path, &target_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to create file symlink from {} to {}",
                        source_path.display(),
                        target_path.display()
                    )
                })?;
        }
    }

    Ok(())
}

/// Remove the Claude agents symlink
pub async fn remove_symlink(config: &ClaudeAgentsIntegration) -> Result<()> {
    let target_path = expand_home_path(&config.target_path)?;

    if target_path.exists() && target_path.is_symlink() {
        fs::remove_file(&target_path)
            .await
            .with_context(|| format!("Failed to remove symlink: {}", target_path.display()))?;
    }

    Ok(())
}

/// Check the status of the Claude agents symlink
pub async fn check_symlink_status(
    config: &ClaudeAgentsIntegration,
    workspace_root: &Path,
) -> Result<SymlinkStatus> {
    let source_path = resolve_source_path(&config.source_path, workspace_root);
    let target_path = expand_home_path(&config.target_path)?;

    if !target_path.exists() {
        // Check if parent directory exists
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                return Ok(SymlinkStatus::ParentMissing);
            }
        }
        return Ok(SymlinkStatus::Missing);
    }

    if !target_path.is_symlink() {
        return Ok(SymlinkStatus::InvalidType);
    }

    // Check if symlink points to the correct target
    let link_target = fs::read_link(&target_path)
        .await
        .with_context(|| format!("Failed to read symlink: {}", target_path.display()))?;

    let resolved_target = if link_target.is_absolute() {
        link_target
    } else {
        target_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(link_target)
    };

    let canonical_source = fs::canonicalize(&source_path).await.with_context(|| {
        format!(
            "Failed to canonicalize source path: {}",
            source_path.display()
        )
    })?;

    let canonical_target = fs::canonicalize(&resolved_target).await.with_context(|| {
        format!(
            "Failed to canonicalize target path: {}",
            resolved_target.display()
        )
    })?;

    if canonical_source == canonical_target {
        Ok(SymlinkStatus::Valid)
    } else {
        Ok(SymlinkStatus::InvalidTarget(resolved_target))
    }
}

/// Validate that the Claude agents configuration paths are valid
pub async fn validate_paths(config: &ClaudeAgentsIntegration, workspace_root: &Path) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let source_path = resolve_source_path(&config.source_path, workspace_root);

    if !source_path.exists() {
        anyhow::bail!(
            "Claude agents source path does not exist: {}",
            source_path.display()
        );
    }

    if !source_path.is_dir() {
        anyhow::bail!(
            "Claude agents source path is not a directory: {}",
            source_path.display()
        );
    }

    // Check if source contains agent files
    let mut has_agents = false;
    let mut entries = fs::read_dir(&source_path)
        .await
        .with_context(|| format!("Failed to read source directory: {}", source_path.display()))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            has_agents = true;
            break;
        }
    }

    if !has_agents {
        eprintln!(
            "Warning: Claude agents source directory appears to be empty: {}",
            source_path.display()
        );
    }

    Ok(())
}

/// Configure Claude agents by creating the symlink
pub async fn configure_claude_agents(config: &WorkspaceConfig) -> Result<()> {
    if let Some(claude_agents) = &config.claude_agents {
        if claude_agents.enabled {
            create_symlink(claude_agents, &config.workspace.root).await?;
        }
    }
    Ok(())
}

/// Get detailed status information about Claude agents configuration
pub async fn get_status_info(
    config: &ClaudeAgentsIntegration,
    workspace_root: &Path,
) -> Result<String> {
    let source_path = resolve_source_path(&config.source_path, workspace_root);
    let target_path = expand_home_path(&config.target_path)?;

    let mut info = Vec::new();

    info.push(format!("Enabled: {}", config.enabled));
    info.push(format!("Source: {}", source_path.display()));
    info.push(format!("Target: {}", target_path.display()));

    if config.enabled {
        let status = check_symlink_status(config, workspace_root).await?;
        let status_str = match status {
            SymlinkStatus::Valid => "✅ Valid symlink".to_string(),
            SymlinkStatus::Missing => "❌ Symlink missing".to_string(),
            SymlinkStatus::InvalidTarget(actual) => {
                format!("⚠️  Symlink points to wrong target: {}", actual.display())
            }
            SymlinkStatus::InvalidType => "⚠️  Target exists but is not a symlink".to_string(),
            SymlinkStatus::ParentMissing => "⚠️  Target parent directory missing".to_string(),
        };
        info.push(format!("Status: {status_str}"));

        // Count agent files
        if source_path.exists() {
            match count_agent_files(&source_path).await {
                Ok(count) => info.push(format!("Agent files: {count}")),
                Err(e) => info.push(format!("Error counting agents: {e}")),
            }
        }
    }

    Ok(info.join("\n"))
}

/// Resolve a potentially relative source path against the workspace root
fn resolve_source_path(source_path: &Path, workspace_root: &Path) -> PathBuf {
    if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        workspace_root.join(source_path)
    }
}

/// Expand ~ in path to home directory
fn expand_home_path(path: &Path) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path: {:?}", path))?;

    if path_str.starts_with("~/") {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join(&path_str[2..]))
    } else if path_str == "~" {
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))
    } else {
        Ok(path.to_path_buf())
    }
}

/// Count the number of agent files (*.md) in the source directory
async fn count_agent_files(source_path: &Path) -> Result<usize> {
    let mut count = 0;
    let mut entries = fs::read_dir(source_path)
        .await
        .with_context(|| format!("Failed to read directory: {}", source_path.display()))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_expand_home_path() {
        let home = dirs::home_dir().unwrap();

        let expanded = expand_home_path(Path::new("~/.claude/agents")).unwrap();
        assert_eq!(expanded, home.join(".claude/agents"));

        let expanded = expand_home_path(Path::new("~")).unwrap();
        assert_eq!(expanded, home);

        let expanded = expand_home_path(Path::new("/absolute/path")).unwrap();
        assert_eq!(expanded, PathBuf::from("/absolute/path"));
    }

    #[tokio::test]
    async fn test_resolve_source_path() {
        let workspace_root = Path::new("/workspace");

        let resolved = resolve_source_path(Path::new("agents"), workspace_root);
        assert_eq!(resolved, PathBuf::from("/workspace/agents"));

        let resolved = resolve_source_path(Path::new("/absolute/agents"), workspace_root);
        assert_eq!(resolved, PathBuf::from("/absolute/agents"));
    }

    #[tokio::test]
    async fn test_symlink_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path();

        // Create source directory with an agent file
        let source_dir = workspace_root.join("wshobson/agents");
        fs::create_dir_all(&source_dir).await?;
        fs::write(source_dir.join("test-agent.md"), "# Test Agent").await?;

        // Create target directory
        let target_dir = workspace_root.join(".claude");
        fs::create_dir_all(&target_dir).await?;
        let target_path = target_dir.join("agents");

        let config = ClaudeAgentsIntegration {
            enabled: true,
            source_path: PathBuf::from("wshobson/agents"),
            target_path: target_path.clone(),
        };

        // Test symlink creation
        create_symlink(&config, workspace_root).await?;

        // Test status check
        let status = check_symlink_status(&config, workspace_root).await?;
        assert_eq!(status, SymlinkStatus::Valid);

        // Test symlink removal
        remove_symlink(&config).await?;

        let status = check_symlink_status(&config, workspace_root).await?;
        assert_eq!(status, SymlinkStatus::Missing);

        Ok(())
    }
}
