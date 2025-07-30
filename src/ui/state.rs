use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// Represents a recently accessed repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRepo {
    /// Repository identifier (path or name)
    pub repo_id: String,
    /// Full path to the repository
    pub path: PathBuf,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Last used app for this repository
    pub last_app: Option<String>,
    /// Number of times this repo has been accessed
    pub access_count: u32,
}

/// User preferences for the vibe workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Default app to use when none is specified
    pub default_app: Option<String>,
    /// Whether to show the setup wizard on startup
    pub show_setup_wizard: bool,
    /// Whether to auto-open last repo on startup
    pub auto_open_last_repo: bool,
    /// Maximum number of recent repos to track
    pub max_recent_repos: usize,
    /// Whether to show hints in the interface
    pub show_hints: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_app: None,
            show_setup_wizard: true,
            auto_open_last_repo: false,
            max_recent_repos: 10,
            show_hints: true,
        }
    }
}

/// Persistent state for user preferences and recent actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeState {
    /// List of recently accessed repositories
    pub recent_repos: Vec<RecentRepo>,
    /// Last used app per repository
    pub last_used_apps: HashMap<String, String>,
    /// User preferences
    pub user_preferences: UserPreferences,
    /// Groups of repositories for batch operations
    pub repo_groups: HashMap<String, Vec<String>>,
    /// First run timestamp (for setup wizard)
    pub first_run: Option<DateTime<Utc>>,
    /// Version of the state file format
    pub version: u32,
}

impl Default for VibeState {
    fn default() -> Self {
        Self {
            recent_repos: Vec::new(),
            last_used_apps: HashMap::new(),
            user_preferences: UserPreferences::default(),
            repo_groups: HashMap::new(),
            first_run: Some(Utc::now()),
            version: 1,
        }
    }
}

impl VibeState {
    /// Load state from the default location
    pub fn load() -> Result<Self> {
        let state_path = Self::default_state_path()?;
        if state_path.exists() {
            Self::load_from_path(&state_path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load state from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let state: VibeState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save state to the default location
    pub fn save(&self) -> Result<()> {
        let state_path = Self::default_state_path()?;
        self.save_to_path(&state_path)
    }

    /// Save state to a specific path
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Get the default state file path
    fn default_state_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".vibe-workspace").join("state.json"))
    }

    /// Add or update a recent repository
    pub fn add_recent_repo(&mut self, repo_id: String, path: PathBuf, app: Option<String>) {
        let now = Utc::now();

        // Update last used app if provided
        if let Some(app_name) = &app {
            self.last_used_apps
                .insert(repo_id.clone(), app_name.clone());
        }

        // Check if repo already exists
        if let Some(existing) = self.recent_repos.iter_mut().find(|r| r.repo_id == repo_id) {
            existing.last_accessed = now;
            existing.access_count += 1;
            if app.is_some() {
                existing.last_app = app;
            }
        } else {
            // Add new repo
            self.recent_repos.push(RecentRepo {
                repo_id: repo_id.clone(),
                path,
                last_accessed: now,
                last_app: app,
                access_count: 1,
            });
        }

        // Sort by last accessed (most recent first) and trim to max
        self.recent_repos
            .sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        self.recent_repos
            .truncate(self.user_preferences.max_recent_repos);
    }

    /// Get the most recently accessed repositories
    pub fn get_recent_repos(&self, limit: usize) -> &[RecentRepo] {
        let end = limit.min(self.recent_repos.len());
        &self.recent_repos[..end]
    }

    /// Get the last used app for a repository
    pub fn get_last_app(&self, repo_id: &str) -> Option<&String> {
        self.last_used_apps.get(repo_id)
    }

    /// Check if this is the first run
    pub fn is_first_run(&self) -> bool {
        self.first_run.is_some() && self.recent_repos.is_empty()
    }

    /// Mark setup wizard as completed
    pub fn complete_setup_wizard(&mut self) {
        self.user_preferences.show_setup_wizard = false;
        self.first_run = None;
    }

    /// Add a repository group
    pub fn add_repo_group(&mut self, name: String, repos: Vec<String>) {
        self.repo_groups.insert(name, repos);
    }

    /// Get repositories in a group
    pub fn get_repo_group(&self, name: &str) -> Option<&Vec<String>> {
        self.repo_groups.get(name)
    }

    /// Get the most frequently accessed repositories
    pub fn get_frequent_repos(&self, limit: usize) -> Vec<&RecentRepo> {
        let mut repos: Vec<&RecentRepo> = self.recent_repos.iter().collect();
        repos.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        repos.truncate(limit);
        repos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("state.json");

        // Create and save state
        let mut state = VibeState::default();
        state.add_recent_repo(
            "test-repo".to_string(),
            PathBuf::from("/path/to/repo"),
            Some("vscode".to_string()),
        );
        state.save_to_path(&state_path).unwrap();

        // Load and verify
        let loaded = VibeState::load_from_path(&state_path).unwrap();
        assert_eq!(loaded.recent_repos.len(), 1);
        assert_eq!(loaded.recent_repos[0].repo_id, "test-repo");
        assert_eq!(
            loaded.get_last_app("test-repo"),
            Some(&"vscode".to_string())
        );
    }

    #[test]
    fn test_recent_repos_ordering() {
        let mut state = VibeState::default();
        state.user_preferences.max_recent_repos = 3;

        // Add repos
        state.add_recent_repo("repo1".to_string(), PathBuf::from("/repo1"), None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        state.add_recent_repo("repo2".to_string(), PathBuf::from("/repo2"), None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        state.add_recent_repo("repo3".to_string(), PathBuf::from("/repo3"), None);

        // Access repo1 again - should move to top
        state.add_recent_repo("repo1".to_string(), PathBuf::from("/repo1"), None);

        let recent = state.get_recent_repos(3);
        assert_eq!(recent[0].repo_id, "repo1");
        assert_eq!(recent[0].access_count, 2);
        assert_eq!(recent[1].repo_id, "repo3");
        assert_eq!(recent[2].repo_id, "repo2");
    }

    #[test]
    fn test_repo_groups() {
        let mut state = VibeState::default();

        state.add_repo_group(
            "frontend".to_string(),
            vec!["web-app".to_string(), "mobile-app".to_string()],
        );
        state.add_repo_group(
            "backend".to_string(),
            vec!["api".to_string(), "services".to_string()],
        );

        assert_eq!(state.get_repo_group("frontend").unwrap().len(), 2);
        assert_eq!(state.get_repo_group("backend").unwrap().len(), 2);
        assert!(state.get_repo_group("nonexistent").is_none());
    }
}
