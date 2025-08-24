//! Cleanup strategies and safety mechanisms for worktree management
//!
//! This module provides safe cleanup operations for merged worktrees with
//! multiple validation layers and recovery options to prevent data loss.

use anyhow::{bail, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};

use crate::worktree::config::WorktreeConfig;
use crate::worktree::operations::{RemoveOptions, WorktreeOperations};
use crate::worktree::status::WorktreeInfo;

/// Different strategies for cleaning up worktrees
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupStrategy {
    /// Simply remove the worktree (safest, requires manual verification)
    Discard,
    /// Merge worktree changes into the feature branch before removal
    MergeToFeature,
    /// Push worktree branch to origin before removal (backup)
    BackupToOrigin,
    /// Stash changes and remove worktree
    StashAndDiscard,
}

/// Options for cleanup operations
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    /// Strategy to use for cleanup
    pub strategy: CleanupStrategy,

    /// Minimum age threshold for cleanup eligibility
    pub min_age_hours: Option<u64>,

    /// Force cleanup even with safety violations
    pub force: bool,

    /// Show what would be done without executing
    pub dry_run: bool,

    /// Skip interactive confirmations
    pub auto_confirm: bool,

    /// Only clean worktrees with specific branch prefix
    pub branch_prefix_filter: Option<String>,

    /// Only clean worktrees that are confirmed merged
    pub merged_only: bool,

    /// Minimum merge confidence required (0.0-1.0)
    pub min_merge_confidence: f32,
}

/// Result of cleanup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    /// Total worktrees evaluated
    pub total_evaluated: usize,

    /// Number of worktrees successfully cleaned
    pub cleaned_count: usize,

    /// Number of worktrees skipped due to safety checks
    pub skipped_count: usize,

    /// Number of worktrees that failed during cleanup
    pub failed_count: usize,

    /// Detailed results for each worktree
    pub worktree_results: Vec<WorktreeCleanupResult>,

    /// Overall cleanup strategy used
    pub strategy_used: CleanupStrategy,

    /// Whether this was a dry run
    pub was_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeCleanupResult {
    /// Path to the worktree
    pub path: PathBuf,

    /// Branch name
    pub branch: String,

    /// Cleanup action taken
    pub action: CleanupAction,

    /// Reason for the action (or why it was skipped)
    pub reason: String,

    /// Any error that occurred
    pub error: Option<String>,

    /// Safety violations that were ignored (if force was used)
    pub safety_violations: Vec<SafetyViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupAction {
    Cleaned,
    Skipped,
    Failed,
    StashCreated,
    MergedToFeature,
    BackedUpToOrigin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    pub violation_type: SafetyViolationType,
    pub description: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyViolationType {
    UncommittedChanges,
    UnpushedCommits,
    BranchTooNew,
    NoRemoteTracking,
    LowMergeConfidence,
    RemoteBranchMissing,
    WorktreeInUse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,  // Can be overridden with --force
    Critical, // Should never be overridden
}

/// Main cleanup orchestrator
pub struct WorktreeCleanup {
    config: WorktreeConfig,
    operations: WorktreeOperations,
}

impl WorktreeCleanup {
    pub fn new(config: WorktreeConfig, operations: WorktreeOperations) -> Self {
        Self { config, operations }
    }

    /// Execute cleanup operation on all eligible worktrees
    pub async fn cleanup_worktrees(&self, options: CleanupOptions) -> Result<CleanupReport> {
        info!(
            "Starting worktree cleanup with strategy: {:?}",
            options.strategy
        );

        let all_worktrees = self.operations.list_worktrees().await?;
        let mut report = CleanupReport {
            total_evaluated: all_worktrees.len(),
            cleaned_count: 0,
            skipped_count: 0,
            failed_count: 0,
            worktree_results: Vec::new(),
            strategy_used: options.strategy.clone(),
            was_dry_run: options.dry_run,
        };

        for worktree in all_worktrees {
            let result = self
                .evaluate_and_cleanup_worktree(&worktree, &options)
                .await;

            match result {
                Ok(cleanup_result) => {
                    match cleanup_result.action {
                        CleanupAction::Cleaned
                        | CleanupAction::MergedToFeature
                        | CleanupAction::BackedUpToOrigin => {
                            report.cleaned_count += 1;
                        }
                        CleanupAction::Skipped => {
                            report.skipped_count += 1;
                        }
                        CleanupAction::Failed => {
                            report.failed_count += 1;
                        }
                        CleanupAction::StashCreated => {
                            // Count as cleaned if the worktree was also removed
                            report.cleaned_count += 1;
                        }
                    }
                    report.worktree_results.push(cleanup_result);
                }
                Err(e) => {
                    warn!(
                        "Failed to process worktree {}: {}",
                        worktree.path.display(),
                        e
                    );
                    report.failed_count += 1;
                    report.worktree_results.push(WorktreeCleanupResult {
                        path: worktree.path.clone(),
                        branch: worktree.branch.clone(),
                        action: CleanupAction::Failed,
                        reason: "Processing error".to_string(),
                        error: Some(e.to_string()),
                        safety_violations: Vec::new(),
                    });
                }
            }
        }

        info!(
            "Cleanup complete: {} cleaned, {} skipped, {} failed",
            report.cleaned_count, report.skipped_count, report.failed_count
        );

        Ok(report)
    }

    /// Evaluate and potentially clean up a single worktree
    async fn evaluate_and_cleanup_worktree(
        &self,
        worktree: &WorktreeInfo,
        options: &CleanupOptions,
    ) -> Result<WorktreeCleanupResult> {
        // Skip the main repository worktree
        if self.is_main_repository_worktree(worktree).await? {
            return Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Skipped,
                reason: "Main repository worktree".to_string(),
                error: None,
                safety_violations: Vec::new(),
            });
        }

        // Apply filters
        if !self.matches_filters(worktree, options) {
            return Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Skipped,
                reason: "Does not match cleanup filters".to_string(),
                error: None,
                safety_violations: Vec::new(),
            });
        }

        // Perform safety checks
        let safety_violations = self.check_safety_violations(worktree, options).await;

        // Determine if we should proceed
        let critical_violations: Vec<_> = safety_violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Critical)
            .collect();

        if !critical_violations.is_empty() {
            return Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Skipped,
                reason: format!(
                    "Critical safety violations: {}",
                    critical_violations
                        .iter()
                        .map(|v| v.description.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                error: None,
                safety_violations,
            });
        }

        let warning_violations: Vec<_> = safety_violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .collect();

        if !warning_violations.is_empty() && !options.force {
            return Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Skipped,
                reason: format!(
                    "Safety violations (use --force to override): {}",
                    warning_violations
                        .iter()
                        .map(|v| v.description.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                error: None,
                safety_violations,
            });
        }

        // Ask for confirmation if not auto-confirming
        if !options.auto_confirm && !options.dry_run {
            if !self
                .confirm_cleanup(worktree, options, &safety_violations)
                .await?
            {
                return Ok(WorktreeCleanupResult {
                    path: worktree.path.clone(),
                    branch: worktree.branch.clone(),
                    action: CleanupAction::Skipped,
                    reason: "User declined cleanup".to_string(),
                    error: None,
                    safety_violations,
                });
            }
        }

        // Perform the cleanup operation
        if options.dry_run {
            Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Cleaned,
                reason: "Would be cleaned (dry run)".to_string(),
                error: None,
                safety_violations,
            })
        } else {
            self.execute_cleanup_strategy(worktree, options, safety_violations)
                .await
        }
    }

    /// Execute the specific cleanup strategy
    async fn execute_cleanup_strategy(
        &self,
        worktree: &WorktreeInfo,
        options: &CleanupOptions,
        safety_violations: Vec<SafetyViolation>,
    ) -> Result<WorktreeCleanupResult> {
        match options.strategy {
            CleanupStrategy::Discard => {
                self.execute_discard_strategy(worktree, options, safety_violations)
                    .await
            }
            CleanupStrategy::MergeToFeature => {
                self.execute_merge_to_feature_strategy(worktree, options, safety_violations)
                    .await
            }
            CleanupStrategy::BackupToOrigin => {
                self.execute_backup_to_origin_strategy(worktree, options, safety_violations)
                    .await
            }
            CleanupStrategy::StashAndDiscard => {
                self.execute_stash_and_discard_strategy(worktree, options, safety_violations)
                    .await
            }
        }
    }

    /// Execute discard strategy (simple removal)
    async fn execute_discard_strategy(
        &self,
        worktree: &WorktreeInfo,
        _options: &CleanupOptions,
        safety_violations: Vec<SafetyViolation>,
    ) -> Result<WorktreeCleanupResult> {
        let remove_options = RemoveOptions {
            target: worktree.branch.clone(),
            force: true, // We've already done safety checks
            delete_branch: self.config.cleanup.auto_delete_branch,
        };

        match self.operations.remove_worktree(remove_options).await {
            Ok(_) => Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Cleaned,
                reason: "Worktree removed".to_string(),
                error: None,
                safety_violations,
            }),
            Err(e) => Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Failed,
                reason: "Failed to remove worktree".to_string(),
                error: Some(e.to_string()),
                safety_violations,
            }),
        }
    }

    /// Execute merge to feature branch strategy
    async fn execute_merge_to_feature_strategy(
        &self,
        worktree: &WorktreeInfo,
        _options: &CleanupOptions,
        safety_violations: Vec<SafetyViolation>,
    ) -> Result<WorktreeCleanupResult> {
        // Determine the target feature branch (remove worktree prefix)
        let feature_branch = self.extract_feature_branch_name(&worktree.branch)?;

        // Ensure target branch exists
        if !self.branch_exists(&feature_branch).await? {
            return Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Failed,
                reason: format!("Target feature branch '{}' does not exist", feature_branch),
                error: None,
                safety_violations,
            });
        }

        // Perform the merge
        match self
            .merge_worktree_to_branch(worktree, &feature_branch)
            .await
        {
            Ok(merge_result) => {
                if merge_result.has_conflicts {
                    // Don't remove worktree if there are conflicts
                    Ok(WorktreeCleanupResult {
                        path: worktree.path.clone(),
                        branch: worktree.branch.clone(),
                        action: CleanupAction::Failed,
                        reason: format!(
                            "Merge conflicts detected: {}",
                            merge_result.conflict_summary
                        ),
                        error: None,
                        safety_violations,
                    })
                } else {
                    // Merge successful, remove worktree
                    let remove_options = RemoveOptions {
                        target: worktree.branch.clone(),
                        force: true,
                        delete_branch: true, // Remove the worktree branch after successful merge
                    };

                    self.operations.remove_worktree(remove_options).await?;

                    Ok(WorktreeCleanupResult {
                        path: worktree.path.clone(),
                        branch: worktree.branch.clone(),
                        action: CleanupAction::MergedToFeature,
                        reason: format!("Merged to '{}' and cleaned", feature_branch),
                        error: None,
                        safety_violations,
                    })
                }
            }
            Err(e) => Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Failed,
                reason: "Failed to merge to feature branch".to_string(),
                error: Some(e.to_string()),
                safety_violations,
            }),
        }
    }

    /// Execute backup to origin strategy
    async fn execute_backup_to_origin_strategy(
        &self,
        worktree: &WorktreeInfo,
        _options: &CleanupOptions,
        safety_violations: Vec<SafetyViolation>,
    ) -> Result<WorktreeCleanupResult> {
        // Push the branch to origin
        match self.push_branch_to_origin(worktree).await {
            Ok(_) => {
                // After successful backup, remove the worktree
                let remove_options = RemoveOptions {
                    target: worktree.branch.clone(),
                    force: true,
                    delete_branch: false, // Keep the branch since it's backed up
                };

                self.operations.remove_worktree(remove_options).await?;

                Ok(WorktreeCleanupResult {
                    path: worktree.path.clone(),
                    branch: worktree.branch.clone(),
                    action: CleanupAction::BackedUpToOrigin,
                    reason: "Backed up to origin and cleaned".to_string(),
                    error: None,
                    safety_violations,
                })
            }
            Err(e) => Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Failed,
                reason: "Failed to backup to origin".to_string(),
                error: Some(e.to_string()),
                safety_violations,
            }),
        }
    }

    /// Execute stash and discard strategy
    async fn execute_stash_and_discard_strategy(
        &self,
        worktree: &WorktreeInfo,
        _options: &CleanupOptions,
        safety_violations: Vec<SafetyViolation>,
    ) -> Result<WorktreeCleanupResult> {
        // Create a stash with uncommitted changes
        let stash_name = format!(
            "vibe-cleanup-{}-{}",
            worktree.branch,
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );

        let stash_result = self.create_stash(worktree, &stash_name).await;

        match stash_result {
            Ok(stash_created) => {
                // Remove the worktree
                let remove_options = RemoveOptions {
                    target: worktree.branch.clone(),
                    force: true,
                    delete_branch: self.config.cleanup.auto_delete_branch,
                };

                match self.operations.remove_worktree(remove_options).await {
                    Ok(_) => {
                        let reason = if stash_created {
                            format!("Stashed changes as '{}' and cleaned", stash_name)
                        } else {
                            "No changes to stash, worktree cleaned".to_string()
                        };

                        Ok(WorktreeCleanupResult {
                            path: worktree.path.clone(),
                            branch: worktree.branch.clone(),
                            action: CleanupAction::StashCreated,
                            reason,
                            error: None,
                            safety_violations,
                        })
                    }
                    Err(e) => Ok(WorktreeCleanupResult {
                        path: worktree.path.clone(),
                        branch: worktree.branch.clone(),
                        action: CleanupAction::Failed,
                        reason: "Stash created but failed to remove worktree".to_string(),
                        error: Some(e.to_string()),
                        safety_violations,
                    }),
                }
            }
            Err(e) => Ok(WorktreeCleanupResult {
                path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                action: CleanupAction::Failed,
                reason: "Failed to create stash".to_string(),
                error: Some(e.to_string()),
                safety_violations,
            }),
        }
    }

    // Helper methods for safety checks and operations

    async fn check_safety_violations(
        &self,
        worktree: &WorktreeInfo,
        options: &CleanupOptions,
    ) -> Vec<SafetyViolation> {
        let mut violations = Vec::new();

        // Check age threshold
        if let Some(min_hours) = options
            .min_age_hours
            .or(Some(self.config.cleanup.age_threshold_hours))
        {
            let min_age = Duration::from_secs(min_hours * 3600);
            if worktree.age < min_age {
                violations.push(SafetyViolation {
                    violation_type: SafetyViolationType::BranchTooNew,
                    description: format!(
                        "Worktree is only {} old (minimum: {} hours)",
                        format_duration(worktree.age),
                        min_hours
                    ),
                    severity: ViolationSeverity::Warning,
                });
            }
        }

        // Check for uncommitted changes
        if !worktree.status.uncommitted_changes.is_empty()
            || !worktree.status.untracked_files.is_empty()
        {
            violations.push(SafetyViolation {
                violation_type: SafetyViolationType::UncommittedChanges,
                description: format!(
                    "{} uncommitted changes, {} untracked files",
                    worktree.status.uncommitted_changes.len(),
                    worktree.status.untracked_files.len()
                ),
                severity: ViolationSeverity::Warning,
            });
        }

        // Check for unpushed commits
        if !worktree.status.unpushed_commits.is_empty() {
            violations.push(SafetyViolation {
                violation_type: SafetyViolationType::UnpushedCommits,
                description: format!(
                    "{} unpushed commits",
                    worktree.status.unpushed_commits.len()
                ),
                severity: ViolationSeverity::Warning,
            });
        }

        // Check merge confidence if filtering by merged branches
        if options.merged_only {
            if let Some(merge_info) = &worktree.status.merge_info {
                if !merge_info.is_merged {
                    violations.push(SafetyViolation {
                        violation_type: SafetyViolationType::LowMergeConfidence,
                        description: "Branch does not appear to be merged".to_string(),
                        severity: ViolationSeverity::Critical,
                    });
                } else if merge_info.confidence < options.min_merge_confidence {
                    violations.push(SafetyViolation {
                        violation_type: SafetyViolationType::LowMergeConfidence,
                        description: format!(
                            "Merge confidence too low: {:.0}% (minimum: {:.0}%)",
                            merge_info.confidence * 100.0,
                            options.min_merge_confidence * 100.0
                        ),
                        severity: ViolationSeverity::Warning,
                    });
                }
            } else {
                violations.push(SafetyViolation {
                    violation_type: SafetyViolationType::LowMergeConfidence,
                    description: "No merge information available".to_string(),
                    severity: ViolationSeverity::Critical,
                });
            }
        }

        // Check if worktree is currently in use (current working directory)
        if let Ok(current_dir) = std::env::current_dir() {
            if current_dir.starts_with(&worktree.path) {
                violations.push(SafetyViolation {
                    violation_type: SafetyViolationType::WorktreeInUse,
                    description: "Worktree is currently in use (current directory)".to_string(),
                    severity: ViolationSeverity::Critical,
                });
            }
        }

        violations
    }

    async fn is_main_repository_worktree(&self, worktree: &WorktreeInfo) -> Result<bool> {
        // Check if this worktree is the main repository (contains .git directory)
        Ok(worktree.path.join(".git").is_dir())
    }

    fn matches_filters(&self, worktree: &WorktreeInfo, options: &CleanupOptions) -> bool {
        // Check branch prefix filter
        if let Some(ref prefix) = options.branch_prefix_filter {
            if !worktree.branch.starts_with(prefix) {
                return false;
            }
        }

        true
    }

    async fn confirm_cleanup(
        &self,
        worktree: &WorktreeInfo,
        options: &CleanupOptions,
        violations: &[SafetyViolation],
    ) -> Result<bool> {
        println!(
            "{} Cleanup worktree: {}",
            "?".yellow(),
            worktree.branch.cyan()
        );
        println!("  Path: {}", worktree.path.display().to_string().blue());
        println!("  Strategy: {:?}", options.strategy);

        if !violations.is_empty() {
            println!("  {} Safety concerns:", "⚠️".yellow());
            for violation in violations {
                let severity_icon = match violation.severity {
                    ViolationSeverity::Warning => "⚠️",
                    ViolationSeverity::Critical => "🚨",
                };
                println!("    {} {}", severity_icon, violation.description);
            }
        }

        use std::io::{self, Write};
        print!("  Proceed? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
    }

    // Strategy implementation helpers

    fn extract_feature_branch_name(&self, worktree_branch: &str) -> Result<String> {
        if let Some(suffix) = worktree_branch.strip_prefix(&self.config.prefix) {
            Ok(suffix.to_string())
        } else {
            bail!(
                "Branch '{}' does not have expected prefix '{}'",
                worktree_branch,
                self.config.prefix
            );
        }
    }

    async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(&[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch_name),
            ])
            .output()
            .await?;

        Ok(output.status.success())
    }

    async fn merge_worktree_to_branch(
        &self,
        worktree: &WorktreeInfo,
        target_branch: &str,
    ) -> Result<MergeResult> {
        // Switch to target branch
        let checkout_output = Command::new("git")
            .args(&["checkout", target_branch])
            .current_dir(&worktree.path.parent().unwrap_or(&worktree.path))
            .output()
            .await?;

        if !checkout_output.status.success() {
            bail!(
                "Failed to checkout target branch: {}",
                String::from_utf8_lossy(&checkout_output.stderr)
            );
        }

        // Attempt merge
        let merge_output = Command::new("git")
            .args(&["merge", &worktree.branch])
            .current_dir(&worktree.path.parent().unwrap_or(&worktree.path))
            .output()
            .await?;

        if merge_output.status.success() {
            Ok(MergeResult {
                success: true,
                has_conflicts: false,
                conflict_summary: String::new(),
            })
        } else {
            // Check if it's a merge conflict
            let stderr = String::from_utf8_lossy(&merge_output.stderr);
            if stderr.contains("conflict") || stderr.contains("CONFLICT") {
                let conflict_summary = self.get_merge_conflict_summary(&worktree.path).await?;
                Ok(MergeResult {
                    success: false,
                    has_conflicts: true,
                    conflict_summary,
                })
            } else {
                bail!("Merge failed: {}", stderr);
            }
        }
    }

    async fn get_merge_conflict_summary(&self, worktree_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(&["diff", "--name-only", "--diff-filter=U"])
            .current_dir(worktree_path)
            .output()
            .await?;

        if output.status.success() {
            let conflicted_files = String::from_utf8_lossy(&output.stdout);
            let file_count = conflicted_files.lines().count();
            Ok(format!("{} conflicted files", file_count))
        } else {
            Ok("Unknown conflicts".to_string())
        }
    }

    async fn push_branch_to_origin(&self, worktree: &WorktreeInfo) -> Result<()> {
        let output = Command::new("git")
            .args(&["push", "origin", &worktree.branch])
            .current_dir(&worktree.path)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to push to origin: {}", stderr);
        }
    }

    async fn create_stash(&self, worktree: &WorktreeInfo, stash_name: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(&["stash", "push", "-m", stash_name])
            .current_dir(&worktree.path)
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Git returns different messages if there's nothing to stash
            Ok(!stdout.contains("No local changes to save"))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to create stash: {}", stderr);
        }
    }
}

#[derive(Debug)]
struct MergeResult {
    success: bool,
    has_conflicts: bool,
    conflict_summary: String,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            strategy: CleanupStrategy::Discard,
            min_age_hours: Some(24),
            force: false,
            dry_run: false,
            auto_confirm: false,
            branch_prefix_filter: None,
            merged_only: false,
            min_merge_confidence: 0.8,
        }
    }
}

/// Format a duration for human-readable display
fn format_duration(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let days = hours / 24;

    if days > 0 {
        format!("{} days", days)
    } else if hours > 0 {
        format!("{} hours", hours)
    } else {
        format!("{} minutes", duration.as_secs() / 60)
    }
}

/// Convenience function to create cleanup options for merged worktrees only
pub fn merged_worktrees_cleanup_options() -> CleanupOptions {
    CleanupOptions {
        merged_only: true,
        min_merge_confidence: 0.7,
        ..Default::default()
    }
}

/// Convenience function to create cleanup options for old worktrees
pub fn old_worktrees_cleanup_options(min_age_days: u64) -> CleanupOptions {
    CleanupOptions {
        min_age_hours: Some(min_age_days * 24),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_options_defaults() {
        let options = CleanupOptions::default();
        assert_eq!(options.strategy, CleanupStrategy::Discard);
        assert_eq!(options.min_age_hours, Some(24));
        assert!(!options.force);
        assert!(!options.dry_run);
    }

    #[test]
    fn test_format_duration() {
        let minutes = Duration::from_secs(30 * 60);
        let hours = Duration::from_secs(5 * 3600);
        let days = Duration::from_secs(3 * 24 * 3600);

        assert_eq!(format_duration(minutes), "30 minutes");
        assert_eq!(format_duration(hours), "5 hours");
        assert_eq!(format_duration(days), "3 days");
    }

    #[test]
    fn test_safety_violation_severity() {
        let warning = SafetyViolation {
            violation_type: SafetyViolationType::UncommittedChanges,
            description: "test".to_string(),
            severity: ViolationSeverity::Warning,
        };

        let critical = SafetyViolation {
            violation_type: SafetyViolationType::WorktreeInUse,
            description: "test".to_string(),
            severity: ViolationSeverity::Critical,
        };

        assert_eq!(warning.severity, ViolationSeverity::Warning);
        assert_eq!(critical.severity, ViolationSeverity::Critical);
    }
}
