use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_rusqlite::{params, Connection};

/// Cached git status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGitStatus {
    pub repository_name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub clean: bool,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub remote_url: Option<String>,
    pub last_updated: DateTime<Utc>,
}

impl From<crate::workspace::operations::GitStatus> for CachedGitStatus {
    fn from(status: crate::workspace::operations::GitStatus) -> Self {
        Self {
            repository_name: status.repository_name,
            path: PathBuf::from(status.path),
            branch: status.branch,
            clean: status.clean,
            ahead: status.ahead,
            behind: status.behind,
            staged: status.staged,
            unstaged: status.unstaged,
            untracked: status.untracked,
            remote_url: status.remote_url,
            last_updated: Utc::now(),
        }
    }
}

impl From<CachedGitStatus> for crate::workspace::operations::GitStatus {
    fn from(cached: CachedGitStatus) -> Self {
        Self {
            repository_name: cached.repository_name,
            path: cached.path.to_string_lossy().to_string(),
            branch: cached.branch,
            clean: cached.clean,
            ahead: cached.ahead,
            behind: cached.behind,
            staged: cached.staged,
            unstaged: cached.unstaged,
            untracked: cached.untracked,
            remote_url: cached.remote_url,
        }
    }
}

/// Fast SQLite-based cache for git status information
pub struct GitStatusCache {
    db_path: PathBuf,
    /// Cache TTL in minutes - how long cached git status is considered valid
    cache_ttl_minutes: i64,
}

impl GitStatusCache {
    /// Create a new git status cache
    pub fn new<P: Into<PathBuf>>(db_path: P) -> Self {
        Self {
            db_path: db_path.into(),
            cache_ttl_minutes: 5, // Default: 5 minutes
        }
    }

    /// Create a git status cache with custom TTL
    #[allow(dead_code)]
    pub fn with_ttl<P: Into<PathBuf>>(db_path: P, ttl_minutes: i64) -> Self {
        Self {
            db_path: db_path.into(),
            cache_ttl_minutes: ttl_minutes,
        }
    }

    /// Initialize the cache database with required tables
    pub async fn initialize(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .await
            .context("Failed to open git status cache database")?;

        conn.call(move |conn| {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS git_status (
                    repository_name TEXT PRIMARY KEY,
                    path TEXT NOT NULL,
                    branch TEXT,
                    clean INTEGER NOT NULL,
                    ahead INTEGER NOT NULL,
                    behind INTEGER NOT NULL,
                    staged INTEGER NOT NULL,
                    unstaged INTEGER NOT NULL,
                    untracked INTEGER NOT NULL,
                    remote_url TEXT,
                    last_updated TEXT NOT NULL
                )
                "#,
                [],
            )?;

            // Create indexes for faster lookups
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_git_status_path ON git_status(path)",
                [],
            )?;

            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_git_status_last_updated ON git_status(last_updated)",
                [],
            )?;

            Ok(())
        })
        .await
        .context("Failed to initialize git status cache tables")?;

        Ok(())
    }

    /// Cache git status information
    #[allow(dead_code)]
    pub async fn cache_git_status(&self, status: &CachedGitStatus) -> Result<()> {
        let conn = Connection::open(&self.db_path).await?;
        let status = status.clone();

        conn.call(move |conn| {
            let last_updated = status.last_updated.to_rfc3339();

            conn.execute(
                r#"
                INSERT OR REPLACE INTO git_status 
                (repository_name, path, branch, clean, ahead, behind, staged, unstaged, untracked, remote_url, last_updated)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    status.repository_name,
                    status.path.to_string_lossy(),
                    status.branch,
                    status.clean as i32,
                    status.ahead as i64,
                    status.behind as i64,
                    status.staged as i64,
                    status.unstaged as i64,
                    status.untracked as i64,
                    status.remote_url,
                    last_updated
                ],
            )?;

            Ok(())
        })
        .await
        .context("Failed to cache git status")?;

        Ok(())
    }

    /// Get cached git status if it's still valid (within TTL)
    #[allow(dead_code)]
    pub async fn get_git_status(&self, repository_name: &str) -> Result<Option<CachedGitStatus>> {
        let conn = Connection::open(&self.db_path).await?;
        let repo_name = repository_name.to_string();
        let ttl_minutes = self.cache_ttl_minutes;

        let result = conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT repository_name, path, branch, clean, ahead, behind, staged, unstaged, untracked, remote_url, last_updated 
                    FROM git_status 
                    WHERE repository_name = ?1
                    "#
                )?;

                let status = stmt.query_row(params![repo_name], |row| {
                    let last_updated_str: String = row.get(10)?;
                    let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e)))?
                        .with_timezone(&Utc);

                    Ok(CachedGitStatus {
                        repository_name: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        branch: row.get(2)?,
                        clean: row.get::<_, i32>(3)? != 0,
                        ahead: row.get::<_, i64>(4)? as usize,
                        behind: row.get::<_, i64>(5)? as usize,
                        staged: row.get::<_, i64>(6)? as usize,
                        unstaged: row.get::<_, i64>(7)? as usize,
                        untracked: row.get::<_, i64>(8)? as usize,
                        remote_url: row.get(9)?,
                        last_updated,
                    })
                });

                match status {
                    Ok(status) => {
                        // Check if the cached status is still valid (within TTL)
                        let now = Utc::now();
                        let age = now.signed_duration_since(status.last_updated);
                        if age <= Duration::minutes(ttl_minutes) {
                            Ok(Some(status))
                        } else {
                            // Cached data is too old
                            Ok(None)
                        }
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(tokio_rusqlite::Error::Rusqlite(e)),
                }
            })
            .await
            .context("Failed to get cached git status")?;

        Ok(result)
    }

    /// Get all cached git statuses (for batch operations)
    pub async fn get_all_git_statuses(&self) -> Result<Vec<CachedGitStatus>> {
        let conn = Connection::open(&self.db_path).await?;
        let ttl_minutes = self.cache_ttl_minutes;

        let statuses = conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT repository_name, path, branch, clean, ahead, behind, staged, unstaged, untracked, remote_url, last_updated 
                    FROM git_status
                    ORDER BY repository_name
                    "#
                )?;

                let status_iter = stmt.query_map([], |row| {
                    let last_updated_str: String = row.get(10)?;
                    let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e)))?
                        .with_timezone(&Utc);

                    Ok(CachedGitStatus {
                        repository_name: row.get(0)?,
                        path: PathBuf::from(row.get::<_, String>(1)?),
                        branch: row.get(2)?,
                        clean: row.get::<_, i32>(3)? != 0,
                        ahead: row.get::<_, i64>(4)? as usize,
                        behind: row.get::<_, i64>(5)? as usize,
                        staged: row.get::<_, i64>(6)? as usize,
                        unstaged: row.get::<_, i64>(7)? as usize,
                        untracked: row.get::<_, i64>(8)? as usize,
                        remote_url: row.get(9)?,
                        last_updated,
                    })
                })?;

                let mut statuses = Vec::new();
                let now = Utc::now();
                for status_result in status_iter {
                    let status = status_result?;
                    // Only include valid (within TTL) cached statuses
                    let age = now.signed_duration_since(status.last_updated);
                    if age <= Duration::minutes(ttl_minutes) {
                        statuses.push(status);
                    }
                }

                Ok(statuses)
            })
            .await
            .context("Failed to get all cached git statuses")?;

        Ok(statuses)
    }

    /// Remove expired cache entries
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let conn = Connection::open(&self.db_path).await?;
        let ttl_minutes = self.cache_ttl_minutes;

        let deleted_count = conn
            .call(move |conn| {
                let cutoff_time = Utc::now() - Duration::minutes(ttl_minutes);
                let cutoff_str = cutoff_time.to_rfc3339();

                let result = conn.execute(
                    "DELETE FROM git_status WHERE last_updated < ?1",
                    params![cutoff_str],
                )?;

                Ok(result)
            })
            .await
            .context("Failed to cleanup expired git status cache entries")?;

        Ok(deleted_count)
    }

    /// Invalidate cache for a specific repository (useful when changes are detected)
    pub async fn invalidate_repository(&self, repository_name: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path).await?;
        let repo_name = repository_name.to_string();

        conn.call(move |conn| {
            conn.execute(
                "DELETE FROM git_status WHERE repository_name = ?1",
                params![repo_name],
            )?;
            Ok(())
        })
        .await
        .context("Failed to invalidate repository cache")?;

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<GitCacheStats> {
        let conn = Connection::open(&self.db_path).await?;
        let ttl_minutes = self.cache_ttl_minutes;

        let stats = conn
            .call(move |conn| {
                let total_entries: i64 =
                    conn.query_row("SELECT COUNT(*) FROM git_status", [], |row| row.get(0))?;

                let cutoff_time = Utc::now() - Duration::minutes(ttl_minutes);
                let cutoff_str = cutoff_time.to_rfc3339();

                let valid_entries: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM git_status WHERE last_updated >= ?1",
                    params![cutoff_str],
                    |row| row.get(0),
                )?;

                let expired_entries = total_entries - valid_entries;

                Ok::<_, tokio_rusqlite::Error>(GitCacheStats {
                    total_entries: total_entries as usize,
                    valid_entries: valid_entries as usize,
                    expired_entries: expired_entries as usize,
                    ttl_minutes: ttl_minutes as usize,
                })
            })
            .await
            .context("Failed to get git cache statistics")?;

        Ok(stats)
    }
}

/// Git cache statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct GitCacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
    pub ttl_minutes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_git_status_cache_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_git_status.db");

        let cache = GitStatusCache::new(db_path);
        cache.initialize().await.unwrap();

        // Test caching git status
        let status = CachedGitStatus {
            repository_name: "test-repo".to_string(),
            path: PathBuf::from("/path/to/repo"),
            branch: Some("main".to_string()),
            clean: false,
            ahead: 2,
            behind: 1,
            staged: 3,
            unstaged: 1,
            untracked: 2,
            remote_url: Some("https://github.com/user/repo.git".to_string()),
            last_updated: Utc::now(),
        };

        cache.cache_git_status(&status).await.unwrap();

        // Test retrieving the git status
        let cached = cache.get_git_status("test-repo").await.unwrap();
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.repository_name, "test-repo");
        assert_eq!(cached.branch, Some("main".to_string()));
        assert_eq!(cached.ahead, 2);
        assert_eq!(cached.behind, 1);
        assert!(!cached.clean);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_ttl.db");

        // Create cache with very short TTL for testing
        let cache = GitStatusCache::with_ttl(db_path, 0); // 0 minutes TTL
        cache.initialize().await.unwrap();

        let status = CachedGitStatus {
            repository_name: "test-repo".to_string(),
            path: PathBuf::from("/path/to/repo"),
            branch: Some("main".to_string()),
            clean: true,
            ahead: 0,
            behind: 0,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            remote_url: None,
            last_updated: Utc::now() - Duration::minutes(1), // 1 minute ago
        };

        cache.cache_git_status(&status).await.unwrap();

        // Should return None because the cache entry is expired
        let cached = cache.get_git_status("test-repo").await.unwrap();
        assert!(cached.is_none());
    }
}
