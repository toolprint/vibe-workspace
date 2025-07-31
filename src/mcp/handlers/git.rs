//! Git-related MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::{GitStatusInfo, VibeToolHandler};
use crate::workspace::{operations::get_git_status, WorkspaceManager};

/// MCP tool for checking git status across repositories
pub struct GitStatusTool;

#[async_trait]
impl VibeToolHandler for GitStatusTool {
    fn tool_name(&self) -> &str {
        "vibe_git_status"
    }

    fn tool_description(&self) -> &str {
        "Show git status across all workspace repositories"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "dirty_only": {
                    "type": "boolean",
                    "description": "Show only repositories with uncommitted changes",
                    "default": false
                },
                "format": {
                    "type": "string",
                    "description": "Output format",
                    "enum": ["json", "table", "compact"],
                    "default": "json"
                },
                "group": {
                    "type": "string",
                    "description": "Filter by repository group"
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
        // Parse arguments
        let dirty_only = args
            .get("dirty_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        let group = args.get("group").and_then(|v| v.as_str());

        // Get workspace manager
        let ws = workspace.lock().await;

        // Get all repositories, optionally filtered by group
        let repos = if let Some(group_name) = group {
            ws.config().get_repositories_in_group(group_name)
        } else {
            ws.list_repositories().iter().collect::<Vec<_>>()
        };

        // Collect status information
        let mut statuses = Vec::new();
        let workspace_root = ws.get_workspace_root();

        for repo in repos {
            // Get git status for this repository
            let repo_path = workspace_root.join(&repo.path);
            let status = get_git_status(&repo_path).await?;

            // Skip clean repositories if dirty_only is set
            if dirty_only && status.clean {
                continue;
            }

            // Create status info
            let status_info = GitStatusInfo {
                repository: repo.name.clone(),
                path: repo.path.to_string_lossy().to_string(),
                is_dirty: !status.clean,
                has_staged_changes: status.staged > 0,
                has_unstaged_changes: status.unstaged > 0,
                has_untracked_files: status.untracked > 0,
                ahead: status.ahead,
                behind: status.behind,
            };

            statuses.push(status_info);
        }

        // Format the response based on requested format
        match format {
            "json" => Ok(json!({
                "repositories": statuses,
                "total": statuses.len(),
                "dirty_count": statuses.iter().filter(|s| s.is_dirty).count()
            })),

            "table" | "compact" => {
                // For non-JSON formats, return a structured response
                // that the MCP client can format appropriately
                let mut output = Vec::new();

                for status in &statuses {
                    let status_str = if status.is_dirty {
                        format!(
                            "{} (dirty: {} staged, {} unstaged, {} untracked)",
                            status.repository,
                            if status.has_staged_changes {
                                "✓"
                            } else {
                                "✗"
                            },
                            if status.has_unstaged_changes {
                                "✓"
                            } else {
                                "✗"
                            },
                            if status.has_untracked_files {
                                "✓"
                            } else {
                                "✗"
                            }
                        )
                    } else {
                        format!("{} (clean)", status.repository)
                    };

                    output.push(status_str);
                }

                Ok(json!({
                    "output": output.join("\n"),
                    "total": statuses.len(),
                    "dirty_count": statuses.iter().filter(|s| s.is_dirty).count()
                }))
            }

            _ => Ok(json!({
                "error": format!("Unknown format: {}", format),
                "supported_formats": ["json", "table", "compact"]
            })),
        }
    }
}
