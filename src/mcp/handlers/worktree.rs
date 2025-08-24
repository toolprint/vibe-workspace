//! MCP tool handlers for worktree management
//! 
//! This module provides Model Context Protocol tools that allow AI systems
//! to manage git worktrees, analyze branch status, and assist with cleanup decisions.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::mcp::types::VibeToolHandler;
use crate::workspace::WorkspaceManager;
use crate::worktree::{
    WorktreeManager, CreateOptions, CleanupOptions, CleanupStrategy,
    status::StatusSeverity,
    cleanup::WorktreeCleanup,
};

/// MCP tool for creating new worktrees
pub struct CreateWorktreeTool;

#[async_trait]
impl VibeToolHandler for CreateWorktreeTool {
    fn tool_name(&self) -> &str {
        "create_worktree"
    }

    fn tool_description(&self) -> &str {
        "Create a new git worktree for parallel development on a specific task or feature"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Task identifier to create worktree for (will be sanitized for branch name)"
                },
                "base_branch": {
                    "type": "string",
                    "description": "Base branch to create worktree from (defaults to current branch)",
                    "default": "HEAD"
                },
                "force": {
                    "type": "boolean",
                    "description": "Force creation even if branch already exists",
                    "default": false
                },
                "custom_path": {
                    "type": "string",
                    "description": "Custom path for the worktree (overrides default path calculation)"
                }
            },
            "required": ["task_id"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let task_id = args["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("task_id is required"))?;
        
        let base_branch = args["base_branch"].as_str().map(|s| s.to_string());
        let force = args["force"].as_bool().unwrap_or(false);
        let custom_path = args["custom_path"].as_str().map(PathBuf::from);
        
        // Get current directory to determine repository
        let current_dir = std::env::current_dir()?;
        
        // Create worktree manager
        let workspace_guard = workspace.lock().await;
        let worktree_manager = WorktreeManager::new_with_workspace_manager(
            &workspace_guard,
            Some(current_dir.clone())
        ).await?;
        drop(workspace_guard);
        
        let options = CreateOptions {
            task_id: task_id.to_string(),
            base_branch,
            force,
            custom_path,
        };
        
        debug!("Creating worktree for task: {}", task_id);
        
        match worktree_manager.create_worktree_with_options(options).await {
            Ok(worktree_info) => {
                Ok(json!({
                    "success": true,
                    "worktree": {
                        "path": worktree_info.path,
                        "branch": worktree_info.branch,
                        "head": worktree_info.head,
                        "age_seconds": worktree_info.age.as_secs()
                    },
                    "message": format!("Created worktree for task '{}' at {}", task_id, worktree_info.path.display())
                }))
            }
            Err(e) => {
                warn!("Failed to create worktree for task '{}': {}", task_id, e);
                Ok(json!({
                    "success": false,
                    "error": e.to_string(),
                    "suggestion": "Check that the repository is clean and the task_id is valid"
                }))
            }
        }
    }
}

/// MCP tool for listing worktrees with detailed status
pub struct ListWorktreesTool;

#[async_trait]
impl VibeToolHandler for ListWorktreesTool {
    fn tool_name(&self) -> &str {
        "list_worktrees"
    }

    fn tool_description(&self) -> &str {
        "List all git worktrees with comprehensive status information for analysis"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "include_status": {
                    "type": "boolean",
                    "description": "Include detailed status information (may be slower)",
                    "default": true
                },
                "prefix_filter": {
                    "type": "string",
                    "description": "Only include worktrees with branches starting with this prefix"
                },
                "severity_filter": {
                    "type": "string",
                    "description": "Filter by status severity: clean, light_warning, warning",
                    "enum": ["clean", "light_warning", "warning"]
                }
            },
            "required": []
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let include_status = args["include_status"].as_bool().unwrap_or(true);
        let prefix_filter = args["prefix_filter"].as_str();
        let severity_filter = args["severity_filter"].as_str();
        
        let current_dir = std::env::current_dir()?;
        
        let workspace_guard = workspace.lock().await;
        let worktree_manager = WorktreeManager::new_with_workspace_manager(
            &workspace_guard,
            Some(current_dir.clone())
        ).await?;
        drop(workspace_guard);
        
        let mut worktrees = worktree_manager.list_worktrees().await?;
        
        // Update status if requested
        if include_status {
            for worktree in &mut worktrees {
                if let Err(e) = worktree.update_status().await {
                    warn!("Failed to update status for worktree {}: {}", worktree.path.display(), e);
                }
            }
        }
        
        // Apply filters
        if let Some(prefix) = prefix_filter {
            worktrees.retain(|w| w.branch.starts_with(prefix));
        }
        
        if let Some(severity) = severity_filter {
            let target_severity = match severity {
                "clean" => StatusSeverity::Clean,
                "light_warning" => StatusSeverity::LightWarning,
                "warning" => StatusSeverity::Warning,
                _ => return Err(anyhow::anyhow!("Invalid severity filter")),
            };
            worktrees.retain(|w| w.status.severity == target_severity);
        }
        
        // Convert to AI-friendly format
        let worktree_data: Vec<Value> = worktrees.into_iter().map(|w| {
            json!({
                "path": w.path,
                "branch": w.branch,
                "head": w.head,
                "is_detached": w.is_detached,
                "age_hours": w.age.as_secs() / 3600,
                "status": {
                    "is_clean": w.status.is_clean,
                    "severity": match w.status.severity {
                        StatusSeverity::Clean => "clean",
                        StatusSeverity::LightWarning => "light_warning",
                        StatusSeverity::Warning => "warning",
                    },
                    "description": w.status.status_description(),
                    "uncommitted_changes_count": w.status.uncommitted_changes.len(),
                    "untracked_files_count": w.status.untracked_files.len(),
                    "unpushed_commits_count": w.status.unpushed_commits.len(),
                    "ahead_count": w.status.ahead_count,
                    "behind_count": w.status.behind_count,
                    "is_safe_to_cleanup": w.status.is_safe_to_cleanup(),
                    "merge_info": w.status.merge_info.as_ref().map(|info| json!({
                        "is_merged": info.is_merged,
                        "detection_method": info.detection_method,
                        "confidence": info.confidence,
                        "details": info.details
                    }))
                },
                "files": if include_status && !w.status.uncommitted_changes.is_empty() {
                    Some(w.status.uncommitted_changes)
                } else {
                    None
                }
            })
        }).collect();
        
        Ok(json!({
            "worktrees": worktree_data,
            "total_count": worktree_data.len(),
            "summary": {
                "clean": worktree_data.iter().filter(|w| w["status"]["severity"] == "clean").count(),
                "light_warning": worktree_data.iter().filter(|w| w["status"]["severity"] == "light_warning").count(),
                "warning": worktree_data.iter().filter(|w| w["status"]["severity"] == "warning").count(),
                "safe_to_cleanup": worktree_data.iter().filter(|w| w["status"]["is_safe_to_cleanup"] == true).count()
            }
        }))
    }
}

/// MCP tool for analyzing conflicts and providing resolution assistance
pub struct AnalyzeConflictsTool;

#[async_trait]
impl VibeToolHandler for AnalyzeConflictsTool {
    fn tool_name(&self) -> &str {
        "analyze_worktree_conflicts"
    }

    fn tool_description(&self) -> &str {
        "Analyze merge conflicts in a worktree and provide resolution guidance for AI assistance"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "branch_name": {
                    "type": "string",
                    "description": "Branch name or worktree to analyze"
                },
                "target_branch": {
                    "type": "string",
                    "description": "Target branch for merge analysis (default: main)",
                    "default": "main"
                },
                "include_diff": {
                    "type": "boolean",
                    "description": "Include detailed diff information",
                    "default": true
                }
            },
            "required": ["branch_name"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let branch_name = args["branch_name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("branch_name is required"))?;
        
        let target_branch = args["target_branch"].as_str().unwrap_or("main");
        let include_diff = args["include_diff"].as_bool().unwrap_or(true);
        
        let current_dir = std::env::current_dir()?;
        
        let workspace_guard = workspace.lock().await;
        let worktree_manager = WorktreeManager::new_with_workspace_manager(
            &workspace_guard,
            Some(current_dir.clone())
        ).await?;
        drop(workspace_guard);
        
        // Find the worktree
        let worktrees = worktree_manager.list_worktrees().await?;
        let target_worktree = worktrees
            .iter()
            .find(|w| w.branch == branch_name || w.path.file_name().unwrap_or_default().to_string_lossy() == branch_name)
            .ok_or_else(|| anyhow::anyhow!("Worktree not found: {}", branch_name))?;
        
        // Get conflict information
        let conflict_analysis = self.analyze_potential_conflicts(
            &target_worktree.path,
            &target_worktree.branch,
            target_branch,
            include_diff
        ).await?;
        
        Ok(json!({
            "worktree": {
                "path": target_worktree.path,
                "branch": target_worktree.branch
            },
            "target_branch": target_branch,
            "analysis": conflict_analysis
        }))
    }
}

impl AnalyzeConflictsTool {
    async fn analyze_potential_conflicts(
        &self,
        worktree_path: &std::path::Path,
        source_branch: &str,
        target_branch: &str,
        include_diff: bool,
    ) -> Result<Value> {
        use tokio::process::Command;
        
        // Check if merge would have conflicts
        let merge_base_output = Command::new("git")
            .args(&["merge-base", target_branch, source_branch])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        if !merge_base_output.status.success() {
            return Ok(json!({
                "has_conflicts": false,
                "error": "Cannot determine merge base",
                "suggestion": "Branches may not share common history"
            }));
        }
        
        let merge_base = String::from_utf8_lossy(&merge_base_output.stdout).trim().to_string();
        
        // Simulate merge to detect conflicts
        let merge_tree_output = Command::new("git")
            .args(&["merge-tree", &merge_base, target_branch, source_branch])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        let merge_tree_result = String::from_utf8_lossy(&merge_tree_output.stdout);
        let has_conflicts = merge_tree_result.contains("<<<<<<< ");
        
        let mut analysis = json!({
            "has_conflicts": has_conflicts,
            "merge_base": merge_base,
        });
        
        if has_conflicts {
            // Parse conflict information
            let conflicted_files = self.parse_conflicted_files(&merge_tree_result);
            analysis["conflicted_files"] = json!(conflicted_files);
            analysis["conflict_count"] = json!(conflicted_files.len());
            
            if include_diff {
                // Get detailed diff for each conflicted file
                let mut file_details = Vec::new();
                for file in &conflicted_files {
                    if let Ok(diff) = self.get_file_diff(worktree_path, file, target_branch, source_branch).await {
                        file_details.push(json!({
                            "file": file,
                            "diff_summary": diff
                        }));
                    }
                }
                analysis["file_details"] = json!(file_details);
            }
            
            analysis["resolution_suggestions"] = json!([
                "Review each conflicted file manually",
                "Consider rebasing the feature branch to reduce conflicts",
                "Use git mergetool for interactive conflict resolution",
                "Consider breaking large changes into smaller commits"
            ]);
        } else {
            analysis["message"] = json!("No merge conflicts detected");
            analysis["suggestion"] = json!("Safe to merge automatically");
        }
        
        Ok(analysis)
    }
    
    fn parse_conflicted_files(&self, merge_tree_output: &str) -> Vec<String> {
        // Simple parsing of merge-tree output to find conflicted files
        let mut files = Vec::new();
        let mut current_file = None;
        
        for line in merge_tree_output.lines() {
            if line.starts_with("@@@") {
                // New file section
                if let Some(file_line) = line.split_whitespace().nth(2) {
                    current_file = Some(file_line.trim_start_matches("@@@ ").to_string());
                }
            } else if line.contains("<<<<<<< ") {
                if let Some(ref file) = current_file {
                    if !files.contains(file) {
                        files.push(file.clone());
                    }
                }
            }
        }
        
        files
    }
    
    async fn get_file_diff(
        &self,
        worktree_path: &std::path::Path,
        file_path: &str,
        target_branch: &str,
        source_branch: &str,
    ) -> Result<String> {
        use tokio::process::Command;
        
        let output = Command::new("git")
            .args(&["diff", "--no-index", &format!("{}:{}", target_branch, file_path), &format!("{}:{}", source_branch, file_path)])
            .current_dir(worktree_path)
            .output()
            .await?;
        
        // Summarize the diff
        let diff_lines = String::from_utf8_lossy(&output.stdout);
        let added_lines = diff_lines.lines().filter(|line| line.starts_with('+')).count();
        let removed_lines = diff_lines.lines().filter(|line| line.starts_with('-')).count();
        
        Ok(format!("Lines added: {}, Lines removed: {}", added_lines, removed_lines))
    }
}

/// MCP tool for intelligent cleanup recommendations
pub struct RecommendCleanupTool;

#[async_trait]
impl VibeToolHandler for RecommendCleanupTool {
    fn tool_name(&self) -> &str {
        "recommend_worktree_cleanup"
    }

    fn tool_description(&self) -> &str {
        "Analyze worktrees and provide intelligent cleanup recommendations based on activity and merge status"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "min_age_days": {
                    "type": "number",
                    "description": "Minimum age in days to consider for cleanup",
                    "default": 1
                },
                "require_merged": {
                    "type": "boolean",
                    "description": "Only recommend cleanup for merged branches",
                    "default": true
                },
                "min_confidence": {
                    "type": "number",
                    "description": "Minimum merge confidence (0.0-1.0) for merged branch cleanup",
                    "default": 0.7
                }
            },
            "required": []
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let min_age_days = args["min_age_days"].as_f64().unwrap_or(1.0);
        let require_merged = args["require_merged"].as_bool().unwrap_or(true);
        let min_confidence = args["min_confidence"].as_f64().unwrap_or(0.7) as f32;
        
        let current_dir = std::env::current_dir()?;
        
        let workspace_guard = workspace.lock().await;
        let worktree_manager = WorktreeManager::new_with_workspace_manager(
            &workspace_guard,
            Some(current_dir.clone())
        ).await?;
        drop(workspace_guard);
        
        // Get all worktrees with status
        let mut worktrees = worktree_manager.list_worktrees().await?;
        
        // Update status for analysis
        for worktree in &mut worktrees {
            if let Err(e) = worktree.update_status().await {
                warn!("Failed to update status for worktree {}: {}", worktree.path.display(), e);
            }
        }
        
        // Analyze each worktree
        let mut recommendations = Vec::new();
        let min_age = std::time::Duration::from_secs((min_age_days * 24.0 * 3600.0) as u64);
        
        for worktree in &worktrees {
            let recommendation = self.analyze_worktree_for_cleanup(
                worktree,
                min_age,
                require_merged,
                min_confidence
            );
            
            if recommendation["action"] != "keep" {
                recommendations.push(recommendation);
            }
        }
        
        // Sort by safety score (safest first)
        recommendations.sort_by(|a, b| {
            let safety_a = a["safety_score"].as_f64().unwrap_or(0.0);
            let safety_b = b["safety_score"].as_f64().unwrap_or(0.0);
            safety_b.partial_cmp(&safety_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(json!({
            "recommendations": recommendations,
            "summary": {
                "total_worktrees": worktrees.len(),
                "cleanup_candidates": recommendations.len(),
                "safe_to_clean": recommendations.iter().filter(|r| r["safety_score"].as_f64().unwrap_or(0.0) > 0.8).count(),
                "merged_branches": recommendations.iter().filter(|r| r["is_merged"].as_bool().unwrap_or(false)).count()
            },
            "criteria": {
                "min_age_days": min_age_days,
                "require_merged": require_merged,
                "min_confidence": min_confidence
            }
        }))
    }
}

impl RecommendCleanupTool {
    fn analyze_worktree_for_cleanup(
        &self,
        worktree: &crate::worktree::status::WorktreeInfo,
        min_age: std::time::Duration,
        require_merged: bool,
        min_confidence: f32,
    ) -> Value {
        let mut safety_score = 1.0;
        let mut reasons = Vec::new();
        let mut warnings = Vec::new();
        
        // Age check
        if worktree.age < min_age {
            return json!({
                "worktree": worktree.branch,
                "path": worktree.path,
                "action": "keep",
                "reason": format!("Too new ({:.1} hours old)", worktree.age.as_secs_f64() / 3600.0),
                "safety_score": 0.0
            });
        }
        
        // Check if it's merged
        let is_merged = worktree.status.merge_info
            .as_ref()
            .map(|info| info.is_merged && info.confidence >= min_confidence)
            .unwrap_or(false);
        
        if require_merged && !is_merged {
            return json!({
                "worktree": worktree.branch,
                "path": worktree.path,
                "action": "keep",
                "reason": "Not confirmed as merged",
                "safety_score": 0.0,
                "merge_info": worktree.status.merge_info
            });
        }
        
        // Safety analysis
        if !worktree.status.uncommitted_changes.is_empty() {
            safety_score -= 0.3;
            warnings.push(format!("{} uncommitted changes", worktree.status.uncommitted_changes.len()));
        }
        
        if !worktree.status.untracked_files.is_empty() {
            safety_score -= 0.2;
            warnings.push(format!("{} untracked files", worktree.status.untracked_files.len()));
        }
        
        if !worktree.status.unpushed_commits.is_empty() && !is_merged {
            safety_score -= 0.4;
            warnings.push(format!("{} unpushed commits", worktree.status.unpushed_commits.len()));
        }
        
        // Determine action
        let action = if safety_score > 0.8 && is_merged {
            "safe_cleanup"
        } else if safety_score > 0.5 {
            "careful_cleanup"
        } else {
            "keep"
        };
        
        if is_merged {
            reasons.push(format!("Merged via {}", 
                worktree.status.merge_info.as_ref()
                    .map(|info| info.detection_method.as_str())
                    .unwrap_or("unknown")));
        }
        
        reasons.push(format!("Age: {:.1} days", worktree.age.as_secs_f64() / 86400.0));
        
        json!({
            "worktree": worktree.branch,
            "path": worktree.path,
            "action": action,
            "safety_score": safety_score,
            "is_merged": is_merged,
            "reasons": reasons,
            "warnings": warnings,
            "age_days": worktree.age.as_secs_f64() / 86400.0,
            "merge_confidence": worktree.status.merge_info
                .as_ref()
                .map(|info| info.confidence)
                .unwrap_or(0.0)
        })
    }
}

/// MCP tool for executing cleanup operations
pub struct ExecuteCleanupTool;

#[async_trait]
impl VibeToolHandler for ExecuteCleanupTool {
    fn tool_name(&self) -> &str {
        "execute_worktree_cleanup"
    }

    fn tool_description(&self) -> &str {
        "Execute worktree cleanup operations with specified strategy and safety options"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "strategy": {
                    "type": "string",
                    "description": "Cleanup strategy to use",
                    "enum": ["discard", "merge_to_feature", "backup_to_origin", "stash_and_discard"],
                    "default": "discard"
                },
                "targets": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Specific branch names to clean (empty = all eligible)"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Show what would be done without executing",
                    "default": true
                },
                "force": {
                    "type": "boolean",
                    "description": "Override safety checks",
                    "default": false
                },
                "min_merge_confidence": {
                    "type": "number",
                    "description": "Minimum merge confidence for cleanup (0.0-1.0)",
                    "default": 0.7
                }
            },
            "required": []
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let strategy_str = args["strategy"].as_str().unwrap_or("discard");
        let _targets: Vec<String> = args["targets"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let dry_run = args["dry_run"].as_bool().unwrap_or(true);
        let force = args["force"].as_bool().unwrap_or(false);
        let min_confidence = args["min_merge_confidence"].as_f64().unwrap_or(0.7) as f32;
        
        let strategy = match strategy_str {
            "discard" => CleanupStrategy::Discard,
            "merge_to_feature" => CleanupStrategy::MergeToFeature,
            "backup_to_origin" => CleanupStrategy::BackupToOrigin,
            "stash_and_discard" => CleanupStrategy::StashAndDiscard,
            _ => return Err(anyhow::anyhow!("Invalid cleanup strategy")),
        };
        
        let current_dir = std::env::current_dir()?;
        
        let workspace_guard = workspace.lock().await;
        let worktree_manager = WorktreeManager::new_with_workspace_manager(
            &workspace_guard,
            Some(current_dir.clone())
        ).await?;
        drop(workspace_guard);
        
        let cleanup_options = CleanupOptions {
            strategy,
            min_age_hours: Some(1), // Minimum 1 hour for AI operations
            force,
            dry_run,
            auto_confirm: true, // AI operations skip interactive prompts
            branch_prefix_filter: Some(worktree_manager.get_config().prefix.clone()),
            merged_only: true,
            min_merge_confidence: min_confidence,
        };
        
        let cleanup = WorktreeCleanup::new(
            worktree_manager.get_config().clone(),
            worktree_manager.get_operations()
        );
        
        let report = cleanup.cleanup_worktrees(cleanup_options).await?;
        
        Ok(json!({
            "report": {
                "total_evaluated": report.total_evaluated,
                "cleaned_count": report.cleaned_count,
                "skipped_count": report.skipped_count,
                "failed_count": report.failed_count,
                "was_dry_run": report.was_dry_run,
                "strategy_used": format!("{:?}", report.strategy_used)
            },
            "results": report.worktree_results.into_iter().map(|result| json!({
                "worktree": result.branch,
                "path": result.path,
                "action": format!("{:?}", result.action),
                "reason": result.reason,
                "error": result.error,
                "safety_violations": result.safety_violations.into_iter().map(|v| json!({
                    "type": format!("{:?}", v.violation_type),
                    "description": v.description,
                    "severity": format!("{:?}", v.severity)
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>(),
            "success": report.failed_count == 0
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[tokio::test]
    async fn test_create_worktree_tool() {
        let tool = CreateWorktreeTool;
        assert_eq!(tool.tool_name(), "create_worktree");
        
        let schema = tool.input_schema();
        assert!(schema["properties"]["task_id"].is_object());
        assert_eq!(schema["required"], json!(["task_id"]));
    }
    
    #[tokio::test]
    async fn test_list_worktrees_tool() {
        let tool = ListWorktreesTool;
        assert_eq!(tool.tool_name(), "list_worktrees");
        
        let schema = tool.input_schema();
        assert!(schema["properties"]["include_status"].is_object());
        assert!(schema["properties"]["prefix_filter"].is_object());
    }
    
    #[tokio::test]
    async fn test_analyze_conflicts_tool() {
        let tool = AnalyzeConflictsTool;
        assert_eq!(tool.tool_name(), "analyze_worktree_conflicts");
        
        let schema = tool.input_schema();
        assert_eq!(schema["required"], json!(["branch_name"]));
    }
    
    #[tokio::test]
    async fn test_recommend_cleanup_tool() {
        let tool = RecommendCleanupTool;
        assert_eq!(tool.tool_name(), "recommend_worktree_cleanup");
        
        let schema = tool.input_schema();
        assert!(schema["properties"]["min_age_days"].is_object());
        assert!(schema["properties"]["require_merged"].is_object());
        assert!(schema["properties"]["min_confidence"].is_object());
    }
    
    #[tokio::test]
    async fn test_execute_cleanup_tool() {
        let tool = ExecuteCleanupTool;
        assert_eq!(tool.tool_name(), "execute_worktree_cleanup");
        
        let schema = tool.input_schema();
        assert!(schema["properties"]["strategy"].is_object());
        assert!(schema["properties"]["dry_run"].is_object());
        assert!(schema["properties"]["force"].is_object());
    }
}