pub mod git_status_cache;
pub mod repository_cache;

pub use git_status_cache::GitStatusCache;
pub use repository_cache::RepositoryCache;

use anyhow::Result;
use std::path::Path;

/// Initialize the cache system
pub async fn initialize_cache<P: AsRef<Path>>(cache_dir: P) -> Result<()> {
    let cache_dir = cache_dir.as_ref();

    // Ensure cache directory exists
    if !cache_dir.exists() {
        tokio::fs::create_dir_all(cache_dir).await?;
    }

    // Initialize repository cache
    let repo_cache = RepositoryCache::new(cache_dir.join("repositories.db"));
    repo_cache.initialize().await?;

    // Initialize git status cache
    let git_cache = GitStatusCache::new(cache_dir.join("git_status.db"));
    git_cache.initialize().await?;

    Ok(())
}
