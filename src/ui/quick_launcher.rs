use anyhow::Result;
use console::style;
use inquire::{InquireError, Select};
use std::collections::HashMap;

use crate::cache::{GitStatusCache, RepositoryCache};
use crate::ui::formatting;
use crate::ui::state::VibeState;
use crate::workspace::{operations::GitStatus, WorkspaceManager};

/// Enhanced repository launcher with caching
pub struct QuickLauncher {
    repo_cache: RepositoryCache,
    git_cache: GitStatusCache,
}

/// Enhanced launch item that supports both configured and unconfigured repositories
#[derive(Debug, Clone)]
pub struct UniversalLaunchItem {
    pub name: String,
    pub display_string: String,
    #[allow(dead_code)]
    pub has_configured_apps: bool,
    #[allow(dead_code)]
    pub configured_apps: Vec<String>,
    #[allow(dead_code)]
    pub available_apps: Vec<String>,
    #[allow(dead_code)]
    pub git_status: Option<GitStatus>,
    #[allow(dead_code)]
    pub is_recent: bool,
    #[allow(dead_code)]
    pub recent_rank: Option<usize>,
    #[allow(dead_code)]
    pub last_accessed: Option<String>,
    pub last_app: Option<String>,
}

impl QuickLauncher {
    /// Create a new quick launcher with cache system
    pub async fn new(cache_dir: &std::path::Path) -> Result<Self> {
        let repo_cache = RepositoryCache::new(cache_dir.join("repositories.db"));
        let git_cache = GitStatusCache::new(cache_dir.join("git_status.db"));

        // Initialize caches
        repo_cache.initialize().await?;
        git_cache.initialize().await?;

        Ok(Self {
            repo_cache,
            git_cache,
        })
    }

    /// Universal launch - shows ALL repositories (configured and unconfigured)
    pub async fn launch(&self, workspace_manager: &mut WorkspaceManager) -> Result<()> {
        // Get ALL repositories from workspace manager
        let all_repos = workspace_manager.list_repositories();

        if all_repos.is_empty() {
            println!("❌ No repositories found in workspace");
            println!("💡 Scan for repositories: 'vibe git scan'");
            println!("💡 Clone a repository: 'vibe clone <url>'");
            return Ok(());
        }

        // Get recent repositories for prioritization
        let user_state = VibeState::load().unwrap_or_default();
        let recent_repos = user_state.get_recent_repos(15);
        let recent_names: HashMap<String, usize> = recent_repos
            .iter()
            .enumerate()
            .map(|(i, repo)| (repo.repo_id.clone(), i + 1))
            .collect();

        // Create a map for recent repo details (time, last app)
        let recent_details: HashMap<String, (&crate::ui::state::RecentRepo, String)> = recent_repos
            .iter()
            .map(|repo| {
                let time_ago = formatting::format_time_ago(&repo.last_accessed);
                (repo.repo_id.clone(), (repo, time_ago))
            })
            .collect();

        // Load git status from cache (optional - don't block if missing)
        let git_statuses = self
            .git_cache
            .get_all_git_statuses()
            .await
            .unwrap_or_default();
        let git_status_map: HashMap<String, GitStatus> = git_statuses
            .into_iter()
            .map(|cached| (cached.repository_name.clone(), cached.into()))
            .collect();

        // Get available apps on system for unconfigured repos
        let available_apps = workspace_manager.get_available_apps().await;

        // Create universal launch items with ALL repositories
        let launch_items: Vec<UniversalLaunchItem> = all_repos
            .iter()
            .map(|repo| {
                let git_status = git_status_map.get(&repo.name).cloned();
                let recent_rank = recent_names.get(&repo.name).cloned();
                let is_recent = recent_rank.is_some();

                // Get recent repo details if available
                let (last_accessed, last_app) =
                    if let Some((recent_repo, time_ago)) = recent_details.get(&repo.name) {
                        (Some(time_ago.clone()), recent_repo.last_app.clone())
                    } else {
                        (None, None)
                    };

                // Check if repository has configured apps
                let configured_apps: Vec<String> = repo.apps.keys().cloned().collect();
                let has_configured_apps = !configured_apps.is_empty();

                // Create clean display with consistent folder icons
                let display_string = if has_configured_apps {
                    format!("📁 {} 📋[{}]", repo.name, configured_apps.len())
                } else {
                    format!("📁 {}", repo.name)
                };

                UniversalLaunchItem {
                    name: repo.name.clone(),
                    display_string,
                    has_configured_apps,
                    configured_apps,
                    available_apps: available_apps.clone(),
                    git_status,
                    is_recent,
                    recent_rank,
                    last_accessed,
                    last_app,
                }
            })
            .collect();

        // Sort all repositories alphabetically for clean browsing
        let mut sorted_items = launch_items;
        sorted_items.sort_by(|a, b| a.name.cmp(&b.name));

        // Create display options for all repositories
        let mut display_options = Vec::new();
        let mut item_map = std::collections::HashMap::new();

        // Add all repositories in alphabetical order
        for item in &sorted_items {
            display_options.push(item.display_string.clone());
            item_map.insert(item.display_string.clone(), item);
        }

        // Show selection UI with updated messaging
        println!("\n📂 Select a repository to open:");
        println!(
            "   {} repositories available • {} apps auto-detected for unconfigured repos",
            sorted_items.len(),
            available_apps.len()
        );

        // Repository selection
        let selected_display_result = Select::new("Repository:", display_options.clone())
            .with_help_message("Use arrow keys to navigate, type to filter • ESC to exit")
            .with_page_size(workspace_manager.get_quick_launch_page_size())
            .prompt();

        let selected_display = match selected_display_result {
            Ok(value) => value,
            Err(InquireError::OperationCanceled) => {
                println!("{} Repository selection cancelled", style("ℹ️").blue());
                return Ok(());
            }
            Err(error) => return Err(anyhow::Error::from(error)),
        };

        let selected_item = item_map.get(&selected_display).copied().ok_or_else(|| {
            anyhow::anyhow!(
                "Selected repository '{}' not found in item map",
                selected_display
            )
        })?;

        // Handle app selection and launch
        self.launch_universal_repository(workspace_manager, selected_item)
            .await?;

        Ok(())
    }

    /// Launch a universal repository (configured or unconfigured) with smart app selection
    async fn launch_universal_repository(
        &self,
        workspace_manager: &mut WorkspaceManager,
        item: &UniversalLaunchItem,
    ) -> Result<()> {
        // Use smart_open_repository for manual selection - this shows choice menu
        workspace_manager.smart_open_repository(&item.name).await?;

        // Update recent repositories state with the last app chosen
        if let Some(repo_info) = workspace_manager.get_repository(&item.name) {
            let mut user_state = VibeState::load().unwrap_or_default();
            user_state.add_recent_repo(
                item.name.clone(),
                repo_info.path.clone(),
                item.last_app.clone(), // Use the app from selection or previous choice
            );
            if let Err(e) = user_state.save() {
                eprintln!("Warning: Failed to save recent repositories: {e}");
            }
        }

        Ok(())
    }

    /// Quick launch from recent repos (position 1-9) - uses immediate selection
    #[allow(dead_code)]
    pub async fn quick_launch_recent(
        &self,
        workspace_manager: &mut WorkspaceManager,
        position: usize,
    ) -> Result<()> {
        let user_state = VibeState::load().unwrap_or_default();
        let recent_repos = user_state.get_recent_repos(9);

        if let Some(recent_repo) = recent_repos.get(position - 1) {
            let repo_name = &recent_repo.repo_id;
            let default_app = "vscode".to_string();
            let last_app = recent_repo.last_app.as_ref().unwrap_or(&default_app);

            // Immediate opening with saved app - NO choice menu
            workspace_manager
                .open_repo_with_app_options(repo_name, last_app, false)
                .await?;

            // Update access tracking
            let mut updated_state = VibeState::load().unwrap_or_default();
            updated_state.add_recent_repo(
                repo_name.clone(),
                recent_repo.path.clone(),
                Some(last_app.clone()),
            );
            if let Err(e) = updated_state.save() {
                eprintln!("Warning: Failed to save recent repositories: {e}");
            }

            println!(
                "{} Opened {} with {} (quick launch #{})",
                style("🚀").green(),
                style(repo_name).cyan().bold(),
                style(last_app).blue(),
                position
            );
        } else {
            anyhow::bail!("No repository found at position {}", position);
        }

        Ok(())
    }

    /// Refresh cache from workspace configuration
    pub async fn refresh_cache(&self, workspace_manager: &WorkspaceManager) -> Result<()> {
        println!("{} Updating repository cache...", style("🔄").blue());

        // Update repository cache
        self.repo_cache
            .refresh_from_config(
                &workspace_manager.config().repositories,
                workspace_manager.get_workspace_root(),
            )
            .await?;

        // Clean up stale entries
        let current_names: Vec<String> = workspace_manager
            .config()
            .repositories
            .iter()
            .map(|r| r.name.clone())
            .collect();

        self.repo_cache
            .cleanup_stale_entries(&current_names)
            .await?;

        println!("{} Repository cache updated", style("✓").green());

        Ok(())
    }

    /// Update git status cache in background for specific repositories
    #[allow(dead_code)]
    pub async fn update_git_status_cache(
        &self,
        workspace_manager: &WorkspaceManager,
        repo_names: &[String],
    ) -> Result<()> {
        for repo_name in repo_names {
            if let Some(repo_config) = workspace_manager
                .config()
                .repositories
                .iter()
                .find(|r| r.name == *repo_name)
            {
                let repo_path = workspace_manager
                    .config()
                    .workspace
                    .root
                    .join(&repo_config.path);

                match crate::workspace::operations::get_git_status(&repo_path).await {
                    Ok(git_status) => {
                        let cached_status = git_status.into();
                        if let Err(e) = self.git_cache.cache_git_status(&cached_status).await {
                            eprintln!("Warning: Failed to cache git status for {repo_name}: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to get git status for {repo_name}: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Get cache statistics for monitoring
    #[allow(dead_code)]
    pub async fn get_cache_stats(&self) -> Result<CacheStatistics> {
        let repo_stats = self.repo_cache.get_stats().await?;
        let git_stats = self.git_cache.get_stats().await?;

        Ok(CacheStatistics {
            repositories: repo_stats,
            git_status: git_stats,
        })
    }
}

/// Combined cache statistics
#[derive(Debug)]
pub struct CacheStatistics {
    pub repositories: crate::cache::repository_cache::CacheStats,
    pub git_status: crate::cache::git_status_cache::GitCacheStats,
}

impl std::fmt::Display for CacheStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "📊 Cache Statistics:")?;
        writeln!(
            f,
            "  Repositories: {} total, {} with apps, {} existing",
            self.repositories.total_repositories,
            self.repositories.repositories_with_apps,
            self.repositories.existing_repositories
        )?;
        writeln!(
            f,
            "  Git Status: {} total, {} valid, {} expired (TTL: {}min)",
            self.git_status.total_entries,
            self.git_status.valid_entries,
            self.git_status.expired_entries,
            self.git_status.ttl_minutes
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_quick_launcher_creation() {
        let temp_dir = tempdir().unwrap();
        let launcher = QuickLauncher::new(temp_dir.path()).await.unwrap();

        // Test that caches are initialized
        let stats = launcher.get_cache_stats().await.unwrap();
        println!("{}", stats);
    }
}
