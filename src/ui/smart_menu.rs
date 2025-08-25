use anyhow::Result;
use std::path::PathBuf;

use crate::ui::formatting;
use crate::ui::state::VibeState;
use crate::workspace::WorkspaceManager;

/// Represents a smart action that can be taken based on context
#[derive(Debug, Clone)]
pub struct SmartAction {
    pub label: String,
    pub description: String,
    pub action_type: SmartActionType,
    pub priority: u8, // Higher is more important
}

#[derive(Debug, Clone)]
pub enum SmartActionType {
    CloneAndOpen(String),              // URL or search term
    ConfigureApps(Vec<String>),        // Repo names that need app configuration
    ConfigureAndOpen(String),          // Configure app for repo and open
    CreateRepository,                  // Create new local repository
    DiscoverRepos,                     // Scan for new repositories
    InstallApps,                       // Install missing apps
    OpenRecent(String),                // Repo name
    OpenWithPreferred(String, String), // Repo name, preferred app
    QuickConfigureBatch(Vec<String>),  // Batch configure multiple repos
    SetupWorkspace,                    // First-time setup
    SyncRepositories,                  // Pull updates for all repos
    CleanupMissing,                    // Remove missing repos from config
    BulkClone(String),                 // Bulk clone from user/org
}

/// Represents a quick launch item
#[derive(Debug, Clone)]
pub struct QuickLaunchItem {
    pub number: usize, // 1-9
    pub repo_name: String,
    pub repo_path: PathBuf,
    pub last_app: Option<String>,
    pub last_accessed: String, // Human-readable time
    pub access_count: u32,
}

/// Analyzes workspace state to provide smart menu options
pub struct SmartMenu {
    workspace_state: WorkspaceState,
    user_state: VibeState,
}

/// Current state of the workspace
#[derive(Debug)]
struct WorkspaceState {
    total_repos: usize,
    unconfigured_repos: Vec<String>,
    missing_repos: Vec<String>,
    available_apps: Vec<String>,
    #[allow(dead_code)]
    has_uncommitted_changes: bool,
    days_since_last_sync: Option<i64>,
}

impl SmartMenu {
    /// Create a new smart menu analyzer
    pub async fn new(workspace_manager: &WorkspaceManager) -> Result<Self> {
        let user_state = VibeState::load().unwrap_or_default();
        let workspace_state = Self::analyze_workspace(workspace_manager).await?;

        Ok(Self {
            workspace_state,
            user_state,
        })
    }

    /// Analyze the current workspace state
    async fn analyze_workspace(manager: &WorkspaceManager) -> Result<WorkspaceState> {
        let repos = manager.list_repositories();
        let total_repos = repos.len();

        // Find unconfigured repos
        let unconfigured_repos: Vec<String> = repos
            .iter()
            .filter(|repo| repo.apps.is_empty())
            .map(|repo| repo.name.clone())
            .collect();

        // Find missing repos (in config but not on disk)
        let mut missing_repos = Vec::new();
        let workspace_root = manager.get_workspace_root();
        for repo in repos {
            let full_path = workspace_root.join(&repo.path);
            if !full_path.exists() {
                missing_repos.push(repo.name.clone());
            }
        }

        // Check available apps
        let mut available_apps = Vec::new();
        for app in &["vscode", "warp", "iterm2", "wezterm", "cursor", "windsurf"] {
            if manager.is_app_available(app).await {
                available_apps.push(app.to_string());
            }
        }

        // TODO: Check for uncommitted changes and sync status
        let has_uncommitted_changes = false;
        let days_since_last_sync = None;

        Ok(WorkspaceState {
            total_repos,
            unconfigured_repos,
            missing_repos,
            available_apps,
            has_uncommitted_changes,
            days_since_last_sync,
        })
    }

    /// Get smart actions based on current context
    pub fn get_smart_actions(&self) -> Vec<SmartAction> {
        let mut actions = Vec::new();

        // First-time setup
        if self.user_state.is_first_run() && self.workspace_state.total_repos == 0 {
            actions.push(SmartAction {
                label: "🎉 Run setup wizard".to_string(),
                description: "Get started with Vibe Workspace".to_string(),
                action_type: SmartActionType::SetupWorkspace,
                priority: 100,
            });
        }

        // Discover repos if workspace is empty
        if self.workspace_state.total_repos == 0 {
            actions.push(SmartAction {
                label: "🔍 Discover repositories".to_string(),
                description: "Scan workspace for git repositories".to_string(),
                action_type: SmartActionType::DiscoverRepos,
                priority: 90,
            });
        }

        // Create new repository action (always available) - HIGH PRIORITY
        actions.push(SmartAction {
            label: "🆕 Create new repository".to_string(),
            description: "Create a new local repository for prototyping".to_string(),
            action_type: SmartActionType::CreateRepository,
            priority: 85,
        });

        // Clone new repo action (always available) - HIGH PRIORITY
        actions.push(SmartAction {
            label: "📥 Clone new repository".to_string(),
            description: "Search and clone from GitHub".to_string(),
            action_type: SmartActionType::CloneAndOpen("".to_string()),
            priority: 80,
        });

        // Open any repository action (if repos exist) - HIGH PRIORITY
        if self.workspace_state.total_repos > 0 {
            actions.push(SmartAction {
                label: "📂 Open repository".to_string(),
                description: "Browse and open any repository in your workspace".to_string(),
                action_type: SmartActionType::OpenRecent("".to_string()),
                priority: 90,
            });
        }

        // Sync repositories if it's been a while - MEDIUM PRIORITY
        if let Some(days) = self.workspace_state.days_since_last_sync {
            if days > 7 {
                actions.push(SmartAction {
                    label: "🔄 Sync all repositories".to_string(),
                    description: format!("Last synced {days} days ago"),
                    action_type: SmartActionType::SyncRepositories,
                    priority: 70,
                });
            }
        }

        // Install apps if none available - LOWER PRIORITY (optional)
        if self.workspace_state.available_apps.is_empty() && self.workspace_state.total_repos > 0 {
            actions.push(SmartAction {
                label: "📱 Install development apps".to_string(),
                description: "Install VS Code, Warp, or other supported apps".to_string(),
                action_type: SmartActionType::InstallApps,
                priority: 60,
            });
        }

        // Configure apps for unconfigured repos - LOWER PRIORITY (optional)
        if !self.workspace_state.unconfigured_repos.is_empty() {
            let count = self.workspace_state.unconfigured_repos.len();
            actions.push(SmartAction {
                label: format!(
                    "⚙️  Set up templates for {} repo{}",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
                description: "Configure advanced templates and automation (optional)".to_string(),
                action_type: SmartActionType::ConfigureApps(
                    self.workspace_state.unconfigured_repos.clone(),
                ),
                priority: 50,
            });
        }

        // Clean up missing repos - LOWER PRIORITY (maintenance)
        if !self.workspace_state.missing_repos.is_empty() {
            let count = self.workspace_state.missing_repos.len();
            actions.push(SmartAction {
                label: format!(
                    "🧹 Clean up {} missing repo{}",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
                description: "Remove deleted repositories from configuration".to_string(),
                action_type: SmartActionType::CleanupMissing,
                priority: 40,
            });
        }

        // Bulk clone suggestions for new/small workspaces - MEDIUM-HIGH PRIORITY (usage)
        if self.workspace_state.total_repos < 10 {
            actions.push(SmartAction {
                label: "📦 Bulk clone repositories".to_string(),
                description: "Clone all repos from a GitHub user or organization".to_string(),
                action_type: SmartActionType::BulkClone("".to_string()),
                priority: 75,
            });
        }

        // Sort by priority (highest first)
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Return top 5 actions
        actions.truncate(5);
        actions
    }

    /// Get quick launch items (recent repositories)
    pub fn get_quick_launch_items(&self) -> Vec<QuickLaunchItem> {
        let recent_repos = self.user_state.get_recent_repos(15);

        recent_repos
            .iter()
            .enumerate()
            .map(|(index, repo)| {
                let time_ago = formatting::format_time_ago(&repo.last_accessed);
                QuickLaunchItem {
                    number: index + 1,
                    repo_name: repo.repo_id.clone(),
                    repo_path: repo.path.clone(),
                    last_app: repo.last_app.clone(),
                    last_accessed: time_ago,
                    access_count: repo.access_count,
                }
            })
            .collect()
    }

    /// Check if setup wizard should be shown
    pub fn should_show_setup_wizard(&self) -> bool {
        self.user_state.is_first_run() && self.user_state.user_preferences.show_setup_wizard
    }

    /// Get smart open actions (open repo with any available app)
    pub fn get_smart_open_actions(&self, workspace_manager: &WorkspaceManager) -> Vec<SmartAction> {
        let mut actions = Vec::new();
        let recent_repos = self.user_state.get_recent_repos(5);
        let all_repos = workspace_manager.list_repositories();

        // Get configured repositories and their apps
        let configured_repos: std::collections::HashMap<String, Vec<String>> = all_repos
            .iter()
            .filter(|repo| !repo.apps.is_empty())
            .map(|repo| (repo.name.clone(), repo.apps.keys().cloned().collect()))
            .collect();

        // Create "Open with preferred app" actions for recent repos with known preferences
        for recent_repo in recent_repos {
            if let Some(last_app) = &recent_repo.last_app {
                // Check if the app is available, regardless of configuration
                if self.workspace_state.available_apps.contains(last_app) {
                    let is_configured = configured_repos.contains_key(&recent_repo.repo_id);
                    let description = if is_configured {
                        format!("Open with your preferred app ({})", last_app)
                    } else {
                        format!("Open with {} (basic mode)", last_app)
                    };

                    actions.push(SmartAction {
                        label: format!("🎯 Open {} → {}", recent_repo.repo_id, last_app),
                        description,
                        action_type: SmartActionType::OpenWithPreferred(
                            recent_repo.repo_id.clone(),
                            last_app.clone(),
                        ),
                        priority: 95, // High priority for preferred actions
                    });
                }
            }
        }

        // Add universal opening options for recent repos without preferences
        for recent_repo in recent_repos {
            if recent_repo.last_app.is_none() && !actions.iter().any(|a| {
                matches!(&a.action_type, SmartActionType::OpenWithPreferred(name, _) if name == &recent_repo.repo_id)
            }) {
                // Add opening options for the most common available apps
                for app in &self.workspace_state.available_apps {
                    if matches!(app.as_str(), "vscode" | "cursor" | "warp" | "iterm2") {
                        let is_configured = configured_repos.contains_key(&recent_repo.repo_id);
                        let description = if is_configured {
                            format!("Open with {} (configured)", app)
                        } else {
                            format!("Open with {} (basic)", app)
                        };

                        actions.push(SmartAction {
                            label: format!("📂 Open {} → {}", recent_repo.repo_id, app),
                            description,
                            action_type: SmartActionType::OpenWithPreferred(
                                recent_repo.repo_id.clone(),
                                app.clone(),
                            ),
                            priority: 80,
                        });

                        // Only show one app option per repo to avoid clutter
                        break;
                    }
                }
            }
        }

        // Add "Configure and open" for unconfigured repos (now as enhancement, not requirement)
        for unconfigured_repo in &self.workspace_state.unconfigured_repos {
            if self.workspace_state.available_apps.len() >= 1 && !actions.iter().any(|a| {
                matches!(&a.action_type, SmartActionType::OpenWithPreferred(name, _) if name == unconfigured_repo)
            }) {
                actions.push(SmartAction {
                    label: format!("⚙️ Configure templates for {}", unconfigured_repo),
                    description: "Set up advanced templates and automation".to_string(),
                    action_type: SmartActionType::ConfigureAndOpen(unconfigured_repo.clone()),
                    priority: 70, // Lower priority since configuration is now optional
                });
            }
        }

        // Add batch configuration for multiple unconfigured repos (as enhancement)
        if self.workspace_state.unconfigured_repos.len() > 3 {
            let count = self.workspace_state.unconfigured_repos.len();
            actions.push(SmartAction {
                label: format!("⚙️ Set up templates for {} repos", count),
                description: "Configure advanced templates and automation".to_string(),
                action_type: SmartActionType::QuickConfigureBatch(
                    self.workspace_state.unconfigured_repos.clone(),
                ),
                priority: 60, // Lower priority since templates are enhancements
            });
        }

        // Sort by priority and limit to top 5 smart open actions
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));
        actions.truncate(5);
        actions
    }
}

/// Create a context-aware menu item label
pub fn create_menu_item(base_label: &str, context: Option<&str>) -> String {
    match context {
        Some(ctx) => format!("{base_label} {ctx}"),
        None => base_label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_action_priority() {
        let action1 = SmartAction {
            label: "Action 1".to_string(),
            description: "Test".to_string(),
            action_type: SmartActionType::DiscoverRepos,
            priority: 50,
        };

        let action2 = SmartAction {
            label: "Action 2".to_string(),
            description: "Test".to_string(),
            action_type: SmartActionType::InstallApps,
            priority: 100,
        };

        let mut actions = vec![action1, action2];
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        assert_eq!(actions[0].priority, 100);
        assert_eq!(actions[1].priority, 50);
    }
}
