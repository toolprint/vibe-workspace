use anyhow::Result;
use colored::*;
use console::style;
use inquire::{Confirm, Select};
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
    ) -> Result<PathBuf> {
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

        Ok(installed.path)
    }

    /// Execute clone with interactive post-clone workflow
    pub async fn execute_interactive(
        url: String,
        path: Option<PathBuf>,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<PathBuf> {
        // Clone the repository first
        let cloned_path = Self::execute(
            url.clone(),
            path,
            false,
            false,
            workspace_manager,
            git_config,
        )
        .await?;

        // Extract repository name from path
        let repo_name = cloned_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine repository name"))?;

        // Run interactive post-clone workflow
        Self::interactive_post_clone_workflow(repo_name, workspace_manager).await?;

        Ok(cloned_path)
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
        let _cloned_path =
            Self::execute(repo.url, None, false, false, workspace_manager, git_config).await?;

        // Run interactive post-clone workflow
        Self::interactive_post_clone_workflow(&repo.name, workspace_manager).await?;

        Ok(())
    }

    /// Interactive workflow after cloning a repository
    pub async fn interactive_post_clone_workflow(
        repo_name: &str,
        workspace_manager: &mut WorkspaceManager,
    ) -> Result<()> {
        println!("\n{} Repository cloned successfully!", style("🎉").green());

        // Step 1: Ask if they want to configure apps
        let configure_apps = Confirm::new(&format!(
            "Would you like to configure apps for '{}'?",
            style(repo_name).cyan().bold()
        ))
        .with_default(true)
        .with_help_message(
            "Configure which applications can open this repository (VS Code, Warp, etc.)",
        )
        .prompt()?;

        if configure_apps {
            Self::configure_repository_apps(repo_name, workspace_manager).await?;
        }

        // Step 2: Ask if they want to open it now
        let open_now = Confirm::new(&format!(
            "Would you like to open '{}' now?",
            style(repo_name).cyan().bold()
        ))
        .with_default(true)
        .with_help_message("Open the repository with your configured app")
        .prompt()?;

        if open_now {
            Self::open_repository_interactive(repo_name, workspace_manager).await?;
        }

        Ok(())
    }

    /// Configure apps for a repository interactively
    async fn configure_repository_apps(
        repo_name: &str,
        workspace_manager: &mut WorkspaceManager,
    ) -> Result<()> {
        // Get available app choices (hardcoded for now, could be made configurable)
        let available_apps = [
            ("vscode", "Visual Studio Code - Code editor"),
            ("warp", "Warp - Modern terminal"),
            ("iterm2", "iTerm2 - Terminal emulator"),
        ];

        println!(
            "\n{} Select an application to configure for this repository:",
            style("📱").green()
        );

        let app_choices: Vec<String> = available_apps
            .iter()
            .map(|(name, desc)| format!("{name} - {desc}"))
            .collect();

        let selected_display = Select::new("Choose an application:", app_choices)
            .with_help_message("Select an application to configure for this repository")
            .prompt()?;

        // Extract app name from the display string
        let app_name = selected_display
            .split(" - ")
            .next()
            .unwrap_or(&selected_display);

        // Configure the app for this repository using the existing method
        workspace_manager
            .configure_app_for_repo(repo_name, app_name, "default")
            .await?;

        println!(
            "{} Configured {} for {}",
            style("✅").green(),
            style(app_name).blue(),
            style(repo_name).cyan()
        );

        Ok(())
    }

    /// Open repository interactively with app selection
    async fn open_repository_interactive(
        repo_name: &str,
        workspace_manager: &mut WorkspaceManager,
    ) -> Result<()> {
        // Get the repository configuration
        if let Some(repo_info) = workspace_manager.get_repository(repo_name) {
            if repo_info.apps.is_empty() {
                println!(
                    "{} No apps configured for this repository",
                    style("⚠️").yellow()
                );
                println!("   Configure apps first using the configuration workflow");
                return Ok(());
            }

            // If only one app is configured, use it directly
            let app_to_use = if repo_info.apps.len() == 1 {
                repo_info.apps.keys().next().unwrap().clone()
            } else {
                // Multiple apps, let user choose
                let app_choices: Vec<String> = repo_info.apps.keys().cloned().collect();
                Select::new("Choose an app to open with:", app_choices)
                    .with_help_message("Select which application to use")
                    .prompt()?
            };

            // Open the repository
            workspace_manager
                .open_repo_with_app(repo_name, &app_to_use)
                .await?;

            println!(
                "{} Opened {} with {}",
                style("🚀").green(),
                style(repo_name).cyan().bold(),
                style(&app_to_use).blue()
            );
        } else {
            println!(
                "{} Repository '{}' not found in workspace",
                style("❌").red(),
                repo_name
            );
        }

        Ok(())
    }
}
