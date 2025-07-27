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
                    .last()
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
            .args(&[
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
        cmd.args(&["search", "repos"]);

        // Build search string
        let mut search_parts = query.keywords.clone();

        if let Some(org) = &query.organization {
            search_parts.push(format!("org:{}", org));
        }

        if let Some(lang) = &query.language {
            search_parts.push(format!("language:{}", lang));
        }

        for tag in &query.tags {
            search_parts.push(format!("topic:{}", tag));
        }

        if !self.config.include_forks {
            search_parts.push("fork:false".to_string());
        }

        let search_string = search_parts.join(" ");
        cmd.arg(&search_string);

        // Add sort method (only if not best-match, which is the default)
        if query.sort != crate::git::SortMethod::BestMatch {
            cmd.args(&["--sort", query.sort.as_str()]);
        }

        // Add limit and request JSON output
        let limit = query.limit.unwrap_or(self.config.default_limit);
        cmd.args(&["--limit", &limit.to_string()]);
        cmd.args(&[
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
