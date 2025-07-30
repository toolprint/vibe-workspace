use anyhow::{Context, Result};
use console;
use console::style;
use inquire::{Confirm, InquireError, MultiSelect, Select, Text};
use std::path::PathBuf;

use crate::git::{GitConfig, SearchCommand};
use crate::ui::smart_menu::{SmartActionType, SmartMenu};
use crate::ui::state::VibeState;
use crate::workspace::WorkspaceManager;

/// Handle prompt results to distinguish between ESC key navigation and fatal errors
/// Returns:
/// - Ok(Some(value)) for successful prompts
/// - Ok(None) for ESC key cancellation (navigation signal)
/// - Err(error) for other fatal errors
fn handle_prompt_result<T>(result: Result<T, InquireError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled) => {
            // ESC key pressed - treat as navigation signal, not error
            Ok(None)
        }
        Err(error) => {
            // Other errors should be propagated
            Err(anyhow::Error::from(error))
        }
    }
}

// Navigation helper utilities for consistent menu structure
const NAVIGATION_SEPARATOR: &str = "────────────────────";

/// Create a visual separator for navigation options
fn create_navigation_separator() -> String {
    format!("{}", style(NAVIGATION_SEPARATOR).dim())
}

/// Format navigation option with brackets
fn format_navigation_option(text: &str) -> String {
    format!("[{}]", text)
}

/// Create a menu with standardized navigation options
fn create_menu_with_navigation(options: Vec<String>, is_main_menu: bool) -> Vec<String> {
    let mut menu_items = options;

    // Add separator
    menu_items.push(create_navigation_separator());

    // Add appropriate navigation option
    if is_main_menu {
        menu_items.push(format_navigation_option("Exit"));
    } else {
        menu_items.push(format_navigation_option("Back"));
    }

    menu_items
}

/// Check if selection is a navigation option
fn is_navigation_option(selection: &str) -> bool {
    selection.starts_with('[') && selection.ends_with(']')
}

/// Extract navigation action from bracketed option
fn get_navigation_action(selection: &str) -> Option<&str> {
    if is_navigation_option(selection) {
        selection
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
    } else {
        None
    }
}

pub async fn run_menu_mode(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    // Check for first-time setup
    let smart_menu = SmartMenu::new(workspace_manager).await?;
    if smart_menu.should_show_setup_wizard() {
        println!("{}", style("🎉 Welcome to Vibe Workspace!").cyan().bold());
        println!("It looks like this is your first time using Vibe.\n");

        if prompt_yes_no("Would you like to run the setup wizard?", true)? {
            run_setup_wizard(workspace_manager).await?;

            // Update state to mark wizard as complete
            let mut state = VibeState::load().unwrap_or_default();
            state.complete_setup_wizard();
            state.save()?;
        }
        println!();
    } else {
        println!("🚀 Welcome to Vibe Workspace!");
        println!();
    }

    loop {
        // Reload smart menu to get fresh state
        let smart_menu = SmartMenu::new(workspace_manager).await?;

        // Build dynamic menu options
        let mut menu_options = Vec::new();

        // Add quick launch items
        let quick_items = smart_menu.get_quick_launch_items();
        if !quick_items.is_empty() {
            menu_options.push(format!("{}", style("🏃 ── Quick Launch ──").dim()));

            for item in &quick_items {
                let label = format!(
                    "{}. {} {} {}",
                    style(item.number).cyan().bold(),
                    style(&item.repo_name).green(),
                    style(format!("({})", item.last_accessed)).dim(),
                    if let Some(app) = &item.last_app {
                        format!("→ {}", style(app).blue())
                    } else {
                        "".to_string()
                    }
                );
                menu_options.push(label);
            }

            menu_options.push(format!("{}", style("🧠 ── Smart Actions ──").dim()));
        }

        // Add smart actions
        let smart_actions = smart_menu.get_smart_actions();
        for action in &smart_actions {
            menu_options.push(action.label.clone());
        }

        // Add standard menu items
        menu_options.push(format!("{}", style("🏠 ── Main Menu ──").dim()));
        let mut standard_options = vec![
            "🚀 Launch app".to_string(),
            "🔀 Manage repos".to_string(),
            "⚙️ Settings".to_string(),
        ];
        menu_options.append(&mut standard_options);

        // Apply navigation formatting (main menu gets [Exit])
        menu_options = create_menu_with_navigation(menu_options, true);

        // Create the select prompt
        let selection_result = Select::new("What would you like to do?", menu_options)
            .with_starting_cursor(if quick_items.is_empty() { 0 } else { 1 })
            .with_page_size(workspace_manager.get_main_menu_page_size())
            .with_help_message("Use arrow keys to navigate • ESC to exit")
            .prompt();

        let selection = match handle_prompt_result(selection_result)? {
            Some(selection) => selection,
            None => {
                // ESC pressed in main menu - exit
                println!("👋 Goodbye!");
                break;
            }
        };

        // Handle number key shortcuts (1-9) for quick launch
        if let Some(digit) = selection.chars().next().and_then(|c| c.to_digit(10)) {
            if digit >= 1 && digit <= 9 {
                let index = (digit - 1) as usize;
                if index < quick_items.len() {
                    let item = &quick_items[index];
                    launch_repository(workspace_manager, &item.repo_name, item.last_app.as_deref())
                        .await?;
                    continue;
                }
            }
        }

        // Handle smart actions
        let mut handled = false;
        for action in &smart_actions {
            if selection == action.label {
                handle_smart_action(workspace_manager, &action.action_type).await?;
                handled = true;
                break;
            }
        }

        if handled {
            println!();
            continue;
        }

        // Handle navigation options first
        if let Some(action) = get_navigation_action(&selection) {
            match action {
                "Exit" => {
                    println!("👋 Goodbye!");
                    break;
                }
                _ => continue,
            }
        }

        // Handle standard menu items
        match selection.as_str() {
            "🚀 Launch app" => {
                launch_repository_with_cache(workspace_manager).await?;
            }
            "🔀 Manage repos" => {
                manage_repos_interactive(workspace_manager).await?;
            }
            "⚙️ Settings" => {
                configure_vibes_interactive(workspace_manager).await?;
            }
            _ => {
                // Skip section headers and separators
                if selection.contains("──") || selection == NAVIGATION_SEPARATOR {
                    continue;
                }
            }
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
        "All repositories".to_string(),
        "Only dirty repositories".to_string(),
        "Select group".to_string(),
    ];

    let menu_options = create_menu_with_navigation(options, false);
    let choice_result = Select::new("Show status for:", menu_options)
        .with_help_message("Choose status display options • ESC to go back")
        .prompt();

    let choice = match handle_prompt_result(choice_result)? {
        Some(choice) => choice,
        None => {
            // ESC pressed - go back
            return Ok(());
        }
    };

    // Handle navigation
    if choice == format_navigation_option("Back") {
        return Ok(());
    }

    match choice.as_str() {
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
    // First, show options menu
    let options = vec![
        "🔍 Scan current workspace".to_string(),
        "📁 Choose directory to scan".to_string(),
    ];

    let menu_options = create_menu_with_navigation(options, false);
    let choice_result = Select::new("Repository Discovery:", menu_options)
        .with_help_message("Choose discovery method • ESC to go back")
        .prompt();

    let choice = match handle_prompt_result(choice_result)? {
        Some(choice) => choice,
        None => {
            // ESC pressed - go back
            return Ok(());
        }
    };

    // Handle navigation
    if choice == format_navigation_option("Back") {
        return Ok(());
    }

    let path = match choice.as_str() {
        "🔍 Scan current workspace" => workspace_manager.get_workspace_root().clone(),
        "📁 Choose directory to scan" => {
            let path_result = Text::new("Directory to scan:")
                .with_default(&std::env::current_dir()?.display().to_string())
                .with_help_message("Enter directory path • ESC to go back")
                .prompt();

            match handle_prompt_result(path_result)? {
                Some(path_input) => PathBuf::from(path_input),
                None => {
                    // ESC pressed - go back
                    return Ok(());
                }
            }
        }
        _ => return Ok(()),
    };

    let depth_result = Text::new("Maximum depth:")
        .with_default("3")
        .with_help_message("Enter depth (1-10) • ESC to go back")
        .prompt();

    let depth = match handle_prompt_result(depth_result)? {
        Some(depth_input) => depth_input.parse::<usize>().unwrap_or(3),
        None => {
            // ESC pressed - go back
            return Ok(());
        }
    };

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
    // Show sync options menu
    let options = vec![
        "🔄 Full sync (fetch + pull)".to_string(),
        "⬇️ Fetch only".to_string(),
        "🗑️ Sync with prune".to_string(),
        "⚙️ Custom options".to_string(),
    ];

    let menu_options = create_menu_with_navigation(options, false);
    let choice_result = Select::new("Sync Options:", menu_options)
        .with_help_message("Choose synchronization method • ESC to go back")
        .prompt();

    let choice = match handle_prompt_result(choice_result)? {
        Some(choice) => choice,
        None => {
            // ESC pressed - go back
            return Ok(());
        }
    };

    // Handle navigation
    if choice == format_navigation_option("Back") {
        return Ok(());
    }

    let (fetch_only, prune) = match choice.as_str() {
        "🔄 Full sync (fetch + pull)" => (false, false),
        "⬇️ Fetch only" => (true, false),
        "🗑️ Sync with prune" => (false, true),
        "⚙️ Custom options" => {
            let fetch_only_result = Confirm::new("Fetch only (don't pull)?")
                .with_default(false)
                .with_help_message("ESC to go back")
                .prompt();
            let fetch_only = match handle_prompt_result(fetch_only_result)? {
                Some(value) => value,
                None => return Ok(()),
            };

            let prune_result = Confirm::new("Prune remote tracking branches?")
                .with_default(false)
                .with_help_message("ESC to go back")
                .prompt();
            let prune = match handle_prompt_result(prune_result)? {
                Some(value) => value,
                None => return Ok(()),
            };

            (fetch_only, prune)
        }
        _ => return Ok(()),
    };

    workspace_manager
        .sync_repositories(fetch_only, prune, false, None)
        .await?;

    Ok(())
}

async fn execute_command_interactive(workspace_manager: &WorkspaceManager) -> Result<()> {
    // Show command execution options
    let options = vec![
        "📋 Common commands".to_string(),
        "⚡ Custom git command".to_string(),
    ];

    let menu_options = create_menu_with_navigation(options, false);
    let choice_result = Select::new("Command Execution:", menu_options)
        .with_help_message("Choose execution method • ESC to go back")
        .prompt();

    let choice = match handle_prompt_result(choice_result)? {
        Some(choice) => choice,
        None => {
            // ESC pressed - go back
            return Ok(());
        }
    };

    // Handle navigation
    if choice == format_navigation_option("Back") {
        return Ok(());
    }

    let git_command = match choice.as_str() {
        "📋 Common commands" => {
            let commands = vec![
                "status".to_string(),
                "pull".to_string(),
                "push".to_string(),
                "fetch".to_string(),
                "log --oneline -10".to_string(),
            ];

            let cmd_menu = create_menu_with_navigation(commands, false);
            let selected_result = Select::new("Select command:", cmd_menu)
                .with_help_message("Choose a common git command • ESC to go back")
                .prompt();

            let selected = match handle_prompt_result(selected_result)? {
                Some(selected) => selected,
                None => return Ok(()),
            };

            if selected == format_navigation_option("Back") {
                return Ok(());
            }

            format!("git {}", selected)
        }
        "⚡ Custom git command" => {
            let command_result = Text::new("Git command to execute:")
                .with_help_message(
                    "Enter git command without 'git' prefix (e.g., 'status', 'pull origin main') • ESC to go back",
                )
                .prompt();

            let command = match handle_prompt_result(command_result)? {
                Some(command) => command,
                None => return Ok(()),
            };

            if command.starts_with("git ") {
                command
            } else {
                format!("git {command}")
            }
        }
        _ => return Ok(()),
    };

    let parallel_result = Confirm::new("Execute in parallel?")
        .with_default(true)
        .with_help_message("ESC to go back")
        .prompt();

    let parallel = match handle_prompt_result(parallel_result)? {
        Some(parallel) => parallel,
        None => return Ok(()),
    };

    workspace_manager
        .execute_command(&git_command, None, None, parallel)
        .await?;

    Ok(())
}

async fn manage_groups_interactive(_workspace_manager: &WorkspaceManager) -> Result<()> {
    println!("🚧 Group management coming soon!");

    let actions = vec![
        "Create new group".to_string(),
        "Add repositories to group".to_string(),
        "Remove repositories from group".to_string(),
        "Delete group".to_string(),
    ];

    let menu_options = create_menu_with_navigation(actions, false);
    let _action = Select::new("Group management:", menu_options).prompt()?;

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
            "List templates".to_string(),
            "Create template".to_string(),
            "Delete template".to_string(),
            "View template content".to_string(),
        ];

        let menu_options = create_menu_with_navigation(actions, false);
        let action_result = Select::new("Template management:", menu_options)
            .with_help_message("Choose template action • ESC to go back")
            .prompt();

        let action = match handle_prompt_result(action_result)? {
            Some(action) => action,
            None => {
                // ESC pressed - go back
                break;
            }
        };

        // Handle navigation options first
        if let Some(nav_action) = get_navigation_action(&action) {
            match nav_action {
                "Back" => break,
                _ => continue,
            }
        }

        match action.as_str() {
            "List templates" => {
                let apps = vec!["warp", "iterm2", "vscode", "wezterm"];
                let app_result = Select::new("Select app:", apps)
                    .with_help_message("Choose app • ESC to go back")
                    .prompt();
                let app = match handle_prompt_result(app_result)? {
                    Some(app) => app,
                    None => continue,
                };

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

            _ => {
                // Skip separators
                if action.contains("──") || action == NAVIGATION_SEPARATOR {
                    continue;
                }
            }
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
        .with_page_size(workspace_manager.get_repository_list_page_size())
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

/// Fast repository launcher using cached data
async fn launch_repository_with_cache(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    // Get the quick launcher with cache system
    let launcher = workspace_manager.get_quick_launcher().await?;

    // Ensure cache is up to date
    launcher.refresh_cache(workspace_manager).await?;

    // Launch the repository selection UI
    launcher.launch(workspace_manager).await?;

    Ok(())
}

async fn manage_repos_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    loop {
        let actions = vec![
            "Show repository status".to_string(),
            "Search & clone from GitHub".to_string(),
            "Discover new repositories".to_string(),
            "Sync repositories".to_string(),
            "Execute command on repositories".to_string(),
            "Manage groups".to_string(),
        ];

        let menu_options = create_menu_with_navigation(actions, false);

        let action_result = Select::new("📁 Repository Management:", menu_options)
            .with_page_size(workspace_manager.get_management_menus_page_size())
            .with_help_message("Choose repository management action • ESC to go back")
            .prompt();

        let action = match handle_prompt_result(action_result)? {
            Some(action) => action,
            None => {
                // ESC pressed - go back
                break;
            }
        };

        // Handle navigation options first
        if let Some(nav_action) = get_navigation_action(&action) {
            match nav_action {
                "Back" => break,
                _ => continue,
            }
        }

        match action.as_str() {
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
            _ => {
                // Skip separators
                if action.contains("──") || action == NAVIGATION_SEPARATOR {
                    continue;
                }
            }
        }

        println!();
    }

    Ok(())
}

async fn configure_vibes_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    loop {
        let actions = vec![
            "Configure apps for repositories".to_string(),
            "Manage app templates".to_string(),
            "Factory Reset".to_string(),
            "Create Backup".to_string(),
            "Restore from Backup".to_string(),
        ];

        let menu_options = create_menu_with_navigation(actions, false);

        let action_result = Select::new("⚙️ Configuration Management:", menu_options)
            .with_page_size(workspace_manager.get_management_menus_page_size())
            .with_help_message("Choose configuration action • ESC to go back")
            .prompt();

        let action = match handle_prompt_result(action_result)? {
            Some(action) => action,
            None => {
                // ESC pressed - go back
                break;
            }
        };

        // Handle navigation options first
        if let Some(nav_action) = get_navigation_action(&action) {
            match nav_action {
                "Back" => break,
                _ => continue,
            }
        }

        match action.as_str() {
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
            "Restore from Backup" => {
                restore_backup_interactive(workspace_manager).await?;
            }
            _ => {
                // Skip separators
                if action.contains("──") || action == NAVIGATION_SEPARATOR {
                    continue;
                }
            }
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

/// Prompt user for yes/no confirmation
pub fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool> {
    Confirm::new(prompt)
        .with_default(default)
        .prompt()
        .context("Failed to get user confirmation")
}

/// Prompt user to select an app
pub fn prompt_app_selection() -> Result<String> {
    let apps = vec!["vscode", "warp", "iterm2", "wezterm"];
    Select::new("Select an app to configure:", apps)
        .prompt()
        .map(|s| s.to_string())
        .context("Failed to select app")
}

/// Handle a smart action
async fn handle_smart_action(
    workspace_manager: &mut WorkspaceManager,
    action_type: &SmartActionType,
) -> Result<()> {
    match action_type {
        SmartActionType::SetupWorkspace => {
            run_setup_wizard(workspace_manager).await?;
        }
        SmartActionType::DiscoverRepos => {
            discover_repositories_interactive(workspace_manager).await?;
        }
        SmartActionType::ConfigureApps(repos) => {
            configure_apps_for_repos(workspace_manager, repos).await?;
        }
        SmartActionType::InstallApps => {
            crate::apps::run_interactive_installer().await?;
        }
        SmartActionType::CleanupMissing => {
            cleanup_missing_repos(workspace_manager).await?;
        }
        SmartActionType::SyncRepositories => {
            println!("{} Syncing all repositories...", style("🔄").blue());
            workspace_manager
                .sync_repositories(false, true, false, None)
                .await?;
        }
        SmartActionType::CloneAndOpen(_) => {
            search_and_clone_interactive(workspace_manager).await?;
        }
        SmartActionType::OpenRecent(repo_name) => {
            launch_repository(workspace_manager, repo_name, None).await?;
        }
    }
    Ok(())
}

/// Launch a repository with the specified app
async fn launch_repository(
    workspace_manager: &mut WorkspaceManager,
    repo_name: &str,
    app: Option<&str>,
) -> Result<()> {
    // Get repository info and clone the path
    let repo_path = workspace_manager
        .get_repository(repo_name)
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_name))?
        .path
        .clone();

    // Determine which app to use
    let app_to_use = if let Some(app_name) = app {
        app_name.to_string()
    } else {
        // Get configured apps for this repo
        let apps = workspace_manager.list_apps_for_repo(repo_name)?;
        if apps.is_empty() {
            // Prompt to configure an app
            println!(
                "{} No apps configured for '{}'",
                style("⚠️").yellow(),
                style(repo_name).cyan()
            );
            if prompt_yes_no("Would you like to configure an app?", true)? {
                let app_name = prompt_app_selection()?;
                workspace_manager
                    .configure_app_for_repo(repo_name, &app_name, "default")
                    .await?;
                app_name
            } else {
                return Ok(());
            }
        } else if apps.len() == 1 {
            apps[0].0.clone()
        } else {
            // Multiple apps configured, let user choose
            let app_names: Vec<&str> = apps.iter().map(|(name, _)| name.as_str()).collect();
            Select::new("Select app to open with:", app_names)
                .prompt()?
                .to_string()
        }
    };

    // Open the repository
    workspace_manager
        .open_repo_with_app(repo_name, &app_to_use)
        .await?;

    // Update state with this access
    let mut state = VibeState::load().unwrap_or_default();
    state.add_recent_repo(repo_name.to_string(), repo_path, Some(app_to_use.clone()));
    state.save()?;

    println!(
        "{} Opened {} with {}",
        style("✓").green().bold(),
        style(repo_name).cyan(),
        style(&app_to_use).blue()
    );

    Ok(())
}

/// Configure apps for multiple repositories
async fn configure_apps_for_repos(
    workspace_manager: &mut WorkspaceManager,
    repo_names: &[String],
) -> Result<()> {
    println!(
        "{} Configure apps for {} repositories",
        style("⚙️").blue(),
        style(repo_names.len()).cyan()
    );

    // Let user choose an app
    let app_name = prompt_app_selection()?;

    // Configure for all repos
    for repo_name in repo_names {
        workspace_manager
            .configure_app_for_repo(repo_name, &app_name, "default")
            .await?;
        println!(
            "  {} Configured {} for {}",
            style("✓").green(),
            style(&app_name).blue(),
            style(repo_name).cyan()
        );
    }

    println!(
        "\n{} Configured {} for all selected repositories",
        style("✓").green().bold(),
        style(&app_name).blue()
    );

    Ok(())
}

/// Clean up missing repositories
async fn cleanup_missing_repos(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    let repos = workspace_manager.list_repositories();
    let mut missing = Vec::new();
    let workspace_root = workspace_manager.get_workspace_root();

    for repo in repos {
        let full_path = workspace_root.join(&repo.path);
        if !full_path.exists() {
            missing.push(repo.name.clone());
        }
    }

    if missing.is_empty() {
        println!("{} No missing repositories found", style("✓").green());
        return Ok(());
    }

    println!(
        "{} Found {} missing repositories:",
        style("🧹").yellow(),
        style(missing.len()).red()
    );

    for name in &missing {
        println!("  {} {}", style("•").dim(), style(name).red());
    }

    if prompt_yes_no("\nRemove these from configuration?", true)? {
        for name in missing {
            workspace_manager.remove_repository(&name).await?;
        }
        println!(
            "{} Cleaned up missing repositories",
            style("✓").green().bold()
        );
    }

    Ok(())
}

/// Run the first-time setup wizard
pub async fn run_setup_wizard(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    println!("{}", style("🎉 Welcome to Vibe Workspace!").cyan().bold());
    println!("\nLet's set up your workspace. This will only take a minute.\n");

    // Step 1: Workspace discovery
    println!(
        "{}",
        style("Step 1: Discovering repositories").yellow().bold()
    );
    let scan_path = workspace_manager.get_workspace_root().clone();
    println!(
        "📂 Scanning {} for git repositories...",
        style(scan_path.display()).dim()
    );

    let repos = workspace_manager
        .discover_repositories(&scan_path, 3)
        .await?;

    if repos.is_empty() {
        println!("{} No git repositories found.", style("ℹ️").blue());
        println!("You can add repositories later using 'vibe git clone' or 'vibe git search'.\n");
    } else {
        println!(
            "{} Found {} repositories!",
            style("✓").green().bold(),
            style(repos.len()).cyan()
        );

        if prompt_yes_no("Add these repositories to your workspace?", true)? {
            workspace_manager
                .add_discovered_repositories(&repos)
                .await?;
            println!(
                "{} Added repositories to workspace",
                style("✓").green().bold()
            );
        }
    }

    // Step 2: App installation check
    println!(
        "\n{}",
        style("Step 2: Checking installed apps").yellow().bold()
    );
    let available_apps = vec!["vscode", "warp", "iterm2", "wezterm"];
    let mut has_apps = false;

    for app in &available_apps {
        if workspace_manager.is_app_available(app).await {
            println!(
                "  {} {} is installed",
                style("✓").green(),
                style(app).cyan()
            );
            has_apps = true;
        }
    }

    if !has_apps {
        println!("{} No supported apps found.", style("⚠️").yellow());
        if prompt_yes_no("Would you like to install some apps?", true)? {
            crate::apps::run_interactive_installer().await?;
        }
    }

    // Step 3: Default app configuration
    println!(
        "\n{}",
        style("Step 3: Default app configuration").yellow().bold()
    );

    if has_apps && !workspace_manager.list_repositories().is_empty() {
        if prompt_yes_no(
            "Would you like to configure a default app for your repositories?",
            true,
        )? {
            let default_app = prompt_app_selection()?;

            // Configure for all repositories
            let repo_names: Vec<String> = workspace_manager
                .list_repositories()
                .iter()
                .map(|r| r.name.clone())
                .collect();

            for repo_name in repo_names {
                workspace_manager
                    .configure_app_for_repo(&repo_name, &default_app, "default")
                    .await?;
            }

            println!(
                "{} Configured {} as default app for all repositories",
                style("✓").green().bold(),
                style(&default_app).cyan()
            );
        }
    }

    // Step 4: Quick tips
    println!("\n{}", style("✨ Setup complete!").green().bold());
    println!("\nHere are some quick tips to get started:");
    println!(
        "  {} Run {} to launch a repository",
        style("•").dim(),
        style("vibe").cyan()
    );
    println!(
        "  {} Use {} to quickly open recent repos",
        style("•").dim(),
        style("vibe launch 1").cyan()
    );
    println!(
        "  {} Clone and open in one command: {}",
        style("•").dim(),
        style("vibe go owner/repo").cyan()
    );
    println!(
        "  {} Press {} in the menu to open recent repos",
        style("•").dim(),
        style("1-9").cyan()
    );

    println!("\nEnjoy using Vibe! 🚀");

    Ok(())
}

async fn restore_backup_interactive(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    println!(
        "{} {}",
        console::style("📦 Restore from Backup").blue().bold(),
        console::style("- Restore configuration from a previous backup").dim()
    );
    println!();

    // Check if there are any backups available
    let backups = workspace_manager.list_available_backups().await?;

    if backups.is_empty() {
        println!("{} No backup files found.", style("❌").red());
        println!(
            "{} Create a backup first with: vibe config backup",
            style("💡").blue()
        );
        return Ok(());
    }

    println!(
        "{} Found {} backup files:",
        style("📋").blue(),
        backups.len()
    );
    for backup in &backups {
        let size_mb = backup.size as f64 / (1024.0 * 1024.0);
        println!(
            "  {} {} ({:.1} MB)",
            style("→").dim(),
            backup.display_name,
            size_mb
        );
    }
    println!();

    // Ask user to confirm they want to proceed
    let proceed = Confirm::new("Do you want to select a backup to restore?")
        .with_default(false)
        .prompt()?;

    if !proceed {
        println!("{} Restore cancelled", style("✓").green());
        return Ok(());
    }

    // Let the workspace manager handle the interactive selection and restoration
    workspace_manager.restore_from_backup(None, false).await?;

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
