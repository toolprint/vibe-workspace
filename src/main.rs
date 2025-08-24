// Warning Denial Implementation (TEMPORARILY DISABLED):
// - Future: RUSTFLAGS="-D warnings" in release build justfile commands
// - Future: #![cfg_attr(not(debug_assertions), deny(warnings))] at crate level
// - Currently disabled until intentional dead code is properly annotated with #[allow(dead_code)]
// - To re-enable: Add RUSTFLAGS="-D warnings" to zigbuild-release and build-release commands

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;

mod apps;
mod cache;
mod git;
mod mcp;
mod output;
mod repository;
mod ui;
mod uri;
mod utils;
mod workspace;
mod worktree;

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
                  3. Clone repos with 'vibe clone owner/repo'\n  \
                  4. Open repos with 'vibe launch [name]'\n\n\
                  For detailed help: vibe guide getting-started"
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
    /// Manage app integrations
    Apps {
        #[command(subcommand)]
        command: AppsCommands,
    },

    /// Interactive menu system
    Menu,

    /// Create a new repository in the workspace
    Create {
        /// Repository name
        name: Option<String>,

        /// App to open with after creating
        #[arg(short, long)]
        app: Option<String>,

        /// Skip app configuration
        #[arg(long)]
        no_configure: bool,

        /// Skip opening after create
        #[arg(long)]
        no_open: bool,
    },

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

    /// Interactive recent repository selector (1-9)
    Launch,

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

    /// Clone, configure, and open a repository in one command
    Clone {
        /// Repository URL, GitHub shorthand (owner/repo), or user/org name for bulk cloning
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

        /// Clone all repositories for user/org (bulk mode)
        #[arg(long)]
        all: bool,

        /// Exclude patterns for bulk cloning (comma-separated glob patterns)
        #[arg(long, requires = "all")]
        exclude: Option<String>,

        /// Include only patterns for bulk cloning (comma-separated glob patterns)
        #[arg(long, requires = "all")]
        include: Option<String>,

        /// Skip confirmation prompts for bulk operations
        #[arg(long, requires = "all")]
        force: bool,
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

    /// Show getting started guide
    Guide,
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

        /// Show only a specific section: workspace, repositories, groups, apps, claude_agents
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
enum WorktreeCommands {
    /// Create a new worktree for parallel development
    Create {
        /// Task identifier to create worktree for
        task_id: String,

        /// Base branch to create worktree from
        #[arg(short, long)]
        base_branch: Option<String>,

        /// Force creation even if branch exists
        #[arg(short, long)]
        force: bool,

        /// Custom path for the worktree (overrides default)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Storage mode: local (within repo) or global (centralized)
        #[arg(short, long)]
        mode: Option<String>,

        /// Open worktree in editor after creation
        #[arg(short, long)]
        open: bool,

        /// Editor command to use (overrides default)
        #[arg(long)]
        editor: Option<String>,
    },

    /// List all git worktrees with status
    List {
        /// Show only worktrees with the specified prefix
        #[arg(short, long)]
        prefix: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,

        /// Output format: table, json, compact
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show only worktrees with uncommitted changes
        #[arg(short, long)]
        dirty_only: bool,
    },

    /// Remove a worktree
    Remove {
        /// Task ID, branch name, or worktree path to remove
        target: String,

        /// Force removal even with uncommitted changes
        #[arg(short, long)]
        force: bool,

        /// Also delete the branch after removing worktree
        #[arg(short, long)]
        delete_branch: bool,

        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },

    /// Show repository worktree health overview
    Status {
        /// Branch name to show detailed status for (shows summary if not specified)
        branch: Option<String>,

        /// Show detailed status for all worktrees
        #[arg(short, long)]
        all: bool,

        /// Output format: table, json, compact (for detailed view)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show only files that have changed (forces detailed view)
        #[arg(long)]
        files_only: bool,
    },

    /// Clean up merged worktrees
    Clean {
        /// Show what would be done without executing
        #[arg(short, long)]
        dry_run: bool,

        /// Force cleanup even with uncommitted changes
        #[arg(short, long)]
        force: bool,

        /// Minimum age in hours before cleanup
        #[arg(long)]
        age: Option<u64>,

        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },

    /// Open a worktree in configured editor
    Open {
        /// Task ID, branch name, or worktree path to open
        target: String,

        /// Editor command to use (overrides default)
        #[arg(short, long)]
        editor: Option<String>,
    },

    /// Manage worktree configuration
    Config {
        #[command(subcommand)]
        action: WorktreeConfigCommands,
    },
}

#[derive(Subcommand)]
enum WorktreeConfigCommands {
    /// Show current configuration
    Show {
        /// Show configuration for specific repository
        #[arg(short = 'R', long)]
        repository: Option<String>,
        
        /// Output format: yaml, json, summary
        #[arg(short, long, default_value = "summary")]
        format: String,
    },
    
    /// Set configuration values
    Set {
        /// Configuration key (e.g., prefix, base_dir, cleanup.age_threshold_hours)
        key: String,
        
        /// Configuration value
        value: String,
        
        /// Apply to specific repository only
        #[arg(short = 'R', long)]
        repository: Option<String>,
    },
    
    /// Reset configuration to defaults
    Reset {
        /// Reset specific key only
        #[arg(short, long)]
        key: Option<String>,
        
        /// Reset configuration for specific repository
        #[arg(short = 'R', long)]
        repository: Option<String>,
    },
    
    /// Validate configuration
    Validate,
    
    /// Show configuration help and environment variables
    Info,
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

    /// Manage git worktrees for parallel development
    Worktree {
        #[command(subcommand)]
        action: WorktreeCommands,
    },
}

/// Handle worktree subcommands
async fn handle_worktree_command(
    command: WorktreeCommands,
    workspace_manager: &WorkspaceManager,
    verbose: bool,
) -> Result<()> {
    use crate::worktree::{CreateOptions, RemoveOptions, WorktreeManager};
    use crate::worktree::config::WorktreeMode;
    use colored::*;

    // Get the current repository root
    let current_dir = std::env::current_dir()?;
    let git_root = find_git_repository_root(&current_dir).await?;

    match command {
        WorktreeCommands::Create {
            task_id,
            base_branch,
            force,
            path,
            mode,
            open,
            editor,
        } => {
            // Handle mode override for create command
            let custom_config = if let Some(mode_str) = &mode {
                use crate::worktree::config::WorktreeConfig;
                let mut config = WorktreeConfig::load_with_overrides().unwrap_or_default();
                config.mode = match mode_str.to_lowercase().as_str() {
                    "global" => WorktreeMode::Global,
                    "local" => WorktreeMode::Local,
                    _ => {
                        eprintln!("❌ Invalid mode '{}'. Use 'local' or 'global'", mode_str);
                        return Ok(());
                    }
                };
                Some(config)
            } else {
                None
            };

            // Create worktree manager with optional custom config
            let worktree_manager = WorktreeManager::new(git_root.clone(), custom_config).await?;
            let options = CreateOptions {
                task_id: task_id.clone(),
                base_branch,
                force,
                custom_path: path,
            };

            println!("Creating worktree for task: {}", task_id.cyan());
            let worktree_info = worktree_manager
                .create_worktree_with_options(options)
                .await?;

            println!("✅ Created worktree:");
            println!("  Branch: {}", worktree_info.branch.yellow());
            println!(
                "  Path: {}",
                worktree_info.path.display().to_string().blue()
            );

            if open {
                let editor_cmd = editor.unwrap_or_else(|| "code".to_string());
                open_worktree_in_editor(&worktree_info.path, &editor_cmd).await?;
            }
        }

        // For all other commands, create a default worktree manager
        other_command => {
            let worktree_manager = WorktreeManager::new(git_root, None).await?;
            
            match other_command {
                WorktreeCommands::Create { .. } => unreachable!(), // Already handled above
                
                WorktreeCommands::List {
            prefix,
            verbose,
            format,
            dirty_only,
        } => {
            let worktrees = worktree_manager.list_worktrees().await?;
            let filtered_worktrees = filter_worktrees(worktrees, prefix.as_deref(), dirty_only);

            match format.as_str() {
                "json" => print_worktrees_json(&filtered_worktrees)?,
                "compact" => print_worktrees_compact(&filtered_worktrees),
                _ => print_worktrees_table(&filtered_worktrees, verbose),
            }
        }

        WorktreeCommands::Remove {
            target,
            force,
            delete_branch,
            yes,
        } => {
            if !yes && !force {
                let confirmation = prompt_for_confirmation(&format!(
                    "Remove worktree '{}'{}?",
                    target,
                    if delete_branch {
                        " and delete branch"
                    } else {
                        ""
                    }
                ))?;

                if !confirmation {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            let options = RemoveOptions {
                target: target.clone(),
                force,
                delete_branch,
            };

            println!("Removing worktree: {}", target.yellow());
            worktree_manager
                .remove_worktree_with_options(options)
                .await?;
            println!("✅ Worktree removed successfully");
        }

        WorktreeCommands::Status {
            branch,
            all,
            format,
            files_only,
        } => {
            let worktrees = worktree_manager.list_worktrees().await?;
            
            // If a specific branch is requested or files_only, show detailed status
            if branch.is_some() || files_only {
                let target_worktrees = if let Some(branch_name) = branch {
                    worktrees
                        .into_iter()
                        .filter(|w| w.branch == branch_name)
                        .collect()
                } else {
                    worktrees
                };

                if target_worktrees.is_empty() {
                    println!("No matching worktrees found");
                    return Ok(());
                }

                match format.as_str() {
                    "json" => print_status_json(&target_worktrees, files_only)?,
                    "compact" => print_status_compact(&target_worktrees, files_only),
                    _ => print_status_table(&target_worktrees, files_only),
                }
            } else {
                // Show repository summary by default
                print_repository_worktree_summary(&worktrees, &format, verbose)?;
            }
        }

        WorktreeCommands::Clean {
            dry_run,
            force,
            age,
            yes,
        } => {
            use crate::worktree::cleanup::{WorktreeCleanup, CleanupOptions, CleanupStrategy};
            
            let cleanup_options = CleanupOptions {
                strategy: CleanupStrategy::Discard,
                min_age_hours: age,
                force,
                dry_run,
                auto_confirm: yes,
                branch_prefix_filter: Some(worktree_manager.get_config().prefix.clone()),
                merged_only: true, // Default to merged only for safety
                min_merge_confidence: 0.7,
            };
            
            let cleanup = WorktreeCleanup::new(
                worktree_manager.get_config().clone(), 
                worktree_manager.get_operations()
            );
            
            println!("🧹 {} worktree cleanup...", 
                     if dry_run { "Simulating" } else { "Starting" });
            
            let report = cleanup.cleanup_worktrees(cleanup_options).await?;
            
            // Display results
            print_cleanup_report(&report);
        }

        WorktreeCommands::Open { target, editor } => {
            // Use the new resolution logic that tries task_id first, then path, then branch
            let worktree = worktree_manager.resolve_worktree_target(&target).await?;
            
            let editor_cmd = editor.unwrap_or_else(|| "code".to_string());
            open_worktree_in_editor(&worktree.path, &editor_cmd).await?;
        }
                WorktreeCommands::Config { action } => {
                    handle_worktree_config_command(action, workspace_manager).await?;
                }
            }
        }
    }

    Ok(())
}

/// Find git repository root from current directory
async fn find_git_repository_root(start_dir: &std::path::Path) -> Result<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return Err(anyhow::anyhow!("Not in a git repository")),
        }
    }
}

/// Open a worktree in the specified editor
async fn open_worktree_in_editor(path: &std::path::Path, editor: &str) -> Result<()> {
    use anyhow::Context;
    use tokio::process::Command;

    println!("Opening worktree in {}: {}", editor, path.display());

    let status = Command::new(editor)
        .arg(path)
        .status()
        .await
        .with_context(|| format!("Failed to execute editor: {}", editor))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Editor command failed with status: {}",
            status
        ));
    }

    println!("✅ Successfully opened worktree in {}", editor);
    Ok(())
}

/// Filter worktrees based on criteria
fn filter_worktrees(
    worktrees: Vec<crate::worktree::status::WorktreeInfo>,
    prefix: Option<&str>,
    dirty_only: bool,
) -> Vec<crate::worktree::status::WorktreeInfo> {
    worktrees
        .into_iter()
        .filter(|w| {
            if let Some(prefix) = prefix {
                if !w.branch.starts_with(prefix) {
                    return false;
                }
            }

            if dirty_only && w.status.is_clean {
                return false;
            }

            true
        })
        .collect()
}

/// Print repository worktree summary with health overview
fn print_repository_worktree_summary(worktrees: &[crate::worktree::status::WorktreeInfo], format: &str, verbose: bool) -> Result<()> {
    use colored::*;
    use crate::worktree::status::RepositoryWorktreeSummary;

    if worktrees.is_empty() {
        println!("No worktrees found");
        return Ok(());
    }

    // Calculate repository summary
    let summary = RepositoryWorktreeSummary::from_worktrees(worktrees);

    match format {
        "json" => {
            let json_output = serde_json::json!({
                "health_score": summary.health_score,
                "health_status": summary.health_description(),
                "health_icon": summary.health_icon(),
                "total_worktrees": summary.total_worktrees,
                "clean_worktrees": summary.clean_worktrees,
                "dirty_worktrees": summary.dirty_worktrees,
                "worktrees_with_remote": summary.worktrees_with_remote,
                "worktrees_with_unpushed": summary.worktrees_with_unpushed,
                "merged_worktrees": summary.merged_worktrees,
                "no_remote_count": summary.total_worktrees - summary.worktrees_with_remote,
                "summary_description": summary.summary_description()
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        },
        "compact" => {
            println!(
                "{} {} ({} health) - {}",
                summary.health_icon(),
                "Repository Health".bold(),
                (summary.health_score * 100.0) as u8,
                summary.summary_description()
            );
        },
        _ => {
            // Default table format
            println!(
                "\n{} Repository Worktree Overview",
                summary.health_icon().to_string().bold()
            );
            
            println!("{}", "─".repeat(50));
            
            println!(
                "{:<20} {}",
                "Health Status:".dimmed(),
                format!("{} ({}%)", summary.health_description(), (summary.health_score * 100.0) as u8).green()
            );
            
            println!(
                "{:<20} {}",
                "Total Worktrees:".dimmed(),
                summary.total_worktrees.to_string().cyan()
            );
            
            if summary.clean_worktrees > 0 {
                println!(
                    "{:<20} {}",
                    "Clean:".dimmed(),
                    summary.clean_worktrees.to_string().green()
                );
            }
            
            if summary.dirty_worktrees > 0 {
                println!(
                    "{:<20} {}",
                    "Dirty:".dimmed(),
                    summary.dirty_worktrees.to_string().yellow()
                );
            }
            
            if summary.worktrees_with_unpushed > 0 {
                println!(
                    "{:<20} {}",
                    "With Unpushed:".dimmed(),
                    summary.worktrees_with_unpushed.to_string().red()
                );
            }
            
            if summary.merged_worktrees > 0 {
                println!(
                    "{:<20} {}",
                    "Merged:".dimmed(),
                    summary.merged_worktrees.to_string().blue()
                );
            }
            
            let no_remote = summary.total_worktrees - summary.worktrees_with_remote;
            if no_remote > 0 {
                println!(
                    "{:<20} {}",
                    "No Remote:".dimmed(),
                    no_remote.to_string().yellow()
                );
            }

            // Show additional verbose information
            if verbose {
                println!();
                println!("{}", "Additional Details:".bold());
                println!("{}", "─".repeat(25));
                
                for worktree in worktrees {
                    let status_indicator = if worktree.status.is_clean {
                        "✓".green()
                    } else {
                        "!".yellow()
                    };
                    
                    println!(
                        "{} {:<30} {}",
                        status_indicator,
                        worktree.branch.cyan(),
                        worktree.path.display().to_string().dimmed()
                    );
                }
            }
            
            println!();
        }
    }

    Ok(())
}

/// Print worktrees in table format
fn print_worktrees_table(worktrees: &[crate::worktree::status::WorktreeInfo], verbose: bool) {
    use colored::*;

    if worktrees.is_empty() {
        println!("No worktrees found");
        return;
    }

    // Calculate optimal TASK ID column width (min 8, max 20)
    let task_id_width = {
        let max_task_id_len = worktrees
            .iter()
            .map(|w| {
                if let Some(ref task_id) = w.task_id {
                    task_id.len()
                } else {
                    "(main)".len()
                }
            })
            .max()
            .unwrap_or(8);
        
        // Constrain between 8 and 20 characters
        std::cmp::max(8, std::cmp::min(20, max_task_id_len))
    };

    // Header
    if verbose {
        println!(
            "{:<width$} {:<12} {:<20} {:<28} {:<8} {}",
            "TASK ID".bold(),
            "STATUS".bold(),
            "BRANCH".bold(),
            "PATH".bold(),
            "AGE".bold(),
            "HEAD".bold(),
            width = task_id_width
        );
        println!("{}", "─".repeat(task_id_width + 12 + 20 + 28 + 8 + 10 + 6)); // Adjust separator length
    } else {
        println!(
            "{:<width$} {}",
            "TASK ID".bold(),
            "STATUS".bold(),
            width = task_id_width
        );
        println!("{}", "─".repeat(task_id_width + 20)); // Adjust separator length
    }

    for worktree in worktrees {
        let path_string = worktree.path.to_string_lossy();
        let path = worktree
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&path_string);

        // Display task_id or indicate main repository with ellipsis handling
        let task_id_raw = if let Some(ref task_id) = worktree.task_id {
            task_id.clone()
        } else {
            "(main)".to_string()
        };
        
        let task_id_str = if task_id_raw.len() > task_id_width {
            format!("{}…", &task_id_raw[..task_id_width - 1])
        } else {
            task_id_raw
        };

        let branch = if worktree.branch.len() > 18 {
            format!("{}…", &worktree.branch[..17])
        } else {
            worktree.branch.clone()
        };

        // For repository summary, we'll calculate this on the entire worktree set
        // This is a placeholder that will be replaced with repository-level stats
        let status = format!(
            "{} {}",
            worktree.status.status_icon(),
            worktree.status.status_description()
        );

        if verbose {
            let age = format_age(worktree.age);
            let head = if worktree.head.len() > 7 {
                &worktree.head[..7]
            } else {
                &worktree.head
            };

            // Format the task_id with proper padding, then apply color
            let task_id_formatted = format!("{:<width$}", task_id_str, width = task_id_width);
            let task_id_colored = if worktree.task_id.is_some() {
                task_id_formatted.green()
            } else {
                task_id_formatted.dimmed()
            };

            // New order: TASK ID | STATUS | BRANCH | PATH | AGE | HEAD
            println!(
                "{} {:<12} {:<20} {:<28} {:<8} {}",
                task_id_colored,
                status,
                branch.yellow(),
                path.blue(),
                age.dimmed(),
                head.dimmed()
            );
        } else {
            // Format the task_id with proper padding, then apply color
            let task_id_formatted = format!("{:<width$}", task_id_str, width = task_id_width);
            let task_id_colored = if worktree.task_id.is_some() {
                task_id_formatted.green()
            } else {
                task_id_formatted.dimmed()
            };

            // New order: TASK ID | STATUS
            println!(
                "{} {}",
                task_id_colored,
                status
            );
        }
    }
}

/// Print worktrees in compact format
fn print_worktrees_compact(worktrees: &[crate::worktree::status::WorktreeInfo]) {
    use colored::*;

    for worktree in worktrees {
        // Display task_id or indicate main repository
        let task_id_display = if let Some(ref task_id) = worktree.task_id {
            format!("[{}]", task_id).green()
        } else {
            "[main]".dimmed()
        };
        
        println!(
            "{} {} {} {}",
            worktree.status.status_icon(),
            task_id_display,
            worktree.branch.yellow(),
            worktree.path.display().to_string().blue()
        );
    }
}

/// Print worktrees in JSON format
fn print_worktrees_json(worktrees: &[crate::worktree::status::WorktreeInfo]) -> Result<()> {
    let json = serde_json::to_string_pretty(worktrees)?;
    println!("{}", json);
    Ok(())
}

/// Enhanced status table printing with detailed information
fn print_detailed_status_table(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    show_files: bool,
) {
    use colored::*;
    use crate::worktree::status::RemoteStatus;
    
    for (i, worktree) in worktrees.iter().enumerate() {
        if i > 0 {
            println!();
        }
        
        // Header
        println!("{} {}", 
                 worktree.status.status_icon().bold(),
                 worktree.branch.cyan().bold());
        println!("Path: {}", worktree.path.display().to_string().blue());
        
        if !worktree.head.is_empty() {
            let short_head = if worktree.head.len() > 7 { 
                &worktree.head[..7] 
            } else { 
                &worktree.head 
            };
            println!("HEAD: {}", short_head.dimmed());
        }
        
        println!("Age: {}", format_age(worktree.age).dimmed());
        
        // Remote status
        match &worktree.status.remote_status {
            RemoteStatus::NoRemote => {
                println!("Remote: {}", "No remote tracking".yellow());
            }
            RemoteStatus::UpToDate => {
                println!("Remote: {}", "Up to date".green());
            }
            RemoteStatus::Ahead(count) => {
                println!("Remote: {} {} ahead", "↑".green(), count);
            }
            RemoteStatus::Behind(count) => {
                println!("Remote: {} {} behind", "↓".red(), count);
            }
            RemoteStatus::Diverged { ahead, behind } => {
                println!("Remote: {} {} ahead, {} {} behind", 
                         "↑".green(), ahead, "↓".red(), behind);
            }
            RemoteStatus::RemoteDeleted => {
                println!("Remote: {}", "Remote branch deleted".red());
            }
        }
        
        // Merge information
        if let Some(merge_info) = &worktree.status.merge_info {
            if merge_info.is_merged {
                println!("Merge Status: {} {} (confidence: {:.0}%)",
                         "✅".green(),
                         merge_info.detection_method,
                         merge_info.confidence * 100.0);
                         
                if let Some(details) = &merge_info.details {
                    println!("  Details: {}", details.dimmed());
                }
            } else {
                println!("Merge Status: {} Not merged", "❌".red());
            }
        }
        
        // Changes summary
        let changes = vec![
            (!worktree.status.uncommitted_changes.is_empty(), 
             format!("{} uncommitted", worktree.status.uncommitted_changes.len())),
            (!worktree.status.untracked_files.is_empty(),
             format!("{} untracked", worktree.status.untracked_files.len())),
            (!worktree.status.unpushed_commits.is_empty(),
             format!("{} unpushed", worktree.status.unpushed_commits.len())),
        ].into_iter()
         .filter(|(has, _)| *has)
         .map(|(_, desc)| desc)
         .collect::<Vec<_>>();
        
        if !changes.is_empty() {
            println!("Changes: {}", changes.join(", ").yellow());
        }
        
        // Show files if requested and present
        if show_files {
            if !worktree.status.uncommitted_changes.is_empty() {
                println!("  {} Uncommitted changes:", "📝".dimmed());
                for file in &worktree.status.uncommitted_changes {
                    println!("    {}", file);
                }
            }
            
            if !worktree.status.untracked_files.is_empty() {
                println!("  {} Untracked files:", "❓".dimmed());
                for file in worktree.status.untracked_files.iter().take(5) {
                    println!("    {}", file);
                }
                if worktree.status.untracked_files.len() > 5 {
                    println!("    {} ... and {} more", 
                             "⋯".dimmed(),
                             worktree.status.untracked_files.len() - 5);
                }
            }
            
            if !worktree.status.unpushed_commits.is_empty() {
                println!("  {} Unpushed commits:", "↑".dimmed());
                for commit in worktree.status.unpushed_commits.iter().take(3) {
                    println!("    {} {} ({})", 
                             commit.id.yellow(),
                             commit.message,
                             commit.author.dimmed());
                }
                if worktree.status.unpushed_commits.len() > 3 {
                    println!("    {} ... and {} more commits",
                             "⋯".dimmed(),
                             worktree.status.unpushed_commits.len() - 3);
                }
            }
        }
    }
}

/// Print status in table format
fn print_status_table(worktrees: &[crate::worktree::status::WorktreeInfo], files_only: bool) {
    print_detailed_status_table(worktrees, files_only);
}

/// Print status in compact format
fn print_status_compact(worktrees: &[crate::worktree::status::WorktreeInfo], _files_only: bool) {
    use colored::*;
    
    for worktree in worktrees {
        let mut status_line = format!(
            "{} {}: {}",
            worktree.status.status_icon(),
            worktree.branch,
            worktree.status.status_description()
        );
        
        // Add merge status if available
        if let Some(merge_info) = &worktree.status.merge_info {
            if merge_info.is_merged {
                status_line.push_str(&format!(" [{}]", "merged".green()));
            }
        }
        
        println!("{}", status_line);
    }
}

/// Print status in JSON format
fn print_status_json(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    _files_only: bool,
) -> Result<()> {
    let json = serde_json::to_string_pretty(worktrees)?;
    println!("{}", json);
    Ok(())
}

/// Format age duration for display
fn format_age(age: std::time::Duration) -> String {
    let hours = age.as_secs() / 3600;
    let days = hours / 24;

    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h", hours)
    } else {
        format!("{}m", age.as_secs() / 60)
    }
}

/// Print cleanup report
fn print_cleanup_report(report: &crate::worktree::cleanup::CleanupReport) {
    use colored::*;
    
    println!();
    println!("{} Cleanup Report", "📊".blue());
    println!("Strategy: {:?}", report.strategy_used);
    if report.was_dry_run {
        println!("Mode: {} (no changes made)", "Dry Run".yellow());
    }
    println!();
    
    println!("Results:");
    println!("  ✅ Cleaned: {}", report.cleaned_count.to_string().green());
    println!("  ⚠️  Skipped: {}", report.skipped_count.to_string().yellow());
    println!("  ❌ Failed:  {}", report.failed_count.to_string().red());
    println!("  📊 Total:   {}", report.total_evaluated);
    
    if !report.worktree_results.is_empty() {
        println!();
        println!("Details:");
        
        for result in &report.worktree_results {
            let action_icon = match result.action {
                crate::worktree::cleanup::CleanupAction::Cleaned => "✅",
                crate::worktree::cleanup::CleanupAction::Skipped => "⚠️",
                crate::worktree::cleanup::CleanupAction::Failed => "❌",
                crate::worktree::cleanup::CleanupAction::MergedToFeature => "🔀",
                crate::worktree::cleanup::CleanupAction::BackedUpToOrigin => "☁️",
                crate::worktree::cleanup::CleanupAction::StashCreated => "📦",
            };
            
            println!("  {} {} - {}", 
                     action_icon, 
                     result.branch.cyan(), 
                     result.reason);
            
            if let Some(error) = &result.error {
                println!("    Error: {}", error.red());
            }
        }
    }
    
    println!();
    if report.cleaned_count > 0 && !report.was_dry_run {
        println!("{} Cleanup completed successfully!", "🎉".green());
    } else if report.was_dry_run {
        println!("{} Run without --dry-run to execute changes", "💡".blue());
    }
}

/// Prompt user for confirmation
fn prompt_for_confirmation(message: &str) -> Result<bool> {
    use std::io::{self, Write};

    print!("{} (y/N): ", message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
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
    let config_path = cli
        .config
        .unwrap_or_else(workspace::constants::get_default_config_path);

    let mut workspace_manager =
        WorkspaceManager::new_with_root_override(config_path.clone(), cli.root).await?;

    match cli.command {
        None => {
            // No command provided, start menu mode
            prompts::run_menu_mode(&mut workspace_manager).await?;
        }
        Some(command) => match command {
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

            Commands::Create {
                name,
                app,
                no_configure,
                no_open,
            } => {
                use ui::workflows::{execute_workflow, CreateRepositoryWorkflow};

                // Use workflow system for repository creation
                let workflow = Box::new(CreateRepositoryWorkflow {
                    suggested_name: name,
                    app,
                    skip_configure: no_configure,
                    skip_open: no_open,
                });

                execute_workflow(workflow, &mut workspace_manager).await?;
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

                GitCommands::Worktree { action } => {
                    handle_worktree_command(action, &workspace_manager, cli.verbose).await?;
                }
            },

            Commands::Open {
                repo,
                app,
                no_itermocil,
            } => {
                // Find repository using flexible lookup
                let repo_info = workspace_manager.get_repository_flexible(&repo)
                    .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found. Try 'vibe launch' to see available repositories.", repo))?;

                let repo_name = &repo_info.name;

                if let Some(app_name) = app {
                    // Open with specific app
                    workspace_manager
                        .open_repo_with_app_options(repo_name, &app_name, no_itermocil)
                        .await?;
                } else {
                    // Open with preferred app or show available options
                    let configured_apps = workspace_manager.list_apps_for_repo(repo_name)?;

                    if configured_apps.len() == 1 {
                        // Only one app configured, use it
                        let (app_name, _) = &configured_apps[0];
                        workspace_manager
                            .open_repo_with_app_options(repo_name, app_name, no_itermocil)
                            .await?;
                    } else if configured_apps.len() > 1 {
                        // Multiple apps configured, show configured options
                        display_println!(
                            "{} Multiple apps configured for '{}'. Please specify one:",
                            style("🤔").yellow(),
                            style(repo_name).cyan()
                        );
                        for (app_name, _template) in &configured_apps {
                            display_println!(
                                "  {} vibe open {} --app {}",
                                style("→").dim(),
                                repo_name,
                                style(app_name).green()
                            );
                        }
                    } else {
                        // No apps configured, show available apps with basic opening
                        display_println!(
                            "{} No templates configured for '{}'. Available apps for basic opening:",
                            style("ℹ️").blue(),
                            style(repo_name).cyan()
                        );

                        let mut available_apps = Vec::new();
                        for app in &["vscode", "cursor", "warp", "iterm2", "wezterm", "windsurf"] {
                            if workspace_manager.is_app_available(app).await {
                                available_apps.push(*app);
                                display_println!(
                                    "  {} vibe open {} --app {} {} (basic mode)",
                                    style("→").dim(),
                                    repo_name,
                                    style(app).green(),
                                    style("").dim()
                                );
                            }
                        }

                        if available_apps.is_empty() {
                            display_println!(
                                "{} No supported apps found on this system",
                                style("⚠️").yellow()
                            );
                        } else {
                            display_println!(
                                "\n{} Or configure templates for enhanced features: vibe apps configure {} <app>",
                                style("💡").yellow(),
                                style(repo_name).cyan()
                            );
                        }
                    }
                }
            }

            Commands::Launch => {
                // Use the QuickLauncher for interactive selection
                let cache_dir = workspace::constants::get_cache_dir();
                let launcher = ui::quick_launcher::QuickLauncher::new(&cache_dir).await?;
                launcher.launch(&mut workspace_manager).await?;
            }

            Commands::Clone {
                url,
                app,
                no_configure,
                no_open,
                all,
                exclude,
                include,
                force,
            } => {
                let git_config = git::GitConfig::default();

                // Handle bulk cloning mode
                if all {
                    use git::bulk_clone::{BulkCloneCommand, BulkCloneOptions};

                    let exclude_patterns = exclude
                        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                        .unwrap_or_default();

                    let include_patterns = include
                        .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
                        .unwrap_or_default();

                    let options = BulkCloneOptions {
                        exclude_patterns,
                        include_patterns,
                        skip_existing: true,
                        custom_path: None,
                        force,
                    };

                    match BulkCloneCommand::execute(
                        url,
                        options,
                        &mut workspace_manager,
                        &git_config,
                    )
                    .await
                    {
                        Ok(result) => {
                            display_println!(
                                "{} Bulk clone completed: {} successful, {} failed",
                                style("✅").green().bold(),
                                result.total_cloned,
                                result.failed.len()
                            );
                        }
                        Err(e) => {
                            display_println!("{} Bulk clone failed: {}", style("❌").red(), e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    // Handle single repository cloning with enhanced detection
                    use git::clone::EnhancedCloneCommand;

                    match EnhancedCloneCommand::execute_with_detection(
                        url,
                        app,
                        no_configure,
                        no_open,
                        &mut workspace_manager,
                        &git_config,
                    )
                    .await
                    {
                        Ok(_) => {
                            display_println!(
                                "{} Repository operation completed successfully!",
                                style("✓").green().bold()
                            );
                        }
                        Err(e) => {
                            display_println!("{} Clone operation failed: {}", style("❌").red(), e);
                            std::process::exit(1);
                        }
                    }
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

            Commands::Guide => {
                print_getting_started_guide();
            }
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
        style("vibe clone owner/repo").cyan()
    );
    display_println!();

    display_println!("{}", style("Basic Commands").yellow().bold());
    display_println!(
        "  {} - Interactive menu with quick launch",
        style("vibe").cyan()
    );
    display_println!(
        "  {} - Clone, configure, and open in one command",
        style("vibe clone <url>").cyan()
    );
    display_println!(
        "  {} - Interactive recent repository selector (1-9)",
        style("vibe launch").cyan()
    );
    display_println!(
        "  {} - Open specific repository",
        style("vibe open <repo>").cyan()
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
        "• Run {} to see recent repositories for selection",
        style("vibe launch").cyan()
    );
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
        style("vibe clone <owner/repo>").cyan()
    );
    display_println!("3. Configure your favorite apps");
    display_println!();
}

/// Handle worktree configuration subcommands
async fn handle_worktree_config_command(
    command: WorktreeConfigCommands,
    workspace_manager: &WorkspaceManager,
) -> Result<()> {
    use crate::worktree::{WorktreeManager, WorktreeConfig};
    use colored::*;

    match command {
        WorktreeConfigCommands::Show { repository, format } => {
            let worktree_manager = if let Some(repo_name) = repository {
                // Load repository-specific configuration
                let repo_path = workspace_manager
                    .get_repository(&repo_name)
                    .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_name))?
                    .path
                    .clone();
                WorktreeManager::new_with_workspace_manager(workspace_manager, Some(repo_path)).await?
            } else {
                // Load global configuration
                WorktreeManager::new_with_workspace_manager(workspace_manager, None).await?
            };

            match format.as_str() {
                "yaml" => {
                    let config = worktree_manager.get_config();
                    let yaml = serde_yaml::to_string(config)?;
                    println!("{}", yaml);
                }
                "json" => {
                    let config = worktree_manager.get_config();
                    let json = serde_json::to_string_pretty(config)?;
                    println!("{}", json);
                }
                _ => {
                    // Summary format (default)
                    if let Ok(summary) = worktree_manager.get_config_summary().await {
                        println!("{}", summary.format_summary());
                    } else {
                        // Fallback to basic config display
                        let config = worktree_manager.get_config();
                        println!("{}", "Worktree Configuration".cyan().bold());
                        println!("Prefix: {}", config.prefix.yellow());
                        println!("Base Directory: {}", config.base_dir.display().to_string().blue());
                        println!("Auto GitIgnore: {}", if config.auto_gitignore { "✅" } else { "❌" });
                        println!("Default Editor: {}", config.default_editor.green());
                        println!("\nCleanup Configuration:");
                        println!("  Age Threshold: {} hours", config.cleanup.age_threshold_hours);
                        println!("  Verify Remote: {}", if config.cleanup.verify_remote { "✅" } else { "❌" });
                        println!("  Auto Delete Branch: {}", if config.cleanup.auto_delete_branch { "✅" } else { "❌" });
                        println!("\nMerge Detection:");
                        println!("  Use GitHub CLI: {}", if config.merge_detection.use_github_cli { "✅" } else { "❌" });
                        println!("  Methods: {}", config.merge_detection.methods.join(", "));
                        println!("  Main Branches: {}", config.merge_detection.main_branches.join(", "));
                    }
                }
            }
        }
        WorktreeConfigCommands::Set { key, value, repository: _ } => {
            // For now, just display what would be set (actual implementation would need to parse keys)
            println!("🔧 Configuration setting is not yet implemented");
            println!("Would set: {} = {}", key.yellow(), value.blue());
            println!("Use environment variables for now:");
            println!("  export VIBE_WORKTREE_PREFIX=\"{}\"", value);
            println!("  export VIBE_WORKTREE_BASE=\"{}\"", value);
            println!("  etc.");
        }
        WorktreeConfigCommands::Reset { key: _, repository: _ } => {
            println!("🔄 Configuration reset is not yet implemented");
            println!("Use 'unset' on environment variables for now");
        }
        WorktreeConfigCommands::Validate => {
            let worktree_manager = WorktreeManager::new_with_workspace_manager(workspace_manager, None).await?;
            let errors = worktree_manager.validate_configuration().await?;
            
            if errors.is_empty() {
                println!("{} Configuration is valid", "✅".green());
            } else {
                println!("{} Found {} configuration errors:", "❌".red(), errors.len());
                for error in &errors {
                    if let Some(repo) = &error.repository {
                        println!("  [{}] {}", repo.yellow(), error.error);
                    } else {
                        println!("  [global] {}", error.error);
                    }
                }
            }
        }
        WorktreeConfigCommands::Info => {
            println!("{}", WorktreeConfig::get_help_text());
        }
    }

    Ok(())
}
