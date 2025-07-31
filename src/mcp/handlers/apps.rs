//! App management MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::workspace::WorkspaceManager;

/// MCP tool for configuring an app for a repository
pub struct ConfigureAppTool;

#[async_trait]
impl VibeToolHandler for ConfigureAppTool {
    fn tool_name(&self) -> &str {
        "configure_app"
    }

    fn tool_description(&self) -> &str {
        "Configure app integration for a repository"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                },
                "app": {
                    "type": "string",
                    "description": "App to configure (warp, iterm2, vscode, wezterm, cursor, windsurf)",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "template": {
                    "type": "string",
                    "description": "Template to use",
                    "default": "default"
                }
            },
            "required": ["repo", "app"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Repository name is required"))?;

        let app = args
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("App name is required"))?;

        let template = args
            .get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let mut ws = workspace.lock().await;
        ws.configure_app_for_repo(repo, app, template).await?;

        Ok(json!({
            "status": "success",
            "repository": repo,
            "app": app,
            "template": template
        }))
    }
}

/// MCP tool for showing app configurations
pub struct ShowAppsTool;

#[async_trait]
impl VibeToolHandler for ShowAppsTool {
    fn tool_name(&self) -> &str {
        "show_apps"
    }

    fn tool_description(&self) -> &str {
        "Show app configurations for repositories"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Filter by repository name"
                },
                "app": {
                    "type": "string",
                    "description": "Filter by app name"
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
        let repo_filter = args.get("repo").and_then(|v| v.as_str());
        let app_filter = args.get("app").and_then(|v| v.as_str());

        let ws = workspace.lock().await;

        if let Some(repo_name) = repo_filter {
            // Show apps for specific repository
            let apps = ws.list_apps_for_repo(repo_name)?;
            Ok(json!({
                "repository": repo_name,
                "apps": apps.into_iter().map(|(app, template)| {
                    json!({
                        "app": app,
                        "template": template
                    })
                }).collect::<Vec<_>>()
            }))
        } else if let Some(app_name) = app_filter {
            // Show repositories with specific app
            let repos = ws.list_repos_with_app(app_name);
            Ok(json!({
                "app": app_name,
                "repositories": repos.into_iter().map(|(repo, template)| {
                    json!({
                        "repository": repo.name,
                        "template": template
                    })
                }).collect::<Vec<_>>()
            }))
        } else {
            // Show all app configurations
            let mut all_configs = Vec::new();
            for repo in ws.list_repositories() {
                let apps = ws.list_apps_for_repo(&repo.name)?;
                if !apps.is_empty() {
                    all_configs.push(json!({
                        "repository": repo.name,
                        "apps": apps.into_iter().map(|(app, template)| {
                            json!({
                                "app": app,
                                "template": template
                            })
                        }).collect::<Vec<_>>()
                    }));
                }
            }
            Ok(json!({
                "configurations": all_configs
            }))
        }
    }
}

/// MCP tool for listing app templates
pub struct ListAppTemplatesTool;

#[async_trait]
impl VibeToolHandler for ListAppTemplatesTool {
    fn tool_name(&self) -> &str {
        "list_app_templates"
    }

    fn tool_description(&self) -> &str {
        "List available templates for an app"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "app": {
                    "type": "string",
                    "description": "App to list templates for",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                }
            },
            "required": ["app"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let app = args
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("App name is required"))?;

        let ws = workspace.lock().await;
        let templates = ws.list_templates(app).await?;

        Ok(json!({
            "app": app,
            "templates": templates
        }))
    }
}

/// MCP tool for creating app templates
pub struct CreateAppTemplateTool;

#[async_trait]
impl VibeToolHandler for CreateAppTemplateTool {
    fn tool_name(&self) -> &str {
        "create_app_template"
    }

    fn tool_description(&self) -> &str {
        "Create a new template for an app"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "app": {
                    "type": "string",
                    "description": "App to create template for",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "name": {
                    "type": "string",
                    "description": "Template name"
                },
                "content": {
                    "type": "string",
                    "description": "Template content (if not provided, uses default as base)"
                }
            },
            "required": ["app", "name"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let app = args
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("App name is required"))?;

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;

        let content = if let Some(content_str) = args.get("content").and_then(|v| v.as_str()) {
            content_str.to_string()
        } else {
            // Use default template as base
            let ws = workspace.lock().await;
            ws.get_default_template(app).await?
        };

        let ws = workspace.lock().await;
        ws.save_template(app, name, &content).await?;

        Ok(json!({
            "status": "success",
            "app": app,
            "template": name
        }))
    }
}

/// MCP tool for deleting app templates
pub struct DeleteAppTemplateTool;

#[async_trait]
impl VibeToolHandler for DeleteAppTemplateTool {
    fn tool_name(&self) -> &str {
        "delete_app_template"
    }

    fn tool_description(&self) -> &str {
        "Delete a template from an app"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "app": {
                    "type": "string",
                    "description": "App to delete template from",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "name": {
                    "type": "string",
                    "description": "Template name to delete"
                }
            },
            "required": ["app", "name"]
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        let app = args
            .get("app")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("App name is required"))?;

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Template name is required"))?;

        let ws = workspace.lock().await;
        ws.delete_template(app, name).await?;

        Ok(json!({
            "status": "success",
            "app": app,
            "deleted_template": name
        }))
    }
}

/// MCP tool for updating default templates
pub struct UpdateDefaultTemplatesTool;

#[async_trait]
impl VibeToolHandler for UpdateDefaultTemplatesTool {
    fn tool_name(&self) -> &str {
        "update_default_templates"
    }

    fn tool_description(&self) -> &str {
        "Update default templates with current bundled versions"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "app": {
                    "type": "string",
                    "description": "Only update specific app's default template",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
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
        let app = args.get("app").and_then(|v| v.as_str());
        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

        let apps_to_update = if let Some(app_name) = app {
            vec![app_name.to_string()]
        } else {
            vec![
                "warp".to_string(),
                "iterm2".to_string(),
                "wezterm".to_string(),
                "vscode".to_string(),
                "cursor".to_string(),
                "windsurf".to_string(),
            ]
        };

        // Note: In the actual CLI, this prompts for confirmation unless force is true
        // For MCP, we'll require explicit force flag to proceed
        if !force {
            return Ok(json!({
                "status": "confirmation_required",
                "message": "This will overwrite existing default templates. Use force=true to proceed.",
                "apps_to_update": apps_to_update
            }));
        }

        let ws = workspace.lock().await;
        ws.update_default_templates(apps_to_update.clone()).await?;

        Ok(json!({
            "status": "success",
            "updated_apps": apps_to_update
        }))
    }
}
