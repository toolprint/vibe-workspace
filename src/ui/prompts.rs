use anyhow::{Context, Result};
use console;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::path::PathBuf;

use crate::git::{GitConfig, SearchCommand};
use crate::workspace::WorkspaceManager;

pub async fn run_interactive_mode(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    println!("🚀 Welcome to Workspace CLI Interactive Mode!");
    println!();

    loop {
        let action = Select::new(
            "What would you like to do?",
            vec![
                "🚀 Launch app",
                "📁 Manage repos",
                "⚙️ Configure vibes",
                "Exit",
            ],
        )
        .with_starting_cursor(0) // Default to "Launch app"
        .prompt()?;

        match action {
            "🚀 Launch app" => {
                open_repository_with_filter(workspace_manager).await?;
            }
            "📁 Manage repos" => {
                manage_repos_interactive(workspace_manager).await?;
            }
            "⚙️ Configure vibes" => {
                configure_vibes_interactive(workspace_manager).await?;
            }
            "Exit" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

async fn search_and_clone_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    let git_config = GitConfig::default();
    SearchCommand::execute_interactive(workspace_manager, &git_config).await?;
    Ok(())
}

async fn show_status_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    let options = vec![
        "All repositories",
        "Only dirty repositories",
        "Select group",
    ];

    let choice = Select::new("Show status for:", options).prompt()?;

    match choice {
        "All repositories" => {
            workspace_manager.show_status(false, "table", None).await?;
        }
        "Only dirty repositories" => {
            workspace_manager.show_status(true, "table", None).await?;
        }
        "Select group" => {
            // TODO: Implement group selection
            println!("Group selection not yet implemented");
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn discover_repositories_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    let path_input = Text::new("Directory to scan:")
        .with_default(&std::env::current_dir()?.display().to_string())
        .prompt()?;

    let path = PathBuf::from(path_input);

    let depth = Text::new("Maximum depth:")
        .with_default("3")
        .prompt()?
        .parse::<usize>()
        .unwrap_or(3);

    println!("🔍 Discovering repositories...");
    let repos = workspace_manager
        .discover_repositories(&path, depth)
        .await?;

    if repos.is_empty() {
        println!("No repositories found");
        return Ok(());
    }

    println!("Found {} repositories:", repos.len());
    for repo in &repos {
        println!("  📁 {}", repo.display());
    }

    let add_repos = Confirm::new("Add these repositories to workspace?")
        .with_default(true)
        .prompt()?;

    if add_repos {
        workspace_manager
            .add_discovered_repositories(&repos)
            .await?;
        println!("✅ Repositories added to workspace");
    }

    Ok(())
}

async fn sync_repositories_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    let fetch_only = Confirm::new("Fetch only (don't pull)?")
        .with_default(false)
        .prompt()?;

    let prune = Confirm::new("Prune remote tracking branches?")
        .with_default(false)
        .prompt()?;

    workspace_manager
        .sync_repositories(fetch_only, prune, None)
        .await?;

    Ok(())
}

async fn execute_command_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    let command = Text::new("Git command to execute:")
        .with_help_message(
            "Enter git command without 'git' prefix (e.g., 'status', 'pull origin main')",
        )
        .prompt()?;

    let git_command = if command.starts_with("git ") {
        command
    } else {
        format!("git {command}")
    };

    let parallel = Confirm::new("Execute in parallel?")
        .with_default(true)
        .prompt()?;

    workspace_manager
        .execute_command(&git_command, None, None, parallel)
        .await?;

    Ok(())
}

async fn manage_groups_interactive(_workspace_manager: &WorkspaceManager) -> Result<()> {
    println!("🚧 Group management coming soon!");

    let actions = vec![
        "Create new group",
        "Add repositories to group",
        "Remove repositories from group",
        "Delete group",
        "Back to main menu",
    ];

    let _action = Select::new("Group management:", actions).prompt()?;

    // TODO: Implement group management functionality
    println!("This feature will be implemented in a future release");

    Ok(())
}

async fn configure_apps_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    let config = workspace_manager.get_config();

    if config.repositories.is_empty() {
        println!("❌ No repositories configured in workspace");
        return Ok(());
    }

    // Select repository to configure
    let repo_names: Vec<&str> = config
        .repositories
        .iter()
        .map(|r| r.name.as_str())
        .collect();

    let repo_name = Select::new("Select repository to configure:", repo_names)
        .prompt()?
        .to_string();

    // Get current app configuration state
    let current_state = workspace_manager.get_current_app_states(&repo_name)?;

    // Create app selection options with current state
    let mut app_options = Vec::new();

    // Warp option
    let warp_status = if let Some(template) = &current_state.warp {
        format!("warp (template: {template})")
    } else {
        "warp".to_string()
    };
    app_options.push(warp_status);

    // iTerm2 option
    let iterm2_status = if let Some(template) = &current_state.iterm2 {
        format!("iterm2 (template: {template})")
    } else {
        "iterm2".to_string()
    };
    app_options.push(iterm2_status);

    // VS Code option
    let vscode_status = if let Some(template) = &current_state.vscode {
        format!("vscode (template: {template})")
    } else {
        "vscode".to_string()
    };
    app_options.push(vscode_status);

    // WezTerm option
    let wezterm_status = if let Some(template) = &current_state.wezterm {
        format!("wezterm (template: {template})")
    } else {
        "wezterm".to_string()
    };
    app_options.push(wezterm_status);

    // Determine which apps are currently selected (pre-populate with indices)
    let mut default_selections = Vec::new();
    if current_state.warp.is_some() {
        default_selections.push(0);
    }
    if current_state.iterm2.is_some() {
        default_selections.push(1);
    }
    if current_state.vscode.is_some() {
        default_selections.push(2);
    }
    if current_state.wezterm.is_some() {
        default_selections.push(3);
    }

    // Also create display selections for the status display
    let mut display_selections = Vec::new();
    if current_state.warp.is_some() {
        display_selections.push(&app_options[0]);
    }
    if current_state.iterm2.is_some() {
        display_selections.push(&app_options[1]);
    }
    if current_state.vscode.is_some() {
        display_selections.push(&app_options[2]);
    }
    if current_state.wezterm.is_some() {
        display_selections.push(&app_options[3]);
    }

    println!(
        "\n{} Current app configurations for '{}':",
        console::style("📱").blue(),
        console::style(&repo_name).cyan().bold()
    );

    if display_selections.is_empty() {
        println!(
            "  {} No apps currently configured",
            console::style("ℹ️").yellow()
        );
    } else {
        for selection in &display_selections {
            println!("  {} {}", console::style("✓").green(), selection);
        }
    }
    println!();

    // Multi-select apps with pre-populated selections
    let selected_apps = MultiSelect::new("Select apps to configure:", app_options.clone())
        .with_default(&default_selections)
        .with_help_message("Use space to select/deselect, enter to confirm. Pre-selected apps are currently configured.")
        .prompt()?;

    // Quick actions check
    if selected_apps.len() == 4 && display_selections.is_empty() {
        println!(
            "{} Selected all apps for configuration",
            console::style("🚀").blue()
        );
    } else if selected_apps.is_empty() && !display_selections.is_empty() {
        println!("{} All apps will be removed", console::style("⚠️").yellow());
    }

    // Create app selections with template choices
    let mut app_selections = Vec::new();

    for app_name in ["warp", "iterm2", "vscode", "wezterm"] {
        let app_option = app_options
            .iter()
            .find(|opt| opt.starts_with(app_name))
            .unwrap();

        let is_selected = selected_apps.contains(app_option);
        let currently_configured = match app_name {
            "warp" => current_state.warp.is_some(),
            "iterm2" => current_state.iterm2.is_some(),
            "vscode" => current_state.vscode.is_some(),
            "wezterm" => current_state.wezterm.is_some(),
            _ => false,
        };

        let mut template = None;

        // If newly selected (not currently configured), ask for template
        if is_selected && !currently_configured {
            let templates = workspace_manager.list_templates(app_name).await?;

            if templates.is_empty() {
                println!("⚠️  No templates found for {app_name}, using default");
                template = Some("default".to_string());
            } else {
                let mut template_choices = templates.clone();
                template_choices.push("Create new template...".to_string());

                let selected_template = Select::new(
                    &format!("Select template for {app_name}:"),
                    template_choices,
                )
                .prompt()?;

                if selected_template == "Create new template..." {
                    let template_name = Text::new("Template name:").prompt()?;

                    println!("📝 Creating template '{template_name}' from default template");
                    let default_content = workspace_manager.get_default_template(app_name).await?;
                    workspace_manager
                        .save_template(app_name, &template_name, &default_content)
                        .await?;
                    println!("✅ Template created");

                    template = Some(template_name);
                } else {
                    template = Some(selected_template);
                }
            }
        } else if is_selected && currently_configured {
            // Keep existing template for already configured apps
            template = match app_name {
                "warp" => current_state.warp.clone(),
                "iterm2" => current_state.iterm2.clone(),
                "vscode" => current_state.vscode.clone(),
                "wezterm" => current_state.wezterm.clone(),
                _ => None,
            };
        }

        app_selections.push(crate::workspace::AppSelection {
            app: app_name.to_string(),
            selected: is_selected,
            template,
            currently_configured,
        });
    }

    // Apply the configuration changes
    let changes = workspace_manager
        .configure_multiple_apps(&repo_name, app_selections)
        .await?;

    // Display results
    println!(
        "\n{} Configuration changes for '{}':",
        console::style("📊").blue(),
        console::style(&repo_name).cyan().bold()
    );

    if changes.is_empty() {
        println!("  {} No changes made", console::style("ℹ️").yellow());
    } else {
        for change in &changes {
            println!("  {change}");
        }

        println!(
            "\n{} Successfully updated {} app configuration{}",
            console::style("✅").green().bold(),
            changes.len(),
            if changes.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

async fn manage_templates_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    loop {
        let actions = vec![
            "List templates",
            "Create template",
            "Delete template",
            "View template content",
            "Back to main menu",
        ];

        let action = Select::new("Template management:", actions).prompt()?;

        match action {
            "List templates" => {
                let apps = vec!["warp", "iterm2", "vscode", "wezterm"];
                let app = Select::new("Select app:", apps).prompt()?;

                let templates = workspace_manager.list_templates(app).await?;

                if templates.is_empty() {
                    println!("📄 No templates found for {app}");
                } else {
                    println!("📄 Templates for {app}:");
                    for template in templates {
                        println!("  → {template}");
                    }
                }
            }

            "Create template" => {
                let apps = vec!["warp", "iterm2", "vscode", "wezterm"];
                let app = Select::new("Select app:", apps).prompt()?;

                let name = Text::new("Template name:").prompt()?;

                let create_from = Select::new(
                    "Create from:",
                    vec!["Default template", "Existing template", "File"],
                )
                .prompt()?;

                let content = match create_from {
                    "Default template" => workspace_manager.get_default_template(app).await?,
                    "Existing template" => {
                        let templates = workspace_manager.list_templates(app).await?;
                        if templates.is_empty() {
                            println!("⚠️  No existing templates to copy from");
                            continue;
                        }

                        let source = Select::new("Copy from template:", templates).prompt()?;

                        workspace_manager
                            .get_template_manager()
                            .load_template(app, &source)
                            .await?
                    }
                    "File" => {
                        let file_path = Text::new("File path:").prompt()?;

                        std::fs::read_to_string(&file_path)?
                    }
                    _ => unreachable!(),
                };

                workspace_manager
                    .save_template(app, &name, &content)
                    .await?;
                println!("✅ Created template '{name}' for {app}");
            }

            "Delete template" => {
                let apps = vec!["warp", "iterm2", "vscode", "wezterm"];
                let app = Select::new("Select app:", apps).prompt()?;

                let templates = workspace_manager.list_templates(app).await?;

                if templates.is_empty() {
                    println!("📄 No templates found for {app}");
                    continue;
                }

                // Filter out default template
                let deletable: Vec<&str> = templates
                    .iter()
                    .filter(|t| *t != "default")
                    .map(|s| s.as_str())
                    .collect();

                if deletable.is_empty() {
                    println!("⚠️  Only the default template exists (cannot be deleted)");
                    continue;
                }

                let template = Select::new("Select template to delete:", deletable).prompt()?;

                let confirm = Confirm::new(&format!("Delete template '{template}'?"))
                    .with_default(false)
                    .prompt()?;

                if confirm {
                    workspace_manager.delete_template(app, template).await?;
                    println!("✅ Deleted template '{template}'");
                }
            }

            "View template content" => {
                let apps = vec!["warp", "iterm2", "vscode", "wezterm"];
                let app = Select::new("Select app:", apps).prompt()?;

                let templates = workspace_manager.list_templates(app).await?;

                if templates.is_empty() {
                    println!("📄 No templates found for {app}");
                    continue;
                }

                let template = Select::new("Select template to view:", templates).prompt()?;

                let content = workspace_manager
                    .get_template_manager()
                    .load_template(app, &template)
                    .await?;

                println!("\n📄 Template: {app} / {template}\n");
                println!("{content}");
                println!("\n");
            }

            "Back to main menu" => break,

            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

async fn open_repository_with_filter(workspace_manager: &WorkspaceManager) -> Result<()> {
    let repos_with_status = workspace_manager.get_repos_with_apps_and_status().await?;

    if repos_with_status.is_empty() {
        println!("❌ No repositories with configured apps found");
        println!("💡 Configure apps for repositories first using 'Configure vibes'");
        return Ok(());
    }

    // Create all repo display strings for autocomplete
    let all_repos: Vec<String> = repos_with_status
        .iter()
        .map(|repo| repo.display_string.clone())
        .collect();

    // Use Select instead of Text with autocomplete to avoid lifetime issues
    println!("\n🚀 Select a repository to open:");
    println!(
        "   {} repositories available with configured apps",
        repos_with_status.len()
    );

    let selected_display = Select::new("Repository:", all_repos)
        .with_help_message("Use arrow keys to navigate, type to filter")
        .prompt()?;

    // Find the repository that matches the selected display string
    let selected_repo = repos_with_status
        .iter()
        .find(|repo| repo.display_string == selected_display)
        .context("Selected repository not found")?;

    // If multiple apps configured, let user choose
    let app_name = if selected_repo.apps.len() == 1 {
        println!(
            "🚀 Opening '{}' with {}",
            selected_repo.name, selected_repo.apps[0].0
        );
        selected_repo.apps[0].0.clone()
    } else {
        let app_choices: Vec<String> = selected_repo
            .apps
            .iter()
            .map(|(app, template)| format!("{app} (template: {template})"))
            .collect();

        let selected = Select::new(
            &format!("Select app to open '{}' with:", selected_repo.name),
            app_choices,
        )
        .prompt()?;

        // Extract app name from selection
        selected_repo
            .apps
            .iter()
            .find(|(app, _)| selected.starts_with(app))
            .map(|(app, _)| app.clone())
            .unwrap()
    };

    // Open the repository with the selected app
    workspace_manager
        .open_repo_with_app(&selected_repo.name, &app_name)
        .await?;

    Ok(())
}

async fn manage_repos_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    loop {
        let actions = vec![
            "Show repository status",
            "Search & clone from GitHub",
            "Discover new repositories",
            "Sync repositories",
            "Execute command on repositories",
            "Manage groups",
            "Back to main menu",
        ];

        let action = Select::new("📁 Repository Management:", actions).prompt()?;

        match action {
            "Show repository status" => {
                show_status_interactive(workspace_manager).await?;
            }
            "Search & clone from GitHub" => {
                search_and_clone_interactive(workspace_manager).await?;
            }
            "Discover new repositories" => {
                discover_repositories_interactive(workspace_manager).await?;
            }
            "Sync repositories" => {
                sync_repositories_interactive(workspace_manager).await?;
            }
            "Execute command on repositories" => {
                execute_command_interactive(workspace_manager).await?;
            }
            "Manage groups" => {
                manage_groups_interactive(workspace_manager).await?;
            }
            "Back to main menu" => {
                break;
            }
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

async fn configure_vibes_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    loop {
        let actions = vec![
            "Configure apps for repositories",
            "Manage app templates",
            "Factory Reset",
            "Create Backup",
            "Back to main menu",
        ];

        let action = Select::new("⚙️ Configuration Management:", actions).prompt()?;

        match action {
            "Configure apps for repositories" => {
                configure_apps_interactive(workspace_manager).await?;
            }
            "Manage app templates" => {
                manage_templates_interactive(workspace_manager).await?;
            }
            "Factory Reset" => {
                factory_reset_interactive(workspace_manager).await?;
            }
            "Create Backup" => {
                create_backup_interactive(workspace_manager).await?;
            }
            "Back to main menu" => {
                break;
            }
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

async fn factory_reset_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    println!(
        "{} {}",
        console::style("⚠️  Factory Reset").red().bold(),
        console::style("- This will permanently delete ALL configuration").red()
    );
    println!();

    // Ask if user wants to create a backup first
    let create_backup = Confirm::new("Create a backup before resetting?")
        .with_default(true)
        .prompt()?;

    if create_backup {
        println!(
            "{} Creating backup before reset...",
            console::style("💾").blue()
        );

        // Create backup with timestamped name
        match workspace_manager.create_backup(None, None).await {
            Ok(backup_path) => {
                println!(
                    "{} Backup created: {}",
                    console::style("✅").green(),
                    console::style(backup_path.display()).cyan()
                );
                println!();
            }
            Err(e) => {
                println!(
                    "{} Failed to create backup: {}",
                    console::style("❌").red(),
                    e
                );

                let continue_anyway = Confirm::new("Continue with reset without backup?")
                    .with_default(false)
                    .prompt()?;

                if !continue_anyway {
                    println!("{} Vibe Check: make sure you're ready for irreversable change and try again", console::style("🔍").yellow());
                    return Ok(());
                }
                println!();
            }
        }
    }

    // Call the factory reset function with final confirmation skipped (since we handle confirmation flow here)
    workspace_manager
        .factory_reset_with_options(false, true)
        .await?;

    Ok(())
}

async fn create_backup_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    println!(
        "{} {}",
        console::style("💾").blue().bold(),
        console::style("Create Backup Archive").blue().bold()
    );
    println!();

    // Ask for output directory
    let use_custom_dir = Confirm::new("Use custom output directory?")
        .with_default(false)
        .prompt()?;

    let output_dir = if use_custom_dir {
        let default_backup_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".vibe-workspace")
            .join("backups");
        let dir_input = Text::new("Output directory:")
            .with_default(&default_backup_dir.display().to_string())
            .prompt()?;
        Some(PathBuf::from(dir_input))
    } else {
        None
    };

    // Ask for custom backup name
    let use_custom_name = Confirm::new("Use custom backup name?")
        .with_default(false)
        .prompt()?;

    let custom_name = if use_custom_name {
        let name_input = Text::new("Backup name (without .tgz extension):").prompt()?;
        Some(name_input)
    } else {
        None
    };

    // Create the backup
    match workspace_manager
        .create_backup(output_dir, custom_name)
        .await
    {
        Ok(backup_path) => {
            println!();
            println!(
                "{} Backup created successfully!",
                console::style("✅").green().bold()
            );
            println!(
                "{} Location: {}",
                console::style("📍").blue(),
                console::style(backup_path.display()).cyan().bold()
            );
        }
        Err(e) => {
            println!(
                "{} Failed to create backup: {}",
                console::style("❌").red(),
                e
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Note: Interactive tests are difficult to automate
    // These would typically be manual/integration tests

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
