use anyhow::{Context, Result};
use console::style;
use inquire::Confirm;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::git::provider::github_cli::GitHubCliProvider;
use crate::git::{GitConfig, Repository};
use crate::workspace::install::RepositoryInstaller;
use crate::workspace::manager::WorkspaceManager;

/// Options for bulk cloning operations
#[derive(Debug, Clone)]
pub struct BulkCloneOptions {
    pub exclude_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub skip_existing: bool,
    pub custom_path: Option<PathBuf>,
    pub force: bool, // Skip confirmation prompts
}

impl Default for BulkCloneOptions {
    fn default() -> Self {
        Self {
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            skip_existing: true,
            custom_path: None,
            force: false,
        }
    }
}

/// Target type for bulk cloning
#[derive(Debug, Clone, PartialEq)]
pub enum TargetType {
    User,
    Organization,
    Unknown,
}

/// Result of filtering repositories before cloning
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub to_clone: Vec<Repository>,
    pub skipped: Vec<SkippedRepository>,
}

/// Result of a bulk clone operation
#[derive(Debug, Clone)]
pub struct BulkCloneResult {
    pub total_discovered: usize,
    pub total_cloned: usize,
    pub skipped: Vec<SkippedRepository>,
    pub failed: Vec<FailedRepository>,
    pub successful: Vec<String>,
    pub duration: Duration,
}

/// Repository that was skipped during bulk cloning
#[derive(Debug, Clone)]
pub struct SkippedRepository {
    pub name: String,
    pub reason: SkipReason,
}

/// Reason why a repository was skipped
#[derive(Debug, Clone)]
pub enum SkipReason {
    AlreadyExists(PathBuf),
    ExcludedByPattern(String),
    NotIncludedByPattern,
    Fork,
    Archived,
}

/// Repository that failed to clone
#[derive(Debug, Clone)]
pub struct FailedRepository {
    pub name: String,
    pub error: String,
    pub url: String,
}

/// Progress information for bulk clone operations
#[derive(Debug)]
pub struct BulkCloneProgress {
    pub current: usize,
    pub total: usize,
    pub current_repo: String,
    pub status: CloneStatus,
}

/// Status of the current clone operation
#[derive(Debug)]
pub enum CloneStatus {
    Discovering,
    Confirming,
    Cloning,
    AddingToWorkspace,
    Complete,
}

/// Rate limiter for API calls
pub struct RateLimiter {
    last_request: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(requests_per_second: f64) -> Self {
        Self {
            last_request: Instant::now(),
            min_interval: Duration::from_secs_f64(1.0 / requests_per_second),
        }
    }

    pub async fn wait(&mut self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.min_interval {
            let wait_time = self.min_interval - elapsed;
            tokio::time::sleep(wait_time).await;
        }
        self.last_request = Instant::now();
    }
}

/// Main bulk clone command implementation
pub struct BulkCloneCommand;

impl BulkCloneCommand {
    /// Execute bulk cloning for a user or organization
    pub async fn execute(
        target: String,
        options: BulkCloneOptions,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<BulkCloneResult> {
        let github_cli =
            GitHubCliProvider::new().context("Failed to initialize GitHub CLI provider")?;

        // Step 1: Discover repositories
        Self::report_progress(BulkCloneProgress {
            current: 0,
            total: 0,
            current_repo: "Discovering repositories...".to_string(),
            status: CloneStatus::Discovering,
        });

        let repositories = Self::discover_repositories(&github_cli, &target).await?;

        if repositories.is_empty() {
            anyhow::bail!("No repositories found for '{}'", target);
        }

        // Step 2: Filter repositories
        let filter_result =
            Self::filter_repositories(&repositories, &options, workspace_manager, git_config)?;

        if filter_result.to_clone.is_empty() {
            anyhow::bail!("No repositories remaining after filtering");
        }

        // Step 3: Show confirmation unless forced
        if !options.force {
            Self::show_confirmation(&filter_result, &target, repositories.len())?;
        }

        // Step 4: Clone repositories in serial
        let result = Self::clone_repositories_serial(
            filter_result.to_clone,
            filter_result.skipped,
            options,
            workspace_manager,
            git_config,
        )
        .await?;

        // Step 5: Display summary
        Self::display_summary(&result);

        Ok(result)
    }

    /// Discover all repositories for a target (user or organization)
    async fn discover_repositories(
        github_cli: &GitHubCliProvider,
        target: &str,
    ) -> Result<Vec<Repository>> {
        // Try as organization first, then as user
        match github_cli.get_organization_repositories(target).await {
            Ok(repos) => {
                info!(
                    "Found {} repositories for organization '{}'",
                    repos.len(),
                    target
                );
                Ok(repos)
            }
            Err(_) => {
                // Try as user
                match github_cli.get_user_repositories(target).await {
                    Ok(repos) => {
                        info!("Found {} repositories for user '{}'", repos.len(), target);
                        Ok(repos)
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to find repositories for '{}': {}", target, e);
                    }
                }
            }
        }
    }

    /// Filter repositories based on patterns and existing state
    fn filter_repositories(
        repositories: &[Repository],
        options: &BulkCloneOptions,
        workspace_manager: &WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<FilterResult> {
        let mut to_clone = Vec::new();
        let mut skipped = Vec::new();
        let workspace_root = workspace_manager.get_workspace_root();

        for repo in repositories {
            // Check if already exists locally
            if options.skip_existing {
                // Parse the repository URL to get org and repo name
                let repo_path = match Self::parse_git_url(&repo.url) {
                    Ok((org, repo_name)) => {
                        Self::calculate_install_path(workspace_root, git_config, &org, &repo_name)
                    }
                    Err(_) => {
                        // Fallback to just using the repo name if URL parsing fails
                        workspace_root.join(&repo.name)
                    }
                };

                if repo_path.exists() {
                    skipped.push(SkippedRepository {
                        name: repo.full_name.clone(),
                        reason: SkipReason::AlreadyExists(repo_path),
                    });
                    continue;
                }
            }

            // Apply exclude patterns
            if !options.exclude_patterns.is_empty() {
                let should_exclude = options.exclude_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&repo.name))
                        .unwrap_or(false)
                });
                if should_exclude {
                    if let Some(pattern) = options.exclude_patterns.iter().find(|pattern| {
                        glob::Pattern::new(pattern)
                            .map(|p| p.matches(&repo.name))
                            .unwrap_or(false)
                    }) {
                        skipped.push(SkippedRepository {
                            name: repo.full_name.clone(),
                            reason: SkipReason::ExcludedByPattern(pattern.clone()),
                        });
                    }
                    continue;
                }
            }

            // Apply include patterns (if any)
            if !options.include_patterns.is_empty() {
                let should_include = options.include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches(&repo.name))
                        .unwrap_or(false)
                });
                if !should_include {
                    skipped.push(SkippedRepository {
                        name: repo.full_name.clone(),
                        reason: SkipReason::NotIncludedByPattern,
                    });
                    continue;
                }
            }

            to_clone.push(repo.clone());
        }

        Ok(FilterResult { to_clone, skipped })
    }

    /// Show confirmation dialog for bulk clone operation
    fn show_confirmation(
        filter_result: &FilterResult,
        target: &str,
        total_discovered: usize,
    ) -> Result<()> {
        let repositories = &filter_result.to_clone;
        let skipped = &filter_result.skipped;

        println!(
            "\n{} {} {}",
            style("📋").blue(),
            style("Bulk Clone Summary").cyan().bold(),
            style(format!("- GitHub target '{}'", target)).dim()
        );

        println!(
            "Total repositories discovered: {}",
            style(total_discovered).blue().bold()
        );

        // Show skipped repositories if any
        if !skipped.is_empty() {
            let existing_count = skipped
                .iter()
                .filter(|s| matches!(s.reason, SkipReason::AlreadyExists(_)))
                .count();

            if existing_count > 0 {
                println!(
                    "{} Already exist locally: {}",
                    style("✅").green(),
                    style(existing_count).green().bold()
                );
            }

            let other_skipped = skipped.len() - existing_count;
            if other_skipped > 0 {
                println!(
                    "{} Skipped (patterns/filters): {}",
                    style("⏭️").yellow(),
                    style(other_skipped).yellow().bold()
                );
            }
        }

        if repositories.is_empty() {
            anyhow::bail!("No repositories to clone after filtering");
        }

        println!(
            "{} {} {}",
            style("📦").blue(),
            style("To confirm clone in bulk:").cyan(),
            style(repositories.len()).green().bold()
        );

        // Show sample repositories that will be cloned
        println!(
            "\n{} Sample repositories to clone (showing first 8):",
            style("🔽").blue()
        );
        for (i, repo) in repositories.iter().take(8).enumerate() {
            let lang = repo.language.as_deref().unwrap_or("unknown");
            let stars = if repo.stars > 0 {
                format!(" {}", style(format!("⭐ {}", repo.stars)).dim())
            } else {
                String::new()
            };

            println!(
                "  {}. {}{} [{}]",
                i + 1,
                style(&repo.full_name).cyan(),
                stars,
                style(lang).dim()
            );
        }

        if repositories.len() > 8 {
            println!("  ... and {} more repositories", repositories.len() - 8);
        }

        // Calculate estimated size (rough approximation)
        let estimated_size_mb = repositories.len() * 15; // Rough estimate of 15MB per repo
        let estimated_time_min = (repositories.len() as f64 * 0.5).ceil() as usize; // ~30s per repo

        println!("\n💾 Estimated size: ~{} MB", estimated_size_mb);
        println!("⏱️  Estimated time: {} minutes", estimated_time_min);

        println!(
            "\n{} {}",
            style("⚠️").yellow(),
            "This will clone ONLY the filtered repositories. Apps will NOT be configured automatically."
        );

        let proceed = Confirm::new(&format!(
            "Proceed with bulk cloning {} repositories?",
            repositories.len()
        ))
        .with_default(false)
        .with_help_message("This operation cannot be easily undone")
        .prompt()?;

        if !proceed {
            anyhow::bail!("User cancelled bulk clone operation");
        }

        Ok(())
    }

    /// Clone repositories in serial with progress reporting
    async fn clone_repositories_serial(
        repositories: Vec<Repository>,
        skipped_from_filter: Vec<SkippedRepository>,
        options: BulkCloneOptions,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<BulkCloneResult> {
        let total = repositories.len();
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let skipped = skipped_from_filter;

        let start_time = Instant::now();

        // Conservative rate limiting: 1 clone every 2 seconds
        let mut rate_limiter = RateLimiter::new(0.5);

        for (index, repo) in repositories.iter().enumerate() {
            // Progress reporting
            Self::report_progress(BulkCloneProgress {
                current: index + 1,
                total,
                current_repo: repo.full_name.clone(),
                status: CloneStatus::Cloning,
            });

            // Rate limiting
            if index > 0 {
                rate_limiter.wait().await;
            }

            // Attempt clone with error isolation
            match Self::clone_single_repository(repo, &options, workspace_manager, git_config).await
            {
                Ok(_) => {
                    successful.push(repo.full_name.clone());
                    info!("Successfully cloned {}", repo.full_name);
                }
                Err(e) => {
                    warn!("Failed to clone {}: {}", repo.full_name, e);
                    failed.push(FailedRepository {
                        name: repo.full_name.clone(),
                        error: e.to_string(),
                        url: repo.url.clone(),
                    });
                }
            }
        }

        let duration = start_time.elapsed();

        // Final progress update
        Self::report_progress(BulkCloneProgress {
            current: total,
            total,
            current_repo: "Complete!".to_string(),
            status: CloneStatus::Complete,
        });

        Ok(BulkCloneResult {
            total_discovered: total + skipped.len(), // Include all discovered repos
            total_cloned: successful.len(),
            skipped,
            failed,
            successful,
            duration,
        })
    }

    /// Clone a single repository without post-install workflow
    async fn clone_single_repository(
        repo: &Repository,
        _options: &BulkCloneOptions,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<()> {
        // Create installer but skip post-install actions for bulk operations
        let installer = RepositoryInstaller::new(
            workspace_manager.get_workspace_root().clone(),
            git_config.clone(),
        );

        // Clone without opening or running install commands (fast bulk mode)
        let installed = installer
            .install_from_url_with_options(
                &repo.url, None,  // Use default path
                false, // don't open
                false, // don't run install commands
            )
            .await
            .context("Failed to clone repository")?;

        // Add to workspace configuration (but skip app configuration)
        workspace_manager
            .add_repository(installed.repository)
            .await
            .context("Failed to add repository to workspace")?;

        Ok(())
    }

    /// Report progress during bulk clone operation
    fn report_progress(progress: BulkCloneProgress) {
        if progress.total == 0 {
            print!("\r🔍 {}", progress.current_repo);
        } else {
            let percent = (progress.current as f64 / progress.total as f64 * 100.0) as usize;
            let bar_length = 20;
            let filled = (progress.current * bar_length) / progress.total.max(1);
            let empty = bar_length - filled;

            let status_icon = match progress.status {
                CloneStatus::Discovering => "🔍",
                CloneStatus::Confirming => "❓",
                CloneStatus::Cloning => "📦",
                CloneStatus::AddingToWorkspace => "➕",
                CloneStatus::Complete => "✅",
            };

            print!(
                "\r{} [{:>3}%] [{}{}] ({}/{}) {}",
                status_icon,
                percent,
                "█".repeat(filled),
                "░".repeat(empty),
                progress.current,
                progress.total,
                if progress.current_repo.len() > 40 {
                    format!("{}...", &progress.current_repo[..37])
                } else {
                    progress.current_repo
                }
            );
        }

        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        if matches!(progress.status, CloneStatus::Complete) {
            println!(); // New line after completion
        }
    }

    /// Parse Git URL to extract org and repo name (mirrors RepositoryInstaller::parse_git_url)
    fn parse_git_url(url: &str) -> Result<(String, String)> {
        let url = url.trim();

        // SSH format: git@github.com:org/repo.git
        if url.starts_with("git@") {
            let parts: Vec<&str> = url.split(':').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid SSH URL format: {}", url);
            }

            let path_parts: Vec<&str> = parts[1].trim_end_matches(".git").split('/').collect();
            if path_parts.len() != 2 {
                anyhow::bail!("Invalid SSH URL path format: {}", url);
            }

            return Ok((path_parts[0].to_string(), path_parts[1].to_string()));
        }

        // HTTPS format: https://github.com/org/repo.git
        if url.starts_with("https://") || url.starts_with("http://") {
            let url_without_scheme = if url.starts_with("https://") {
                &url[8..]
            } else {
                &url[7..]
            };

            let parts: Vec<&str> = url_without_scheme.split('/').collect();
            if parts.len() >= 3 && parts[0].contains("github.com") {
                let org = parts[1];
                let repo = parts[2].trim_end_matches(".git");
                return Ok((org.to_string(), repo.to_string()));
            }
        }

        anyhow::bail!("Unsupported URL format: {}", url);
    }

    /// Calculate install path (mirrors RepositoryInstaller::calculate_install_path)
    fn calculate_install_path(
        workspace_root: &std::path::Path,
        git_config: &GitConfig,
        org: &str,
        repo: &str,
    ) -> std::path::PathBuf {
        if git_config.standardize_paths {
            workspace_root.join(org).join(repo)
        } else {
            workspace_root.join(repo)
        }
    }

    /// Display summary of bulk clone operation
    fn display_summary(result: &BulkCloneResult) {
        println!("\n{} Bulk Clone Complete!", style("🎉").green().bold());

        println!(
            "📊 Total repositories discovered: {}",
            style(result.total_discovered).blue().bold()
        );

        println!(
            "✅ Successfully cloned: {}",
            style(result.total_cloned).green().bold()
        );

        if !result.skipped.is_empty() {
            let existing_count = result
                .skipped
                .iter()
                .filter(|s| matches!(s.reason, SkipReason::AlreadyExists(_)))
                .count();
            let pattern_count = result.skipped.len() - existing_count;

            if existing_count > 0 {
                println!(
                    "✅ Already existed locally: {}",
                    style(existing_count).green().bold()
                );
            }
            if pattern_count > 0 {
                println!(
                    "⏭️  Skipped by filters: {}",
                    style(pattern_count).yellow().bold()
                );
            }
        }

        if !result.failed.is_empty() {
            println!(
                "❌ Failed: {} repositories",
                style(result.failed.len()).red().bold()
            );

            for failed in &result.failed {
                println!(
                    "  • {} - {}",
                    style(&failed.name).red(),
                    style(&failed.error).dim()
                );
            }
        }

        let minutes = result.duration.as_secs() / 60;
        let seconds = result.duration.as_secs() % 60;
        println!("⏱️  Total time: {}m {}s", minutes, seconds);

        if result.total_cloned > 0 {
            println!("\n{} Next steps:", style("💡").yellow());
            println!(
                "• Configure apps: {}",
                style("vibe apps configure <repo>").cyan()
            );
            println!("• Explore repos: {}", style("vibe launch").cyan());
            println!("• Check status: {}", style("vibe git status").cyan());
        }
    }
}
