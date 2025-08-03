use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod clone;
pub mod provider;
pub mod search;

pub use clone::CloneCommand;
pub use search::SearchCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub full_name: String, // org/repo
    pub description: Option<String>,
    pub url: String,
    pub ssh_url: String,
    pub stars: u32,
    pub language: Option<String>,
    pub license: Option<String>, // License key (e.g., "mit", "apache-2.0")
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMethod {
    #[default]
    BestMatch,
    Stars,
    Forks,
    Updated,
}

impl SortMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortMethod::BestMatch => "best-match",
            SortMethod::Stars => "stars",
            SortMethod::Forks => "forks",
            SortMethod::Updated => "updated",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SortMethod::BestMatch => "Best Match",
            SortMethod::Stars => "Most Stars",
            SortMethod::Forks => "Most Forks",
            SortMethod::Updated => "Recently Updated",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub organization: Option<String>,
    pub limit: Option<usize>,
    pub sort: SortMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub default_clone_location: PathBuf,
    pub standardize_paths: bool,
    pub auto_install_dependencies: bool,
    pub search_providers: Vec<String>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            default_clone_location: dirs::home_dir().unwrap_or_default().join("Workspace"),
            standardize_paths: true,
            auto_install_dependencies: false,
            search_providers: vec!["github_cli".to_string()],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository already exists at {path}")]
    RepositoryExists { path: PathBuf },

    #[error("Invalid Git URL: {url}")]
    InvalidUrl { url: String },

    #[error("GitHub CLI not found. Please install 'gh' command.")]
    GitHubCliNotFound,

    #[error("Search returned no results for query: {query}")]
    NoSearchResults { query: String },

    #[error("Clone failed: {message}")]
    CloneFailed { message: String },

    #[error("Search provider error: {provider}")]
    ProviderError {
        provider: String,
        source: anyhow::Error,
    },
}
