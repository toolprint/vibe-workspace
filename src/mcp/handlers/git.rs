//! Git-related MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
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

/// MCP tool for scanning workspace for git repositories
pub struct ScanReposTool;

#[async_trait]
impl VibeToolHandler for ScanReposTool {
    fn tool_name(&self) -> &str {
        "scan_repos"
    }

    fn tool_description(&self) -> &str {
        "Scan workspace for git repositories"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to scan for repositories"
                },
                "depth": {
                    "type": "integer",
                    "description": "Maximum depth to scan",
                    "default": 3
                },
                "import": {
                    "type": "boolean",
                    "description": "Add newly found repositories to config",
                    "default": false
                },
                "restore": {
                    "type": "boolean",
                    "description": "Re-clone missing repositories from config",
                    "default": false
                },
                "clean": {
                    "type": "boolean",
                    "description": "Remove missing repositories from config",
                    "default": false
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
        let path = args.get("path").and_then(|v| v.as_str()).map(PathBuf::from);

        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let import = args
            .get("import")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let restore = args
            .get("restore")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let clean = args.get("clean").and_then(|v| v.as_bool()).unwrap_or(false);

        // Validate conflicting flags
        if restore && clean {
            return Ok(json!({
                "status": "error",
                "message": "Cannot use 'restore' and 'clean' together"
            }));
        }

        let mut ws = workspace.lock().await;
        let scan_path = path.unwrap_or_else(|| ws.get_workspace_root().clone());

        ws.scan_repositories(&scan_path, depth, import, restore, clean)
            .await?;

        Ok(json!({
            "status": "success",
            "scanned_path": scan_path.to_string_lossy(),
            "depth": depth,
            "operations": {
                "import": import,
                "restore": restore,
                "clean": clean
            }
        }))
    }
}

/// MCP tool for syncing repositories
pub struct SyncReposTool;

#[async_trait]
impl VibeToolHandler for SyncReposTool {
    fn tool_name(&self) -> &str {
        "sync_repos"
    }

    fn tool_description(&self) -> &str {
        "Sync repositories (fetch and pull)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "fetch_only": {
                    "type": "boolean",
                    "description": "Only fetch, don't pull",
                    "default": false
                },
                "prune": {
                    "type": "boolean",
                    "description": "Prune remote tracking branches",
                    "default": false
                },
                "save_dirty": {
                    "type": "boolean",
                    "description": "Auto-commit dirty changes to dirty/{timestamp} branch before sync",
                    "default": false
                },
                "group": {
                    "type": "string",
                    "description": "Target group"
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
        let fetch_only = args
            .get("fetch_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let prune = args.get("prune").and_then(|v| v.as_bool()).unwrap_or(false);

        let save_dirty = args
            .get("save_dirty")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let group = args.get("group").and_then(|v| v.as_str());

        let ws = workspace.lock().await;
        ws.sync_repositories(fetch_only, prune, save_dirty, group)
            .await?;

        Ok(json!({
            "status": "success",
            "options": {
                "fetch_only": fetch_only,
                "prune": prune,
                "save_dirty": save_dirty,
                "group": group
            }
        }))
    }
}

/// MCP tool for cloning a repository
pub struct CloneRepoTool;

#[async_trait]
impl VibeToolHandler for CloneRepoTool {
    fn tool_name(&self) -> &str {
        "clone_repo"
    }

    fn tool_description(&self) -> &str {
        "Clone a repository to the workspace"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Repository URL or identifier"
                },
                "path": {
                    "type": "string",
                    "description": "Override default clone location"
                },
                "open": {
                    "type": "boolean",
                    "description": "Open in configured editor after cloning",
                    "default": false
                },
                "install": {
                    "type": "boolean",
                    "description": "Run post-install commands (npm install, etc.)",
                    "default": false
                }
            },
            "required": ["url"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Repository URL is required"))?;

        let path = args.get("path").and_then(|v| v.as_str()).map(PathBuf::from);

        let open = args.get("open").and_then(|v| v.as_bool()).unwrap_or(false);

        let install = args
            .get("install")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let git_config = crate::git::GitConfig::default();
        let mut ws = workspace.lock().await;
        let cloned_path = crate::git::CloneCommand::execute(
            url.to_string(),
            path,
            open,
            install,
            &mut *ws,
            &git_config,
        )
        .await?;

        Ok(json!({
            "status": "success",
            "cloned_path": cloned_path.to_string_lossy(),
            "url": url
        }))
    }
}

/// MCP tool for executing git commands across repositories
pub struct ExecGitCommandTool;

#[async_trait]
impl VibeToolHandler for ExecGitCommandTool {
    fn tool_name(&self) -> &str {
        "exec_git_command"
    }

    fn tool_description(&self) -> &str {
        "Execute git commands across repositories"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Git command to execute"
                },
                "repos": {
                    "type": "string",
                    "description": "Target repositories (comma-separated)"
                },
                "group": {
                    "type": "string",
                    "description": "Target group"
                },
                "parallel": {
                    "type": "boolean",
                    "description": "Run in parallel",
                    "default": false
                }
            },
            "required": ["command"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Git command is required"))?;

        let repos = args.get("repos").and_then(|v| v.as_str());
        let group = args.get("group").and_then(|v| v.as_str());
        let parallel = args
            .get("parallel")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ws = workspace.lock().await;
        ws.execute_command(command, repos, group, parallel).await?;

        Ok(json!({
            "status": "success",
            "command": command,
            "parallel": parallel
        }))
    }
}

/// MCP tool for resetting repository configuration
pub struct ResetGitConfigTool;

#[async_trait]
impl VibeToolHandler for ResetGitConfigTool {
    fn tool_name(&self) -> &str {
        "reset_git_config"
    }

    fn tool_description(&self) -> &str {
        "Reset repository configuration (clear all tracked repositories)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "force": {
                    "type": "boolean",
                    "description": "Skip confirmation prompt",
                    "default": false
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
        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut ws = workspace.lock().await;
        ws.reset_repositories(force).await?;

        Ok(json!({
            "status": "success",
            "message": "Repository configuration has been reset"
        }))
    }
}
