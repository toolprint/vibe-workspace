use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;

mod apps;
mod cache;
mod git;
mod mcp;
mod output;
mod ui;
mod uri;
mod utils;
mod workspace;

use ui::{prompts, state::VibeState};
use workspace::WorkspaceManager;

#[derive(Parser)]
#[command(name = "vibe")]
#[command(
    about = "Lightweight CLI for managing multiple git repositories and workspace configurations",
    long_about = "Vibe helps you manage multiple git repositories from a single place.\n\
                  Think of it as a smart launcher for your development projects.\n\n\
                  GETTING STARTED:\n  \
                  1. Run 'vibe' to start the interactive menu\n  \
                  2. Or run 'vibe setup' for first-time configuration\n  \
                  3. Clone repos with 'vibe go owner/repo'\n  \
                  4. Open repos with 'vibe launch [name]'\n\n\
                  For detailed help: vibe help getting-started"
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Override workspace root directory
    #[arg(short, long, global = true)]
    root: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize workspace configuration in current directory
    Init {
        /// Workspace name
        #[arg(short, long)]
        name: Option<String>,

        /// Root directory for workspace
        #[arg(short, long)]
        root: Option<PathBuf>,
    },

    /// Manage app integrations
    Apps {
        #[command(subcommand)]
        command: AppsCommands,
    },

    /// Interactive menu system
    Menu,

    /// Manage workspace configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Git repository operations
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Open repository with configured app
    Open {
        /// Repository name
        repo: String,

        /// App to open with (warp, iterm2, vscode, wezterm, cursor, windsurf)
        #[arg(short, long)]
        app: Option<String>,

        /// Disable iTermocil for iTerm2 (use Dynamic Profiles instead)
        #[arg(long)]
        no_itermocil: bool,
    },

    /// Quick launch recent repository or specific repository
    Launch {
        /// Repository name or number (1-9 for recent repos)
        #[arg(value_name = "REPO")]
        repo: Option<String>,

        /// App to open with (overrides default/last used)
        #[arg(short, long)]
        app: Option<String>,
    },

    /// Clone, configure, and open a repository in one command
    Go {
        /// Repository URL or GitHub shorthand (owner/repo)
        url: String,

        /// App to open with after cloning
        #[arg(short, long)]
        app: Option<String>,

        /// Skip app configuration
        #[arg(long)]
        no_configure: bool,

        /// Skip opening after clone
        #[arg(long)]
        no_open: bool,
    },

    /// Run first-time setup wizard
    Setup {
        /// Skip the setup wizard
        #[arg(long)]
        skip: bool,
    },

    /// Run as MCP (Model Context Protocol) server
    Mcp {
        /// Use HTTP transport on specified port
        #[arg(long, conflicts_with = "stdio")]
        port: Option<u16>,

        /// Use stdio transport (default)
        #[arg(long, default_value = "true")]
        stdio: bool,
    },

    /// Show help for specific topics
    Help {
        #[command(subcommand)]
        topic: HelpTopic,
    },
}

#[derive(Subcommand)]
enum HelpTopic {
    /// Getting started guide
    GettingStarted,
}

#[derive(Subcommand)]
enum AppsCommands {
    /// Configure app integration for a repository
    Configure {
        /// Repository name
        repo: String,

        /// App to configure (warp, iterm2, vscode, wezterm, cursor, windsurf)
        app: String,

        /// Template to use
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Show app configurations
    Show {
        /// Filter by repository name
        #[arg(long)]
        repo: Option<String>,

        /// Filter by app name
        #[arg(long)]
        app: Option<String>,
    },

    /// Manage app templates
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },

    /// Install developer tools and applications
    Install,
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// List available templates
    List {
        /// App to list templates for
        app: String,
    },

    /// Create a new template
    Create {
        /// App to create template for
        app: String,

        /// Template name
        name: String,

        /// Source file for template content
        #[arg(short, long)]
        from_file: Option<PathBuf>,
    },

    /// Delete a template
    Delete {
        /// App to delete template from
        app: String,

        /// Template name
        name: String,
    },

    /// Update default templates with current bundled versions
    UpdateDefaults {
        /// Only update specific app's default template
        #[arg(short, long)]
        app: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Initialize a new workspace configuration
    Init {
        /// Workspace name
        #[arg(short, long)]
        name: Option<String>,

        /// Workspace root directory
        #[arg(short, long)]
        root: Option<PathBuf>,

        /// Enable auto-discovery of repositories
        #[arg(short, long)]
        auto_discover: bool,
    },

    /// Edit workspace configuration with default editor
    Edit {
        /// Open editor directly without prompts
        #[arg(short, long)]
        direct: bool,
    },

    /// Show current workspace configuration
    Show {
        /// Output format: yaml (default), json, pretty
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Show only a specific section: workspace, repositories, groups, apps
        #[arg(short, long)]
        section: Option<String>,
    },

    /// Validate workspace configuration
    Validate {
        /// Check if all repository paths exist
        #[arg(short, long)]
        check_paths: bool,

        /// Check if all remote URLs are accessible
        #[arg(short, long)]
        check_remotes: bool,

        /// Validate app integrations
        #[arg(short, long)]
        check_apps: bool,
    },

    /// Factory reset - clear all configuration and reinitialize
    Reset {
        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
    },

    /// Create backup archive of all configuration files
    Backup {
        /// Output directory for backup file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Custom backup name (default: timestamp)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Restore configuration from backup archive
    Restore {
        /// Backup file to restore from
        #[arg(short, long)]
        backup: Option<PathBuf>,

        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum GitCommands {
    /// Scan workspace for git repositories
    Scan {
        /// Directory to scan for repositories
        path: Option<PathBuf>,

        /// Maximum depth to scan
        #[arg(long, default_value = "3")]
        depth: usize,

        /// Add newly found repositories to config
        #[arg(short, long)]
        import: bool,

        /// Re-clone missing repositories from config
        #[arg(long)]
        restore: bool,

        /// Remove missing repositories from config
        #[arg(long)]
        clean: bool,
    },

    /// Discover git repositories in directory structure (deprecated: use scan)
    Discover {
        /// Directory to scan for repositories
        path: Option<PathBuf>,

        /// Maximum depth to scan
        #[arg(short, long, default_value = "3")]
        depth: usize,

        /// Auto-add discovered repositories to config
        #[arg(short, long)]
        auto_add: bool,
    },

    /// Show status across all repositories
    Status {
        /// Show only repositories with changes
        #[arg(short, long)]
        dirty_only: bool,

        /// Output format: table, json, compact
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Filter by group name
        #[arg(short, long)]
        group: Option<String>,
    },

    /// Execute git commands across repositories
    Exec {
        /// Git command to execute
        command: String,

        /// Target repositories (comma-separated)
        #[arg(short, long)]
        repos: Option<String>,

        /// Target group
        #[arg(short, long)]
        group: Option<String>,

        /// Run in parallel
        #[arg(short, long)]
        parallel: bool,
    },

    /// Sync repositories (fetch and pull)
    Sync {
        /// Only fetch, don't pull
        #[arg(short, long)]
        fetch_only: bool,

        /// Prune remote tracking branches
        #[arg(short, long)]
        prune: bool,

        /// Auto-commit dirty changes to dirty/{timestamp} branch before sync
        #[arg(short, long)]
        save_dirty: bool,

        /// Target group
        #[arg(short, long)]
        group: Option<String>,
    },

    /// Clone a repository to the workspace
    Clone {
        /// Repository URL or identifier
        url: String,

        /// Override default clone location
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Open in configured editor after cloning
        #[arg(short, long)]
        open: bool,

        /// Run post-install commands (npm install, etc.)
        #[arg(short, long)]
        install: bool,
    },

    /// Search for repositories interactively
    Search,

    /// Reset repository configuration (clear all tracked repositories)
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine output mode based on command
    let output_mode = match &cli.command {
        Some(Commands::Mcp { .. }) => output::OutputMode::Mcp,
        _ => output::OutputMode::Cli,
    };

    // Initialize output system (this handles tracing setup)
    output::init_with_verbosity(output_mode, cli.verbose);

    // Load or create workspace configuration
    let config_path = cli.config.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibe-workspace")
            .join("config.yaml")
    });

    let mut workspace_manager =
        WorkspaceManager::new_with_root_override(config_path.clone(), cli.root).await?;

    match cli.command {
        None => {
            // No command provided, start menu mode
            prompts::run_menu_mode(&mut workspace_manager).await?;
        }
        Some(command) => match command {
            Commands::Init { name, root } => {
                let workspace_name = name.unwrap_or_else(|| {
                    std::env::current_dir()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                        .unwrap_or_else(|| "workspace".to_string())
                });

                let workspace_root = root.unwrap_or_else(|| std::env::current_dir().unwrap());

                workspace_manager
                    .init_workspace(&workspace_name, &workspace_root)
                    .await?;

                display_println!(
                    "{} Initialized workspace '{}' in {}",
                    style("✓").green().bold(),
                    style(&workspace_name).cyan().bold(),
                    style(workspace_root.display()).dim()
                );
            }

            Commands::Apps { command } => match command {
                AppsCommands::Configure {
                    repo,
                    app,
                    template,
                } => {
                    let template_name = template.as_deref().unwrap_or("default");
                    workspace_manager
                        .configure_app_for_repo(&repo, &app, template_name)
                        .await?;
                    display_println!(
                        "{} Configured {} for repository '{}' with template '{}'",
                        style("✓").green().bold(),
                        style(&app).cyan(),
                        style(&repo).cyan(),
                        style(template_name).dim()
                    );
                }

                AppsCommands::Show { repo, app } => {
                    if let Some(repo_name) = repo {
                        let apps = workspace_manager.list_apps_for_repo(&repo_name)?;
                        display_println!(
                            "{} Apps configured for repository '{}':",
                            style("📱").blue(),
                            style(&repo_name).cyan().bold()
                        );
                        for (app_name, template) in apps {
                            display_println!(
                                "  {} {} (template: {})",
                                style("→").dim(),
                                style(&app_name).green(),
                                style(&template).dim()
                            );
                        }
                    } else if let Some(app_name) = app {
                        let repos = workspace_manager.list_repos_with_app(&app_name);
                        display_println!(
                            "{} Repositories with {} configured:",
                            style("📱").blue(),
                            style(&app_name).cyan().bold()
                        );
                        for (repo, template) in repos {
                            display_println!(
                                "  {} {} (template: {})",
                                style("→").dim(),
                                style(&repo.name).green(),
                                style(&template).dim()
                            );
                        }
                    } else {
                        // Show all app configurations
                        workspace_manager.show_app_configurations().await?;
                    }
                }

                AppsCommands::Template { command } => match command {
                    TemplateCommands::List { app } => {
                        let templates = workspace_manager.list_templates(&app).await?;
                        display_println!(
                            "{} Available templates for {}:",
                            style("📄").blue(),
                            style(&app).cyan().bold()
                        );
                        for template in templates {
                            display_println!("  {} {}", style("→").dim(), style(&template).green());
                        }
                    }

                    TemplateCommands::Create {
                        app,
                        name,
                        from_file,
                    } => {
                        let content = if let Some(file_path) = from_file {
                            tokio::fs::read_to_string(&file_path).await?
                        } else {
                            // Use default template as starting point
                            workspace_manager.get_default_template(&app).await?
                        };

                        workspace_manager
                            .save_template(&app, &name, &content)
                            .await?;
                        display_println!(
                            "{} Created template '{}' for {}",
                            style("✓").green().bold(),
                            style(&name).cyan(),
                            style(&app).cyan()
                        );
                    }

                    TemplateCommands::Delete { app, name } => {
                        workspace_manager.delete_template(&app, &name).await?;
                        display_println!(
                            "{} Deleted template '{}' from {}",
                            style("✓").green().bold(),
                            style(&name).cyan(),
                            style(&app).cyan()
                        );
                    }

                    TemplateCommands::UpdateDefaults { app, force } => {
                        let apps_to_update = if let Some(app_name) = app {
                            vec![app_name]
                        } else {
                            vec![
                                "warp".to_string(),
                                "iterm2".to_string(),
                                "wezterm".to_string(),
                                "vscode".to_string(),
                                "cursor".to_string(),
                                "windsurf".to_string(),
                            ]
                        };

                        if !force {
                            display_println!(
                                "{} This will overwrite existing default templates for: {}",
                                style("⚠️").yellow(),
                                apps_to_update.join(", ")
                            );
                            print!("Continue? [y/N] ");
                            use std::io::{self, Write};
                            io::stdout().flush()?;

                            let mut input = String::new();
                            io::stdin().read_line(&mut input)?;

                            if !input.trim().eq_ignore_ascii_case("y") {
                                display_println!("{} Update cancelled", style("ℹ️").blue());
                                return Ok(());
                            }
                        }

                        workspace_manager
                            .update_default_templates(apps_to_update)
                            .await?;
                        display_println!(
                            "{} Updated default templates with current bundled versions",
                            style("✓").green().bold()
                        );
                    }
                },

                AppsCommands::Install => {
                    // Run the interactive installer
                    apps::run_interactive_installer().await?;
                }
            },

            Commands::Menu => {
                prompts::run_menu_mode(&mut workspace_manager).await?;
            }

            Commands::Config { command } => match command {
                ConfigCommands::Init {
                    name,
                    root,
                    auto_discover,
                } => {
                    workspace_manager
                        .init_config(name.as_deref(), root.as_deref(), auto_discover)
                        .await?;
                }

                ConfigCommands::Edit { direct } => {
                    workspace_manager.edit_config(direct).await?;
                }

                ConfigCommands::Show { format, section } => {
                    workspace_manager
                        .show_config(&format, section.as_deref())
                        .await?;
                }

                ConfigCommands::Validate {
                    check_paths,
                    check_remotes,
                    check_apps,
                } => {
                    workspace_manager
                        .validate_config(check_paths, check_remotes, check_apps)
                        .await?;
                }

                ConfigCommands::Reset { force } => {
                    workspace_manager.factory_reset(force).await?;
                }

                ConfigCommands::Backup { output, name } => {
                    let backup_path = workspace_manager.create_backup(output, name).await?;
                    display_println!(
                        "{} Backup created successfully: {}",
                        style("✓").green().bold(),
                        style(backup_path.display()).cyan()
                    );
                }

                ConfigCommands::Restore { backup, force } => {
                    workspace_manager.restore_from_backup(backup, force).await?;
                }
            },

            Commands::Git { command } => match command {
                GitCommands::Scan {
                    path,
                    depth,
                    import,
                    restore,
                    clean,
                } => {
                    // Validate conflicting flags
                    if restore && clean {
                        anyhow::bail!("Cannot use --restore and --clean together");
                    }

                    let scan_path =
                        path.unwrap_or_else(|| workspace_manager.get_workspace_root().clone());

                    workspace_manager
                        .scan_repositories(&scan_path, depth, import, restore, clean)
                        .await?;
                }

                GitCommands::Discover {
                    path,
                    depth,
                    auto_add,
                } => {
                    // Show deprecation warning
                    display_println!(
                        "{} The 'discover' command is deprecated. Use 'scan --import' instead.",
                        style("⚠️").yellow()
                    );

                    let scan_path =
                        path.unwrap_or_else(|| workspace_manager.get_workspace_root().clone());

                    display_println!(
                        "{} Discovering repositories in {} (depth: {})",
                        style("🔍").blue(),
                        style(scan_path.display()).cyan(),
                        depth
                    );

                    let repos = workspace_manager
                        .discover_repositories(&scan_path, depth)
                        .await?;

                    if repos.is_empty() {
                        display_println!("{} No git repositories found", style("ℹ").yellow());
                        return Ok(());
                    }

                    display_println!(
                        "\n{} Found {} repositories:",
                        style("📁").green(),
                        style(repos.len()).bold()
                    );

                    for repo in &repos {
                        display_println!("  {} {}", style("→").dim(), style(repo.display()).cyan());
                    }

                    if auto_add {
                        workspace_manager
                            .add_discovered_repositories(&repos)
                            .await?;
                        display_println!(
                            "\n{} Added repositories to workspace configuration",
                            style("✓").green().bold()
                        );
                    } else {
                        display_println!(
                            "\n{} Run with --auto-add to add these to your workspace",
                            style("💡").yellow()
                        );
                    }
                }

                GitCommands::Status {
                    dirty_only,
                    format,
                    group,
                } => {
                    workspace_manager
                        .show_status(dirty_only, &format, group.as_deref())
                        .await?;
                }

                GitCommands::Exec {
                    command,
                    repos,
                    group,
                    parallel,
                } => {
                    workspace_manager
                        .execute_command(&command, repos.as_deref(), group.as_deref(), parallel)
                        .await?;
                }

                GitCommands::Sync {
                    fetch_only,
                    prune,
                    save_dirty,
                    group,
                } => {
                    workspace_manager
                        .sync_repositories(fetch_only, prune, save_dirty, group.as_deref())
                        .await?;
                }

                GitCommands::Clone {
                    url,
                    path,
                    open,
                    install,
                } => {
                    let git_config = git::GitConfig::default();
                    let _cloned_path = git::CloneCommand::execute(
                        url,
                        path,
                        open,
                        install,
                        &mut workspace_manager,
                        &git_config,
                    )
                    .await?;
                }

                GitCommands::Search => {
                    let git_config = git::GitConfig::default();
                    git::SearchCommand::execute_interactive(&mut workspace_manager, &git_config)
                        .await?;
                }

                GitCommands::Reset { force } => {
                    workspace_manager.reset_repositories(force).await?;
                }
            },

            Commands::Open {
                repo,
                app,
                no_itermocil,
            } => {
                if let Some(app_name) = app {
                    // Open with specific app
                    workspace_manager
                        .open_repo_with_app_options(&repo, &app_name, no_itermocil)
                        .await?;
                } else {
                    // Open with default or show available apps
                    let apps = workspace_manager.list_apps_for_repo(&repo)?;
                    if apps.is_empty() {
                        display_println!(
                        "{} No apps configured for repository '{}'. Configure with: vibe apps configure {} <app>",
                        style("⚠️").yellow(),
                        style(&repo).cyan(),
                        style(&repo).cyan()
                    );
                    } else if apps.len() == 1 {
                        // Only one app configured, use it
                        let (app_name, _) = &apps[0];
                        workspace_manager
                            .open_repo_with_app(&repo, app_name)
                            .await?;
                    } else {
                        // Multiple apps configured, show options
                        display_println!(
                            "{} Multiple apps configured for '{}'. Please specify one:",
                            style("🤔").yellow(),
                            style(&repo).cyan()
                        );
                        for (app_name, _template) in &apps {
                            display_println!(
                                "  {} vibe open {} --app {}",
                                style("→").dim(),
                                &repo,
                                style(app_name).green()
                            );
                        }
                    }
                }
            }

            Commands::Launch { repo, app } => {
                // Load state to get recent repos
                let mut state = VibeState::load().unwrap_or_default();

                let repo_to_open = if let Some(repo_name) = repo {
                    // Check if it's a number (1-9) for recent repos
                    if let Ok(num) = repo_name.parse::<usize>() {
                        if num >= 1 && num <= 9 {
                            let recent_repos = state.get_recent_repos(15);
                            if num <= recent_repos.len() {
                                recent_repos[num - 1].repo_id.clone()
                            } else {
                                anyhow::bail!("No recent repository at position {}", num);
                            }
                        } else {
                            repo_name
                        }
                    } else {
                        repo_name
                    }
                } else {
                    // No repo specified, open the most recent one
                    let recent_repos = state.get_recent_repos(1);
                    if recent_repos.is_empty() {
                        anyhow::bail!(
                            "No recent repositories found. Use 'vibe' to browse repositories."
                        );
                    }
                    recent_repos[0].repo_id.clone()
                };

                // Get the repository info
                let repo_info = workspace_manager
                    .get_repository(&repo_to_open)
                    .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_to_open))?;

                // Determine which app to use
                let app_to_use = if let Some(app_name) = app {
                    app_name
                } else if let Some(last_app) = state.get_last_app(&repo_to_open) {
                    last_app.clone()
                } else {
                    // Get configured apps and use first one
                    let apps = workspace_manager.list_apps_for_repo(&repo_to_open)?;
                    if apps.is_empty() {
                        anyhow::bail!("No apps configured for repository '{}'", repo_to_open);
                    }
                    apps[0].0.clone()
                };

                // Open the repository
                workspace_manager
                    .open_repo_with_app(&repo_to_open, &app_to_use)
                    .await?;

                // Update state with this access
                state.add_recent_repo(
                    repo_to_open.clone(),
                    repo_info.path.clone(),
                    Some(app_to_use.clone()),
                );
                state.save()?;

                display_println!(
                    "{} Launched {} with {}",
                    style("🚀").green(),
                    style(&repo_to_open).cyan().bold(),
                    style(&app_to_use).blue()
                );
            }

            Commands::Go {
                url,
                app,
                no_configure,
                no_open,
            } => {
                use ui::workflows::{execute_workflow, CloneAndOpenWorkflow};

                // Use workflow system if not skipping steps
                if !no_configure || !no_open {
                    let workflow = Box::new(CloneAndOpenWorkflow {
                        url: url.clone(),
                        app: app.clone(),
                    });

                    execute_workflow(workflow, &mut workspace_manager).await?;
                } else {
                    // Just clone without workflow
                    let git_config = git::GitConfig::default();
                    let _cloned_path = git::CloneCommand::execute(
                        url,
                        None,
                        false,
                        false,
                        &mut workspace_manager,
                        &git_config,
                    )
                    .await?;

                    display_println!(
                        "{} Repository cloned successfully!",
                        style("✓").green().bold()
                    );
                }
            }

            Commands::Setup { skip } => {
                if skip {
                    let mut state = VibeState::load().unwrap_or_default();
                    state.complete_setup_wizard();
                    state.save()?;
                    display_println!("{} Setup wizard skipped", style("ℹ️").blue());
                } else {
                    use ui::workflows::{execute_workflow, SetupWorkspaceWorkflow};

                    // Run setup workflow
                    let workflow = Box::new(SetupWorkspaceWorkflow {
                        auto_discover: true,
                    });

                    execute_workflow(workflow, &mut workspace_manager).await?;

                    // Mark setup as complete
                    let mut state = VibeState::load().unwrap_or_default();
                    state.complete_setup_wizard();
                    state.save()?;
                }
            }

            Commands::Mcp { port, stdio: _ } => {
                use std::sync::Arc;
                use tokio::sync::Mutex;

                // Create shared workspace manager for MCP server
                let shared_workspace = Arc::new(Mutex::new(workspace_manager));

                if let Some(port_num) = port {
                    // HTTP transport not implemented in this example
                    // You would need to implement HTTP transport support
                    display_eprintln!(
                        "{} HTTP transport on port {} not yet implemented",
                        style("⚠️").yellow(),
                        port_num
                    );
                    display_eprintln!(
                        "{} Use --stdio flag for stdio transport",
                        style("💡").blue()
                    );
                    std::process::exit(1);
                } else {
                    // Run with stdio transport (default)
                    display_eprintln!(
                        "{} Starting vibe-workspace MCP server (stdio transport)...",
                        style("🚀").green()
                    );

                    let mcp_server = mcp::VibeMCPServer::new(shared_workspace);
                    mcp_server.run().await?;
                }
            }

            Commands::Help { topic } => match topic {
                HelpTopic::GettingStarted => {
                    print_getting_started_guide();
                }
            },
        },
    }

    Ok(())
}

/// Print the getting started guide
fn print_getting_started_guide() {
    display_println!("{}", style("🚀 Getting Started with Vibe").cyan().bold());
    display_println!("{}", style("═".repeat(40)).dim());
    display_println!();

    display_println!("{}", style("What is Vibe?").yellow().bold());
    display_println!("Vibe helps you manage multiple git repositories from a single place.");
    display_println!("Think of it as a smart launcher for your development projects.");
    display_println!();

    display_println!("{}", style("Quick Start").yellow().bold());
    display_println!(
        "1. {} - Start the interactive menu (recommended)",
        style("vibe").cyan().bold()
    );
    display_println!("2. {} - Run the setup wizard", style("vibe setup").cyan());
    display_println!(
        "3. {} - Clone and open a GitHub repo",
        style("vibe go owner/repo").cyan()
    );
    display_println!();

    display_println!("{}", style("Basic Commands").yellow().bold());
    display_println!(
        "  {} - Interactive menu with quick launch",
        style("vibe").cyan()
    );
    display_println!(
        "  {} - Clone, configure, and open in one command",
        style("vibe go <url>").cyan()
    );
    display_println!(
        "  {} - Open a specific repository",
        style("vibe launch <name>").cyan()
    );
    display_println!(
        "  {} - Quick open recent (use 1-9)",
        style("vibe launch <number>").cyan()
    );
    display_println!(
        "  {} - Open repository with specific app",
        style("vibe open <repo> -a <app>").cyan()
    );
    display_println!();

    display_println!("{}", style("Repository Management").yellow().bold());
    display_println!(
        "  {} - Clone a repository",
        style("vibe git clone <url>").cyan()
    );
    display_println!(
        "  {} - Search GitHub repositories",
        style("vibe git search <query>").cyan()
    );
    display_println!(
        "  {} - Scan directory for repositories",
        style("vibe git scan [path]").cyan()
    );
    display_println!(
        "  {} - Show git status across all repos",
        style("vibe git status").cyan()
    );
    display_println!(
        "  {} - Sync all repositories",
        style("vibe git sync").cyan()
    );
    display_println!();

    display_println!("{}", style("App Configuration").yellow().bold());
    display_println!(
        "  {} - Configure app for repository",
        style("vibe apps configure <repo>").cyan()
    );
    display_println!(
        "  {} - Install app integrations",
        style("vibe apps install").cyan()
    );
    display_println!(
        "  {} - Show app configurations",
        style("vibe apps show").cyan()
    );
    display_println!();

    display_println!("{}", style("Tips").yellow().bold());
    display_println!(
        "• In the menu, press {} to quickly open recent repositories",
        style("1-9").cyan()
    );
    display_println!("• Apps are your development tools (VS Code, iTerm2, etc.)");
    display_println!("• Templates define how to open your repo in each app");
    display_println!(
        "• Run {} to see all available commands",
        style("vibe --help").cyan()
    );
    display_println!();

    display_println!("{}", style("Next Steps").green().bold());
    display_println!("1. Run {} to start exploring", style("vibe").cyan().bold());
    display_println!(
        "2. Clone your first repo with {}",
        style("vibe go <owner/repo>").cyan()
    );
    display_println!("3. Configure your favorite apps");
    display_println!();
}
