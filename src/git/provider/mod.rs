use anyhow::Result;
use async_trait::async_trait;

pub mod github_cli;
pub mod vector_store;

pub use github_cli::GitHubCliProvider;

use super::{Repository, SearchQuery};

#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Repository>>;
    async fn get_repository(&self, id: &str) -> Result<Repository>;
    fn name(&self) -> &str;
}

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create_provider(name: &str) -> Result<Box<dyn SearchProvider>> {
        match name {
            "github_cli" => Ok(Box::new(GitHubCliProvider::new()?)),
            _ => anyhow::bail!("Unknown search provider: {}", name),
        }
    }
}
