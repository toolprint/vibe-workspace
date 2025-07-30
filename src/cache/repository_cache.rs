use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_rusqlite::{params, Connection};

/// Cached repository information for fast lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRepository {
    pub name: String,
    pub path: PathBuf,
    pub configured_apps: Vec<String>,
    pub last_updated: DateTime<Utc>,
    pub path_exists: bool,
    pub is_git_repo: bool,
}

/// Fast SQLite-based cache for repository metadata
pub struct RepositoryCache {
    db_path: PathBuf,
}

impl RepositoryCache {
    /// Create a new repository cache
    pub fn new<P: Into<PathBuf>>(db_path: P) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    /// Initialize the cache database with required tables
    pub async fn initialize(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .await
            .context("Failed to open repository cache database")?;

        conn.call(move |conn| {
            // Check if we need to migrate from old schema
            let has_old_schema = conn
                .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='repositories'")
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        let sql: String = row.get(0)?;
                        Ok(sql.contains("exists INTEGER"))
                    })
                })
                .unwrap_or(false);

            if has_old_schema {
                // Drop old table and recreate with new schema
                conn.execute("DROP TABLE IF EXISTS repositories", [])?;
            }

            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS repositories (
                    name TEXT PRIMARY KEY,
                    path TEXT NOT NULL,
                    configured_apps TEXT NOT NULL, -- JSON array of app names
                    last_updated TEXT NOT NULL,    -- ISO 8601 datetime
                    path_exists INTEGER NOT NULL,       -- boolean as integer
                    is_git_repo INTEGER NOT NULL   -- boolean as integer
                )
                "#,
                [],
            )?;

            // Create index for faster lookups
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_repositories_path ON repositories(path)",
                [],
            )?;

            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_repositories_last_updated ON repositories(last_updated)",
                [],
            )?;

            Ok(())
        })
        .await
        .context("Failed to initialize repository cache tables")?;

        Ok(())
    }

    /// Cache repository information
    pub async fn cache_repository(&self, repo: &CachedRepository) -> Result<()> {
        let conn = Connection::open(&self.db_path).await?;
        let repo = repo.clone();

        conn.call(move |conn| {
            let apps_json = serde_json::to_string(&repo.configured_apps)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let last_updated = repo.last_updated.to_rfc3339();

            conn.execute(
                r#"
                INSERT OR REPLACE INTO repositories 
                (name, path, configured_apps, last_updated, path_exists, is_git_repo)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    repo.name,
                    repo.path.to_string_lossy(),
                    apps_json,
                    last_updated,
                    repo.path_exists as i32,
                    repo.is_git_repo as i32
                ],
            )?;

            Ok(())
        })
        .await
        .context("Failed to cache repository information")?;

        Ok(())
    }

    /// Get cached repository by name
    pub async fn get_repository(&self, name: &str) -> Result<Option<CachedRepository>> {
        let conn = Connection::open(&self.db_path).await?;
        let name = name.to_string();

        let result = conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT name, path, configured_apps, last_updated, path_exists, is_git_repo 
                     FROM repositories WHERE name = ?1",
                )?;

                let repo = stmt.query_row(params![name], |row| {
                    let apps_json: String = row.get(2)?;
                    let configured_apps: Vec<String> =
                        serde_json::from_str(&apps_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let last_updated_str: String = row.get(3)?;
                    let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                3,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .with_timezone(&Utc);

                    Ok(CachedRepository {
                        name: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        configured_apps,
                        last_updated,
                        path_exists: row.get::<_, i32>(4)? != 0,
                        is_git_repo: row.get::<_, i32>(5)? != 0,
                    })
                });

                match repo {
                    Ok(repo) => Ok(Some(repo)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(tokio_rusqlite::Error::Rusqlite(e)),
                }
            })
            .await
            .context("Failed to get cached repository")?;

        Ok(result)
    }

    /// Get all repositories with configured apps (for launch UI)
    pub async fn get_repositories_with_apps(&self) -> Result<Vec<CachedRepository>> {
        let conn = Connection::open(&self.db_path).await?;

        let repositories = conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT name, path, configured_apps, last_updated, path_exists, is_git_repo 
                    FROM repositories 
                    WHERE json_array_length(configured_apps) > 0 
                    AND path_exists = 1
                    ORDER BY name
                    "#,
                )?;

                let repo_iter = stmt.query_map([], |row| {
                    let apps_json: String = row.get(2)?;
                    let configured_apps: Vec<String> =
                        serde_json::from_str(&apps_json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                2,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let last_updated_str: String = row.get(3)?;
                    let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                3,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .with_timezone(&Utc);

                    Ok(CachedRepository {
                        name: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        configured_apps,
                        last_updated,
                        path_exists: row.get::<_, i32>(4)? != 0,
                        is_git_repo: row.get::<_, i32>(5)? != 0,
                    })
                })?;

                let mut repositories = Vec::new();
                for repo in repo_iter {
                    repositories.push(repo?);
                }

                Ok(repositories)
            })
            .await
            .context("Failed to get repositories with apps")?;

        Ok(repositories)
    }

    /// Update cache for all repositories in workspace config
    pub async fn refresh_from_config(
        &self,
        repositories: &[crate::workspace::Repository],
        workspace_root: &std::path::Path,
    ) -> Result<()> {
        for repo in repositories {
            let full_path = workspace_root.join(&repo.path);
            let cached_repo = CachedRepository {
                name: repo.name.clone(),
                path: repo.path.clone(),
                configured_apps: repo.apps.keys().cloned().collect(),
                last_updated: Utc::now(),
                path_exists: full_path.exists(),
                is_git_repo: full_path.join(".git").exists(),
            };

            self.cache_repository(&cached_repo).await?;
        }

        Ok(())
    }

    /// Remove repositories that no longer exist in config
    pub async fn cleanup_stale_entries(&self, current_repo_names: &[String]) -> Result<()> {
        let conn = Connection::open(&self.db_path).await?;
        let current_names = current_repo_names.to_vec();

        conn.call(move |conn| {
            // Create a temporary table with current repo names
            conn.execute("CREATE TEMP TABLE current_repos (name TEXT)", [])?;

            let mut stmt = conn.prepare("INSERT INTO current_repos (name) VALUES (?1)")?;
            for name in current_names {
                stmt.execute(params![name])?;
            }

            // Delete repositories not in current config
            conn.execute(
                "DELETE FROM repositories WHERE name NOT IN (SELECT name FROM current_repos)",
                [],
            )?;

            Ok(())
        })
        .await
        .context("Failed to cleanup stale cache entries")?;

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let conn = Connection::open(&self.db_path).await?;

        let stats = conn
            .call(move |conn| {
                let total_repos: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM repositories",
                    [],
                    |row| row.get(0)
                )?;

                let repos_with_apps: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM repositories WHERE json_array_length(configured_apps) > 0",
                    [],
                    |row| row.get(0)
                )?;

                let existing_repos: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM repositories WHERE path_exists = 1",
                    [],
                    |row| row.get(0)
                )?;

                Ok(CacheStats {
                    total_repositories: total_repos as usize,
                    repositories_with_apps: repos_with_apps as usize,
                    existing_repositories: existing_repos as usize,
                })
            })
            .await
            .context("Failed to get cache statistics")?;

        Ok(stats)
    }
}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_repositories: usize,
    pub repositories_with_apps: usize,
    pub existing_repositories: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_repository_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_repos.db");

        let cache = RepositoryCache::new(db_path);
        cache.initialize().await.unwrap();

        // Test caching a repository
        let repo = CachedRepository {
            name: "test-repo".to_string(),
            path: PathBuf::from("/path/to/repo"),
            configured_apps: vec!["vscode".to_string(), "warp".to_string()],
            last_updated: Utc::now(),
            path_exists: true,
            is_git_repo: true,
        };

        cache.cache_repository(&repo).await.unwrap();

        // Test retrieving the repository
        let cached = cache.get_repository("test-repo").await.unwrap();
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.name, "test-repo");
        assert_eq!(cached.configured_apps, vec!["vscode", "warp"]);

        // Test getting repositories with apps
        let repos_with_apps = cache.get_repositories_with_apps().await.unwrap();
        assert_eq!(repos_with_apps.len(), 1);
        assert_eq!(repos_with_apps[0].name, "test-repo");
    }
}
