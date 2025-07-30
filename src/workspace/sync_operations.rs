use anyhow::{Context, Result};
use console::style;
use std::path::Path;

use super::config::{Repository, WorkspaceConfig};
use super::discovery::get_current_branch;
use super::repo_analyzer::WorkspaceAnalysis;
use crate::git;

pub struct SyncOptions {
    pub import_new: bool,
    pub restore_missing: bool,
    pub clean_missing: bool,
}

impl SyncOptions {
    pub fn new() -> Self {
        Self {
            import_new: false,
            restore_missing: false,
            clean_missing: false,
        }
    }

    pub fn with_import(mut self) -> Self {
        self.import_new = true;
        self
    }

    pub fn with_restore(mut self) -> Self {
        self.restore_missing = true;
        self
    }

    pub fn with_clean(mut self) -> Self {
        self.clean_missing = true;
        self
    }

    pub fn has_actions(&self) -> bool {
        self.import_new || self.restore_missing || self.clean_missing
    }
}

pub async fn execute_sync_operations(
    workspace_root: &Path,
    config: &mut WorkspaceConfig,
    analysis: &WorkspaceAnalysis,
    options: &SyncOptions,
) -> Result<()> {
    let mut changes_made = false;

    if options.import_new {
        changes_made |= import_new_repositories(workspace_root, config, analysis).await?;
    }

    if options.restore_missing {
        changes_made |= restore_missing_repositories(workspace_root, config, analysis).await?;
    }

    if options.clean_missing {
        changes_made |= clean_missing_repositories(config, analysis).await?;
    }

    if changes_made {
        println!(
            "{} Configuration updated successfully",
            style("✓").green().bold()
        );
    }

    Ok(())
}

async fn import_new_repositories(
    workspace_root: &Path,
    config: &mut WorkspaceConfig,
    analysis: &WorkspaceAnalysis,
) -> Result<bool> {
    let new_repos = analysis.get_new_repos();

    if new_repos.is_empty() {
        return Ok(false);
    }

    println!(
        "{} Importing {} new repositories...",
        style("📥").blue(),
        new_repos.len()
    );

    for repo_info in new_repos {
        let relative_path = repo_info
            .path
            .strip_prefix(workspace_root)
            .unwrap_or(&repo_info.path)
            .to_path_buf();

        let mut repo = Repository::new(repo_info.name.clone(), relative_path);

        // Add remote URL if available
        if let Some(url) = &repo_info.remote_url {
            repo = repo.with_url(url.clone());
        }

        // Try to get current branch
        if let Ok(Some(branch)) = get_current_branch(&repo_info.path) {
            repo = repo.with_branch(branch);
        }

        config.add_repository(repo);

        println!(
            "  {} Added {}",
            style("✓").green(),
            style(&repo_info.name).cyan()
        );
    }

    Ok(true)
}

async fn restore_missing_repositories(
    workspace_root: &Path,
    _config: &WorkspaceConfig,
    analysis: &WorkspaceAnalysis,
) -> Result<bool> {
    let missing_repos = analysis.get_missing_repos();

    if missing_repos.is_empty() {
        return Ok(false);
    }

    println!(
        "{} Restoring {} missing repositories...",
        style("🔄").blue(),
        missing_repos.len()
    );

    let _git_config = git::GitConfig::default();

    for repo_info in missing_repos {
        if let Some(config_repo) = &repo_info.config_repo {
            if let Some(url) = &config_repo.url {
                let target_path = workspace_root.join(&config_repo.path);

                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    tokio::fs::create_dir_all(parent).await.with_context(|| {
                        format!("Failed to create parent directory: {}", parent.display())
                    })?;
                }

                println!(
                    "  {} Cloning {} from {}...",
                    style("⬇️").blue(),
                    style(&config_repo.name).cyan(),
                    style(url).dim()
                );

                // Use the existing clone functionality
                match clone_repository(url, &target_path).await {
                    Ok(_) => {
                        println!(
                            "    {} Successfully restored {}",
                            style("✓").green(),
                            style(&config_repo.name).cyan()
                        );
                    }
                    Err(e) => {
                        println!(
                            "    {} Failed to restore {}: {}",
                            style("✗").red(),
                            style(&config_repo.name).cyan(),
                            e
                        );
                    }
                }
            } else {
                println!(
                    "  {} Skipping {} (no remote URL configured)",
                    style("⚠️").yellow(),
                    style(&config_repo.name).cyan()
                );
            }
        }
    }

    Ok(true)
}

async fn clean_missing_repositories(
    config: &mut WorkspaceConfig,
    analysis: &WorkspaceAnalysis,
) -> Result<bool> {
    let missing_repos = analysis.get_missing_repos();

    if missing_repos.is_empty() {
        return Ok(false);
    }

    println!(
        "{} Removing {} missing repositories from config...",
        style("🧹").blue(),
        missing_repos.len()
    );

    for repo_info in missing_repos {
        if let Some(config_repo) = &repo_info.config_repo {
            // Remove from config
            config.repositories.retain(|r| r.name != config_repo.name);

            println!(
                "  {} Removed {}",
                style("✓").green(),
                style(&config_repo.name).cyan()
            );
        }
    }

    Ok(true)
}

// Simple clone implementation - in a real implementation, we'd use the git module
async fn clone_repository(url: &str, target_path: &Path) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(&["clone", url, &target_path.to_string_lossy()])
        .output()
        .with_context(|| "Failed to execute git clone")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {}", error_msg);
    }

    Ok(())
}

pub fn print_sync_summary(analysis: &WorkspaceAnalysis, options: &SyncOptions) {
    if !options.has_actions() {
        return;
    }

    println!("{} Sync Operations Summary", style("📋").blue().bold());
    println!("{}", "─".repeat(40));

    if options.import_new {
        let new_count = analysis.get_new_repos().len();
        if new_count > 0 {
            println!(
                "• {} new repositories will be imported",
                style(new_count).green().bold()
            );
        }
    }

    if options.restore_missing {
        let missing_count = analysis.get_missing_repos().len();
        if missing_count > 0 {
            println!(
                "• {} missing repositories will be restored",
                style(missing_count).blue().bold()
            );
        }
    }

    if options.clean_missing {
        let missing_count = analysis.get_missing_repos().len();
        if missing_count > 0 {
            println!(
                "• {} missing repositories will be removed from config",
                style(missing_count).red().bold()
            );
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_options_creation() {
        let options = SyncOptions::new();
        assert!(!options.import_new);
        assert!(!options.restore_missing);
        assert!(!options.clean_missing);
        assert!(!options.has_actions());
    }

    #[test]
    fn test_sync_options_builder() {
        let options = SyncOptions::new().with_import().with_restore();

        assert!(options.import_new);
        assert!(options.restore_missing);
        assert!(!options.clean_missing);
        assert!(options.has_actions());
    }

    #[test]
    fn test_sync_options_conflict() {
        let options = SyncOptions::new().with_restore().with_clean();

        assert!(options.restore_missing);
        assert!(options.clean_missing);
        // Note: The conflict validation should happen at the CLI level
    }
}
