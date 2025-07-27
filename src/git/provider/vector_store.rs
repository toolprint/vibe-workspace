// Future implementation for local semantic vector store
// This is a placeholder for future development

use anyhow::Result;
use async_trait::async_trait;

use crate::git::{Repository, SearchQuery};

use super::SearchProvider;

pub struct VectorStoreProvider;

#[async_trait]
impl SearchProvider for VectorStoreProvider {
    async fn search(&self, _query: &SearchQuery) -> Result<Vec<Repository>> {
        // Future implementation
        anyhow::bail!("Vector store search not yet implemented")
    }

    async fn get_repository(&self, _id: &str) -> Result<Repository> {
        // Future implementation
        anyhow::bail!("Vector store repository lookup not yet implemented")
    }

    fn name(&self) -> &str {
        "vector_store"
    }
}
