use anyhow::{Context, Result};
use serde_yaml;
use serde_json;
use std::path::Path;
use tokio::fs;

use crate::fixtures::{WorkspaceTestConfig, RepositoryConfig};

/// Assertion helpers for validating test outcomes
pub struct Assert;

impl Assert {
    /// Assert that a file exists
    pub async fn file_exists(path: &Path) -> Result<()> {
        if !path.exists() {
            anyhow::bail!("File does not exist: {}", path.display());
        }
        Ok(())
    }
    
    /// Assert that a directory exists
    pub async fn dir_exists(path: &Path) -> Result<()> {
        if !path.is_dir() {
            anyhow::bail!("Directory does not exist: {}", path.display());
        }
        Ok(())
    }
    
    /// Assert that a file contains specific text
    pub async fn file_contains(path: &Path, expected: &str) -> Result<()> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        
        if !content.contains(expected) {
            anyhow::bail!(
                "File {} does not contain expected text: '{}'",
                path.display(),
                expected
            );
        }
        Ok(())
    }
    
    /// Assert that output contains specific text
    pub fn output_contains(output: &str, expected: &str) -> Result<()> {
        if !output.contains(expected) {
            anyhow::bail!(
                "Output does not contain expected text: '{}'\nActual output:\n{}",
                expected,
                output
            );
        }
        Ok(())
    }
    
    /// Assert that output matches a pattern
    pub fn output_matches(output: &str, pattern: &str) -> Result<()> {
        let re = regex::Regex::new(pattern)?;
        if !re.is_match(output) {
            anyhow::bail!(
                "Output does not match pattern: '{}'\nActual output:\n{}",
                pattern,
                output
            );
        }
        Ok(())
    }
}

/// Config-specific assertions
pub struct ConfigAssert;

impl ConfigAssert {
    /// Load and validate workspace configuration
    pub async fn load_config(config_path: &Path) -> Result<WorkspaceTestConfig> {
        let content = fs::read_to_string(config_path)
            .await
            .context("Failed to read config file")?;
        
        let config: WorkspaceTestConfig = serde_yaml::from_str(&content)
            .context("Failed to parse config YAML")?;
        
        Ok(config)
    }
    
    /// Assert workspace name
    pub async fn workspace_name(config_path: &Path, expected: &str) -> Result<()> {
        let config = Self::load_config(config_path).await?;
        
        if config.workspace.name != expected {
            anyhow::bail!(
                "Workspace name mismatch. Expected: '{}', Got: '{}'",
                expected,
                config.workspace.name
            );
        }
        Ok(())
    }
    
    /// Assert repository exists in config
    pub async fn has_repository(config_path: &Path, repo_name: &str) -> Result<()> {
        let config = Self::load_config(config_path).await?;
        
        if !config.repositories.iter().any(|r| r.name == repo_name) {
            anyhow::bail!(
                "Repository '{}' not found in config. Available: {:?}",
                repo_name,
                config.repositories.iter().map(|r| &r.name).collect::<Vec<_>>()
            );
        }
        Ok(())
    }
    
    /// Assert repository count
    pub async fn repository_count(config_path: &Path, expected: usize) -> Result<()> {
        let config = Self::load_config(config_path).await?;
        
        if config.repositories.len() != expected {
            anyhow::bail!(
                "Repository count mismatch. Expected: {}, Got: {}",
                expected,
                config.repositories.len()
            );
        }
        Ok(())
    }
    
    /// Assert app is configured for repository
    pub async fn repo_has_app(config_path: &Path, repo_name: &str, app: &str) -> Result<()> {
        let config = Self::load_config(config_path).await?;
        
        let repo = config.repositories
            .iter()
            .find(|r| r.name == repo_name)
            .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_name))?;
        
        if !repo.apps.contains_key(app) {
            anyhow::bail!(
                "App '{}' not configured for repository '{}'. Configured apps: {:?}",
                app,
                repo_name,
                repo.apps.keys().collect::<Vec<_>>()
            );
        }
        Ok(())
    }
}

/// App configuration assertions
pub struct AppAssert;

impl AppAssert {
    /// Assert that app configuration file exists
    pub async fn config_exists(app_config_dir: &Path, expected_file: &str) -> Result<()> {
        let config_file = app_config_dir.join(expected_file);
        
        if !config_file.exists() {
            anyhow::bail!(
                "App configuration file does not exist: {}",
                config_file.display()
            );
        }
        Ok(())
    }
    
    /// Assert Warp configuration is valid
    pub async fn warp_config_valid(config_path: &Path) -> Result<()> {
        let content = fs::read_to_string(config_path).await?;
        
        // Basic YAML validation
        let _: serde_yaml::Value = serde_yaml::from_str(&content)
            .context("Invalid Warp configuration YAML")?;
        
        // Check for required fields
        Assert::file_contains(config_path, "name:").await?;
        Assert::file_contains(config_path, "paths:").await?;
        
        Ok(())
    }
    
    /// Assert iTerm2 configuration is valid
    pub async fn iterm2_config_valid(config_path: &Path) -> Result<()> {
        let content = fs::read_to_string(config_path).await?;
        
        // Basic JSON validation
        let value: serde_json::Value = serde_json::from_str(&content)
            .context("Invalid iTerm2 configuration JSON")?;
        
        // Check for required fields
        if !value.is_object() {
            anyhow::bail!("iTerm2 config should be a JSON object");
        }
        
        let obj = value.as_object().unwrap();
        for field in ["Guid", "Name", "Working Directory"] {
            if !obj.contains_key(field) {
                anyhow::bail!("iTerm2 config missing required field: {}", field);
            }
        }
        
        Ok(())
    }
    
    /// Assert VSCode workspace is valid
    pub async fn vscode_workspace_valid(config_path: &Path) -> Result<()> {
        let content = fs::read_to_string(config_path).await?;
        
        // Basic JSON validation
        let value: serde_json::Value = serde_json::from_str(&content)
            .context("Invalid VSCode workspace JSON")?;
        
        // Check for folders array
        if !value.get("folders").map(|f| f.is_array()).unwrap_or(false) {
            anyhow::bail!("VSCode workspace must contain 'folders' array");
        }
        
        Ok(())
    }
}

/// Git repository assertions
pub struct GitAssert;

impl GitAssert {
    /// Assert that a directory is a git repository
    pub async fn is_git_repo(path: &Path) -> Result<()> {
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            anyhow::bail!("Not a git repository: {}", path.display());
        }
        Ok(())
    }
    
    /// Assert current branch name
    pub async fn current_branch(repo_path: &Path, expected: &str) -> Result<()> {
        use git2::Repository;
        
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;
        
        let branch_name = head
            .shorthand()
            .ok_or_else(|| anyhow::anyhow!("Could not get branch name"))?;
        
        if branch_name != expected {
            anyhow::bail!(
                "Branch mismatch. Expected: '{}', Got: '{}'",
                expected,
                branch_name
            );
        }
        Ok(())
    }
    
    /// Assert repository has specific remote
    pub async fn has_remote(repo_path: &Path, remote_name: &str, expected_url: &str) -> Result<()> {
        use git2::Repository;
        
        let repo = Repository::open(repo_path)?;
        let remote = repo.find_remote(remote_name)?;
        
        let url = remote.url().ok_or_else(|| anyhow::anyhow!("Remote has no URL"))?;
        
        if url != expected_url {
            anyhow::bail!(
                "Remote URL mismatch. Expected: '{}', Got: '{}'",
                expected_url,
                url
            );
        }
        Ok(())
    }
}

/// Test result builder for fluent assertions
pub struct TestResultBuilder {
    success: bool,
    errors: Vec<String>,
}

impl TestResultBuilder {
    pub fn new() -> Self {
        Self {
            success: true,
            errors: Vec::new(),
        }
    }
    
    /// Add an assertion
    pub fn assert<F>(mut self, assertion: F) -> Self
    where
        F: FnOnce() -> Result<()>,
    {
        if let Err(e) = assertion() {
            self.success = false;
            self.errors.push(e.to_string());
        }
        self
    }
    
    /// Add an async assertion
    pub async fn assert_async<F, Fut>(mut self, assertion: F) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        if let Err(e) = assertion().await {
            self.success = false;
            self.errors.push(e.to_string());
        }
        self
    }
    
    /// Build the final result
    pub fn build(self) -> Result<()> {
        if self.success {
            Ok(())
        } else {
            anyhow::bail!("Test failed with {} errors:\n{}", 
                self.errors.len(),
                self.errors.join("\n")
            )
        }
    }
}

// Add regex to Cargo.toml dependencies
pub use regex;