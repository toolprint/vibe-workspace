use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::process::Command;

use crate::git::{GitError, Repository, SearchQuery};
use crate::utils::git::is_github_cli_available;

use super::SearchProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCliConfig {
    pub executable_path: Option<PathBuf>,
    pub default_limit: usize,
    pub include_forks: bool,
}

impl Default for GitHubCliConfig {
    fn default() -> Self {
        Self {
            executable_path: None,
            default_limit: 20,
            include_forks: false,
        }
    }
}

pub struct GitHubCliProvider {
    gh_path: PathBuf,
    config: GitHubCliConfig,
}

impl GitHubCliProvider {
    pub fn new() -> Result<Self> {
        Self::with_config(GitHubCliConfig::default())
    }

    pub fn with_config(config: GitHubCliConfig) -> Result<Self> {
        if !is_github_cli_available() {
            return Err(GitError::GitHubCliNotFound.into());
        }

        let gh_path = config
            .executable_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("gh"));

        Ok(Self { gh_path, config })
    }

    /// Get the currently authenticated GitHub username
    pub async fn get_username(&self) -> Result<String> {
        let output = Command::new(&self.gh_path)
            .args(["api", "user", "--jq", ".login"])
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

    /// Get the organizations the authenticated user belongs to
    pub async fn get_user_organizations(&self) -> Result<Vec<String>> {
        let output = Command::new(&self.gh_path)
            .args(["api", "user/orgs", "--jq", ".[].login"])
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
            .map(|login| login.trim().to_string())
            .collect();

        Ok(organizations)
    }

    /// Check if a repository exists for the given owner and name
    pub async fn repository_exists(&self, owner: &str, repo_name: &str) -> Result<bool> {
        let output = Command::new(&self.gh_path)
            .args(["api", &format!("repos/{owner}/{repo_name}")])
            .output()
            .await
            .context("Failed to check repository existence")?;

        // If the repository exists, the command will succeed
        // If it doesn't exist, it will fail with 404
        Ok(output.status.success())
    }

    /// Get all repositories for a specific user
    pub async fn get_user_repositories(&self, username: &str) -> Result<Vec<Repository>> {
        let output = Command::new(&self.gh_path)
            .args([
                "api",
                &format!("users/{}/repos", username),
                "--paginate",
                "--jq",
                r#".[] | {
                    fullName: .full_name,
                    name: .name,
                    description: .description,
                    url: .clone_url,
                    sshUrl: .ssh_url,
                    stargazersCount: .stargazers_count,
                    language: .language,
                    fork: .fork,
                    archived: .archived,
                    topics: .topics
                }"#,
            ])
            .output()
            .await
            .context("Failed to get user repositories")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to get user repositories for '{}': {}",
                username,
                error_msg
            );
        }

        self.parse_repository_list(&output.stdout).await
    }

    /// Get all repositories for an organization
    pub async fn get_organization_repositories(&self, org: &str) -> Result<Vec<Repository>> {
        let output = Command::new(&self.gh_path)
            .args([
                "api",
                &format!("orgs/{}/repos", org),
                "--paginate",
                "--jq",
                r#".[] | {
                    fullName: .full_name,
                    name: .name,
                    description: .description,
                    url: .clone_url,
                    sshUrl: .ssh_url,
                    stargazersCount: .stargazers_count,
                    language: .language,
                    fork: .fork,
                    archived: .archived,
                    topics: .topics
                }"#,
            ])
            .output()
            .await
            .context("Failed to get organization repositories")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to get organization repositories for '{}': {}",
                org,
                error_msg
            );
        }

        self.parse_repository_list(&output.stdout).await
    }

    /// Check if a target exists as either a user or organization
    pub async fn user_or_org_exists(&self, target: &str) -> Result<bool> {
        // Try user first
        let user_check = Command::new(&self.gh_path)
            .args(["api", &format!("users/{}", target)])
            .output()
            .await
            .context("Failed to check user existence")?;

        if user_check.status.success() {
            return Ok(true);
        }

        // Try organization
        let org_check = Command::new(&self.gh_path)
            .args(["api", &format!("orgs/{}", target)])
            .output()
            .await
            .context("Failed to check organization existence")?;

        Ok(org_check.status.success())
    }

    /// Get the target type (user or organization)
    pub async fn get_target_type(
        &self,
        target: &str,
    ) -> Result<crate::git::bulk_clone::TargetType> {
        // Try organization first (more likely to have multiple repos)
        let org_check = Command::new(&self.gh_path)
            .args(["api", &format!("orgs/{}", target)])
            .output()
            .await
            .context("Failed to check organization")?;

        if org_check.status.success() {
            return Ok(crate::git::bulk_clone::TargetType::Organization);
        }

        // Try user
        let user_check = Command::new(&self.gh_path)
            .args(["api", &format!("users/{}", target)])
            .output()
            .await
            .context("Failed to check user")?;

        if user_check.status.success() {
            Ok(crate::git::bulk_clone::TargetType::User)
        } else {
            Ok(crate::git::bulk_clone::TargetType::Unknown)
        }
    }

    /// Count repositories for a user or organization (quick check)
    pub async fn count_repositories(&self, target: &str) -> Result<usize> {
        // Try as organization first
        match self.count_organization_repositories(target).await {
            Ok(count) => Ok(count),
            Err(_) => {
                // Try as user
                self.count_user_repositories(target).await
            }
        }
    }

    /// Count repositories for a user
    async fn count_user_repositories(&self, username: &str) -> Result<usize> {
        let output = Command::new(&self.gh_path)
            .args([
                "api",
                &format!("users/{}/repos", username),
                "--jq",
                "length",
            ])
            .output()
            .await
            .context("Failed to count user repositories")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to count repositories for user '{}': {}",
                username,
                error_msg
            );
        }

        let count_str = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in count response")?
            .trim()
            .to_string();

        count_str
            .parse::<usize>()
            .context("Failed to parse repository count")
    }

    /// Count repositories for an organization
    async fn count_organization_repositories(&self, org: &str) -> Result<usize> {
        let output = Command::new(&self.gh_path)
            .args(["api", &format!("orgs/{}/repos", org), "--jq", "length"])
            .output()
            .await
            .context("Failed to count organization repositories")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Failed to count repositories for org '{}': {}",
                org,
                error_msg
            );
        }

        let count_str = String::from_utf8(output.stdout)
            .context("Invalid UTF-8 in count response")?
            .trim()
            .to_string();

        count_str
            .parse::<usize>()
            .context("Failed to parse repository count")
    }

    async fn parse_repository_list(&self, output: &[u8]) -> Result<Vec<Repository>> {
        #[derive(Deserialize)]
        struct RepoData {
            #[serde(rename = "fullName")]
            full_name: String,
            name: String,
            description: Option<String>,
            url: String,
            #[serde(rename = "sshUrl")]
            ssh_url: String,
            #[serde(rename = "stargazersCount")]
            stars: u32,
            language: Option<String>,
            #[serde(default)]
            fork: bool,
            #[serde(default)]
            archived: bool,
            #[serde(default)]
            topics: Vec<String>,
        }

        let output_str = String::from_utf8_lossy(output);
        if output_str.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut repositories = Vec::new();

        // Parse line by line since GitHub CLI outputs one JSON object per line
        for line in output_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let repo_data: RepoData = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse repository data: {}", line))?;

            let repository = Repository {
                id: repo_data.full_name.clone(),
                name: repo_data.name,
                full_name: repo_data.full_name,
                description: repo_data.description,
                url: repo_data.url,
                ssh_url: repo_data.ssh_url,
                stars: repo_data.stars,
                language: repo_data.language,
                license: None, // Not available in this endpoint
                topics: repo_data.topics,
            };

            repositories.push(repository);
        }

        Ok(repositories)
    }

    async fn parse_search_results(&self, output: &[u8]) -> Result<Vec<Repository>> {
        #[derive(Deserialize)]
        struct SearchResult {
            #[serde(rename = "fullName")]
            full_name: String,
            name: Option<String>,
            description: Option<String>,
            url: String,
            #[serde(rename = "stargazersCount")]
            stars: u32,
            language: Option<String>,
            license: Option<LicenseInfo>,
            #[serde(default)]
            visibility: Option<String>,
        }

        #[derive(Deserialize)]
        struct LicenseInfo {
            key: String,
            #[allow(dead_code)]
            name: Option<String>,
        }

        let results: Vec<SearchResult> = serde_json::from_slice(output).with_context(|| {
            format!(
                "Failed to parse search results. Raw output: {}",
                String::from_utf8_lossy(output)
            )
        })?;

        let mut repositories = Vec::new();

        for result in results {
            let name = result.name.unwrap_or_else(|| {
                result
                    .full_name
                    .split('/')
                    .next_back()
                    .unwrap_or(&result.full_name)
                    .to_string()
            });

            let repo =
                Repository {
                    id: result.full_name.clone(),
                    name,
                    full_name: result.full_name.clone(),
                    description: result.description,
                    url: result.url,
                    ssh_url: format!("git@github.com:{}.git", result.full_name),
                    stars: result.stars,
                    language: result.language,
                    license: result.license.and_then(|l| {
                        if l.key.is_empty() {
                            None
                        } else {
                            Some(l.key)
                        }
                    }),
                    topics: vec![], // Topics not returned in search results
                };

            repositories.push(repo);
        }

        Ok(repositories)
    }

    async fn get_repo_details(&self, repo_name: &str) -> Result<Repository> {
        let output = Command::new(&self.gh_path)
            .args([
                "repo",
                "view",
                repo_name,
                "--json",
                "name,description,url,sshUrl,stargazerCount,primaryLanguage,repositoryTopics",
            ])
            .output()
            .await
            .context("Failed to execute gh repo view")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get repository details: {}", error_msg);
        }

        #[derive(Deserialize)]
        struct RepoDetails {
            name: String,
            description: Option<String>,
            url: String,
            #[serde(rename = "sshUrl")]
            ssh_url: String,
            #[serde(rename = "stargazerCount", default)]
            stars: u32,
            #[serde(rename = "primaryLanguage")]
            primary_language: Option<LanguageInfo>,
            #[serde(rename = "repositoryTopics", default)]
            topics: Vec<String>,
        }

        #[derive(Deserialize)]
        struct LanguageInfo {
            name: String,
        }

        let details: RepoDetails =
            serde_json::from_slice(&output.stdout).context("Failed to parse repository details")?;

        Ok(Repository {
            id: repo_name.to_string(),
            name: details.name,
            full_name: repo_name.to_string(),
            description: details.description,
            url: details.url,
            ssh_url: details.ssh_url,
            stars: details.stars,
            language: details.primary_language.map(|l| l.name),
            license: None, // License not available in repo view, only in search
            topics: details.topics,
        })
    }
}

#[async_trait]
impl SearchProvider for GitHubCliProvider {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Repository>> {
        let mut cmd = Command::new(&self.gh_path);
        cmd.args(["search", "repos"]);

        // Build search string
        let mut search_parts = query.keywords.clone();

        if let Some(org) = &query.organization {
            search_parts.push(format!("org:{org}"));
        }

        if let Some(lang) = &query.language {
            search_parts.push(format!("language:{lang}"));
        }

        for tag in &query.tags {
            search_parts.push(format!("topic:{tag}"));
        }

        if !self.config.include_forks {
            search_parts.push("fork:false".to_string());
        }

        let search_string = search_parts.join(" ");
        cmd.arg(&search_string);

        // Add sort method (only if not best-match, which is the default)
        if query.sort != crate::git::SortMethod::BestMatch {
            cmd.args(["--sort", query.sort.as_str()]);
        }

        // Add limit and request JSON output
        let limit = query.limit.unwrap_or(self.config.default_limit);
        cmd.args(["--limit", &limit.to_string()]);
        cmd.args([
            "--json",
            "fullName,name,description,url,stargazersCount,language,license",
        ]);

        let output = cmd
            .output()
            .await
            .context("Failed to execute gh search repos")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            if error_msg.contains("No repositories matched") {
                return Err(GitError::NoSearchResults {
                    query: search_string,
                }
                .into());
            }
            anyhow::bail!("GitHub CLI search failed: {}", error_msg);
        }

        self.parse_search_results(&output.stdout).await
    }

    async fn get_repository(&self, id: &str) -> Result<Repository> {
        self.get_repo_details(id).await
    }

    fn name(&self) -> &str {
        "github_cli"
    }
}
