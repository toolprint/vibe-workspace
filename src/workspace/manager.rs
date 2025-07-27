use anyhow::{Context, Result};
use colored::*;
use console::style;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::{
    config::{AppConfig, Repository, WorkspaceConfig},
    discovery::{
        discover_git_repositories, get_current_branch, get_remote_url, get_repository_name,
    },
    operations::{get_git_status, GitOperation, GitStatus},
    templates::TemplateManager,
};

#[derive(Debug, Clone)]
pub struct AppSelection {
    pub app: String,
    pub selected: bool,
    pub template: Option<String>,
    pub currently_configured: bool,
}

#[derive(Debug, Default)]
pub struct AppConfigState {
    pub warp: Option<String>, // template name if configured
    pub iterm2: Option<String>,
    pub wezterm: Option<String>,
    pub vscode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RepoWithStatus {
    pub name: String,
    pub path: String,
    pub apps: Vec<(String, String)>, // (app_name, template)
    pub git_status: GitStatus,
    pub display_string: String, // Formatted for display
}

pub struct WorkspaceManager {
    config_path: PathBuf,
    config: WorkspaceConfig,
    template_manager: TemplateManager,
}

impl WorkspaceManager {
    pub async fn new(config_path: PathBuf) -> Result<Self> {
        let config = WorkspaceConfig::load_from_file(&config_path).await?;

        let vibe_dir = dirs::home_dir().unwrap_or_default().join(".vibe-workspace");
        let template_manager = TemplateManager::new(vibe_dir.join("templates"));

        Ok(Self {
            config_path,
            config,
            template_manager,
        })
    }

    pub async fn new_with_root_override(
        config_path: PathBuf,
        root_override: Option<PathBuf>,
    ) -> Result<Self> {
        let mut config = WorkspaceConfig::load_from_file(&config_path).await?;

        // Override the workspace root if specified
        if let Some(root) = root_override {
            let expanded_root = crate::utils::fs::expand_tilde(&root);
            info!("Overriding workspace root to: {}", expanded_root.display());
            config.workspace.root = expanded_root;
        }

        let vibe_dir = dirs::home_dir().unwrap_or_default().join(".vibe-workspace");
        let template_manager = TemplateManager::new(vibe_dir.join("templates"));

        Ok(Self {
            config_path,
            config,
            template_manager,
        })
    }

    pub async fn init_workspace(&mut self, name: &str, root: &Path) -> Result<()> {
        info!("Initializing workspace '{}' in {}", name, root.display());

        // Update workspace configuration
        self.config.workspace.name = name.to_string();
        self.config.workspace.root = root.to_path_buf();

        // Auto-discover repositories if requested
        if self.config.workspace.auto_discover {
            let discovered = discover_git_repositories(root, 3).await?;

            for repo_path in discovered {
                let repo_name =
                    get_repository_name(&repo_path).unwrap_or_else(|| "unknown".to_string());

                let relative_path = repo_path
                    .strip_prefix(root)
                    .unwrap_or(&repo_path)
                    .to_path_buf();

                let mut repo = Repository::new(repo_name, relative_path);

                // Try to get remote URL and branch
                if let Ok(Some(url)) = get_remote_url(&repo_path) {
                    repo = repo.with_url(url);
                }

                if let Ok(Some(branch)) = get_current_branch(&repo_path) {
                    repo = repo.with_branch(branch);
                }

                self.config.add_repository(repo);
            }
        }

        // Save configuration
        self.save_config().await?;

        // Initialize default templates
        if let Err(e) = self.init_templates().await {
            warn!("Failed to initialize default templates: {}", e);
        }

        Ok(())
    }

    pub async fn discover_repositories(&self, path: &Path, depth: usize) -> Result<Vec<PathBuf>> {
        discover_git_repositories(path, depth).await
    }

    pub async fn add_discovered_repositories(&mut self, repo_paths: &[PathBuf]) -> Result<()> {
        let workspace_root = self.config.workspace.root.clone();

        for repo_path in repo_paths {
            let repo_name = get_repository_name(repo_path).unwrap_or_else(|| "unknown".to_string());

            let relative_path = repo_path
                .strip_prefix(&workspace_root)
                .unwrap_or(repo_path)
                .to_path_buf();

            let mut repo = Repository::new(repo_name, relative_path);

            // Try to get additional repository information
            if let Ok(Some(url)) = get_remote_url(repo_path) {
                repo = repo.with_url(url);
            }

            if let Ok(Some(branch)) = get_current_branch(repo_path) {
                repo = repo.with_branch(branch);
            }

            self.config.add_repository(repo);
        }

        self.save_config().await?;
        Ok(())
    }

    pub async fn show_status(
        &self,
        dirty_only: bool,
        format: &str,
        group: Option<&str>,
    ) -> Result<()> {
        let repositories = if let Some(group_name) = group {
            self.config.get_repositories_in_group(group_name)
        } else {
            self.config.repositories.iter().collect()
        };

        if repositories.is_empty() {
            println!("{} No repositories found", style("ℹ").yellow());
            return Ok(());
        }

        println!(
            "{} Checking status for {} repositories...",
            style("🔍").blue(),
            repositories.len()
        );

        let mut statuses = Vec::new();

        for repo in repositories {
            let repo_path = self.config.workspace.root.join(&repo.path);

            match get_git_status(&repo_path).await {
                Ok(status) => {
                    if !dirty_only || status.is_dirty() {
                        statuses.push(status);
                    }
                }
                Err(e) => {
                    warn!("Failed to get status for {}: {}", repo.name, e);
                    eprintln!(
                        "{} Failed to get status for {}: {}",
                        style("⚠").yellow(),
                        style(&repo.name).cyan(),
                        e
                    );
                }
            }
        }

        if statuses.is_empty() {
            if dirty_only {
                println!("{} All repositories are clean", style("✓").green());
            } else {
                println!("{} No repositories to display", style("ℹ").yellow());
            }
            return Ok(());
        }

        match format {
            "json" => {
                let json = serde_json::to_string_pretty(&statuses)
                    .context("Failed to serialize status to JSON")?;
                println!("{json}");
            }
            "compact" => {
                for status in &statuses {
                    let indicator = if status.clean {
                        "✓".green()
                    } else {
                        "●".red()
                    };
                    println!("{} {}", indicator, status.repository_name.cyan());
                }
            }
            _ => {
                println!();
                for status in &statuses {
                    println!("{}", status.format_status_line());
                }

                // Summary
                println!();
                let clean_count = statuses.iter().filter(|s| s.clean).count();
                let dirty_count = statuses.len() - clean_count;

                println!(
                    "{} {} clean, {} with changes",
                    style("📊").blue(),
                    style(clean_count).green(),
                    style(dirty_count).red()
                );
            }
        }

        Ok(())
    }

    pub async fn execute_command(
        &self,
        command: &str,
        repos: Option<&str>,
        group: Option<&str>,
        parallel: bool,
    ) -> Result<()> {
        let repositories = self.get_target_repositories(repos, group);

        if repositories.is_empty() {
            println!(
                "{} No repositories found to execute command on",
                style("ℹ").yellow()
            );
            return Ok(());
        }

        println!(
            "{} Executing '{}' on {} repositories...",
            style("⚡").blue(),
            style(command).cyan(),
            repositories.len()
        );

        let operation = GitOperation::Custom(command.to_string());

        if parallel {
            // Execute commands in parallel
            let mut tasks = Vec::new();

            for repo in repositories {
                let repo_path = self.config.workspace.root.join(&repo.path);
                let operation = operation.clone();
                let repo_name = repo.name.clone();

                let task =
                    tokio::spawn(async move { (repo_name, operation.execute(&repo_path).await) });

                tasks.push(task);
            }

            // Wait for all tasks to complete
            for task in tasks {
                match task.await {
                    Ok((repo_name, result)) => match result {
                        Ok(output) => {
                            if !output.trim().is_empty() {
                                println!(
                                    "{} {}:\n{}",
                                    style("✓").green(),
                                    style(&repo_name).cyan(),
                                    output
                                );
                            } else {
                                println!(
                                    "{} {} (no output)",
                                    style("✓").green(),
                                    style(&repo_name).cyan()
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "{} {} failed: {}",
                                style("✗").red(),
                                style(&repo_name).cyan(),
                                e
                            );
                        }
                    },
                    Err(e) => {
                        eprintln!("{} Task failed: {}", style("✗").red(), e);
                    }
                }
            }
        } else {
            // Execute commands sequentially
            for repo in repositories {
                let repo_path = self.config.workspace.root.join(&repo.path);

                print!(
                    "{} Executing on {}... ",
                    style("→").dim(),
                    style(&repo.name).cyan()
                );

                match operation.execute(&repo_path).await {
                    Ok(output) => {
                        println!("{}", style("✓").green());
                        if !output.trim().is_empty() {
                            println!("{output}");
                        }
                    }
                    Err(e) => {
                        println!("{}", style("✗").red());
                        eprintln!("  Error: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn sync_repositories(
        &self,
        fetch_only: bool,
        prune: bool,
        group: Option<&str>,
    ) -> Result<()> {
        let repositories = if let Some(group_name) = group {
            self.config.get_repositories_in_group(group_name)
        } else {
            self.config.repositories.iter().collect()
        };

        if repositories.is_empty() {
            println!("{} No repositories found", style("ℹ").yellow());
            return Ok(());
        }

        let action = if fetch_only { "Fetching" } else { "Syncing" };
        println!(
            "{} {} {} repositories...",
            style("🔄").blue(),
            action,
            repositories.len()
        );

        let mut operations = vec![GitOperation::Fetch];
        if prune {
            operations.push(GitOperation::Custom("fetch --prune".to_string()));
        }
        if !fetch_only {
            operations.push(GitOperation::Pull);
        }

        for repo in repositories {
            let repo_path = self.config.workspace.root.join(&repo.path);

            print!("{} {}... ", style("→").dim(), style(&repo.name).cyan());

            let mut success = true;
            for operation in &operations {
                match operation.execute(&repo_path).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{}", style("✗").red());
                        eprintln!("  Error: {e}");
                        success = false;
                        break;
                    }
                }
            }

            if success {
                println!("{}", style("✓").green());
            }
        }

        Ok(())
    }

    fn get_target_repositories(
        &self,
        repos: Option<&str>,
        group: Option<&str>,
    ) -> Vec<&Repository> {
        if let Some(group_name) = group {
            self.config.get_repositories_in_group(group_name)
        } else if let Some(repo_names) = repos {
            repo_names
                .split(',')
                .filter_map(|name| self.config.get_repository(name.trim()))
                .collect()
        } else {
            self.config.repositories.iter().collect()
        }
    }

    pub fn get_workspace_root(&self) -> &PathBuf {
        &self.config.workspace.root
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub async fn add_repository(&mut self, repo: Repository) -> Result<()> {
        self.config.add_repository(repo);
        self.save_config().await
    }

    pub fn get_config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub fn get_template_manager(&self) -> &TemplateManager {
        &self.template_manager
    }

    async fn save_config(&self) -> Result<()> {
        self.config.save_to_file(&self.config_path).await
    }

    pub async fn init_config(
        &mut self,
        name: Option<&str>,
        root: Option<&Path>,
        auto_discover: bool,
    ) -> Result<()> {
        info!("Initializing workspace configuration");

        // Set workspace name
        if let Some(n) = name {
            self.config.workspace.name = n.to_string();
        } else {
            let current_dir = std::env::current_dir()?;
            self.config.workspace.name = current_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "workspace".to_string());
        }

        // Set workspace root
        if let Some(r) = root {
            self.config.workspace.root = r.to_path_buf();
        } else {
            self.config.workspace.root = std::env::current_dir()?;
        }

        // Set auto-discover
        self.config.workspace.auto_discover = auto_discover;

        // Auto-discover repositories if enabled
        if auto_discover {
            let discovered = discover_git_repositories(&self.config.workspace.root, 3).await?;
            for repo_path in discovered {
                let repo_name =
                    get_repository_name(&repo_path).unwrap_or_else(|| "unknown".to_string());

                let relative_path = repo_path
                    .strip_prefix(&self.config.workspace.root)
                    .unwrap_or(&repo_path)
                    .to_path_buf();

                let mut repo = Repository::new(repo_name, relative_path);

                // Try to get remote URL and branch
                if let Ok(Some(url)) = get_remote_url(&repo_path) {
                    repo = repo.with_url(url);
                }

                if let Ok(Some(branch)) = get_current_branch(&repo_path) {
                    repo = repo.with_branch(branch);
                }

                self.config.add_repository(repo);
            }
        }

        // Save configuration
        self.save_config().await?;

        println!(
            "{} Initialized workspace '{}' in {}",
            style("✓").green().bold(),
            style(&self.config.workspace.name).cyan().bold(),
            style(self.config.workspace.root.display()).dim()
        );

        if auto_discover && !self.config.repositories.is_empty() {
            println!(
                "{} Auto-discovered {} repositories",
                style("📁").green(),
                self.config.repositories.len()
            );
        }

        Ok(())
    }

    pub async fn edit_config(&self, direct: bool) -> Result<()> {
        use std::process::Command;

        // Get editor from environment
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });

        if !direct {
            println!(
                "{} Opening config file in {}...",
                style("📝").blue(),
                style(&editor).cyan()
            );
        }

        // Open editor
        let status = Command::new(&editor)
            .arg(&self.config_path)
            .status()
            .with_context(|| format!("Failed to open editor: {editor}"))?;

        if !status.success() {
            anyhow::bail!("Editor exited with non-zero status");
        }

        println!(
            "{} Configuration edited successfully",
            style("✓").green().bold()
        );

        Ok(())
    }

    pub async fn show_config(&self, format: &str, section: Option<&str>) -> Result<()> {
        let output = match section {
            Some("workspace") => match format {
                "json" => serde_json::to_string_pretty(&self.config.workspace)?,
                "pretty" => format!(
                    "🏗️  Workspace Configuration\n\
                     ━━━━━━━━━━━━━━━━━━━━━━━━\n\
                     Name: {}\n\
                     Root: {}\n\
                     Auto-discover: {}",
                    style(&self.config.workspace.name).cyan(),
                    style(self.config.workspace.root.display()).dim(),
                    if self.config.workspace.auto_discover {
                        style("enabled").green()
                    } else {
                        style("disabled").red()
                    }
                ),
                _ => serde_yaml::to_string(&self.config.workspace)?,
            },
            Some("repositories") => match format {
                "json" => serde_json::to_string_pretty(&self.config.repositories)?,
                "pretty" => {
                    let mut output = format!(
                        "📁 Repositories ({})\n━━━━━━━━━━━━━━━━━",
                        self.config.repositories.len()
                    );
                    for repo in &self.config.repositories {
                        output.push_str(&format!(
                            "\n\n• {}\n  Path: {}\n  URL: {}\n  Branch: {}",
                            style(&repo.name).cyan().bold(),
                            style(repo.path.display()).dim(),
                            repo.url.as_deref().unwrap_or("(none)"),
                            repo.branch.as_deref().unwrap_or("(default)")
                        ));
                    }
                    output
                }
                _ => serde_yaml::to_string(&self.config.repositories)?,
            },
            Some("groups") => match format {
                "json" => serde_json::to_string_pretty(&self.config.groups)?,
                "pretty" => {
                    let mut output =
                        format!("👥 Groups ({})\n━━━━━━━━━━━━", self.config.groups.len());
                    for group in &self.config.groups {
                        output.push_str(&format!(
                            "\n\n• {}\n  Repositories: {}",
                            style(&group.name).cyan().bold(),
                            group.repos.join(", ")
                        ));
                    }
                    output
                }
                _ => serde_yaml::to_string(&self.config.groups)?,
            },
            Some("apps") => match format {
                "json" => serde_json::to_string_pretty(&self.config.apps)?,
                "pretty" => {
                    let mut output = "🔧 App Integrations\n━━━━━━━━━━━━━━━━━".to_string();

                    if let Some(github) = &self.config.apps.github {
                        output.push_str(&format!(
                            "\n\n• GitHub: {}\n  Token source: {}",
                            if github.enabled {
                                style("enabled").green()
                            } else {
                                style("disabled").red()
                            },
                            github.token_source
                        ));
                    }

                    if let Some(warp) = &self.config.apps.warp {
                        output.push_str(&format!(
                            "\n\n• Warp: {}\n  Config dir: {}",
                            if warp.enabled {
                                style("enabled").green()
                            } else {
                                style("disabled").red()
                            },
                            warp.config_dir.display()
                        ));
                    }

                    if let Some(iterm2) = &self.config.apps.iterm2 {
                        output.push_str(&format!(
                            "\n\n• iTerm2: {}\n  Config dir: {}",
                            if iterm2.enabled {
                                style("enabled").green()
                            } else {
                                style("disabled").red()
                            },
                            iterm2.config_dir.display()
                        ));
                    }

                    if let Some(vscode) = &self.config.apps.vscode {
                        output.push_str(&format!(
                            "\n\n• VSCode: {}\n  Workspace dir: {}",
                            if vscode.enabled {
                                style("enabled").green()
                            } else {
                                style("disabled").red()
                            },
                            vscode.workspace_dir.display()
                        ));
                    }

                    output
                }
                _ => serde_yaml::to_string(&self.config.apps)?,
            },
            _ => match format {
                "json" => serde_json::to_string_pretty(&self.config)?,
                "pretty" => {
                    // Show all sections in pretty format
                    let mut output = String::new();

                    // Workspace section
                    output.push_str(&format!(
                        "🏗️  Workspace Configuration\n\
                         ━━━━━━━━━━━━━━━━━━━━━━━━\n\
                         Name: {}\n\
                         Root: {}\n\
                         Auto-discover: {}\n\n",
                        style(&self.config.workspace.name).cyan(),
                        style(self.config.workspace.root.display()).dim(),
                        if self.config.workspace.auto_discover {
                            style("enabled").green()
                        } else {
                            style("disabled").red()
                        }
                    ));

                    // Repositories section
                    output.push_str(&format!(
                        "📁 Repositories ({})\n━━━━━━━━━━━━━━━━━",
                        self.config.repositories.len()
                    ));
                    for repo in &self.config.repositories {
                        output.push_str(&format!(
                            "\n• {} ({})",
                            style(&repo.name).cyan(),
                            style(repo.path.display()).dim()
                        ));
                    }

                    // Groups section
                    if !self.config.groups.is_empty() {
                        output.push_str(&format!(
                            "\n\n👥 Groups ({})\n━━━━━━━━━━━",
                            self.config.groups.len()
                        ));
                        for group in &self.config.groups {
                            output.push_str(&format!(
                                "\n• {} ({} repos)",
                                style(&group.name).cyan(),
                                group.repos.len()
                            ));
                        }
                    }

                    output
                }
                _ => serde_yaml::to_string(&self.config)?,
            },
        };

        println!("{output}");
        Ok(())
    }

    pub async fn validate_config(
        &self,
        check_paths: bool,
        check_remotes: bool,
        check_apps: bool,
    ) -> Result<()> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        println!(
            "{} Validating workspace configuration...",
            style("🔍").blue()
        );

        // Check workspace root
        if !self.config.workspace.root.exists() {
            issues.push(format!(
                "Workspace root does not exist: {}",
                self.config.workspace.root.display()
            ));
        }

        // Check repository paths
        if check_paths {
            println!("  {} Checking repository paths...", style("→").dim());
            for repo in &self.config.repositories {
                let repo_path = self.config.workspace.root.join(&repo.path);
                if !repo_path.exists() {
                    issues.push(format!(
                        "Repository path does not exist: {} ({})",
                        repo.name,
                        repo_path.display()
                    ));
                } else if !repo_path.join(".git").exists() {
                    warnings.push(format!(
                        "Path exists but is not a git repository: {} ({})",
                        repo.name,
                        repo_path.display()
                    ));
                }
            }
        }

        // Check remote URLs
        if check_remotes {
            println!("  {} Checking remote URLs...", style("→").dim());
            for repo in &self.config.repositories {
                if let Some(url) = &repo.url {
                    // Basic URL validation
                    if !url.starts_with("https://")
                        && !url.starts_with("git@")
                        && !url.starts_with("ssh://")
                    {
                        warnings.push(format!(
                            "Unusual remote URL format for {}: {}",
                            repo.name, url
                        ));
                    }
                }
            }
        }

        // Check app integrations
        if check_apps {
            println!("  {} Checking app integrations...", style("→").dim());

            if let Some(warp) = &self.config.apps.warp {
                if warp.enabled && !warp.config_dir.exists() {
                    warnings.push(format!(
                        "Warp config directory does not exist: {}",
                        warp.config_dir.display()
                    ));
                }
            }

            if let Some(iterm2) = &self.config.apps.iterm2 {
                if iterm2.enabled && !iterm2.config_dir.exists() {
                    warnings.push(format!(
                        "iTerm2 config directory does not exist: {}",
                        iterm2.config_dir.display()
                    ));
                }
            }

            if let Some(vscode) = &self.config.apps.vscode {
                if vscode.enabled && !vscode.workspace_dir.exists() {
                    warnings.push(format!(
                        "VSCode workspace directory does not exist: {}",
                        vscode.workspace_dir.display()
                    ));
                }
            }
        }

        // Check groups reference existing repositories
        for group in &self.config.groups {
            for repo_name in &group.repos {
                if !self
                    .config
                    .repositories
                    .iter()
                    .any(|r| &r.name == repo_name)
                {
                    issues.push(format!(
                        "Group '{}' references non-existent repository: {}",
                        group.name, repo_name
                    ));
                }
            }
        }

        // Report results
        println!();
        if issues.is_empty() && warnings.is_empty() {
            println!("{} Configuration is valid!", style("✓").green().bold());
        } else {
            if !issues.is_empty() {
                println!("{} Issues found:", style("❌").red().bold());
                for issue in &issues {
                    println!("  • {issue}");
                }
            }

            if !warnings.is_empty() {
                println!("\n{} Warnings:", style("⚠️").yellow().bold());
                for warning in &warnings {
                    println!("  • {warning}");
                }
            }

            if !issues.is_empty() {
                anyhow::bail!(
                    "Configuration validation failed with {} issues",
                    issues.len()
                );
            }
        }

        Ok(())
    }

    // App configuration management methods

    /// Configure an app for a repository
    pub async fn configure_app_for_repo(
        &mut self,
        repo_name: &str,
        app: &str,
        template: &str,
    ) -> Result<()> {
        let repo = self
            .config
            .repositories
            .iter_mut()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        // Check if template exists
        let templates = self.template_manager.list_templates(app).await?;
        if !templates.contains(&template.to_string()) {
            anyhow::bail!("Template '{}' not found for app '{}'", template, app);
        }

        repo.apps.insert(
            app.to_string(),
            AppConfig::WithTemplate {
                template: template.to_string(),
            },
        );

        self.config.save_to_file(&self.config_path).await?;

        Ok(())
    }

    /// List configured apps for a repository
    pub fn list_apps_for_repo(&self, repo_name: &str) -> Result<Vec<(String, String)>> {
        let repo = self
            .config
            .repositories
            .iter()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        let mut apps = Vec::new();
        for (app_name, config) in &repo.apps {
            if let AppConfig::WithTemplate { template } = config {
                apps.push((app_name.clone(), template.clone()));
            } else if config.is_enabled() {
                apps.push((app_name.clone(), "default".to_string()));
            }
        }

        Ok(apps)
    }

    /// Get repositories that have at least one app configured, with git status
    pub async fn get_repos_with_apps_and_status(&self) -> Result<Vec<RepoWithStatus>> {
        let mut repos_with_status = Vec::new();

        for repo in &self.config.repositories {
            let apps = self.list_apps_for_repo(&repo.name)?;

            // Only include repos that have at least one app configured
            if !apps.is_empty() {
                let repo_path = self.config.workspace.root.join(&repo.path);
                let git_status = get_git_status(&repo_path)
                    .await
                    .unwrap_or_else(|_| GitStatus {
                        repository_name: repo.name.clone(),
                        path: repo.path.display().to_string(),
                        branch: None,
                        clean: true,
                        ahead: 0,
                        behind: 0,
                        staged: 0,
                        unstaged: 0,
                        untracked: 0,
                        remote_url: None,
                    });

                // Create display string with status indicators
                let status_indicator = if git_status.clean {
                    "✓".to_string()
                } else {
                    let mut indicators = Vec::new();
                    if git_status.staged > 0 {
                        indicators.push(format!("{}S", git_status.staged));
                    }
                    if git_status.unstaged > 0 {
                        indicators.push(format!("{}U", git_status.unstaged));
                    }
                    if git_status.untracked > 0 {
                        indicators.push(format!("{}?", git_status.untracked));
                    }
                    if git_status.ahead > 0 {
                        indicators.push(format!("↑{}", git_status.ahead));
                    }
                    if git_status.behind > 0 {
                        indicators.push(format!("↓{}", git_status.behind));
                    }
                    if indicators.is_empty() {
                        "●".to_string()
                    } else {
                        indicators.join(" ")
                    }
                };

                let app_names: Vec<String> = apps.iter().map(|(name, _)| name.clone()).collect();
                let display_string = format!(
                    "{} [{}] (apps: {})",
                    repo.name,
                    status_indicator,
                    app_names.join(", ")
                );

                repos_with_status.push(RepoWithStatus {
                    name: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    apps,
                    git_status,
                    display_string,
                });
            }
        }

        Ok(repos_with_status)
    }

    /// List repositories configured with a specific app
    pub fn list_repos_with_app(&self, app: &str) -> Vec<(&Repository, String)> {
        let mut repos = Vec::new();

        for repo in &self.config.repositories {
            if let Some(config) = repo.apps.get(app) {
                if config.is_enabled() {
                    let template = match config {
                        AppConfig::WithTemplate { template } => template.clone(),
                        AppConfig::WithConfig { template, .. } => template.clone(),
                        AppConfig::Enabled(_) => "default".to_string(),
                    };
                    repos.push((repo, template));
                }
            }
        }

        repos
    }

    /// Initialize default templates if they don't exist
    pub async fn init_templates(&self) -> Result<()> {
        self.template_manager.init_default_templates().await?;

        println!(
            "{} Initialized default templates in {}",
            style("✓").green().bold(),
            style("~/.vibe-workspace/templates").cyan()
        );

        Ok(())
    }

    /// List available templates for an app
    pub async fn list_templates(&self, app: &str) -> Result<Vec<String>> {
        self.template_manager.list_templates(app).await
    }

    /// Create a new template from an existing one
    pub async fn create_template(
        &self,
        app: &str,
        template_name: &str,
        from_template: &str,
    ) -> Result<()> {
        let content = self
            .template_manager
            .load_template(app, from_template)
            .await?;
        self.template_manager
            .save_template(app, template_name, &content)
            .await?;

        println!(
            "{} Created template '{}' for {}",
            style("✓").green().bold(),
            style(template_name).cyan(),
            style(app).cyan()
        );

        Ok(())
    }

    /// Delete a template
    pub async fn delete_template(&self, app: &str, template_name: &str) -> Result<()> {
        if template_name == "default" {
            anyhow::bail!("Cannot delete the default template");
        }

        self.template_manager
            .delete_template(app, template_name)
            .await?;

        println!(
            "{} Deleted template '{}' for {}",
            style("✓").green().bold(),
            style(template_name).cyan(),
            style(app).cyan()
        );

        Ok(())
    }

    /// Show configured apps for all repositories
    pub async fn show_app_configurations(&self) -> Result<()> {
        println!("\n{} App Configurations:", style("📱").blue());
        println!();

        for repo in &self.config.repositories {
            if repo.apps.is_empty() {
                continue;
            }

            println!("{} {}", style("→").dim(), style(&repo.name).cyan().bold());
            for (app_name, config) in &repo.apps {
                if config.is_enabled() {
                    let template = match config {
                        AppConfig::WithTemplate { template } => template.as_str(),
                        AppConfig::WithConfig { template, .. } => template.as_str(),
                        AppConfig::Enabled(_) => "default",
                    };
                    println!(
                        "    {} {} (template: {})",
                        style("•").dim(),
                        style(app_name).green(),
                        style(template).yellow()
                    );
                }
            }
            println!();
        }
        Ok(())
    }

    /// Get the default template content for an app
    pub async fn get_default_template(&self, app: &str) -> Result<String> {
        self.template_manager.load_template(app, "default").await
    }

    /// Save a template with content
    pub async fn save_template(&self, app: &str, name: &str, content: &str) -> Result<()> {
        self.template_manager
            .save_template(app, name, content)
            .await
    }

    /// Update default templates with current bundled versions
    pub async fn update_default_templates(&self, apps: Vec<String>) -> Result<()> {
        for app in apps {
            let default_content = match app.as_str() {
                "warp" => crate::workspace::templates::DEFAULT_WARP_TEMPLATE,
                "iterm2" => crate::workspace::templates::DEFAULT_ITERM2_TEMPLATE,
                "wezterm" => crate::workspace::templates::DEFAULT_WEZTERM_TEMPLATE,
                "vscode" => crate::workspace::templates::DEFAULT_VSCODE_TEMPLATE,
                _ => {
                    println!("{} Unknown app '{}', skipping", style("⚠️").yellow(), app);
                    continue;
                }
            };

            self.template_manager
                .save_template(&app, "default", default_content)
                .await?;
            println!(
                "{} Updated default template for {}",
                style("✓").green(),
                style(&app).cyan()
            );
        }

        Ok(())
    }

    /// Open a repository with a configured app
    pub async fn open_repo_with_app(&self, repo_name: &str, app: &str) -> Result<()> {
        self.open_repo_with_app_options(repo_name, app, false).await
    }

    /// Open a repository with a configured app with options
    pub async fn open_repo_with_app_options(
        &self,
        repo_name: &str,
        app: &str,
        no_itermocil: bool,
    ) -> Result<()> {
        let repo = self
            .config
            .repositories
            .iter()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        if !repo.is_app_enabled(app) {
            anyhow::bail!(
                "App '{}' is not configured for repository '{}'",
                app,
                repo_name
            );
        }

        match app {
            "warp" => {
                crate::apps::open_with_warp(&self.config, repo, &self.template_manager).await?;
            }
            "iterm2" => {
                crate::apps::open_with_iterm2_options(
                    &self.config,
                    repo,
                    &self.template_manager,
                    no_itermocil,
                )
                .await?;
            }
            "wezterm" => {
                crate::apps::open_with_wezterm_options(
                    &self.config,
                    repo,
                    &self.template_manager,
                    no_itermocil,
                )
                .await?;
            }
            "vscode" => {
                crate::apps::open_with_vscode(&self.config, repo, &self.template_manager).await?;
            }
            _ => {
                anyhow::bail!("Unknown app: {}", app);
            }
        }

        Ok(())
    }

    /// Get current app configuration states for a repository
    pub fn get_current_app_states(&self, repo_name: &str) -> Result<AppConfigState> {
        let repo = self
            .config
            .repositories
            .iter()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        let mut state = AppConfigState::default();

        for (app_name, config) in &repo.apps {
            if config.is_enabled() {
                let template = match config {
                    AppConfig::WithTemplate { template } => template.clone(),
                    AppConfig::WithConfig { template, .. } => template.clone(),
                    AppConfig::Enabled(_) => "default".to_string(),
                };

                match app_name.as_str() {
                    "warp" => state.warp = Some(template),
                    "iterm2" => state.iterm2 = Some(template),
                    "wezterm" => state.wezterm = Some(template),
                    "vscode" => state.vscode = Some(template),
                    _ => {} // ignore unknown apps
                }
            }
        }

        Ok(state)
    }

    /// Remove app configuration for a repository
    pub async fn remove_app_for_repo(&mut self, repo_name: &str, app: &str) -> Result<()> {
        let repo = self
            .config
            .repositories
            .iter_mut()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        repo.apps.remove(app);
        self.config.save_to_file(&self.config_path).await?;

        Ok(())
    }

    /// Clean up app-generated files for a repository
    pub async fn cleanup_app_files(&self, repo_name: &str, app: &str) -> Result<()> {
        let repo = self
            .config
            .repositories
            .iter()
            .find(|r| r.name == repo_name)
            .context("Repository not found")?;

        match app {
            "warp" => {
                crate::apps::cleanup_warp_config(&self.config, repo).await?;
            }
            "iterm2" => {
                crate::apps::cleanup_iterm2_config(&self.config, repo).await?;
            }
            "wezterm" => {
                crate::apps::cleanup_wezterm_config(&self.config, repo).await?;
            }
            "vscode" => {
                crate::apps::cleanup_vscode_config(&self.config, repo).await?;
            }
            _ => {
                warn!("Unknown app '{}' for cleanup", app);
            }
        }

        Ok(())
    }

    /// Configure multiple apps for a repository
    pub async fn configure_multiple_apps(
        &mut self,
        repo_name: &str,
        app_selections: Vec<AppSelection>,
    ) -> Result<Vec<String>> {
        let mut changes = Vec::new();

        // Get current state to determine what needs to be added/removed
        let current_state = self.get_current_app_states(repo_name)?;
        let current_apps = [
            ("warp", current_state.warp.as_ref()),
            ("iterm2", current_state.iterm2.as_ref()),
            ("vscode", current_state.vscode.as_ref()),
        ];

        // Process each app selection
        for selection in app_selections {
            let currently_configured = current_apps
                .iter()
                .find(|(app, _)| *app == selection.app)
                .map(|(_, template)| template.is_some())
                .unwrap_or(false);

            if selection.selected && !currently_configured {
                // Add new app configuration
                let template = selection.template.as_deref().unwrap_or("default");
                self.configure_app_for_repo(repo_name, &selection.app, template)
                    .await?;
                changes.push(format!(
                    "✅ Configured {} with template '{}'",
                    selection.app, template
                ));
            } else if selection.selected && currently_configured {
                // Update existing app configuration if template changed
                let current_template = current_apps
                    .iter()
                    .find(|(app, _)| *app == selection.app)
                    .and_then(|(_, template)| template.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("default");

                let new_template = selection.template.as_deref().unwrap_or("default");
                if current_template != new_template {
                    self.configure_app_for_repo(repo_name, &selection.app, new_template)
                        .await?;
                    changes.push(format!(
                        "🔄 Updated {} template to '{}'",
                        selection.app, new_template
                    ));
                }
            } else if !selection.selected && currently_configured {
                // Remove app configuration and clean up files
                self.cleanup_app_files(repo_name, &selection.app).await?;
                self.remove_app_for_repo(repo_name, &selection.app).await?;
                changes.push(format!("🗑️  Removed {} configuration", selection.app));
            }
        }

        Ok(changes)
    }

    /// Discover all configuration files that would be affected by reset or backup
    async fn discover_all_config_files(&self) -> Result<Vec<PathBuf>> {
        let mut config_files = Vec::new();

        // Main config file
        if self.config_path.exists() {
            config_files.push(self.config_path.clone());
        }

        // Templates directory
        let vibe_dir = dirs::home_dir().unwrap_or_default().join(".vibe-workspace");
        let templates_dir = vibe_dir.join("templates");
        if templates_dir.exists() {
            config_files.push(templates_dir);
        }

        // App configuration files for each repository
        for repo in &self.config.repositories {
            // Skip repos that don't have any app configurations
            if repo.apps.is_empty() {
                continue;
            }

            for app in repo.apps.keys() {
                match app.as_str() {
                    "warp" => {
                        if let Some(warp_integration) = &self.config.apps.warp {
                            let config_name =
                                format!("vibe-{}-{}.yaml", self.config.workspace.name, repo.name);
                            let config_path = warp_integration.config_dir.join(&config_name);
                            if config_path.exists() {
                                config_files.push(config_path);
                            }
                        }
                    }
                    "iterm2" => {
                        if let Some(iterm2_integration) = &self.config.apps.iterm2 {
                            let config_name =
                                format!("vibe-{}-{}.json", self.config.workspace.name, repo.name);
                            let config_path = iterm2_integration.config_dir.join(&config_name);
                            if config_path.exists() {
                                config_files.push(config_path);
                            }
                        }
                    }
                    "wezterm" => {
                        if let Some(wezterm_integration) = &self.config.apps.wezterm {
                            let config_name =
                                format!("vibe-{}-{}.lua", self.config.workspace.name, repo.name);
                            let config_path = wezterm_integration.config_dir.join(&config_name);
                            if config_path.exists() {
                                config_files.push(config_path);
                            }
                        }
                    }
                    "vscode" => {
                        if let Some(vscode_integration) = &self.config.apps.vscode {
                            let config_name = format!(
                                "vibe-{}-{}.code-workspace",
                                self.config.workspace.name, repo.name
                            );
                            let config_path = vscode_integration.workspace_dir.join(&config_name);
                            if config_path.exists() {
                                config_files.push(config_path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(config_files)
    }

    /// Clean up all app configuration files for all repositories
    async fn cleanup_all_app_configs(&self) -> Result<()> {
        for repo in &self.config.repositories {
            for app in repo.apps.keys() {
                if let Err(e) = self.cleanup_app_files(&repo.name, app).await {
                    warn!("Failed to cleanup {} config for {}: {}", app, repo.name, e);
                }
            }
        }
        Ok(())
    }

    /// Create a backup archive of all configuration files
    pub async fn create_backup(
        &self,
        output_dir: Option<PathBuf>,
        custom_name: Option<String>,
    ) -> Result<PathBuf> {
        use chrono::Utc;
        use std::process::Command;

        // Determine output directory - default to ~/.vibe-workspace/backups/
        let backup_dir = output_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vibe-workspace")
                .join("backups")
        });

        // Create backup directory if it doesn't exist
        tokio::fs::create_dir_all(&backup_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create backup directory: {}",
                    backup_dir.display()
                )
            })?;

        // Create timestamped backup name
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let backup_name = custom_name.unwrap_or_else(|| format!("vibe-backup-{timestamp}"));
        let backup_filename = format!("{backup_name}.tgz");
        let backup_path = backup_dir.join(&backup_filename);

        println!("{} Creating backup archive...", style("📦").blue());

        // Discover all configuration files
        let config_files = self.discover_all_config_files().await?;

        if config_files.is_empty() {
            println!(
                "{} No configuration files found to backup",
                style("⚠️").yellow()
            );
            return Ok(backup_path);
        }

        // Create temporary directory for organizing backup content
        let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
        let temp_path = temp_dir.path();

        // Copy files to temporary directory with organized structure
        for config_file in &config_files {
            let file_name = config_file
                .file_name()
                .context("Invalid file name")?
                .to_string_lossy();

            if config_file == &self.config_path {
                // Main config file goes to root
                let dest_path = temp_path.join("config.yaml");
                tokio::fs::copy(config_file, &dest_path)
                    .await
                    .with_context(|| format!("Failed to copy {}", config_file.display()))?;
            } else if config_file.to_string_lossy().contains("templates") {
                // Templates directory
                let dest_dir = temp_path.join("templates");
                tokio::fs::create_dir_all(&dest_dir).await?;
                copy_dir_recursive(config_file, &dest_dir)?;
            } else {
                // App config files - organize by app type
                let app_type = if file_name.ends_with(".yaml") {
                    "warp"
                } else if file_name.ends_with(".json") {
                    "iterm2"
                } else if file_name.ends_with(".code-workspace") {
                    "vscode"
                } else {
                    "other"
                };

                let app_dir = temp_path.join("app-configs").join(app_type);
                tokio::fs::create_dir_all(&app_dir).await?;
                let dest_path = app_dir.join(file_name.as_ref());
                tokio::fs::copy(config_file, &dest_path)
                    .await
                    .with_context(|| format!("Failed to copy {}", config_file.display()))?;
            }
        }

        // Create tar archive
        let tar_output = Command::new("tar")
            .args(["-czf"])
            .arg(&backup_path)
            .args(["-C"])
            .arg(temp_path)
            .arg(".")
            .output()
            .context("Failed to execute tar command")?;

        if !tar_output.status.success() {
            let error_msg = String::from_utf8_lossy(&tar_output.stderr);
            anyhow::bail!("Tar command failed: {}", error_msg);
        }

        println!(
            "{} Backup contains {} configuration files:",
            style("📋").green(),
            config_files.len()
        );
        for file in &config_files {
            println!("  {} {}", style("→").dim(), style(file.display()).cyan());
        }

        Ok(backup_path)
    }

    /// Factory reset - clear all configuration and reinitialize
    pub async fn factory_reset(&mut self, force: bool) -> Result<()> {
        self.factory_reset_with_options(force, false).await
    }

    pub async fn factory_reset_with_options(
        &mut self,
        force: bool,
        skip_final_confirmation: bool,
    ) -> Result<()> {
        if !force {
            // Show warning and get confirmation
            println!(
                "{} {}",
                style("⚠️  WARNING").red().bold(),
                style("This will permanently delete ALL vibe-workspace configuration!").red()
            );
            println!();

            // Discover and show files that will be deleted
            let config_files = self.discover_all_config_files().await?;

            if !config_files.is_empty() {
                println!("{} The following files will be deleted:", style("🗑️").red());
                for file in &config_files {
                    println!("  {} {}", style("×").red(), style(file.display()).dim());
                }
                println!();
            }

            // Require typing exact confirmation
            use inquire::Text;
            let confirmation = Text::new("Type 'reset my vibe' to confirm factory reset:")
                .prompt()
                .context("Failed to get user confirmation")?;

            if confirmation != "reset my vibe" {
                println!(
                    "{} Vibe Check: make sure you're ready for irreversable change and try again",
                    style("🔍").yellow()
                );
                return Ok(());
            }

            // Final confirmation (only if not skipped)
            if !skip_final_confirmation {
                use inquire::Confirm;
                let final_confirm = Confirm::new("Are you absolutely sure? This cannot be undone.")
                    .with_default(false)
                    .prompt()
                    .context("Failed to get final confirmation")?;

                if !final_confirm {
                    println!("{} Vibe Check: make sure you're ready for irreversable change and try again", style("🔍").yellow());
                    return Ok(());
                }
            }
        }

        println!("{} Performing factory reset...", style("🔄").blue());

        // Clean up all app configuration files first
        self.cleanup_all_app_configs().await?;

        // Delete main config file
        if self.config_path.exists() {
            tokio::fs::remove_file(&self.config_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove config file: {}",
                        self.config_path.display()
                    )
                })?;
            println!("{} Removed main configuration file", style("✓").green());
        }

        // Delete templates directory
        let vibe_dir = dirs::home_dir().unwrap_or_default().join(".vibe-workspace");
        let templates_dir = vibe_dir.join("templates");
        if templates_dir.exists() {
            tokio::fs::remove_dir_all(&templates_dir)
                .await
                .with_context(|| {
                    format!(
                        "Failed to remove templates directory: {}",
                        templates_dir.display()
                    )
                })?;
            println!("{} Removed templates directory", style("✓").green());
        }

        println!("{} Factory reset completed", style("✅").green().bold());
        println!();

        // Reinitialize workspace
        println!("{} Reinitializing workspace...", style("🚀").blue());
        let workspace_name = "workspace".to_string();
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        self.init_workspace(&workspace_name, &workspace_root)
            .await?;

        println!(
            "{} Workspace reinitialized successfully",
            style("✅").green().bold()
        );
        println!(
            "{} You can now configure repositories and apps",
            style("💡").yellow()
        );

        Ok(())
    }
}

// Helper function to recursively copy directories using std::fs
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    use std::fs;

    if src.is_dir() {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    } else {
        fs::copy(src, dst)?;
    }

    Ok(())
}
