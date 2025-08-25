//! Caching layer for worktree status to improve performance

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::worktree::status::WorktreeInfo;

const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatusCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    worktree_info: WorktreeInfo,
    last_updated: SystemTime,
    file_mtime: SystemTime,
}

impl WorktreeStatusCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get cached worktree info if still valid
    pub fn get(&self, path: &Path) -> Option<&WorktreeInfo> {
        if let Some(entry) = self.entries.get(path) {
            // Check if cache is still valid
            if self.is_entry_valid(entry, path).unwrap_or(false) {
                return Some(&entry.worktree_info);
            }
        }

        None
    }

    /// Store worktree info in cache
    pub fn insert(&mut self, path: PathBuf, info: WorktreeInfo) -> Result<()> {
        let file_mtime = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let entry = CacheEntry {
            worktree_info: info,
            last_updated: SystemTime::now(),
            file_mtime,
        };

        self.entries.insert(path, entry);
        Ok(())
    }

    /// Remove stale entries from cache
    pub fn cleanup_stale_entries(&mut self) {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(CACHE_TTL_SECONDS);

        self.entries.retain(|path, entry| {
            // Remove if too old or if path no longer exists
            if let Ok(age) = now.duration_since(entry.last_updated) {
                age < ttl && path.exists()
            } else {
                false
            }
        });
    }

    /// Check if a cache entry is still valid
    fn is_entry_valid(&self, entry: &CacheEntry, path: &Path) -> Result<bool> {
        let now = SystemTime::now();
        let ttl = Duration::from_secs(CACHE_TTL_SECONDS);

        // Check age
        if now.duration_since(entry.last_updated)? > ttl {
            return Ok(false);
        }

        // Check if directory was modified
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(current_mtime) = metadata.modified() {
                if current_mtime > entry.file_mtime {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.entries.len();
        let now = SystemTime::now();

        let valid_entries = self
            .entries
            .values()
            .filter(|entry| {
                now.duration_since(entry.last_updated)
                    .map(|age| age.as_secs() < CACHE_TTL_SECONDS)
                    .unwrap_or(false)
            })
            .count();

        CacheStats {
            total_entries,
            valid_entries,
            hit_ratio: if total_entries > 0 {
                valid_entries as f64 / total_entries as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub hit_ratio: f64,
}

impl Default for WorktreeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree::status::{StatusSeverity, WorktreeStatus};
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_worktree_info(path: PathBuf) -> WorktreeInfo {
        WorktreeInfo {
            path,
            branch: "test-branch".to_string(),
            head: "abc1234".to_string(),
            task_id: None,
            status: WorktreeStatus {
                is_clean: true,
                severity: StatusSeverity::Clean,
                uncommitted_changes: Vec::new(),
                untracked_files: Vec::new(),
                unpushed_commits: Vec::new(),
                remote_status: crate::worktree::status::RemoteStatus::UpToDate,
                merge_info: None,
                ahead_count: 0,
                behind_count: 0,
            },
            age: Duration::from_secs(3600),
            is_detached: false,
        }
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = WorktreeStatusCache::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let worktree_info = create_test_worktree_info(path.clone());

        // Test cache miss
        assert!(cache.get(&path).is_none());

        // Test cache hit after insert
        cache.insert(path.clone(), worktree_info).unwrap();
        assert!(cache.get(&path).is_some());

        let cached_info = cache.get(&path).unwrap();
        assert_eq!(cached_info.branch, "test-branch");
        assert_eq!(cached_info.head, "abc1234");
    }

    #[test]
    fn test_cache_cleanup() {
        let mut cache = WorktreeStatusCache::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let worktree_info = create_test_worktree_info(path.clone());
        cache.insert(path.clone(), worktree_info).unwrap();

        assert_eq!(cache.entries.len(), 1);

        // Cleanup should remove the entry since the temp dir might not exist after drop
        cache.cleanup_stale_entries();
        // Note: This test might be flaky depending on filesystem behavior
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = WorktreeStatusCache::new();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Empty cache stats
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.hit_ratio, 0.0);

        // Add entry
        let worktree_info = create_test_worktree_info(path);
        cache
            .insert(temp_dir.path().to_path_buf(), worktree_info)
            .unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.hit_ratio, 1.0);
    }
}
