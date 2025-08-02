use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;
use tokio::process::Command;

use crate::workspace::Repository;
use crate::workspace::WorkspaceManager;
use crate::{display_println, utils::git::is_github_cli_available};

#[derive(Debug, Clone)]
pub struct GitHubUserInfo {
    pub username: String,
    pub organizations: Vec<GitHubOrganization>,
}

#[derive(Debug, Clone)]
pub struct GitHubOrganization {
    pub login: String,
    pub name: Option<String>,
}

pub struct RepositoryCreator {
    workspace_root: PathBuf,
}

impl RepositoryCreator {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Get GitHub user information including organizations
    pub async fn get_github_user_info(&self) -> Result<GitHubUserInfo> {
        if !is_github_cli_available() {
            anyhow::bail!("GitHub CLI is not available. Please install 'gh' command.");
        }

        // Get username
        let username = self.get_github_username().await?;

        // Get organizations
        let organizations = self.get_github_organizations().await?;

        Ok(GitHubUserInfo {
            username,
            organizations,
        })
    }

    async fn get_github_username(&self) -> Result<String> {
        let output = Command::new("gh")
            .args(&["api", "user", "--jq", ".login"])
            .output()
            .await
            .context("Failed to get GitHub username")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get GitHub username: {}", error_msg);
        }

        let username = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in username response")?
            .trim()
            .to_string();

        if username.is_empty() {
            anyhow::bail!("No GitHub username found. Please authenticate with 'gh auth login'");
        }

        Ok(username)
    }

    async fn get_github_organizations(&self) -> Result<Vec<GitHubOrganization>> {
        let output = Command::new("gh")
            .args(&["api", "user/orgs", "--jq", ".[].login"])
            .output()
            .await
            .context("Failed to get GitHub organizations")?;

        if !output.status.success() {
            // Organizations query might fail if user has no orgs, which is fine
            return Ok(Vec::new());
        }

        let orgs_output =
            String::from_utf8(output.stdout).context("Invalid UTF-8 in organizations response")?;

        let organizations = orgs_output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|login| GitHubOrganization {
                login: login.trim().to_string(),
                name: None, // We could fetch names separately if needed
            })
            .collect();

        Ok(organizations)
    }

    /// Check if a repository name is available on GitHub for the given owner
    pub async fn check_repository_availability(
        &self,
        owner: &str,
        repo_name: &str,
    ) -> Result<bool> {
        if !is_github_cli_available() {
            // If GitHub CLI is not available, we can't check, so assume it's available
            return Ok(true);
        }

        let output = Command::new("gh")
            .args(&["api", &format!("repos/{}/{}", owner, repo_name)])
            .output()
            .await
            .context("Failed to check repository availability")?;

        // If the repository exists, the command will succeed
        // If it doesn't exist, it will fail with 404
        Ok(!output.status.success())
    }

    /// Create a new local repository with the given name and structure
    pub async fn create_local_repository(
        &self,
        owner: &str,
        repo_name: &str,
        workspace_manager: &mut WorkspaceManager,
    ) -> Result<PathBuf> {
        // Create the repository path structure: workspace_root/owner/repo_name
        let repo_path = self.workspace_root.join(owner).join(repo_name);

        // Check if directory already exists
        if repo_path.exists() {
            anyhow::bail!("Directory already exists: {}", repo_path.display());
        }

        // Create the directory structure
        tokio::fs::create_dir_all(&repo_path)
            .await
            .context("Failed to create repository directory")?;

        display_println!(
            "{} Created directory: {}",
            style("📁").blue(),
            style(repo_path.display()).cyan()
        );

        // Initialize git repository
        self.initialize_git_repository(&repo_path).await?;

        // Apply default template
        self.apply_default_template(&repo_path, repo_name).await?;

        // Create initial commit
        self.create_initial_commit(&repo_path, repo_name).await?;

        // Add to workspace configuration
        let repository_config = Repository {
            name: repo_name.to_string(),
            path: PathBuf::from(owner).join(repo_name),
            url: Some(format!("https://github.com/{}/{}", owner, repo_name)),
            branch: Some("main".to_string()),
            apps: std::collections::HashMap::new(),
        };

        workspace_manager.add_repository(repository_config).await?;

        display_println!(
            "{} Repository '{}' created successfully!",
            style("✅").green().bold(),
            style(repo_name).cyan()
        );

        Ok(repo_path)
    }

    async fn initialize_git_repository(&self, repo_path: &PathBuf) -> Result<()> {
        let output = Command::new("git")
            .args(&["init"])
            .current_dir(repo_path)
            .output()
            .await
            .context("Failed to initialize git repository")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git init failed: {}", error_msg);
        }

        // Set default branch to main
        let _output = Command::new("git")
            .args(&["branch", "-M", "main"])
            .current_dir(repo_path)
            .output()
            .await
            .context("Failed to set default branch")?;

        display_println!("{} Initialized git repository", style("📝").blue());

        Ok(())
    }

    async fn apply_default_template(&self, repo_path: &PathBuf, repo_name: &str) -> Result<()> {
        // Create README.md
        let readme_content = format!(
            "# {}\n\nA new repository created with vibe-workspace.\n\n## Getting Started\n\nThis repository is ready for development. Add your code in the `src/` directory.\n\n## TODO\n\n- [ ] Choose your development framework\n- [ ] Set up your development environment\n- [ ] Add project-specific configuration\n- [ ] Update this README with project details\n",
            repo_name
        );

        tokio::fs::write(repo_path.join("README.md"), readme_content)
            .await
            .context("Failed to create README.md")?;

        // Create basic .gitignore
        let gitignore_content = r#"# OS generated files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# IDE files
.vscode/
.idea/
*.swp
*.swo
*~

# Logs
logs
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Runtime data
pids
*.pid
*.seed
*.pid.lock

# Dependency directories
node_modules/
vendor/

# Build outputs
dist/
build/
target/
*.o
*.so
*.dylib
*.exe

# Environment files
.env
.env.local
.env.development.local
.env.test.local
.env.production.local
"#;

        tokio::fs::write(repo_path.join(".gitignore"), gitignore_content)
            .await
            .context("Failed to create .gitignore")?;

        // Create src directory with placeholder
        let src_dir = repo_path.join("src");
        tokio::fs::create_dir_all(&src_dir)
            .await
            .context("Failed to create src directory")?;

        let main_content = r#"// TODO: Add your main application code here
// This is a placeholder file to get you started

fn main() {
    println!("Hello from your new repository!");
    
    // TODO: Replace this with your actual application logic
}
"#;

        tokio::fs::write(src_dir.join("main.rs"), main_content)
            .await
            .context("Failed to create main.rs")?;

        // Create docs directory with TODO
        let docs_dir = repo_path.join("docs");
        tokio::fs::create_dir_all(&docs_dir)
            .await
            .context("Failed to create docs directory")?;

        let todo_content = r#"# Development Setup TODOs

This file contains setup hooks and next steps for your new repository.

## Framework Setup

Choose and set up your development framework:

### Web Development
- [ ] Initialize npm/yarn project: `npm init` or `yarn init`
- [ ] Install React/Vue/Angular: `npm install react` etc.
- [ ] Set up build tools (Vite, Webpack, etc.)

### Backend Development
- [ ] Initialize project: `cargo init`, `go mod init`, `npm init`, etc.
- [ ] Set up database connections
- [ ] Configure environment variables

### Mobile Development
- [ ] Initialize React Native: `npx react-native init`
- [ ] Set up Flutter: `flutter create`
- [ ] Configure platform-specific settings

### Desktop Development  
- [ ] Set up Electron: `npm install electron`
- [ ] Configure Tauri: `cargo install tauri-cli`
- [ ] Set up native development environment

## Development Environment

- [ ] Configure your preferred development app (already done via vibe!)
- [ ] Set up debugging configuration
- [ ] Configure linting and formatting
- [ ] Set up testing framework
- [ ] Configure CI/CD pipeline

## Next Steps

1. Delete this file once you've completed the setup
2. Update the main README.md with project-specific information
3. Start building your application!

## Deployment

When ready to deploy:
- [ ] Create GitHub repository: `gh repo create`
- [ ] Set up hosting (Vercel, Netlify, Heroku, etc.)
- [ ] Configure domain and SSL
"#;

        tokio::fs::write(docs_dir.join("TODO.md"), todo_content)
            .await
            .context("Failed to create TODO.md")?;

        display_println!("{} Applied default template", style("📄").blue());

        Ok(())
    }

    async fn create_initial_commit(&self, repo_path: &PathBuf, repo_name: &str) -> Result<()> {
        // Add all files
        let output = Command::new("git")
            .args(&["add", "."])
            .current_dir(repo_path)
            .output()
            .await
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git add failed: {}", error_msg);
        }

        // Create initial commit
        let commit_message = format!("Initial commit for {}", repo_name);
        let output = Command::new("git")
            .args(&["commit", "-m", &commit_message])
            .current_dir(repo_path)
            .output()
            .await
            .context("Failed to create initial commit")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git commit failed: {}", error_msg);
        }

        display_println!("{} Created initial commit", style("📝").blue());

        Ok(())
    }

    /// Validate repository name (basic validation)
    pub fn validate_repository_name(&self, name: &str) -> Result<()> {
        if name.is_empty() {
            anyhow::bail!("Repository name cannot be empty");
        }

        if name.len() > 100 {
            anyhow::bail!("Repository name is too long (max 100 characters)");
        }

        // Basic character validation (GitHub allows alphanumeric, hyphens, underscores, periods)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            anyhow::bail!("Repository name contains invalid characters. Use only letters, numbers, hyphens, underscores, and periods.");
        }

        if name.starts_with('.') || name.ends_with('.') {
            anyhow::bail!("Repository name cannot start or end with a period");
        }

        if name.starts_with('-') || name.ends_with('-') {
            anyhow::bail!("Repository name cannot start or end with a hyphen");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_repository_name() {
        let creator = RepositoryCreator::new(PathBuf::from("/tmp"));

        // Valid names
        assert!(creator.validate_repository_name("my-repo").is_ok());
        assert!(creator.validate_repository_name("my_repo").is_ok());
        assert!(creator.validate_repository_name("MyRepo123").is_ok());
        assert!(creator.validate_repository_name("repo.config").is_ok());

        // Invalid names
        assert!(creator.validate_repository_name("").is_err());
        assert!(creator.validate_repository_name(".hidden").is_err());
        assert!(creator.validate_repository_name("repo.").is_err());
        assert!(creator.validate_repository_name("-repo").is_err());
        assert!(creator.validate_repository_name("repo-").is_err());
        assert!(creator
            .validate_repository_name("repo with spaces")
            .is_err());
        assert!(creator.validate_repository_name("repo@invalid").is_err());
    }
}
