use anyhow::Result;
use colored::*;
use console::style;
use inquire::{Confirm, Select};
use std::path::PathBuf;

use crate::git::{GitConfig, Repository};
use crate::git::provider::github_cli::GitHubCliProvider;
use crate::git::bulk_clone::{BulkCloneCommand, BulkCloneOptions};
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

/// Enhanced clone command with bulk detection capabilities
pub struct EnhancedCloneCommand;

impl EnhancedCloneCommand {
    /// Execute clone with automatic detection of user/org patterns
    pub async fn execute_with_detection(
        url_or_target: String,
        app: Option<String>,
        no_configure: bool,
        no_open: bool,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        let contains_slash = url_or_target.contains('/');
        let is_url = url_or_target.starts_with("http") || url_or_target.starts_with("git@");
        
        // Route based on input pattern
        match (contains_slash, is_url) {
            // Traditional repository URL or owner/repo format
            (true, _) | (false, true) => {
                Self::single_repository_workflow(
                    url_or_target,
                    app,
                    no_configure,
                    no_open,
                    workspace_manager,
                    git_config,
                ).await
            }
            
            // Potential user/org name - check if it exists
            (false, false) => {
                Self::detect_and_route(
                    url_or_target,
                    app,
                    no_configure,
                    no_open,
                    workspace_manager,
                    git_config,
                ).await
            }
        }
    }
    
    /// Detect if target is user/org and route accordingly
    async fn detect_and_route(
        target: String,
        app: Option<String>,
        no_configure: bool,
        no_open: bool,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        println!("🔍 Analyzing '{}'...", style(&target).cyan());
        
        // Initialize GitHub CLI provider
        let github_cli = match GitHubCliProvider::new() {
            Ok(cli) => cli,
            Err(_) => {
                println!("{} GitHub CLI not available, searching repositories...", 
                    style("⚠️").yellow());
                return Self::fallback_to_search(target, workspace_manager, git_config).await;
            }
        };
        
        // Check if target exists as user or organization
        match github_cli.user_or_org_exists(&target).await {
            Ok(true) => {
                // Get repository count
                match github_cli.count_repositories(&target).await {
                    Ok(0) => {
                        println!("{} '{}' has no public repositories.", 
                            style("ℹ️").blue(), 
                            style(&target).cyan()
                        );
                        Self::fallback_to_search(target, workspace_manager, git_config).await
                    }
                    Ok(count) => {
                        Self::interactive_clone_selection(
                            target,
                            count,
                            app,
                            no_configure,
                            no_open,
                            workspace_manager,
                            git_config,
                        ).await
                    }
                    Err(_) => {
                        println!("{} Failed to count repositories for '{}', searching instead...", 
                            style("⚠️").yellow(),
                            style(&target).cyan()
                        );
                        Self::fallback_to_search(target, workspace_manager, git_config).await
                    }
                }
            }
            Ok(false) => {
                println!("🔍 '{}' not found as a GitHub user or organization.", &target);
                println!("🔍 Searching repositories for '{}'...", &target);
                Self::fallback_to_search(target, workspace_manager, git_config).await
            }
            Err(_) => {
                println!("{} Failed to check GitHub, searching repositories instead...", 
                    style("⚠️").yellow()
                );
                Self::fallback_to_search(target, workspace_manager, git_config).await
            }
        }
    }
    
    /// Show interactive options for user/org with repositories
    async fn interactive_clone_selection(
        target: String,
        repo_count: usize,
        _app: Option<String>,
        _no_configure: bool,
        _no_open: bool,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        println!("✅ Found GitHub target '{}' with {} repositories", 
            style(&target).cyan().bold(),
            style(repo_count).green().bold()
        );
        
        let options = vec![
            format!("Clone all {} repositories", repo_count),
            "Search for specific repository".to_string(),
            "Cancel".to_string(),
        ];
        
        let selection = Select::new("What would you like to do?", options)
            .with_help_message("Choose how to proceed with this GitHub target")
            .prompt()?;
        
        match selection.as_str() {
            s if s.starts_with("Clone all") => {
                Self::bulk_clone_workflow(
                    target,
                    workspace_manager,
                    git_config,
                ).await
            }
            "Search for specific repository" => {
                Self::fallback_to_search(target, workspace_manager, git_config).await
            }
            _ => {
                println!("{} Operation cancelled", style("ℹ️").blue());
                Ok(())
            }
        }
    }
    
    /// Execute bulk cloning workflow
    async fn bulk_clone_workflow(
        target: String,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        let options = BulkCloneOptions {
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            skip_existing: true,
            custom_path: None,
            force: false, // Always show confirmation in interactive mode
        };
        
        match BulkCloneCommand::execute(target, options, workspace_manager, git_config).await {
            Ok(result) => {
                println!(
                    "{} Bulk clone completed: {} successful, {} failed",
                    style("✅").green().bold(),
                    result.total_cloned,
                    result.failed.len()
                );
                Ok(())
            }
            Err(e) => {
                println!("{} Bulk clone failed: {}", style("❌").red(), e);
                Err(e)
            }
        }
    }
    
    /// Fallback to repository search when user/org detection fails
    async fn fallback_to_search(
        target: String,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        use crate::git::SearchCommand;
        
        // Use the existing search functionality
        SearchCommand::execute_with_query(&target, workspace_manager, git_config).await
    }
    
    /// Execute single repository clone workflow
    async fn single_repository_workflow(
        url: String,
        app: Option<String>,
        no_configure: bool,
        no_open: bool,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        use crate::ui::workflows::{execute_workflow, CloneWorkflow};
        
        // Use existing workflow system if not skipping steps
        if !no_configure || !no_open {
            let workflow = Box::new(CloneWorkflow {
                url: url.clone(),
                app: app.clone(),
            });
            
            execute_workflow(workflow, workspace_manager).await?;
        } else {
            // Just clone without workflow
            let _cloned_path = CloneCommand::execute(
                url,
                None,
                false,
                false,
                workspace_manager,
                git_config,
            ).await?;
        }
        
        Ok(())
    }
}
