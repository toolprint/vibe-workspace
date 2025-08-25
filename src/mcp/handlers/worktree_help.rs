//! Documentation and help tools for worktree MCP integration

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::workspace::WorkspaceManager;

/// MCP tool for getting worktree help and documentation
pub struct WorktreeHelpTool;

#[async_trait]
impl VibeToolHandler for WorktreeHelpTool {
    fn tool_name(&self) -> &str {
        "worktree_help"
    }

    fn tool_description(&self) -> &str {
        "Get comprehensive help and documentation for worktree management system"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "Specific help topic",
                    "enum": ["overview", "configuration", "commands", "workflows", "troubleshooting"],
                    "default": "overview"
                }
            },
            "required": []
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        _workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let topic = args["topic"].as_str().unwrap_or("overview");

        let help_content = match topic {
            "overview" => self.get_overview_help(),
            "configuration" => self.get_configuration_help(),
            "commands" => self.get_commands_help(),
            "workflows" => self.get_workflows_help(),
            "troubleshooting" => self.get_troubleshooting_help(),
            _ => return Err(anyhow::anyhow!("Unknown help topic")),
        };

        Ok(json!({
            "topic": topic,
            "content": help_content,
            "available_topics": ["overview", "configuration", "commands", "workflows", "troubleshooting"]
        }))
    }
}

impl WorktreeHelpTool {
    fn get_overview_help(&self) -> Value {
        json!({
            "title": "Worktree Management Overview",
            "description": "Git worktrees enable parallel development by creating multiple working directories for the same repository",
            "key_concepts": {
                "worktrees": "Separate working directories that share the same git repository",
                "branches": "Each worktree is associated with a specific branch for isolated development",
                "cleanup": "Automated removal of merged or abandoned worktrees",
                "status_tracking": "Comprehensive monitoring of worktree health and merge status"
            },
            "benefits": [
                "Work on multiple features simultaneously without stashing",
                "Test different branches without switching context",
                "Parallel code review and testing",
                "Clean workspace organization"
            ],
            "available_tools": [
                "create_worktree - Create new worktrees for features",
                "list_worktrees - View all worktrees with status",
                "analyze_worktree_conflicts - Check for merge conflicts",
                "recommend_worktree_cleanup - Get cleanup suggestions",
                "execute_worktree_cleanup - Perform cleanup operations"
            ]
        })
    }

    fn get_configuration_help(&self) -> Value {
        json!({
            "title": "Worktree Configuration",
            "description": "Configuration options for customizing worktree behavior",
            "configuration_file": "~/.toolprint/vibe-workspace/config.yaml",
            "environment_variables": [
                {
                    "name": "VIBE_WORKTREE_BASE",
                    "default": ".worktrees",
                    "description": "Base directory for worktree storage"
                },
                {
                    "name": "VIBE_WORKTREE_PREFIX",
                    "default": "vibe-ws/",
                    "description": "Branch prefix for worktree branches"
                },
                {
                    "name": "VIBE_WORKTREE_AUTO_GITIGNORE",
                    "default": "true",
                    "description": "Automatically add worktree base directory to .gitignore"
                },
                {
                    "name": "VIBE_WORKTREE_AGE_THRESHOLD",
                    "default": "168",
                    "description": "Age threshold in hours for cleanup eligibility"
                },
                {
                    "name": "VIBE_WORKTREE_MIN_CONFIDENCE",
                    "default": "0.8",
                    "description": "Minimum merge confidence for automated cleanup"
                }
            ],
            "sample_config": {
                "worktree": {
                    "base_dir": ".worktrees",
                    "prefix": "vibe-ws/",
                    "auto_gitignore": true,
                    "cleanup": {
                        "age_threshold_hours": 168,
                        "min_merge_confidence": 0.8,
                        "require_merged": true,
                        "preserve_uncommitted": true
                    },
                    "merge_detection": {
                        "enabled": true,
                        "methods": ["standard", "squash", "commit_title"],
                        "confidence_threshold": 0.7
                    }
                }
            },
            "repository_overrides": {
                "description": "Repository-specific configuration can override global settings",
                "location": "repositories[].worktree_config section in config.yaml",
                "supported_overrides": ["base_dir", "prefix", "cleanup", "merge_detection"]
            }
        })
    }

    fn get_commands_help(&self) -> Value {
        json!({
            "title": "Worktree Commands",
            "description": "Available commands for worktree management",
            "mcp_tools": {
                "create_worktree": {
                    "purpose": "Create a new worktree for a task or feature",
                    "required_params": ["task_id"],
                    "optional_params": ["base_branch", "force", "custom_path"],
                    "example": {
                        "task_id": "feature-123",
                        "base_branch": "main"
                    },
                    "returns": "Worktree creation result with path and branch information"
                },
                "list_worktrees": {
                    "purpose": "List all worktrees with detailed status",
                    "optional_params": ["include_status", "prefix_filter", "severity_filter"],
                    "returns": "Array of worktrees with comprehensive status information",
                    "filters": {
                        "severity_filter": ["clean", "light_warning", "warning"],
                        "prefix_filter": "Branch name prefix to filter by"
                    }
                },
                "analyze_worktree_conflicts": {
                    "purpose": "Analyze potential merge conflicts",
                    "required_params": ["branch_name"],
                    "optional_params": ["target_branch", "include_diff"],
                    "returns": "Conflict analysis with resolution suggestions and affected files"
                },
                "recommend_worktree_cleanup": {
                    "purpose": "Get intelligent cleanup recommendations",
                    "optional_params": ["min_age_days", "require_merged", "min_confidence"],
                    "returns": "Sorted list of cleanup candidates with safety scores and detailed analysis"
                },
                "execute_worktree_cleanup": {
                    "purpose": "Execute cleanup operations",
                    "optional_params": ["strategy", "targets", "dry_run", "force"],
                    "strategies": ["discard", "merge_to_feature", "backup_to_origin", "stash_and_discard"],
                    "safety_features": ["dry_run mode", "safety score analysis", "merge confirmation"]
                }
            },
            "cli_commands": {
                "note": "All MCP tools have corresponding CLI commands",
                "prefix": "vibe git worktree",
                "examples": [
                    "vibe git worktree create feature-123",
                    "vibe git worktree list --verbose",
                    "vibe git worktree clean --dry-run"
                ]
            }
        })
    }

    fn get_workflows_help(&self) -> Value {
        json!({
            "title": "Worktree Workflows",
            "description": "Common workflows and best practices for worktree management",
            "workflows": {
                "feature_development": {
                    "description": "Parallel feature development workflow",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Create worktree",
                            "tool": "create_worktree",
                            "params": {"task_id": "feature-name", "base_branch": "main"},
                            "description": "Creates isolated workspace for feature development"
                        },
                        {
                            "step": 2,
                            "action": "Develop feature",
                            "description": "Work on feature in isolated environment without affecting main workspace"
                        },
                        {
                            "step": 3,
                            "action": "Test and iterate",
                            "description": "Run tests and make changes without context switching"
                        },
                        {
                            "step": 4,
                            "action": "Analyze conflicts",
                            "tool": "analyze_worktree_conflicts",
                            "description": "Check for merge conflicts before integration"
                        },
                        {
                            "step": 5,
                            "action": "Clean up",
                            "tool": "execute_worktree_cleanup",
                            "description": "Remove worktree after successful merge"
                        }
                    ]
                },
                "code_review": {
                    "description": "Review multiple pull requests simultaneously",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Create review worktrees",
                            "tool": "create_worktree",
                            "description": "Create separate worktrees for each PR branch"
                        },
                        {
                            "step": 2,
                            "action": "Test each branch",
                            "description": "Test each implementation independently"
                        },
                        {
                            "step": 3,
                            "action": "Compare implementations",
                            "description": "Review code side by side across different worktrees"
                        },
                        {
                            "step": 4,
                            "action": "Batch cleanup",
                            "tool": "recommend_worktree_cleanup",
                            "description": "Clean up all review worktrees when done"
                        }
                    ]
                },
                "hotfix_development": {
                    "description": "Urgent fixes while maintaining current work",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Create hotfix worktree",
                            "tool": "create_worktree",
                            "params": {"task_id": "hotfix-issue", "base_branch": "main"},
                            "description": "Create worktree for hotfix from production branch"
                        },
                        {
                            "step": 2,
                            "action": "Implement fix",
                            "description": "Fix issue without disturbing feature development work"
                        },
                        {
                            "step": 3,
                            "action": "Test and deploy",
                            "description": "Test hotfix and deploy to production"
                        },
                        {
                            "step": 4,
                            "action": "Merge back to features",
                            "description": "Integrate hotfix into ongoing feature branches"
                        }
                    ]
                },
                "cleanup_maintenance": {
                    "description": "Regular workspace maintenance",
                    "frequency": "Weekly or after major releases",
                    "steps": [
                        {
                            "step": 1,
                            "action": "Analyze workspace",
                            "tool": "list_worktrees",
                            "params": {"include_status": true},
                            "description": "Get comprehensive view of all worktrees"
                        },
                        {
                            "step": 2,
                            "action": "Get recommendations",
                            "tool": "recommend_worktree_cleanup",
                            "description": "Identify cleanup candidates with safety analysis"
                        },
                        {
                            "step": 3,
                            "action": "Review recommendations",
                            "description": "Manually review recommendations, especially those with lower safety scores"
                        },
                        {
                            "step": 4,
                            "action": "Execute cleanup",
                            "tool": "execute_worktree_cleanup",
                            "params": {"dry_run": false},
                            "description": "Perform cleanup operations on approved candidates"
                        }
                    ]
                }
            },
            "best_practices": [
                "Use descriptive task IDs that map to your project management system",
                "Regularly clean up merged worktrees to save disk space",
                "Always use dry-run mode before executing bulk operations",
                "Monitor merge confidence scores for cleanup decisions",
                "Keep worktree base directory in .gitignore to avoid accidental commits",
                "Use repository-specific configuration for different project needs",
                "Create worktrees from stable branches (main/develop) when possible",
                "Test worktree functionality before integrating into CI/CD pipelines"
            ]
        })
    }

    fn get_troubleshooting_help(&self) -> Value {
        json!({
            "title": "Worktree Troubleshooting",
            "description": "Common issues and solutions for worktree management",
            "common_issues": {
                "worktree_creation_fails": {
                    "symptoms": [
                        "Branch already exists error",
                        "Permission denied during creation",
                        "Invalid task ID characters"
                    ],
                    "solutions": [
                        {
                            "issue": "Branch already exists",
                            "solution": "Use force option to recreate existing worktree",
                            "tool": "create_worktree",
                            "params": {"force": true}
                        },
                        {
                            "issue": "Permission denied",
                            "solution": "Check repository permissions and disk space in worktree base directory",
                            "verification": "Ensure base directory is writable and has sufficient space"
                        },
                        {
                            "issue": "Invalid task ID",
                            "solution": "Task IDs are automatically sanitized, but ensure they contain valid characters",
                            "note": "Special characters are converted to hyphens automatically"
                        }
                    ]
                },
                "cleanup_skipped": {
                    "symptoms": [
                        "Safety violations prevent cleanup",
                        "Low merge confidence scores",
                        "Uncommitted changes detected"
                    ],
                    "solutions": [
                        {
                            "issue": "Uncommitted changes",
                            "solution": "Review changes and either commit, stash, or discard them",
                            "tools": ["list_worktrees", "analyze_worktree_conflicts"],
                            "strategy": "Use stash_and_discard cleanup strategy to preserve changes"
                        },
                        {
                            "issue": "Low merge confidence",
                            "solution": "Lower confidence threshold or verify merge status manually",
                            "tool": "recommend_worktree_cleanup",
                            "params": {"min_confidence": 0.5}
                        },
                        {
                            "issue": "Safety violations",
                            "solution": "Use force option to override warnings (carefully)",
                            "tool": "execute_worktree_cleanup",
                            "params": {"force": true},
                            "warning": "Only use force when you're certain the worktree can be safely removed"
                        }
                    ]
                },
                "merge_conflicts": {
                    "symptoms": [
                        "Conflicts detected during analysis",
                        "Merge simulation shows conflicts"
                    ],
                    "solutions": [
                        {
                            "analysis": "Use analyze_worktree_conflicts to understand conflict scope",
                            "resolution_strategies": [
                                "Resolve conflicts manually in worktree",
                                "Rebase feature branch to reduce conflicts",
                                "Use git mergetool for interactive resolution",
                                "Break large changes into smaller, focused commits"
                            ]
                        },
                        {
                            "prevention": "Regular rebasing and smaller commits reduce conflict likelihood",
                            "tools": ["analyze_worktree_conflicts with include_diff: true"]
                        }
                    ]
                },
                "configuration_errors": {
                    "symptoms": [
                        "Invalid configuration values",
                        "Environment variable parsing errors",
                        "Permission issues with configured paths"
                    ],
                    "solutions": [
                        {
                            "validation": "Use worktree_help with configuration topic to review settings",
                            "reset": "Reset configuration to defaults if severely corrupted",
                            "incremental": "Reconfigure one setting at a time to identify issues"
                        }
                    ]
                },
                "disk_space_issues": {
                    "symptoms": [
                        "Creation fails due to insufficient space",
                        "Large number of old worktrees consuming space"
                    ],
                    "solutions": [
                        {
                            "immediate": "Use recommend_worktree_cleanup to identify space-consuming worktrees",
                            "prevention": "Set up regular cleanup schedules",
                            "monitoring": "Use list_worktrees to monitor worktree count and age"
                        }
                    ]
                }
            },
            "diagnostic_tools": [
                {
                    "tool": "list_worktrees",
                    "params": {"include_status": true},
                    "purpose": "Comprehensive workspace overview"
                },
                {
                    "tool": "worktree_help",
                    "params": {"topic": "configuration"},
                    "purpose": "Verify configuration settings"
                },
                {
                    "tool": "analyze_worktree_conflicts",
                    "params": {"include_diff": true},
                    "purpose": "Detailed conflict analysis"
                },
                {
                    "tool": "recommend_worktree_cleanup",
                    "params": {"min_age_days": 0},
                    "purpose": "Analyze all worktrees for issues"
                }
            ],
            "recovery_procedures": {
                "lost_worktree": {
                    "description": "Worktree directory deleted but git references remain",
                    "steps": [
                        "Recreate worktree with same branch name using force option",
                        "Restore work from stash or remote branch",
                        "Update status to reflect current state"
                    ]
                },
                "corrupted_config": {
                    "description": "Configuration file is invalid or corrupted",
                    "steps": [
                        "Create backup of current configuration",
                        "Reset to default configuration",
                        "Incrementally restore custom settings",
                        "Validate each change"
                    ]
                },
                "permission_problems": {
                    "description": "File system permission issues",
                    "steps": [
                        "Check directory permissions for worktree base",
                        "Verify git repository access permissions",
                        "Update file ownership if necessary",
                        "Test with minimal worktree creation"
                    ]
                }
            },
            "performance_optimization": {
                "large_repositories": "Use shallow clones or sparse checkouts for large repositories",
                "many_worktrees": "Regular cleanup prevents performance degradation",
                "slow_status_updates": "Cache status information and update incrementally",
                "disk_io": "Place worktree base directory on fast storage when possible"
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worktree_help_tool() {
        let tool = WorktreeHelpTool;
        assert_eq!(tool.tool_name(), "worktree_help");

        let schema = tool.input_schema();
        assert!(schema["properties"]["topic"].is_object());

        // Test available topics
        let topics = &schema["properties"]["topic"]["enum"];
        assert!(topics.as_array().unwrap().contains(&json!("overview")));
        assert!(topics.as_array().unwrap().contains(&json!("configuration")));
        assert!(topics.as_array().unwrap().contains(&json!("commands")));
        assert!(topics.as_array().unwrap().contains(&json!("workflows")));
        assert!(topics
            .as_array()
            .unwrap()
            .contains(&json!("troubleshooting")));
    }

    #[test]
    fn test_help_content_structure() {
        let tool = WorktreeHelpTool;

        // Test each help section has required structure
        let overview = tool.get_overview_help();
        assert!(overview["title"].is_string());
        assert!(overview["description"].is_string());
        assert!(overview["key_concepts"].is_object());
        assert!(overview["benefits"].is_array());
        assert!(overview["available_tools"].is_array());

        let config = tool.get_configuration_help();
        assert!(config["title"].is_string());
        assert!(config["environment_variables"].is_array());
        assert!(config["sample_config"].is_object());

        let commands = tool.get_commands_help();
        assert!(commands["mcp_tools"].is_object());
        assert!(commands["cli_commands"].is_object());

        let workflows = tool.get_workflows_help();
        assert!(workflows["workflows"].is_object());
        assert!(workflows["best_practices"].is_array());

        let troubleshooting = tool.get_troubleshooting_help();
        assert!(troubleshooting["common_issues"].is_object());
        assert!(troubleshooting["diagnostic_tools"].is_array());
        assert!(troubleshooting["recovery_procedures"].is_object());
    }
}
