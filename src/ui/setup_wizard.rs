//! Enhanced setup wizard for first-time vibe users
//! Provides clear explanations and guided workflow

use anyhow::Result;
use console::style;
use inquire::{Select, Text};
use std::path::PathBuf;

use crate::workspace::WorkspaceManager;
use crate::{display_println, ui::prompts::prompt_yes_no};

/// Run the enhanced setup wizard
pub async fn run_enhanced_setup_wizard(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    // Show welcome message with clear explanation
    show_welcome_message();

    // Step 1: Choose workspace root and discover existing repos
    let (workspace_root, discovered_repos) = choose_workspace_root(workspace_manager).await?;

    // Initialize workspace with chosen root
    let workspace_name = workspace_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string();

    workspace_manager
        .init_workspace(&workspace_name, &workspace_root)
        .await?;

    display_println!(
        "\n{} Workspace initialized at: {}",
        style("✓").green().bold(),
        style(workspace_root.display()).cyan()
    );

    // Step 2: Automatically branch based on discovered repositories
    if discovered_repos.is_empty() {
        // No existing repos found - fresh start workflow
        display_println!(
            "\n{} {}",
            style("🚀 Fresh Start").green().bold(),
            style("Ready to clone your first repositories!").dim()
        );
        run_fresh_start_workflow(workspace_manager).await?;
    } else {
        // Existing repos found - manage existing workflow
        display_println!(
            "\n{} {}",
            style("📂 Found Existing Repositories").green().bold(),
            style("Setting up repository management...").dim()
        );
        run_existing_repos_workflow_with_discovered(workspace_manager, &discovered_repos).await?;
    }

    // Show next steps
    show_next_steps();

    Ok(())
}

/// Display welcome message with clear explanation
fn show_welcome_message() {
    display_println!("\n{}", style("Welcome to Vibe Workspace! 🚀").cyan().bold());
    display_println!();
    display_println!("Vibe helps you manage multiple git repositories from a single place.");
    display_println!("Think of it as a smart launcher for your development projects.");
    display_println!();
    display_println!("First, let's set up your workspace root - this is where all your");
    display_println!("repositories will be organized. We'll automatically detect if you");
    display_println!("have existing repositories or need to start fresh.");
    display_println!();
}

/// Choose workspace root directory with smart defaults
/// Returns (workspace_root, discovered_repos)
async fn choose_workspace_root(
    workspace_manager: &WorkspaceManager,
) -> Result<(PathBuf, Vec<PathBuf>)> {
    // Get home directory
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    // Smart default suggestions
    let mut suggestions = vec![];

    // Check common development directories (ordered by popularity)
    let common_dirs = [
        "Projects",
        "Code",
        "workspace",
        "src",
        "dev",
        "Development",
        "repos",
        "code",
        "work",
        "git",
        "source",
    ];

    for dir_name in &common_dirs {
        let path = home.join(dir_name);
        if path.exists() && path.is_dir() {
            suggestions.push(path);
        }
    }

    // Add sensible fallbacks (even if they don't exist yet)
    if suggestions.is_empty() {
        suggestions.push(home.join("Projects"));
        suggestions.push(home.join("Code"));
        suggestions.push(home.join("workspace"));
    } else if suggestions.len() < 3 {
        // Add a few more good options if we found some but not many
        for fallback in &["Projects", "Code", "workspace"] {
            let path = home.join(fallback);
            if !suggestions.contains(&path) {
                suggestions.push(path);
                if suggestions.len() >= 4 {
                    // Don't overwhelm with too many options
                    break;
                }
            }
        }
    }

    // Build options
    let mut options: Vec<String> = suggestions
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    options.push("Choose a different directory...".to_string());

    let selection = Select::new("Where should vibe manage your repositories?", options)
        .with_help_message("This will be your workspace root directory")
        .prompt()?;

    let workspace_root = if selection == "Choose a different directory..." {
        // Use text input with helpful guidance (file path completion not available in inquire 0.7)
        let custom_path = Text::new("Enter workspace root path:")
            .with_placeholder("e.g., /Users/you/Code or ~/workspace")
            .with_help_message("Common paths: ~/Projects, ~/Code, ~/workspace, ~/dev, ~/src")
            .prompt()?;

        // Handle tilde expansion
        let expanded_path = if custom_path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(&custom_path[2..])
            } else {
                PathBuf::from(custom_path)
            }
        } else {
            PathBuf::from(custom_path)
        };

        expanded_path
    } else {
        PathBuf::from(selection)
    };

    // Ensure directory exists
    if !workspace_root.exists() {
        display_println!(
            "\n{} Directory doesn't exist: {}",
            style("ℹ️").blue(),
            workspace_root.display()
        );

        if prompt_yes_no("Create this directory?", true)? {
            std::fs::create_dir_all(&workspace_root)?;
            display_println!("{} Directory created", style("✓").green().bold());
        } else {
            // Recurse to choose again
            return Box::pin(choose_workspace_root(workspace_manager)).await;
        }
    }

    // Preview directory contents and ask for confirmation
    let (confirmed, discovered_repos) =
        preview_workspace_directory(workspace_manager, &workspace_root).await?;

    if !confirmed {
        // User wants to choose a different directory
        return Box::pin(choose_workspace_root(workspace_manager)).await;
    }

    // Show what will happen
    display_println!(
        "\n{} Repositories will be cloned to: {}/[owner]/[repo-name]",
        style("📁").blue(),
        workspace_root.display()
    );

    Ok((workspace_root, discovered_repos))
}

/// Run fresh start workflow: Clone → Configure → Launch
async fn run_fresh_start_workflow(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    // Guide to clone a repository
    display_println!(
        "\n{} How to clone your first repository:",
        style("📚").blue()
    );
    display_println!(
        "  {} {} - Clone and set up any GitHub repository",
        style("•").dim(),
        style("vibe go owner/repo").cyan()
    );
    display_println!(
        "  {} {} - Search GitHub for repositories",
        style("•").dim(),
        style("vibe git search [query]").cyan()
    );
    display_println!(
        "  {} {} - Clone by full URL",
        style("•").dim(),
        style("vibe git clone [url]").cyan()
    );

    // Check for available apps
    check_and_explain_apps(workspace_manager).await?;

    Ok(())
}

/// Run existing repos workflow with already discovered repos
async fn run_existing_repos_workflow_with_discovered(
    workspace_manager: &mut WorkspaceManager,
    discovered_repos: &[PathBuf],
) -> Result<()> {
    // Show preview of already discovered repos
    display_println!(
        "\n{} Found {} repositories:",
        style("✓").green().bold(),
        style(discovered_repos.len()).cyan()
    );

    for (i, repo) in discovered_repos.iter().enumerate() {
        if i < 10 {
            let repo_name = repo
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            display_println!("  {} {}", style("•").dim(), style(repo_name).green());
        } else if i == 10 {
            display_println!(
                "  {} ... and {} more",
                style("•").dim(),
                discovered_repos.len() - 10
            );
            break;
        }
    }

    // Add repositories automatically (user already confirmed in directory preview)
    display_println!(
        "\nAdding your {} repositories to vibe workspace...",
        discovered_repos.len()
    );

    workspace_manager
        .add_discovered_repositories(discovered_repos)
        .await?;
    display_println!(
        "{} Added {} repositories!",
        style("✓").green().bold(),
        discovered_repos.len()
    );

    // Offer to configure default app
    configure_default_app(workspace_manager).await?;

    Ok(())
}

/// Run existing repos workflow: Scan → Preview → Configure (legacy - kept for compatibility)
async fn run_existing_repos_workflow(
    workspace_manager: &mut WorkspaceManager,
    workspace_root: &PathBuf,
) -> Result<()> {
    display_println!(
        "\n{} {}",
        style("📂 Existing Repositories").green().bold(),
        style("Let's find your repositories!").dim()
    );

    // Ask where to scan
    let scan_root = if prompt_yes_no(
        &format!("Scan {} for repositories?", workspace_root.display()),
        true,
    )? {
        workspace_root.clone()
    } else {
        // Choose different directory
        let custom_path = Text::new("Enter directory to scan:")
            .with_placeholder(&workspace_root.display().to_string())
            .with_help_message("Press Tab for path completion")
            .prompt()?;

        PathBuf::from(custom_path)
    };

    // Scan for repositories
    display_println!(
        "\n{} Scanning {} for git repositories...",
        style("🔍").blue(),
        style(scan_root.display()).dim()
    );

    let repos = workspace_manager
        .discover_repositories(&scan_root, 3)
        .await?;

    if repos.is_empty() {
        display_println!(
            "{} No git repositories found in {}",
            style("ℹ️").blue(),
            scan_root.display()
        );
        display_println!("You can add repositories later using the clone commands.");
        return Ok(());
    }

    // Show preview
    display_println!(
        "\n{} Found {} repositories:",
        style("✓").green().bold(),
        style(repos.len()).cyan()
    );

    for (i, repo) in repos.iter().enumerate() {
        if i < 10 {
            let repo_name = repo
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            display_println!("  {} {}", style("•").dim(), style(repo_name).green());
        } else if i == 10 {
            display_println!("  {} ... and {} more", style("•").dim(), repos.len() - 10);
            break;
        }
    }

    // Confirm import
    if prompt_yes_no("\nAdd these repositories to vibe?", true)? {
        workspace_manager
            .add_discovered_repositories(&repos)
            .await?;
        display_println!(
            "{} Added {} repositories!",
            style("✓").green().bold(),
            repos.len()
        );

        // Offer to configure default app
        configure_default_app(workspace_manager).await?;
    }

    Ok(())
}

/// Check for available apps and explain the concept
async fn check_and_explain_apps(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    display_println!("\n{}", style("🛠️  Development Tools").yellow().bold());
    display_println!("Vibe can open your repositories in different development tools:");
    display_println!(
        "  {} {} - Code editor",
        style("•").dim(),
        style("VS Code").blue()
    );
    display_println!(
        "  {} {} - Terminal emulator",
        style("•").dim(),
        style("iTerm2/Warp").blue()
    );
    display_println!(
        "  {} {} - AI-powered editors",
        style("•").dim(),
        style("Cursor/Windsurf").blue()
    );

    // Check what's installed
    let apps = vec!["vscode", "warp", "iterm2", "wezterm", "cursor", "windsurf"];
    let mut available_apps = vec![];

    display_println!("\n{} Checking installed apps...", style("🔍").blue());

    for app in &apps {
        if workspace_manager.is_app_available(app).await {
            display_println!(
                "  {} {} is installed",
                style("✓").green(),
                style(app).cyan()
            );
            available_apps.push(*app);
        }
    }

    if available_apps.is_empty() {
        display_println!("\n{} No supported apps found.", style("⚠️").yellow());
        display_println!("You can install them later and configure with:");
        display_println!("  {}", style("vibe apps install").cyan());
    }

    Ok(())
}

/// Configure default app for repositories
async fn configure_default_app(workspace_manager: &mut WorkspaceManager) -> Result<()> {
    // Check available apps
    let apps = vec!["vscode", "warp", "iterm2", "wezterm", "cursor", "windsurf"];
    let mut available_apps = vec![];

    for app in &apps {
        if workspace_manager.is_app_available(app).await {
            available_apps.push(*app);
        }
    }

    if available_apps.is_empty() {
        return Ok(());
    }

    display_println!(
        "\n{}",
        style("🔧 Default App Configuration").yellow().bold()
    );

    if prompt_yes_no("Configure a default app for your repositories?", true)? {
        let app_name = if available_apps.len() == 1 {
            available_apps[0].to_string()
        } else {
            let selection = Select::new(
                "Select default app:",
                available_apps.iter().map(|a| a.to_string()).collect(),
            )
            .with_help_message("This app will be used to open repositories by default")
            .prompt()?;
            selection
        };

        // Ensure default template exists
        ensure_default_template(workspace_manager, &app_name).await?;

        // Configure for all repositories
        let repo_names: Vec<String> = workspace_manager
            .list_repositories()
            .iter()
            .map(|r| r.name.clone())
            .collect();

        for repo_name in &repo_names {
            workspace_manager
                .configure_app_for_repo(repo_name, &app_name, "default")
                .await?;
        }

        display_println!(
            "{} Configured {} as default app for all repositories!",
            style("✓").green().bold(),
            style(&app_name).cyan()
        );
    }

    Ok(())
}

/// Ensure default template exists for an app
async fn ensure_default_template(
    workspace_manager: &mut WorkspaceManager,
    app_name: &str,
) -> Result<()> {
    let templates = workspace_manager.list_templates(app_name).await?;

    if !templates.contains(&"default".to_string()) {
        // Get bundled default template
        let default_content = workspace_manager.get_default_template(app_name).await?;

        // Save as "default" template
        workspace_manager
            .save_template(app_name, "default", &default_content)
            .await?;

        display_println!(
            "{} Created default template for {}",
            style("✓").green(),
            style(app_name).cyan()
        );
    }

    Ok(())
}

/// Preview workspace directory contents and ask for confirmation
/// Returns (confirmed, discovered_repos)
async fn preview_workspace_directory(
    workspace_manager: &WorkspaceManager,
    directory: &PathBuf,
) -> Result<(bool, Vec<PathBuf>)> {
    display_println!(
        "\n{} Previewing workspace directory: {}",
        style("🔍").blue(),
        style(directory.display()).dim()
    );

    // Scan for git repositories
    let repos = workspace_manager
        .discover_repositories(directory, 3)
        .await?;

    if repos.is_empty() {
        display_println!(
            "{} No git repositories found in this directory.",
            style("ℹ️").blue()
        );
        display_println!("This appears to be an empty workspace - perfect for a fresh start!");
    } else {
        display_println!(
            "\n{} Found {} existing repositories:",
            style("📁").green(),
            style(repos.len()).cyan().bold()
        );

        // Show preview (limit to first 10, then show "and X more")
        for (i, repo) in repos.iter().enumerate() {
            if i < 10 {
                let repo_name = repo
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                display_println!("  {} {}", style("•").dim(), style(repo_name).green());
            } else if i == 10 {
                display_println!("  {} ... and {} more", style("•").dim(), repos.len() - 10);
                break;
            }
        }

        display_println!(
            "\n{} These repositories can be imported into your vibe workspace.",
            style("💡").yellow()
        );
    }

    // Ask for confirmation
    display_println!();
    let confirmed = prompt_yes_no("Does this look like the right workspace directory?", true)?;

    if !confirmed {
        display_println!(
            "{} No problem! Let's choose a different directory.",
            style("👍").blue()
        );
    }

    Ok((confirmed, repos))
}

/// Show next steps after setup
fn show_next_steps() {
    display_println!("\n{}", style("✨ Setup complete!").green().bold());
    display_println!("\n{}", style("Getting Started:").yellow().bold());

    display_println!("\n{} Quick commands:", style("📚").blue());
    display_println!(
        "  {} {} - Open interactive menu",
        style("•").dim(),
        style("vibe").cyan().bold()
    );
    display_println!(
        "  {} {} - Clone and open a GitHub repo",
        style("•").dim(),
        style("vibe go owner/repo").cyan()
    );
    display_println!(
        "  {} {} - Quick open recent (in menu)",
        style("•").dim(),
        style("Press 1-9").cyan()
    );
    display_println!(
        "  {} {} - Open specific repository",
        style("•").dim(),
        style("vibe launch [name]").cyan()
    );

    display_println!("\n{} Next steps:", style("💡").yellow());
    display_println!(
        "  1. Clone a repository: {}",
        style("vibe go <owner/repo>").cyan()
    );
    display_println!(
        "  2. Or run {} to explore all features",
        style("vibe").cyan().bold()
    );

    display_println!(
        "\nFor more help: {}",
        style("vibe help getting-started").dim()
    );
    display_println!("\nEnjoy using Vibe! 🚀");
}
