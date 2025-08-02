use anyhow::Result;
use console::style;
use fuzzy_matcher::skim::SkimMatcherV2;
use inquire::{InquireError, Select};
use std::collections::HashMap;

use crate::cache::{GitStatusCache, RepositoryCache};
use crate::ui::formatting;
use crate::ui::state::VibeState;
use crate::workspace::{operations::GitStatus, WorkspaceManager};

/// Enhanced repository launcher with fuzzy search and caching
pub struct QuickLauncher {
    repo_cache: RepositoryCache,
    git_cache: GitStatusCache,
    matcher: SkimMatcherV2,
}

/// Repository display item for the launcher
#[derive(Debug, Clone)]
pub struct LaunchItem {
    pub name: String,
    pub display_string: String,
    pub apps: Vec<String>,
    pub git_status: Option<GitStatus>,
    pub is_recent: bool,
    pub recent_rank: Option<usize>,    // 1-9 for recent repos
    pub last_accessed: Option<String>, // Human-readable time for recent repos
    pub last_app: Option<String>,      // Last used app for recent repos
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
            matcher: SkimMatcherV2::default(),
        })
    }

    /// Launch the repository selection UI with fast cached data
    pub async fn launch(&self, workspace_manager: &mut WorkspaceManager) -> Result<()> {
        // Load cached repository data (fast)
        let mut cached_repos = self.repo_cache.get_repositories_with_apps().await?;

        if cached_repos.is_empty() {
            println!("❌ No repositories with configured apps found in cache");
            println!("💡 Refreshing cache from workspace configuration...");

            // Fallback: refresh cache from current config
            self.refresh_cache(workspace_manager).await?;
            cached_repos = self.repo_cache.get_repositories_with_apps().await?;

            if cached_repos.is_empty() {
                println!("❌ No repositories with configured apps found");
                println!("💡 Configure apps for repositories first using 'Configure vibes'");
                return Ok(());
            }
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

        // Create launch items with all available information
        let launch_items: Vec<LaunchItem> = cached_repos
            .into_iter()
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

                // Create display string using shared formatting
                let display_string = formatting::format_repository_launch_item(
                    &repo.name,
                    &repo.configured_apps,
                    git_status.as_ref(),
                    recent_rank,
                    last_accessed.as_deref(),
                    last_app.as_deref(),
                );

                LaunchItem {
                    name: repo.name,
                    display_string,
                    apps: repo.configured_apps,
                    git_status,
                    is_recent,
                    recent_rank,
                    last_accessed,
                    last_app,
                }
            })
            .collect();

        // Separate recent and other repositories
        let (recent_items, other_items): (Vec<_>, Vec<_>) = launch_items
            .into_iter()
            .partition(|item| item.recent_rank.is_some());

        // Sort recent items by rank, other items alphabetically
        let mut recent_items = recent_items;
        recent_items.sort_by(|a, b| a.recent_rank.cmp(&b.recent_rank));

        let mut other_items = other_items;
        other_items.sort_by(|a, b| a.name.cmp(&b.name));

        // Create display options with section headers
        let mut display_options = Vec::new();
        let mut item_map = std::collections::HashMap::new();

        // Add recent repositories section if any exist
        if !recent_items.is_empty() {
            display_options.push("🏃 ── Recent Repositories ──".to_string());
            for item in &recent_items {
                display_options.push(item.display_string.clone());
                item_map.insert(item.display_string.clone(), item);
            }
        }

        // Add all repositories section if there are non-recent ones
        if !other_items.is_empty() {
            if !recent_items.is_empty() {
                display_options.push("📁 ── All Repositories ──".to_string());
            }
            for item in &other_items {
                display_options.push(item.display_string.clone());
                item_map.insert(item.display_string.clone(), item);
            }
        }

        // Show selection UI
        println!("\n🚀 Select a repository to open:");
        println!(
            "   {} repositories available",
            recent_items.len() + other_items.len()
        );
        if !recent_items.is_empty() {
            println!("   {} recent repositories", recent_items.len());
        }

        // Selection loop to handle section headers
        let selected_item = loop {
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

            // Handle section headers (ignore and re-prompt)
            if selected_display.starts_with("🏃 ──") || selected_display.starts_with("📁 ──")
            {
                println!(
                    "{} Section headers are not selectable. Please choose a repository.",
                    style("ℹ️").blue()
                );
                continue;
            }

            // Find the selected repository
            if let Some(selected_item) = item_map.get(&selected_display) {
                break selected_item;
            } else {
                return Err(anyhow::anyhow!("Selected repository not found"));
            }
        };

        // Handle app selection and launch
        self.launch_repository(workspace_manager, selected_item)
            .await?;

        Ok(())
    }

    /// Launch a specific repository with app selection
    async fn launch_repository(
        &self,
        workspace_manager: &mut WorkspaceManager,
        item: &LaunchItem,
    ) -> Result<()> {
        let app_to_use = if item.apps.len() == 1 {
            // Only one app configured, use it directly
            item.apps[0].clone()
        } else {
            // Multiple apps, let user choose
            println!(
                "\n📱 Multiple apps configured for '{}'. Select one:",
                style(&item.name).cyan()
            );

            let app_options: Vec<String> = item.apps.iter().cloned().collect();

            let selected_app_result = Select::new("App:", app_options)
                .with_help_message("Select the app to open this repository with • ESC to cancel")
                .with_page_size(workspace_manager.get_app_selection_page_size())
                .prompt();

            match selected_app_result {
                Ok(app) => app,
                Err(InquireError::OperationCanceled) => {
                    println!("{} App selection cancelled", style("ℹ️").blue());
                    return Ok(());
                }
                Err(error) => return Err(anyhow::Error::from(error)),
            }
        };

        // Launch the repository
        workspace_manager
            .open_repo_with_app(&item.name, &app_to_use)
            .await?;

        // Update recent repositories state
        if let Some(repo_info) = workspace_manager.get_repository(&item.name) {
            let mut user_state = VibeState::load().unwrap_or_default();
            user_state.add_recent_repo(
                item.name.clone(),
                repo_info.path.clone(),
                Some(app_to_use.clone()),
            );
            if let Err(e) = user_state.save() {
                eprintln!("Warning: Failed to save recent repositories: {}", e);
            }
        }

        println!(
            "{} Launched {} with {}",
            style("🚀").green(),
            style(&item.name).cyan().bold(),
            style(&app_to_use).blue()
        );

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
                            eprintln!(
                                "Warning: Failed to cache git status for {}: {}",
                                repo_name, e
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to get git status for {}: {}", repo_name, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get cache statistics for monitoring
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
