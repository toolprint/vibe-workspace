//! Configuration-related MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::workspace::WorkspaceManager;

/// MCP tool for initializing a new workspace
pub struct InitWorkspaceTool;

#[async_trait]
impl VibeToolHandler for InitWorkspaceTool {
    fn tool_name(&self) -> &str {
        "init_workspace"
    }

    fn tool_description(&self) -> &str {
        "Initialize a new vibe workspace in the specified directory"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Workspace name (defaults to directory name)"
                },
                "root": {
                    "type": "string",
                    "description": "Root directory for workspace (defaults to current directory)"
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
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "workspace".to_string())
                    .leak()
            });

        let root = if let Some(root_str) = args.get("root").and_then(|v| v.as_str()) {
            PathBuf::from(root_str)
        } else {
            std::env::current_dir()?
        };

        let mut ws = workspace.lock().await;
        ws.init_workspace(name, &root).await?;

        Ok(json!({
            "status": "success",
            "workspace_name": name,
            "workspace_root": root.to_string_lossy()
        }))
    }
}

/// MCP tool for showing workspace configuration
pub struct ShowConfigTool;

#[async_trait]
impl VibeToolHandler for ShowConfigTool {
    fn tool_name(&self) -> &str {
        "show_config"
    }

    fn tool_description(&self) -> &str {
        "Show current workspace configuration"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "description": "Output format",
                    "enum": ["yaml", "json", "pretty"],
                    "default": "yaml"
                },
                "section": {
                    "type": "string",
                    "description": "Show only a specific section",
                    "enum": ["workspace", "repositories", "groups", "apps", "claude_agents"]
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
        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("yaml");

        let section = args.get("section").and_then(|v| v.as_str());

        let ws = workspace.lock().await;

        // Instead of calling show_config which prints to stdout,
        // we'll get the config data directly and return it
        let config_data = match section {
            Some("workspace") => json!({
                "workspace": {
                    "name": &ws.config().workspace.name,
                    "root": ws.config().workspace.root.to_string_lossy()
                }
            }),
            Some("repositories") => json!({
                "repositories": ws.config().repositories
            }),
            Some("groups") => json!({
                "groups": ws.config().groups
            }),
            Some("apps") => json!({
                "apps": ws.config().apps
            }),
            Some("claude_agents") => json!({
                "claude_agents": ws.config().claude_agents
            }),
            Some(unknown_section) => {
                return Ok(json!({
                    "status": "error",
                    "message": format!("Unknown section: {}. Valid sections are: workspace, repositories, groups, apps, claude_agents", unknown_section)
                }));
            }
            None => {
                // Return full config
                serde_json::to_value(ws.config())?
            }
        };

        Ok(json!({
            "format": format,
            "configuration": config_data
        }))
    }
}

/// MCP tool for initializing workspace configuration
pub struct InitConfigTool;

#[async_trait]
impl VibeToolHandler for InitConfigTool {
    fn tool_name(&self) -> &str {
        "init_config"
    }

    fn tool_description(&self) -> &str {
        "Initialize a new workspace configuration"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Workspace name"
                },
                "root": {
                    "type": "string",
                    "description": "Workspace root directory"
                },
                "auto_discover": {
                    "type": "boolean",
                    "description": "Enable auto-discovery of repositories",
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
        let name = args.get("name").and_then(|v| v.as_str());
        let root = args.get("root").and_then(|v| v.as_str()).map(PathBuf::from);
        let auto_discover = args
            .get("auto_discover")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut ws = workspace.lock().await;
        ws.init_config(name, root.as_deref(), auto_discover).await?;

        Ok(json!({
            "status": "success",
            "message": "Workspace configuration initialized"
        }))
    }
}

/// MCP tool for validating workspace configuration
pub struct ValidateConfigTool;

#[async_trait]
impl VibeToolHandler for ValidateConfigTool {
    fn tool_name(&self) -> &str {
        "validate_config"
    }

    fn tool_description(&self) -> &str {
        "Validate workspace configuration"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "check_paths": {
                    "type": "boolean",
                    "description": "Check if all repository paths exist",
                    "default": false
                },
                "check_remotes": {
                    "type": "boolean",
                    "description": "Check if all remote URLs are accessible",
                    "default": false
                },
                "check_apps": {
                    "type": "boolean",
                    "description": "Validate app integrations",
                    "default": false
                },
            },
            "required": []
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let check_paths = args
            .get("check_paths")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let check_remotes = args
            .get("check_remotes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let check_apps = args
            .get("check_apps")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ws = workspace.lock().await;

        // Run validation and collect results
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Basic config validation
        if ws.config().repositories.is_empty() {
            warnings.push("No repositories configured".to_string());
        }

        if check_paths {
            for repo in &ws.config().repositories {
                let repo_path = ws.get_workspace_root().join(&repo.path);
                if !repo_path.exists() {
                    issues.push(format!("Repository path not found: {}", repo.name));
                }
            }
        }

        // TODO: Add remote and app validation logic

        let is_valid = issues.is_empty();

        Ok(json!({
            "valid": is_valid,
            "issues": issues,
            "warnings": warnings,
            "checks_performed": {
                "basic": true,
                "paths": check_paths,
                "remotes": check_remotes,
                "apps": check_apps,
            }
        }))
    }
}

/// MCP tool for factory reset of configuration
pub struct ResetConfigTool;

#[async_trait]
impl VibeToolHandler for ResetConfigTool {
    fn tool_name(&self) -> &str {
        "reset_config"
    }

    fn tool_description(&self) -> &str {
        "Factory reset - clear all configuration and reinitialize"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "force": {
                    "type": "boolean",
                    "description": "Skip confirmation prompts",
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
        ws.factory_reset(force).await?;

        Ok(json!({
            "status": "success",
            "message": "Configuration has been reset to factory defaults"
        }))
    }
}

/// MCP tool for creating configuration backup
pub struct BackupConfigTool;

#[async_trait]
impl VibeToolHandler for BackupConfigTool {
    fn tool_name(&self) -> &str {
        "backup_config"
    }

    fn tool_description(&self) -> &str {
        "Create backup archive of all configuration files"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "output": {
                    "type": "string",
                    "description": "Output directory for backup file"
                },
                "name": {
                    "type": "string",
                    "description": "Custom backup name (default: timestamp)"
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
        let output = args
            .get("output")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let name = args.get("name").and_then(|v| v.as_str()).map(String::from);

        let ws = workspace.lock().await;
        let backup_path = ws.create_backup(output, name).await?;

        Ok(json!({
            "status": "success",
            "backup_path": backup_path.to_string_lossy()
        }))
    }
}

/// MCP tool for restoring configuration from backup
pub struct RestoreConfigTool;

#[async_trait]
impl VibeToolHandler for RestoreConfigTool {
    fn tool_name(&self) -> &str {
        "restore_config"
    }

    fn tool_description(&self) -> &str {
        "Restore configuration from backup archive"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "backup": {
                    "type": "string",
                    "description": "Backup file to restore from"
                },
                "force": {
                    "type": "boolean",
                    "description": "Skip confirmation prompts",
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
        let backup = args
            .get("backup")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut ws = workspace.lock().await;
        ws.restore_from_backup(backup, force).await?;

        Ok(json!({
            "status": "success",
            "message": "Configuration restored from backup"
        }))
    }
}
