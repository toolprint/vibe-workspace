use anyhow::Result;
use console::style;

use crate::git::{CloneCommand, GitConfig};
use crate::ui::prompts::{prompt_app_selection, prompt_yes_no};
use crate::ui::state::VibeState;
use crate::workspace::WorkspaceManager;

/// Represents the next action in a workflow
pub enum NextAction {
    /// Workflow is complete
    Complete,
    /// Continue to another workflow
    Continue(Box<dyn Workflow + Send + Sync>),
    /// Suggest actions to the user
    Suggest(Vec<String>),
}

/// Trait for implementing workflows with continuity
pub trait Workflow: Send + Sync {
    /// Execute the workflow and return the next action
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>>;

    /// Get a description of this workflow
    fn description(&self) -> String;
}

/// Clone and open workflow
pub struct CloneAndOpenWorkflow {
    pub url: String,
    pub app: Option<String>,
}

impl Workflow for CloneAndOpenWorkflow {
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>> {
        Box::pin(async move {
            println!("{} Cloning repository...", style("📥").blue());

            // Clone the repository
            let git_config = GitConfig::default();
            let cloned_path = CloneCommand::execute(
                self.url.clone(),
                None,
                false, // Don't open yet
                false, // Don't install yet
                manager,
                &git_config,
            )
            .await?;

            // Extract repo name
            let repo_name = cloned_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Could not determine repository name"))?
                .to_string();

            println!(
                "{} Repository '{}' cloned successfully!",
                style("✓").green().bold(),
                style(&repo_name).cyan()
            );

            // Continue to app configuration
            Ok(NextAction::Continue(Box::new(ConfigureAppWorkflow {
                repo_name,
                suggested_app: self.app.clone(),
                open_after: true,
            })))
        })
    }

    fn description(&self) -> String {
        format!("Clone repository from {}", self.url)
    }
}

/// Configure app for repository workflow
pub struct ConfigureAppWorkflow {
    pub repo_name: String,
    pub suggested_app: Option<String>,
    pub open_after: bool,
}

impl Workflow for ConfigureAppWorkflow {
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>> {
        Box::pin(async move {
            // Check if apps are already configured
            let apps = manager.list_apps_for_repo(&self.repo_name)?;

            if !apps.is_empty() {
                println!(
                    "{} Repository already has {} app{} configured",
                    style("ℹ️").blue(),
                    apps.len(),
                    if apps.len() == 1 { "" } else { "s" }
                );

                if self.open_after {
                    return Ok(NextAction::Continue(Box::new(OpenRepositoryWorkflow {
                        repo_name: self.repo_name.clone(),
                        preferred_app: self.suggested_app.clone(),
                    })));
                } else {
                    return Ok(NextAction::Complete);
                }
            }

            // Prompt to configure app
            println!(
                "\n{} Configure app for '{}'?",
                style("🔧").blue(),
                style(&self.repo_name).cyan()
            );

            if prompt_yes_no("Configure app", true)? {
                let app_name = if let Some(app) = &self.suggested_app {
                    app.clone()
                } else {
                    prompt_app_selection()?
                };

                manager
                    .configure_app_for_repo(&self.repo_name, &app_name, "default")
                    .await?;

                println!(
                    "{} Configured {} for repository",
                    style("✓").green().bold(),
                    style(&app_name).cyan()
                );

                if self.open_after {
                    // Continue to open
                    Ok(NextAction::Continue(Box::new(OpenRepositoryWorkflow {
                        repo_name: self.repo_name.clone(),
                        preferred_app: Some(app_name),
                    })))
                } else {
                    Ok(NextAction::Complete)
                }
            } else {
                // Skip configuration
                if self.open_after {
                    println!("\n{} Opening with default app...", style("🚀").blue());
                    Ok(NextAction::Continue(Box::new(OpenRepositoryWorkflow {
                        repo_name: self.repo_name.clone(),
                        preferred_app: None,
                    })))
                } else {
                    Ok(NextAction::Complete)
                }
            }
        })
    }

    fn description(&self) -> String {
        format!("Configure app for {}", self.repo_name)
    }
}

/// Open repository workflow
pub struct OpenRepositoryWorkflow {
    pub repo_name: String,
    pub preferred_app: Option<String>,
}

impl Workflow for OpenRepositoryWorkflow {
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>> {
        Box::pin(async move {
            // Get repo info
            let repo = manager
                .get_repository(&self.repo_name)
                .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", self.repo_name))?;

            let repo_path = repo.path.clone();

            // Determine app to use
            let app_to_use = if let Some(app) = &self.preferred_app {
                app.clone()
            } else {
                // Get configured apps
                let apps = manager.list_apps_for_repo(&self.repo_name)?;
                if apps.is_empty() {
                    "vscode".to_string() // Default fallback
                } else {
                    apps[0].0.clone()
                }
            };

            // Open the repository
            manager
                .open_repo_with_app(&self.repo_name, &app_to_use)
                .await?;

            // Update state
            let mut state = VibeState::load().unwrap_or_default();
            state.add_recent_repo(self.repo_name.clone(), repo_path, Some(app_to_use.clone()));
            state.save()?;

            println!(
                "{} Opened {} with {}!",
                style("🎉").green().bold(),
                style(&self.repo_name).cyan(),
                style(&app_to_use).blue()
            );

            // Suggest next actions
            Ok(NextAction::Suggest(vec![
                "Run 'vibe' to manage more repositories".to_string(),
                format!("Use 'vibe launch 1' to quickly reopen {}", self.repo_name),
                "Configure additional apps with 'vibe apps configure'".to_string(),
            ]))
        })
    }

    fn description(&self) -> String {
        format!("Open {} repository", self.repo_name)
    }
}

/// Setup workspace workflow (first-time setup)
pub struct SetupWorkspaceWorkflow {
    pub auto_discover: bool,
}

impl Workflow for SetupWorkspaceWorkflow {
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>> {
        Box::pin(async move {
            println!("{}", style("🎉 Setting up your workspace!").cyan().bold());

            // Discover repositories
            if self.auto_discover {
                let workspace_root = manager.get_workspace_root().clone();
                println!("\n{} Discovering repositories...", style("🔍").blue());

                let repos = manager.discover_repositories(&workspace_root, 3).await?;

                if !repos.is_empty() {
                    println!(
                        "{} Found {} repositories!",
                        style("✓").green().bold(),
                        style(repos.len()).cyan()
                    );

                    manager.add_discovered_repositories(&repos).await?;

                    // Continue to app configuration
                    return Ok(NextAction::Continue(Box::new(
                        ConfigureDefaultAppWorkflow {
                            repo_count: repos.len(),
                        },
                    )));
                } else {
                    println!("{} No repositories found in workspace", style("ℹ️").blue());
                }
            }

            // Suggest next actions
            Ok(NextAction::Suggest(vec![
                "Clone a repository with 'vibe go <url>'".to_string(),
                "Search GitHub with 'vibe git search'".to_string(),
                "Manually add repositories with 'vibe git scan'".to_string(),
            ]))
        })
    }

    fn description(&self) -> String {
        "Setup workspace for first-time use".to_string()
    }
}

/// Configure default app workflow
pub struct ConfigureDefaultAppWorkflow {
    pub repo_count: usize,
}

impl Workflow for ConfigureDefaultAppWorkflow {
    fn execute<'a>(
        &'a self,
        manager: &'a mut WorkspaceManager,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NextAction>> + Send + 'a>> {
        Box::pin(async move {
            println!(
                "\n{} Configure a default app for your {} repositories?",
                style("🔧").blue(),
                self.repo_count
            );

            if prompt_yes_no("Configure default app", true)? {
                let app_name = prompt_app_selection()?;

                // Configure for all repositories
                let repo_names: Vec<String> = manager
                    .list_repositories()
                    .iter()
                    .map(|r| r.name.clone())
                    .collect();

                for repo_name in &repo_names {
                    manager
                        .configure_app_for_repo(repo_name, &app_name, "default")
                        .await?;
                }

                println!(
                    "{} Configured {} as default app for all repositories!",
                    style("✓").green().bold(),
                    style(&app_name).cyan()
                );
            }

            // Setup complete
            println!("\n{} Workspace setup complete!", style("✨").green().bold());

            Ok(NextAction::Suggest(vec![
                "Run 'vibe' to start managing repositories".to_string(),
                "Use 'vibe launch' to quickly open recent repos".to_string(),
                "Clone new repos with 'vibe go <url>'".to_string(),
            ]))
        })
    }

    fn description(&self) -> String {
        format!("Configure default app for {} repositories", self.repo_count)
    }
}

/// Execute a workflow and handle continuations
pub async fn execute_workflow(
    workflow: Box<dyn Workflow + Send + Sync>,
    manager: &mut WorkspaceManager,
) -> Result<()> {
    let mut current_workflow = workflow;

    loop {
        println!(
            "\n{} {}",
            style("▶").blue(),
            style(current_workflow.description()).dim()
        );

        match current_workflow.execute(manager).await? {
            NextAction::Complete => {
                println!("\n{} Workflow complete!", style("✓").green().bold());
                break;
            }
            NextAction::Continue(next) => {
                println!("\n{} Continuing workflow...", style("→").cyan());
                current_workflow = next;
            }
            NextAction::Suggest(suggestions) => {
                println!("\n{} Next steps:", style("💡").yellow());
                for suggestion in suggestions {
                    println!("  {} {}", style("•").dim(), suggestion);
                }
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_description() {
        let workflow = CloneAndOpenWorkflow {
            url: "https://github.com/user/repo".to_string(),
            app: Some("vscode".to_string()),
        };

        assert_eq!(
            workflow.description(),
            "Clone repository from https://github.com/user/repo"
        );
    }
}
