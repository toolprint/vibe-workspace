use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub workspace: WorkspaceInfo,
    pub repositories: Vec<Repository>,
    pub groups: Vec<RepositoryGroup>,
    pub apps: AppIntegrations,
    #[serde(default)]
    pub preferences: Option<Preferences>,
    #[serde(default)]
    pub claude_agents: Option<ClaudeAgentsIntegration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root: PathBuf,
    pub auto_discover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub path: PathBuf,
    pub url: Option<String>,
    pub branch: Option<String>,
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppConfig {
    Enabled(bool),
    WithTemplate {
        template: String,
    },
    WithConfig {
        template: String,
        #[serde(default)]
        config: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryGroup {
    pub name: String,
    pub repos: Vec<String>,
    pub apps: HashMap<String, AppIntegration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIntegrations {
    pub github: Option<GitHubIntegration>,
    pub warp: Option<WarpIntegration>,
    pub iterm2: Option<ITerm2Integration>,
    pub vscode: Option<VSCodeIntegration>,
    pub wezterm: Option<WezTermIntegration>,
    pub cursor: Option<CursorIntegration>,
    pub windsurf: Option<WindsurfIntegration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIntegration {
    pub enabled: bool,
    pub token_source: String, // "gh", "env", or "file"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpIntegration {
    pub enabled: bool,
    pub config_dir: PathBuf,
    #[serde(default = "default_warp_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ITerm2Integration {
    pub enabled: bool,
    pub config_dir: PathBuf,
    #[serde(default = "default_iterm2_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WezTermIntegration {
    pub enabled: bool,
    pub config_dir: PathBuf,
    #[serde(default = "default_wezterm_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VSCodeIntegration {
    pub enabled: bool,
    pub workspace_dir: PathBuf,
    #[serde(default = "default_vscode_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorIntegration {
    pub enabled: bool,
    pub workspace_dir: PathBuf,
    #[serde(default = "default_cursor_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindsurfIntegration {
    pub enabled: bool,
    pub workspace_dir: PathBuf,
    #[serde(default = "default_windsurf_template_dir")]
    pub template_dir: PathBuf,
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeAgentsIntegration {
    pub enabled: bool,
    #[serde(default = "default_claude_agents_source_path")]
    pub source_path: PathBuf,
    #[serde(default = "default_claude_agents_target_path")]
    pub target_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Preferences {
    #[serde(default)]
    pub page_sizes: PageSizes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSizes {
    #[serde(default = "default_main_menu_page_size")]
    pub main_menu: usize,
    #[serde(default = "default_repository_list_page_size")]
    pub repository_list: usize,
    #[serde(default = "default_quick_launch_page_size")]
    pub quick_launch: usize,
    #[serde(default = "default_app_selection_page_size")]
    pub app_selection: usize,
    #[serde(default = "default_git_search_results_page_size")]
    pub git_search_results: usize,
    #[serde(default = "default_management_menus_page_size")]
    pub management_menus: usize,
    #[serde(default = "default_app_installer_page_size")]
    pub app_installer: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppIntegration {
    Simple(bool),
    Warp { commands: Vec<String> },
    VSCode { extensions: Vec<String> },
    ITerm2 { profile: String },
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        let vibe_dir = super::constants::get_config_dir();

        Self {
            workspace: WorkspaceInfo {
                name: "default".to_string(),
                root: PathBuf::from("."),
                auto_discover: true,
            },
            repositories: Vec::new(),
            groups: Vec::new(),
            apps: AppIntegrations {
                github: Some(GitHubIntegration {
                    enabled: true,
                    token_source: "gh".to_string(),
                }),
                warp: Some(WarpIntegration {
                    enabled: true,
                    config_dir: dirs::home_dir()
                        .unwrap_or_default()
                        .join(".warp")
                        .join("launch_configurations"),
                    template_dir: vibe_dir.join("templates").join("warp"),
                    default_template: "default".to_string(),
                }),
                iterm2: Some(ITerm2Integration {
                    enabled: true,
                    config_dir: dirs::home_dir()
                        .unwrap_or_default()
                        .join("Library")
                        .join("Application Support")
                        .join("iTerm2")
                        .join("DynamicProfiles"),
                    template_dir: vibe_dir.join("templates").join("iterm2"),
                    default_template: "default".to_string(),
                }),
                wezterm: Some(WezTermIntegration {
                    enabled: true,
                    config_dir: dirs::config_dir()
                        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
                        .join("wezterm"),
                    template_dir: vibe_dir.join("templates").join("wezterm"),
                    default_template: "default".to_string(),
                }),
                vscode: Some(VSCodeIntegration {
                    enabled: true,
                    workspace_dir: dirs::home_dir()
                        .unwrap_or_default()
                        .join(".vscode")
                        .join("workspaces"),
                    template_dir: vibe_dir.join("templates").join("vscode"),
                    default_template: "default".to_string(),
                }),
                cursor: Some(CursorIntegration {
                    enabled: true,
                    workspace_dir: dirs::home_dir()
                        .unwrap_or_default()
                        .join(".cursor")
                        .join("workspaces"),
                    template_dir: vibe_dir.join("templates").join("cursor"),
                    default_template: "default".to_string(),
                }),
                windsurf: Some(WindsurfIntegration {
                    enabled: true,
                    workspace_dir: dirs::home_dir()
                        .unwrap_or_default()
                        .join(".windsurf")
                        .join("workspaces"),
                    template_dir: vibe_dir.join("templates").join("windsurf"),
                    default_template: "default".to_string(),
                }),
            },
            preferences: Some(Preferences::default()),
            claude_agents: Some(ClaudeAgentsIntegration {
                enabled: true,
                source_path: PathBuf::from(".").join("wshobson").join("agents"),
                target_path: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".claude")
                    .join("agents"),
            }),
        }
    }
}

impl WorkspaceConfig {
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let mut config: Self = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Ensure all app integrations are initialized
        config.ensure_app_integrations_initialized().await?;

        Ok(config)
    }

    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let yaml = serde_yaml::to_string(self).context("Failed to serialize config to YAML")?;

        fs::write(path, yaml)
            .await
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    pub fn get_repository(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|repo| repo.name == name)
    }

    /// Get a repository by flexible name lookup (supports owner/repo format)
    pub fn get_repository_flexible(&self, name: &str) -> Option<&Repository> {
        // First try exact match
        if let Some(repo) = self.get_repository(name) {
            return Some(repo);
        }

        // Try case-insensitive match
        let lower_name = name.to_lowercase();
        if let Some(repo) = self
            .repositories
            .iter()
            .find(|repo| repo.name.to_lowercase() == lower_name)
        {
            return Some(repo);
        }

        // Try extracting repo name from owner/repo format
        if let Some((_owner, repo_name)) = name.split_once('/') {
            // Try exact match on repo name
            if let Some(repo) = self.get_repository(repo_name) {
                return Some(repo);
            }

            // Try case-insensitive match on repo name
            let lower_repo_name = repo_name.to_lowercase();
            if let Some(repo) = self
                .repositories
                .iter()
                .find(|repo| repo.name.to_lowercase() == lower_repo_name)
            {
                return Some(repo);
            }
        }

        // Try to match against URL if present
        let lower_search = name.to_lowercase();
        self.repositories.iter().find(|repo| {
            if let Some(url) = &repo.url {
                let lower_url = url.to_lowercase();
                // Check if URL contains the search term (handles owner/repo in URLs)
                lower_url.contains(&lower_search) ||
                // Check if the last part of the URL path matches
                lower_url.split('/').next_back()
                    .map(|last| last.trim_end_matches(".git") == lower_search)
                    .unwrap_or(false)
            } else {
                false
            }
        })
    }

    pub fn get_repositories_in_group(&self, group_name: &str) -> Vec<&Repository> {
        if let Some(group) = self.groups.iter().find(|g| g.name == group_name) {
            group
                .repos
                .iter()
                .filter_map(|repo_name| self.get_repository(repo_name))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn add_repository(&mut self, repo: Repository) {
        // Remove existing repository with same name if present
        self.repositories.retain(|r| r.name != repo.name);
        self.repositories.push(repo);
    }

    pub fn add_group(&mut self, group: RepositoryGroup) {
        // Remove existing group with same name if present
        self.groups.retain(|g| g.name != group.name);
        self.groups.push(group);
    }

    /// Ensure all app integrations are properly initialized
    /// This method handles migration from older configurations that may not have all apps configured
    pub async fn ensure_app_integrations_initialized(&mut self) -> Result<()> {
        let vibe_dir = super::constants::get_config_dir();

        // Ensure WezTerm integration is initialized
        if self.apps.wezterm.is_none() {
            self.apps.wezterm = Some(WezTermIntegration {
                enabled: true,
                config_dir: dirs::config_dir()
                    .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
                    .join("wezterm"),
                template_dir: vibe_dir.join("templates").join("wezterm"),
                default_template: "default".to_string(),
            });
        }

        // Initialize other app integrations if they're missing
        if self.apps.warp.is_none() {
            self.apps.warp = Some(WarpIntegration {
                enabled: true,
                config_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".warp")
                    .join("launch_configurations"),
                template_dir: vibe_dir.join("templates").join("warp"),
                default_template: "default".to_string(),
            });
        }

        if self.apps.iterm2.is_none() {
            self.apps.iterm2 = Some(ITerm2Integration {
                enabled: true,
                config_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library")
                    .join("Application Support")
                    .join("iTerm2")
                    .join("DynamicProfiles"),
                template_dir: vibe_dir.join("templates").join("iterm2"),
                default_template: "default".to_string(),
            });
        }

        if self.apps.vscode.is_none() {
            self.apps.vscode = Some(VSCodeIntegration {
                enabled: true,
                workspace_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".vscode")
                    .join("workspaces"),
                template_dir: vibe_dir.join("templates").join("vscode"),
                default_template: "default".to_string(),
            });
        }

        if self.apps.cursor.is_none() {
            self.apps.cursor = Some(CursorIntegration {
                enabled: true,
                workspace_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".cursor")
                    .join("workspaces"),
                template_dir: vibe_dir.join("templates").join("cursor"),
                default_template: "default".to_string(),
            });
        }

        if self.apps.windsurf.is_none() {
            self.apps.windsurf = Some(WindsurfIntegration {
                enabled: true,
                workspace_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".windsurf")
                    .join("workspaces"),
                template_dir: vibe_dir.join("templates").join("windsurf"),
                default_template: "default".to_string(),
            });
        }

        // Initialize claude_agents integration if missing
        if self.claude_agents.is_none() {
            self.claude_agents = Some(ClaudeAgentsIntegration {
                enabled: true,
                source_path: self.workspace.root.join("wshobson").join("agents"),
                target_path: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".claude")
                    .join("agents"),
            });
        }

        Ok(())
    }
}

impl Repository {
    pub fn new<S: Into<String>, P: Into<PathBuf>>(name: S, path: P) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            url: None,
            branch: None,
            apps: HashMap::new(),
        }
    }

    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_branch<S: Into<String>>(mut self, branch: S) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn enable_app<S: Into<String>>(mut self, app: S) -> Self {
        self.apps.insert(app.into(), AppConfig::Enabled(true));
        self
    }

    pub fn enable_app_with_template<S: Into<String>, T: Into<String>>(
        mut self,
        app: S,
        template: T,
    ) -> Self {
        self.apps.insert(
            app.into(),
            AppConfig::WithTemplate {
                template: template.into(),
            },
        );
        self
    }

    pub fn is_app_enabled(&self, app: &str) -> bool {
        match self.apps.get(app) {
            Some(AppConfig::Enabled(enabled)) => *enabled,
            Some(AppConfig::WithTemplate { .. }) => true,
            Some(AppConfig::WithConfig { .. }) => true,
            None => false,
        }
    }

    pub fn get_app_template(&self, app: &str) -> Option<&str> {
        match self.apps.get(app) {
            Some(AppConfig::WithTemplate { template }) => Some(template),
            Some(AppConfig::WithConfig { template, .. }) => Some(template),
            _ => None,
        }
    }
}

impl AppConfig {
    pub fn is_enabled(&self) -> bool {
        match self {
            AppConfig::Enabled(enabled) => *enabled,
            AppConfig::WithTemplate { .. } => true,
            AppConfig::WithConfig { .. } => true,
        }
    }
}

// Default functions for serde
fn default_template_name() -> String {
    "default".to_string()
}

fn default_warp_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("warp")
}

fn default_iterm2_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("iterm2")
}

fn default_wezterm_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("wezterm")
}

fn default_vscode_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("vscode")
}

fn default_cursor_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("cursor")
}

fn default_windsurf_template_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(super::constants::CONFIG_DIR_PATH)
        .join("templates")
        .join("windsurf")
}

fn default_claude_agents_source_path() -> PathBuf {
    PathBuf::from(".").join("wshobson").join("agents")
}

fn default_claude_agents_target_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("agents")
}

// Page size defaults
fn default_main_menu_page_size() -> usize {
    15
}

fn default_repository_list_page_size() -> usize {
    15
}

fn default_quick_launch_page_size() -> usize {
    9
}

fn default_app_selection_page_size() -> usize {
    10
}

fn default_git_search_results_page_size() -> usize {
    15
}

fn default_management_menus_page_size() -> usize {
    10
}

fn default_app_installer_page_size() -> usize {
    15
}

impl Default for PageSizes {
    fn default() -> Self {
        Self {
            main_menu: default_main_menu_page_size(),
            repository_list: default_repository_list_page_size(),
            quick_launch: default_quick_launch_page_size(),
            app_selection: default_app_selection_page_size(),
            git_search_results: default_git_search_results_page_size(),
            management_menus: default_management_menus_page_size(),
            app_installer: default_app_installer_page_size(),
        }
    }
}

impl PageSizes {
    /// Validate page size values and return errors for invalid ranges
    pub fn validate(&self) -> Result<()> {
        if self.quick_launch == 0 || self.quick_launch > 9 {
            anyhow::bail!("quick_launch page size must be between 1 and 9 (limited by number key shortcuts), got {}", self.quick_launch);
        }

        let sizes = [
            ("main_menu", self.main_menu),
            ("repository_list", self.repository_list),
            ("app_selection", self.app_selection),
            ("git_search_results", self.git_search_results),
            ("management_menus", self.management_menus),
            ("app_installer", self.app_installer),
        ];

        for (name, size) in &sizes {
            if *size == 0 || *size > 15 {
                anyhow::bail!("{} page size must be between 1 and 15, got {}", name, size);
            }
        }

        Ok(())
    }
}
