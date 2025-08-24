//! Advanced merge detection algorithms for worktree branches
//!
//! This module implements multiple strategies for detecting when a branch
//! has been merged into main, including regular merges, squash merges,
//! and rebase merges that are difficult to detect with standard git commands.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::warn;

use crate::worktree::config::WorktreeMergeDetectionConfig;
use crate::worktree::status::MergeInfo;

/// Different methods available for merge detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeDetectionMethod {
    /// Standard git merge detection using `git branch --merged`
    Standard,
    /// Detect squash merges by analyzing commit content
    Squash,
    /// Use GitHub CLI to check PR merge status
    GitHubPR,
    /// Compare file contents between branch and main
    FileContent,
}

impl MergeDetectionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeDetectionMethod::Standard => "standard",
            MergeDetectionMethod::Squash => "squash",
            MergeDetectionMethod::GitHubPR => "github_pr", 
            MergeDetectionMethod::FileContent => "file_content",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "standard" => Some(MergeDetectionMethod::Standard),
            "squash" => Some(MergeDetectionMethod::Squash),
            "github_pr" => Some(MergeDetectionMethod::GitHubPR),
            "file_content" => Some(MergeDetectionMethod::FileContent),
            _ => None,
        }
    }
}

/// Result of merge detection analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDetectionResult {
    /// Whether the branch appears to be merged
    pub is_merged: bool,
    
    /// Method that detected the merge (or was most confident)
    pub detection_method: String,
    
    /// Confidence score from 0.0 (no confidence) to 1.0 (certain)
    pub confidence: f32,
    
    /// Additional details about the detection
    pub details: Option<String>,
    
    /// Results from all attempted methods
    pub method_results: Vec<MethodResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodResult {
    pub method: String,
    pub is_merged: bool,
    pub confidence: f32,
    pub details: Option<String>,
    pub error: Option<String>,
}

/// Comprehensive merge detection engine
pub struct MergeDetector {
    config: WorktreeMergeDetectionConfig,
}

impl MergeDetector {
    pub fn new(config: WorktreeMergeDetectionConfig) -> Self {
        Self { config }
    }
    
    /// Detect if a branch has been merged using all configured methods
    pub async fn detect_merge(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<MergeDetectionResult> {
        let mut method_results = Vec::new();
        
        // Try each configured method in order of preference
        for method_name in &self.config.methods {
            if let Some(method) = MergeDetectionMethod::from_str(method_name) {
                let result = self.try_detection_method(&method, worktree_path, branch_name).await;
                method_results.push(result);
            } else {
                warn!("Unknown merge detection method: {}", method_name);
            }
        }
        
        // Analyze results to determine overall merge status
        self.analyze_method_results(method_results)
    }
    
    /// Try a specific detection method
    async fn try_detection_method(
        &self,
        method: &MergeDetectionMethod,
        worktree_path: &Path,
        branch_name: &str,
    ) -> MethodResult {
        let method_name = method.as_str().to_string();
        
        match method {
            MergeDetectionMethod::Standard => {
                match self.detect_standard_merge(worktree_path, branch_name).await {
                    Ok((is_merged, details)) => MethodResult {
                        method: method_name,
                        is_merged,
                        confidence: if is_merged { 0.95 } else { 0.8 },
                        details,
                        error: None,
                    },
                    Err(e) => MethodResult {
                        method: method_name,
                        is_merged: false,
                        confidence: 0.0,
                        details: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            
            MergeDetectionMethod::Squash => {
                match self.detect_squash_merge(worktree_path, branch_name).await {
                    Ok((is_merged, confidence, details)) => MethodResult {
                        method: method_name,
                        is_merged,
                        confidence,
                        details,
                        error: None,
                    },
                    Err(e) => MethodResult {
                        method: method_name,
                        is_merged: false,
                        confidence: 0.0,
                        details: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            
            MergeDetectionMethod::GitHubPR => {
                if !self.config.use_github_cli {
                    return MethodResult {
                        method: method_name,
                        is_merged: false,
                        confidence: 0.0,
                        details: Some("GitHub CLI integration disabled".to_string()),
                        error: None,
                    };
                }
                
                match self.detect_github_pr_merge(worktree_path, branch_name).await {
                    Ok((is_merged, details)) => MethodResult {
                        method: method_name,
                        is_merged,
                        confidence: if is_merged { 0.9 } else { 0.0 },
                        details,
                        error: None,
                    },
                    Err(e) => MethodResult {
                        method: method_name,
                        is_merged: false,
                        confidence: 0.0,
                        details: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            
            MergeDetectionMethod::FileContent => {
                match self.detect_file_content_merge(worktree_path, branch_name).await {
                    Ok((is_merged, confidence, details)) => MethodResult {
                        method: method_name,
                        is_merged,
                        confidence,
                        details,
                        error: None,
                    },
                    Err(e) => MethodResult {
                        method: method_name,
                        is_merged: false,
                        confidence: 0.0,
                        details: None,
                        error: Some(e.to_string()),
                    },
                }
            }
        }
    }
    
    /// Standard git merge detection
    async fn detect_standard_merge(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<(bool, Option<String>)> {
        // Try each main branch
        for main_branch in &self.config.main_branches {
            let output = Command::new("git")
                .args(&["branch", "--merged", main_branch])
                .current_dir(worktree_path)
                .output()
                .await?;
            
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    let clean_line = line.trim().trim_start_matches('*').trim();
                    if clean_line == branch_name {
                        return Ok((true, Some(format!("merged into {}", main_branch))));
                    }
                }
            }
        }
        
        Ok((false, None))
    }
    
    /// Detect squash merges by analyzing commit content and diffs
    async fn detect_squash_merge(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<(bool, f32, Option<String>)> {
        // Find the best main branch to compare against
        let main_branch = self.find_best_main_branch(worktree_path).await?;
        
        // Get merge base
        let merge_base_output = Command::new("git")
            .args(&["merge-base", &main_branch, branch_name])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if !merge_base_output.status.success() {
            return Ok((false, 0.0, Some("Cannot find merge base".to_string())));
        }
        
        let merge_base = String::from_utf8_lossy(&merge_base_output.stdout).trim().to_string();
        
        // Check if there are any changes between merge-base and branch tip
        let diff_output = Command::new("git")
            .args(&["diff", "--exit-code", &format!("{}..{}", merge_base, branch_name)])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        if diff_output.status.success() {
            // No changes means branch is identical to merge-base (likely rebased or no commits)
            return Ok((true, 0.6, Some("no unique changes".to_string())));
        }
        
        // Analyze commit patterns in main branch for squash evidence
        let commit_analysis = self.analyze_main_branch_for_squash(
            worktree_path, 
            &main_branch, 
            branch_name, 
            &merge_base
        ).await?;
        
        if commit_analysis.confidence > 0.5 {
            return Ok((true, commit_analysis.confidence, commit_analysis.details));
        }
        
        // Compare file contents between branch and main
        let file_analysis = self.compare_file_contents(
            worktree_path,
            &main_branch,
            branch_name,
            &merge_base
        ).await?;
        
        Ok((file_analysis.is_merged, file_analysis.confidence, file_analysis.details))
    }
    
    /// Detect merges using GitHub CLI PR information
    async fn detect_github_pr_merge(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<(bool, Option<String>)> {
        // Check if branch has an associated merged PR
        let output = Command::new("gh")
            .args(&[
                "pr", "list",
                "--state", "merged",
                "--head", branch_name,
                "--json", "number,title,mergedAt"
            ])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("No such file") {
                return Err(anyhow::anyhow!("GitHub CLI not available"));
            }
            return Err(anyhow::anyhow!("GitHub CLI failed: {}", stderr));
        }
        
        let json_str = String::from_utf8_lossy(&output.stdout);
        if json_str.trim().is_empty() || json_str.trim() == "[]" {
            return Ok((false, None));
        }
        
        // Parse JSON to get PR information
        let prs: serde_json::Value = serde_json::from_str(&json_str)?;
        if let Some(pr_array) = prs.as_array() {
            if let Some(pr) = pr_array.first() {
                if let Some(pr_number) = pr.get("number").and_then(|n| n.as_u64()) {
                    return Ok((true, Some(format!("PR #{} merged", pr_number))));
                }
            }
        }
        
        Ok((false, None))
    }
    
    /// Detect merges by comparing file contents
    async fn detect_file_content_merge(
        &self,
        worktree_path: &Path,
        branch_name: &str,
    ) -> Result<(bool, f32, Option<String>)> {
        let main_branch = self.find_best_main_branch(worktree_path).await?;
        let merge_base = self.get_merge_base(worktree_path, &main_branch, branch_name).await?;
        
        // Get list of files changed in the branch
        let changed_files = self.get_changed_files(worktree_path, &merge_base, branch_name).await?;
        
        if changed_files.is_empty() {
            return Ok((true, 0.8, Some("no file changes".to_string())));
        }
        
        // Compare each changed file between branch and main
        let mut matching_files = 0;
        let mut total_files = 0;
        
        for file in &changed_files {
            total_files += 1;
            
            if self.files_have_same_content(worktree_path, file, &main_branch, branch_name).await? {
                matching_files += 1;
            }
        }
        
        let match_ratio = matching_files as f32 / total_files as f32;
        let confidence = match_ratio * 0.7; // Conservative confidence for file content matching
        
        let details = if match_ratio > 0.8 {
            Some(format!("file contents match ({}/{})", matching_files, total_files))
        } else {
            None
        };
        
        Ok((match_ratio > 0.8, confidence, details))
    }
    
    // Helper methods
    
    async fn find_best_main_branch(&self, worktree_path: &Path) -> Result<String> {
        for branch in &self.config.main_branches {
            let output = Command::new("git")
                .args(&["rev-parse", "--verify", branch])
                .current_dir(worktree_path)
                .output()
                .await?;
                
            if output.status.success() {
                return Ok(branch.clone());
            }
        }
        
        Err(anyhow::anyhow!("No main branch found"))
    }
    
    async fn get_merge_base(
        &self,
        worktree_path: &Path, 
        main_branch: &str, 
        branch_name: &str
    ) -> Result<String> {
        let output = Command::new("git")
            .args(&["merge-base", main_branch, branch_name])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Cannot find merge base"))
        }
    }
    
    async fn get_changed_files(
        &self,
        worktree_path: &Path,
        merge_base: &str,
        branch_name: &str,
    ) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(&["diff", "--name-only", &format!("{}..{}", merge_base, branch_name)])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if output.status.success() {
            let files = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            Ok(files)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn files_have_same_content(
        &self,
        worktree_path: &Path,
        file_path: &str,
        main_branch: &str,
        branch_name: &str,
    ) -> Result<bool> {
        // Compare file content between branch and main
        let main_content_result = Command::new("git")
            .args(&["show", &format!("{}:{}", main_branch, file_path)])
            .current_dir(worktree_path)
            .output()
            .await;
            
        let branch_content_result = Command::new("git")
            .args(&["show", &format!("{}:{}", branch_name, file_path)])
            .current_dir(worktree_path)
            .output()
            .await;
        
        match (main_content_result, branch_content_result) {
            (Ok(main_output), Ok(branch_output)) => {
                Ok(main_output.stdout == branch_output.stdout)
            }
            _ => Ok(false), // If we can't read either file, assume they're different
        }
    }
    
    async fn analyze_main_branch_for_squash(
        &self,
        worktree_path: &Path,
        main_branch: &str,
        branch_name: &str,
        merge_base: &str,
    ) -> Result<SquashAnalysis> {
        // Look for commits in main that might be squash merges of this branch
        let search_range = format!("{}..{}", merge_base, main_branch);
        
        // Search for commits that mention the branch name or PR numbers
        let output = Command::new("git")
            .args(&[
                "log",
                "--oneline",
                "--grep", &format!("{}\\|#[0-9]+", branch_name),
                &search_range
            ])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        if output.status.success() && !output.stdout.is_empty() {
            let commit_messages = String::from_utf8_lossy(&output.stdout);
            let commit_count = commit_messages.lines().count();
            
            if commit_count > 0 {
                return Ok(SquashAnalysis {
                    is_merged: true,
                    confidence: 0.7,
                    details: Some(format!("found {} potential squash commits", commit_count)),
                });
            }
        }
        
        // Look for commits with similar timing to branch development
        let branch_commit_times = self.get_branch_commit_times(worktree_path, merge_base, branch_name).await?;
        if !branch_commit_times.is_empty() {
            let main_commits_in_timeframe = self.get_main_commits_in_timeframe(
                worktree_path,
                main_branch,
                merge_base,
                &branch_commit_times
            ).await?;
            
            if !main_commits_in_timeframe.is_empty() {
                return Ok(SquashAnalysis {
                    is_merged: true,
                    confidence: 0.5,
                    details: Some("commits with similar timing found".to_string()),
                });
            }
        }
        
        Ok(SquashAnalysis {
            is_merged: false,
            confidence: 0.0,
            details: None,
        })
    }
    
    async fn get_branch_commit_times(
        &self,
        worktree_path: &Path,
        merge_base: &str,
        branch_name: &str,
    ) -> Result<Vec<i64>> {
        let output = Command::new("git")
            .args(&["log", "--format=%ct", &format!("{}..{}", merge_base, branch_name)])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if output.status.success() {
            let times = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.parse::<i64>().ok())
                .collect();
            Ok(times)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn get_main_commits_in_timeframe(
        &self,
        worktree_path: &Path,
        main_branch: &str,
        merge_base: &str,
        timeframe: &[i64],
    ) -> Result<Vec<String>> {
        if timeframe.is_empty() {
            return Ok(Vec::new());
        }
        
        let min_time = timeframe.iter().min().unwrap();
        let max_time = timeframe.iter().max().unwrap();
        
        let output = Command::new("git")
            .args(&[
                "log",
                "--oneline",
                &format!("--since={}", min_time - 3600), // 1 hour buffer
                &format!("--until={}", max_time + 3600),
                &format!("{}..{}", merge_base, main_branch)
            ])
            .current_dir(worktree_path)
            .output()
            .await?;
            
        if output.status.success() {
            let commits = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.to_string())
                .collect();
            Ok(commits)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn compare_file_contents(
        &self,
        worktree_path: &Path,
        main_branch: &str,
        branch_name: &str,
        merge_base: &str,
    ) -> Result<SquashAnalysis> {
        // Get files that changed in the branch
        let changed_files = self.get_changed_files(worktree_path, merge_base, branch_name).await?;
        
        if changed_files.is_empty() {
            return Ok(SquashAnalysis {
                is_merged: true,
                confidence: 0.8,
                details: Some("no file changes".to_string()),
            });
        }
        
        // Compare each file between branch and main
        let mut matching_files = 0;
        let mut total_files = 0;
        
        for file in &changed_files {
            total_files += 1;
            
            if self.files_have_same_content(worktree_path, file, main_branch, branch_name).await? {
                matching_files += 1;
            }
        }
        
        let match_ratio = matching_files as f32 / total_files as f32;
        let confidence = match_ratio * 0.7; // Conservative confidence
        
        let details = if match_ratio > 0.8 {
            Some(format!("file contents match ({}/{})", matching_files, total_files))
        } else {
            Some(format!("partial file match ({}/{})", matching_files, total_files))
        };
        
        Ok(SquashAnalysis {
            is_merged: match_ratio > 0.8,
            confidence,
            details,
        })
    }
    
    fn analyze_method_results(&self, method_results: Vec<MethodResult>) -> Result<MergeDetectionResult> {
        if method_results.is_empty() {
            return Ok(MergeDetectionResult {
                is_merged: false,
                detection_method: "none".to_string(),
                confidence: 0.0,
                details: Some("No detection methods available".to_string()),
                method_results,
            });
        }
        
        // Find the most confident positive result
        let best_positive = method_results.iter()
            .filter(|r| r.is_merged)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());
        
        if let Some(positive_result) = best_positive {
            // We have a positive detection
            return Ok(MergeDetectionResult {
                is_merged: true,
                detection_method: positive_result.method.clone(),
                confidence: positive_result.confidence,
                details: positive_result.details.clone(),
                method_results,
            });
        }
        
        // No positive results, find the most confident negative result
        let best_negative = method_results.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());
        
        if let Some(negative_result) = best_negative {
            Ok(MergeDetectionResult {
                is_merged: false,
                detection_method: negative_result.method.clone(),
                confidence: negative_result.confidence,
                details: negative_result.details.clone(),
                method_results,
            })
        } else {
            Ok(MergeDetectionResult {
                is_merged: false,
                detection_method: "unknown".to_string(),
                confidence: 0.0,
                details: Some("All detection methods failed".to_string()),
                method_results,
            })
        }
    }
}

#[derive(Debug)]
struct SquashAnalysis {
    is_merged: bool,
    confidence: f32,
    details: Option<String>,
}

impl From<MergeDetectionResult> for MergeInfo {
    fn from(result: MergeDetectionResult) -> Self {
        MergeInfo {
            is_merged: result.is_merged,
            detection_method: result.detection_method,
            details: result.details,
            confidence: result.confidence,
        }
    }
}

/// Convenience function to detect merge status for a worktree
pub async fn detect_worktree_merge_status(
    worktree_path: &Path,
    branch_name: &str,
    config: &WorktreeMergeDetectionConfig,
) -> Result<MergeInfo> {
    let detector = MergeDetector::new(config.clone());
    let result = detector.detect_merge(worktree_path, branch_name).await?;
    Ok(result.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_merge_detection_method_conversion() {
        assert_eq!(
            MergeDetectionMethod::from_str("standard"),
            Some(MergeDetectionMethod::Standard)
        );
        assert_eq!(
            MergeDetectionMethod::from_str("invalid"),
            None
        );
        
        assert_eq!(
            MergeDetectionMethod::Standard.as_str(),
            "standard"
        );
    }
    
    #[tokio::test]
    async fn test_merge_detector_creation() {
        let config = WorktreeMergeDetectionConfig::default();
        let detector = MergeDetector::new(config);
        
        // Basic instantiation test
        assert!(!detector.config.methods.is_empty());
    }
    
    #[test]
    fn test_merge_detection_result_conversion() {
        let result = MergeDetectionResult {
            is_merged: true,
            detection_method: "standard".to_string(),
            confidence: 0.95,
            details: Some("merged into main".to_string()),
            method_results: vec![],
        };
        
        let merge_info: MergeInfo = result.into();
        assert!(merge_info.is_merged);
        assert_eq!(merge_info.detection_method, "standard");
        assert_eq!(merge_info.confidence, 0.95);
        assert_eq!(merge_info.details, Some("merged into main".to_string()));
    }
    
    #[test]
    fn test_method_result_creation() {
        let method_result = MethodResult {
            method: "test".to_string(),
            is_merged: true,
            confidence: 0.8,
            details: Some("test details".to_string()),
            error: None,
        };
        
        assert_eq!(method_result.method, "test");
        assert!(method_result.is_merged);
        assert_eq!(method_result.confidence, 0.8);
        assert!(method_result.error.is_none());
    }
    
    // Add more comprehensive tests for different merge scenarios
    // These would require setting up git repositories with various merge states
}
