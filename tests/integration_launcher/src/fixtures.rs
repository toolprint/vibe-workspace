use anyhow::{Context, Result};
use git2::{Repository, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;

/// Test environment containing all paths and resources
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub home_dir: PathBuf,
    pub workspace_root: PathBuf,
    pub config_path: PathBuf,
    pub vibe_dir: PathBuf,
    pub keep_temp: bool,
}

impl TestEnvironment {
    /// Create a new test environment with all necessary directories
    pub async fn new(test_name: &str, keep_temp: bool) -> Result<Self> {
        let temp_dir = TempDir::new()
            .context("Failed to create temp directory")?;
        
        let base_path = temp_dir.path();
        
        // Create directory structure
        let home_dir = base_path.join("home");
        let workspace_root = base_path.join("workspace");
        let vibe_dir = home_dir.join(".toolprint/vibe-workspace");
        let config_path = vibe_dir.join("config.yaml");
        
        // Create all directories
        fs::create_dir_all(&home_dir).await?;
        fs::create_dir_all(&workspace_root).await?;
        fs::create_dir_all(&vibe_dir).await?;
        fs::create_dir_all(vibe_dir.join("templates")).await?;
        
        // Create template directories for each app
        for app in ["warp", "iterm2", "vscode", "wezterm"] {
            fs::create_dir_all(vibe_dir.join("templates").join(app)).await?;
        }
        
        if keep_temp {
            println!("Test environment for '{}': {}", test_name, base_path.display());
        }
        
        Ok(Self {
            temp_dir,
            home_dir,
            workspace_root,
            config_path,
            vibe_dir,
            keep_temp,
        })
    }
    
    /// Get environment variables for running the CLI
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        
        // Override HOME to isolate from user's actual home
        env.insert("HOME".to_string(), self.home_dir.to_string_lossy().to_string());
        env.insert("USERPROFILE".to_string(), self.home_dir.to_string_lossy().to_string());
        
        // Disable any real app integrations during tests
        env.insert("VIBE_TEST_MODE".to_string(), "1".to_string());
        
        env
    }
    
    /// Create a mock git repository
    pub async fn create_git_repo(&self, name: &str) -> Result<PathBuf> {
        let repo_path = self.workspace_root.join(name);
        fs::create_dir_all(&repo_path).await?;
        
        // Create a README file first
        let readme_path = repo_path.join("README.md");
        fs::write(&readme_path, format!("# {}\n\nTest repository", name)).await?;
        
        // Initialize git repository and create commit in blocking context
        let repo_path_clone = repo_path.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let repo = Repository::init(&repo_path_clone)?;
            
            // Create initial commit
            let sig = Signature::now("Test User", "test@example.com")?;
            let tree_id = {
                let mut index = repo.index()?;
                index.add_path(Path::new("README.md"))?;
                index.write()?;
                index.write_tree()?
            };
            
            let tree = repo.find_tree(tree_id)?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Initial commit",
                &tree,
                &[],
            )?;
            
            Ok(())
        }).await??;
        
        Ok(repo_path)
    }
    
    /// Create a basic workspace configuration
    pub async fn create_basic_config(&self) -> Result<()> {
        let config = WorkspaceTestConfig {
            workspace: WorkspaceInfo {
                name: "test-workspace".to_string(),
                root: self.workspace_root.clone(),
                auto_discover: false,
            },
            repositories: vec![],
            groups: vec![],
            apps: AppIntegrations::default(),
        };
        
        let yaml = serde_yaml::to_string(&config)?;
        fs::write(&self.config_path, yaml).await?;
        
        Ok(())
    }
    
    /// Create default app templates
    pub async fn create_default_templates(&self) -> Result<()> {
        // Warp template
        let warp_template = r#"
name: "{{name}}"
paths:
  - "{{path}}"
launch_command: "cd {{path}}"
"#;
        fs::write(
            self.vibe_dir.join("templates/warp/default.yaml"),
            warp_template,
        ).await?;
        
        // iTerm2 template
        let iterm2_template = r#"{
  "Guid": "{{guid}}",
  "Name": "{{name}}",
  "Working Directory": "{{path}}",
  "Custom Directory": "Yes"
}"#;
        fs::write(
            self.vibe_dir.join("templates/iterm2/default.json"),
            iterm2_template,
        ).await?;
        
        // VSCode template
        let vscode_template = r#"{
  "folders": [
    {
      "path": "{{path}}",
      "name": "{{name}}"
    }
  ]
}"#;
        fs::write(
            self.vibe_dir.join("templates/vscode/default.json"),
            vscode_template,
        ).await?;
        
        // WezTerm template
        let wezterm_template = r#"return {
  label = "{{name}}",
  cwd = "{{path}}",
}"#;
        fs::write(
            self.vibe_dir.join("templates/wezterm/default.lua"),
            wezterm_template,
        ).await?;
        
        Ok(())
    }
    
    /// Get the path for app configurations
    pub fn get_app_config_path(&self, app: &str) -> PathBuf {
        match app {
            "warp" => self.home_dir.join(".warp/launch_configurations"),
            "iterm2" => self.home_dir.join("Library/Application Support/iTerm2/DynamicProfiles"),
            "vscode" => self.home_dir.join(".vscode/workspaces"),
            "wezterm" => self.home_dir.join(".config/wezterm/workspaces"),
            _ => panic!("Unknown app: {}", app),
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if self.keep_temp {
            // Prevent automatic cleanup
            let _ = self.temp_dir.path().to_path_buf();
            std::mem::forget(std::mem::replace(&mut self.temp_dir, TempDir::new().unwrap()));
        }
    }
}

// Test configuration structures (simplified versions of the main config)
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceTestConfig {
    pub workspace: WorkspaceInfo,
    pub repositories: Vec<RepositoryConfig>,
    pub groups: Vec<GroupConfig>,
    pub apps: AppIntegrations,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root: PathBuf,
    pub auto_discover: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub name: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppConfig {
    Enabled(bool),
    WithTemplate { template: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    pub repos: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppIntegrations {
    pub warp: Option<AppIntegration>,
    pub iterm2: Option<AppIntegration>,
    pub vscode: Option<AppIntegration>,
    pub wezterm: Option<AppIntegration>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppIntegration {
    pub enabled: bool,
    pub config_dir: PathBuf,
    pub template_dir: PathBuf,
    pub default_template: String,
}

impl Default for AppIntegration {
    fn default() -> Self {
        Self {
            enabled: true,
            config_dir: PathBuf::new(),
            template_dir: PathBuf::new(),
            default_template: "default".to_string(),
        }
    }
}

/// Test data builder for creating complex test scenarios
pub struct TestDataBuilder<'a> {
    env: &'a TestEnvironment,
}

impl<'a> TestDataBuilder<'a> {
    pub fn new(env: &'a TestEnvironment) -> Self {
        Self { env }
    }
    
    /// Create a workspace with multiple repositories
    pub async fn with_repositories(self, count: usize) -> Result<Self> {
        for i in 0..count {
            let name = format!("repo-{}", i + 1);
            self.env.create_git_repo(&name).await?;
        }
        Ok(self)
    }
    
    /// Create a workspace with nested repositories
    pub async fn with_nested_repos(self) -> Result<Self> {
        // Create parent repo
        self.env.create_git_repo("parent").await?;
        
        // Create nested repos
        let nested_path = self.env.workspace_root.join("parent/nested");
        fs::create_dir_all(&nested_path).await?;
        
        let child1_path = nested_path.join("child1");
        let child2_path = nested_path.join("child2");
        
        tokio::task::spawn_blocking(move || -> Result<()> {
            Repository::init(child1_path)?;
            Repository::init(child2_path)?;
            Ok(())
        }).await??;
        
        Ok(self)
    }
    
    /// Create repositories with specific branch names
    pub async fn with_branches(self, branches: &[(&str, &str)]) -> Result<Self> {
        for (repo_name, branch_name) in branches {
            let repo_path = self.env.create_git_repo(repo_name).await?;
            
            let repo_path_clone = repo_path.clone();
            let branch_name = branch_name.to_string();
            tokio::task::spawn_blocking(move || -> Result<()> {
                let repo = Repository::open(&repo_path_clone)?;
                let head = repo.head()?.target().unwrap();
                let commit = repo.find_commit(head)?;
                
                repo.branch(&branch_name, &commit, false)?;
                
                // Switch to the new branch
                repo.set_head(&format!("refs/heads/{}", branch_name))?;
                Ok(())
            }).await??;
        }
        
        Ok(self)
    }
    
    pub fn build(self) -> &'a TestEnvironment {
        self.env
    }
}