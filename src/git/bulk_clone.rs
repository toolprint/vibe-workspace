use anyhow::{Context, Result};
use console::style;
use inquire::Confirm;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::git::{GitConfig, Repository};
use crate::workspace::install::RepositoryInstaller;
use crate::workspace::manager::WorkspaceManager;
use crate::git::provider::github_cli::GitHubCliProvider;

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
        let github_cli = GitHubCliProvider::new()
            .context("Failed to initialize GitHub CLI provider")?;
        
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
        let filtered_repos = Self::filter_repositories(&repositories, &options, workspace_manager)?;
        
        if filtered_repos.is_empty() {
            anyhow::bail!("No repositories remaining after filtering");
        }
        
        // Step 3: Show confirmation unless forced
        if !options.force {
            Self::show_confirmation(&filtered_repos, &target)?;
        }
        
        // Step 4: Clone repositories in serial
        let result = Self::clone_repositories_serial(
            filtered_repos,
            options,
            workspace_manager,
            git_config,
        ).await?;
        
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
                info!("Found {} repositories for organization '{}'", repos.len(), target);
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
    ) -> Result<Vec<Repository>> {
        let mut filtered = Vec::new();
        let workspace_root = workspace_manager.get_workspace_root();
        
        for repo in repositories {
            // Check if already exists locally
            if options.skip_existing {
                let repo_path = workspace_root.join(&repo.name);
                if repo_path.exists() {
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
                    continue;
                }
            }
            
            filtered.push(repo.clone());
        }
        
        Ok(filtered)
    }
    
    /// Show confirmation dialog for bulk clone operation
    fn show_confirmation(repositories: &[Repository], target: &str) -> Result<()> {
        println!("\n{} {} {}",
            style("📋").blue(),
            style("Bulk Clone Summary").cyan().bold(),
            style(format!("- GitHub target '{}'", target)).dim()
        );
        
        println!("Repositories found: {}", style(repositories.len()).green().bold());
        
        // Show sample repositories
        println!("\n{} Sample repositories (showing first 8):", style("📦").blue());
        for (i, repo) in repositories.iter().take(8).enumerate() {
            let lang = repo.language.as_deref().unwrap_or("unknown");
            let stars = if repo.stars > 0 { 
                format!(" {}", style(format!("⭐ {}", repo.stars)).dim()) 
            } else { 
                String::new() 
            };
            
            println!("  {}. {}{} [{}]",
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
        
        println!("\n{} {}",
            style("⚠️").yellow(),
            "This will clone ALL repositories. Apps will NOT be configured automatically."
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
        options: BulkCloneOptions,
        workspace_manager: &mut WorkspaceManager,
        git_config: &GitConfig,
    ) -> Result<BulkCloneResult> {
        let total = repositories.len();
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let skipped = Vec::new();
        
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
            match Self::clone_single_repository(repo, &options, workspace_manager, git_config).await {
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
            total_discovered: total,
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
                &repo.url,
                None, // Use default path
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
            
            print!("\r{} [{:>3}%] [{}{}] ({}/{}) {}",
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
    
    /// Display summary of bulk clone operation
    fn display_summary(result: &BulkCloneResult) {
        println!("\n{} Bulk Clone Complete!", style("🎉").green().bold());
        
        println!("✅ Successfully cloned: {}/{} repositories",
            style(result.total_cloned).green().bold(),
            result.total_discovered
        );
        
        if !result.failed.is_empty() {
            println!("❌ Failed: {} repositories", style(result.failed.len()).red().bold());
            
            for failed in &result.failed {
                println!("  • {} - {}", 
                    style(&failed.name).red(),
                    style(&failed.error).dim()
                );
            }
        }
        
        if !result.skipped.is_empty() {
            println!("⏭️  Skipped: {} repositories", result.skipped.len());
        }
        
        let minutes = result.duration.as_secs() / 60;
        let seconds = result.duration.as_secs() % 60;
        println!("⏱️  Total time: {}m {}s", minutes, seconds);
        
        if result.total_cloned > 0 {
            println!("\n{} Next steps:", style("💡").yellow());
            println!("• Configure apps: {}", style("vibe apps configure <repo>").cyan());
            println!("• Explore repos: {}", style("vibe launch").cyan());
            println!("• Check status: {}", style("vibe git status").cyan());
        }
    }
}