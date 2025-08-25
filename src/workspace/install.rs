use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::git::{GitConfig, GitError};
use crate::utils::fs::expand_tilde;
use crate::utils::git::is_git_available;
use crate::workspace::config::Repository as ConfigRepository;

pub struct RepositoryInstaller {
    workspace_root: PathBuf,
    git_config: GitConfig,
}

#[derive(Debug, Clone)]
pub struct InstalledRepository {
    pub repository: ConfigRepository,
    pub path: PathBuf,
    pub post_install_actions: Vec<PostInstallAction>,
}

#[derive(Debug, Clone)]
pub enum PostInstallAction {
    RunNpmInstall,
    RunCargoCheck,
    OpenInEditor(String),
}

impl RepositoryInstaller {
    pub fn new(workspace_root: PathBuf, git_config: GitConfig) -> Self {
        Self {
            workspace_root: expand_tilde(&workspace_root),
            git_config,
        }
    }

    pub async fn install_from_url(&self, url: &str) -> Result<InstalledRepository> {
        self.install_from_url_with_options(url, None, false, false)
            .await
    }

    pub async fn install_from_url_with_options(
        &self,
        url: &str,
        custom_path: Option<PathBuf>,
        open_after_clone: bool,
        run_install_commands: bool,
    ) -> Result<InstalledRepository> {
        if !is_git_available() {
            anyhow::bail!("Git is not available on the system");
        }

        let (org, repo_name) = self.parse_git_url(url)?;
        let target_path = if let Some(path) = custom_path {
            expand_tilde(&path)
        } else {
            self.calculate_install_path(&org, &repo_name)
        };

        // Check if repository already exists
        if target_path.exists() {
            return Err(GitError::RepositoryExists { path: target_path }.into());
        }

        println!(
            "{} Cloning {} to {}",
            "📦".cyan(),
            url.cyan().bold(),
            target_path.display().to_string().green()
        );

        // Create parent directory if needed
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directory")?;
        }

        // Clone repository
        self.clone_repository(url, &target_path).await?;

        // Create repository config
        let installed_repo = self.create_repository_config(&org, &repo_name, url, &target_path)?;

        // Determine post-install actions
        let mut post_install_actions = Vec::new();

        if run_install_commands {
            // Check for package.json
            if target_path.join("package.json").exists() {
                post_install_actions.push(PostInstallAction::RunNpmInstall);
            }
            // Check for Cargo.toml
            if target_path.join("Cargo.toml").exists() {
                post_install_actions.push(PostInstallAction::RunCargoCheck);
            }
        }

        if open_after_clone {
            post_install_actions.push(PostInstallAction::OpenInEditor("vscode".to_string()));
        }

        println!("{} Successfully cloned repository", "✅".green());

        Ok(InstalledRepository {
            repository: installed_repo,
            path: target_path,
            post_install_actions,
        })
    }

    fn parse_git_url(&self, url: &str) -> Result<(String, String)> {
        // Handle different URL formats
        let url = url.trim();

        // SSH format: git@github.com:org/repo.git
        if url.starts_with("git@") {
            let parts: Vec<&str> = url.split(':').collect();
            if parts.len() != 2 {
                return Err(GitError::InvalidUrl {
                    url: url.to_string(),
                }
                .into());
            }

            let path_parts: Vec<&str> = parts[1].trim_end_matches(".git").split('/').collect();

            if path_parts.len() != 2 {
                return Err(GitError::InvalidUrl {
                    url: url.to_string(),
                }
                .into());
            }

            return Ok((path_parts[0].to_string(), path_parts[1].to_string()));
        }

        // HTTPS format: https://github.com/org/repo or https://github.com/org/repo.git
        if url.starts_with("https://") || url.starts_with("http://") {
            let parsed_url = url::Url::parse(url).map_err(|_| GitError::InvalidUrl {
                url: url.to_string(),
            })?;

            let path = parsed_url
                .path()
                .trim_start_matches('/')
                .trim_end_matches(".git");
            let path_parts: Vec<&str> = path.split('/').collect();

            if path_parts.len() < 2 {
                return Err(GitError::InvalidUrl {
                    url: url.to_string(),
                }
                .into());
            }

            // Handle potential subdirectories (e.g., gitlab.com/group/subgroup/repo)
            // For now, we'll use the last two components
            let org = path_parts[path_parts.len() - 2].to_string();
            let repo = path_parts[path_parts.len() - 1].to_string();

            return Ok((org, repo));
        }

        // Shorthand format: org/repo
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }

        Err(GitError::InvalidUrl {
            url: url.to_string(),
        }
        .into())
    }

    fn calculate_install_path(&self, org: &str, repo: &str) -> PathBuf {
        if self.git_config.standardize_paths {
            self.workspace_root.join(org).join(repo)
        } else {
            self.workspace_root.join(repo)
        }
    }

    async fn clone_repository(&self, url: &str, target_path: &Path) -> Result<()> {
        let output = Command::new("git")
            .args(["clone", url, target_path.to_str().unwrap()])
            .output()
            .await
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::CloneFailed {
                message: error_msg.to_string(),
            }
            .into());
        }

        Ok(())
    }

    fn create_repository_config(
        &self,
        org: &str,
        repo_name: &str,
        url: &str,
        path: &Path,
    ) -> Result<ConfigRepository> {
        use std::collections::HashMap;

        Ok(ConfigRepository {
            name: format!("{org}/{repo_name}"),
            path: path.to_path_buf(),
            url: Some(url.to_string()),
            branch: None, // Will be detected from the actual repository
            apps: HashMap::new(),
            worktree_config: None,
        })
    }

    pub async fn execute_post_install_actions(
        &self,
        actions: &[PostInstallAction],
        repo_path: &Path,
    ) -> Result<()> {
        for action in actions {
            match action {
                PostInstallAction::RunNpmInstall => {
                    println!("{} Running npm install...", "📦".cyan());
                    let output = Command::new("npm")
                        .arg("install")
                        .current_dir(repo_path)
                        .output()
                        .await?;

                    if !output.status.success() {
                        eprintln!("Warning: npm install failed");
                    }
                }
                PostInstallAction::RunCargoCheck => {
                    println!("{} Running cargo check...", "🦀".cyan());
                    let output = Command::new("cargo")
                        .arg("check")
                        .current_dir(repo_path)
                        .output()
                        .await?;

                    if !output.status.success() {
                        eprintln!("Warning: cargo check failed");
                    }
                }
                PostInstallAction::OpenInEditor(editor) => {
                    println!("{} Opening in {}...", "📝".cyan(), editor);
                    // This would integrate with the existing app launching functionality
                    // For now, we'll just print a message
                }
            }
        }
        Ok(())
    }
}
