use anyhow::Result;
use colored::*;
use std::path::PathBuf;

use crate::git::{GitConfig, Repository};
use crate::workspace::install::RepositoryInstaller;
use crate::workspace::manager::WorkspaceManager;

pub struct CloneCommand;

impl CloneCommand {
    pub async fn execute(
        url: String,
        path: Option<PathBuf>,
        open: bool,
        install: bool,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        // Get workspace root from manager
        let workspace_root = workspace_manager.config().workspace.root.clone();

        // Create installer
        let installer = RepositoryInstaller::new(workspace_root, git_config.clone());

        // Clone repository
        let installed = installer
            .install_from_url_with_options(&url, path, open, install)
            .await?;

        // Add to workspace configuration
        workspace_manager
            .add_repository(installed.repository.clone())
            .await?;

        // Execute post-install actions
        if !installed.post_install_actions.is_empty() {
            installer
                .execute_post_install_actions(&installed.post_install_actions, &installed.path)
                .await?;
        }

        println!(
            "\n{} Repository successfully added to workspace!",
            "🎉".green()
        );

        println!("Path: {}", installed.path.display().to_string().cyan());

        Ok(())
    }

    pub async fn clone_from_search_result(
        repo: Repository,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        println!(
            "\n{} Selected: {}",
            "✅".green(),
            repo.full_name.cyan().bold()
        );

        // Use HTTPS URL by default (more universal than SSH)
        Self::execute(repo.url, None, false, false, workspace_manager, git_config).await
    }
}
