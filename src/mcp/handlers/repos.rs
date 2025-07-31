//! Repository operation MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::ui::state::VibeState;
use crate::ui::workflows::{execute_workflow, CloneAndOpenWorkflow};
use crate::workspace::WorkspaceManager;

/// MCP tool for launching a repository
pub struct LaunchRepoTool;

#[async_trait]
impl VibeToolHandler for LaunchRepoTool {
    fn tool_name(&self) -> &str {
        "launch_repo"
    }

    fn tool_description(&self) -> &str {
        "Quick launch recent repository or specific repository"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": {
                    "type": "string",
                    "description": "Repository name or number (1-9 for recent repos)"
                },
                "app": {
                    "type": "string",
                    "description": "App to open with (overrides default/last used)",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
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
        // Load state to get recent repos
        let mut state = VibeState::load().unwrap_or_default();

        let repo_to_open = if let Some(repo_name) = args.get("repo").and_then(|v| v.as_str()) {
            // Check if it's a number (1-9) for recent repos
            if let Ok(num) = repo_name.parse::<usize>() {
                if num >= 1 && num <= 9 {
                    let recent_repos = state.get_recent_repos(15);
                    if num <= recent_repos.len() {
                        recent_repos[num - 1].repo_id.clone()
                    } else {
                        return Ok(json!({
                            "status": "error",
                            "message": format!("No recent repository at position {}", num)
                        }));
                    }
                } else {
                    repo_name.to_string()
                }
            } else {
                repo_name.to_string()
            }
        } else {
            // No repo specified, open the most recent one
            let recent_repos = state.get_recent_repos(1);
            if recent_repos.is_empty() {
                return Ok(json!({
                    "status": "error",
                    "message": "No recent repositories found"
                }));
            }
            recent_repos[0].repo_id.clone()
        };

        let ws = workspace.lock().await;

        // Get the repository info
        let repo_info = ws
            .get_repository(&repo_to_open)
            .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_to_open))?;

        // Determine which app to use
        let app_to_use = if let Some(app_name) = args.get("app").and_then(|v| v.as_str()) {
            app_name.to_string()
        } else if let Some(last_app) = state.get_last_app(&repo_to_open) {
            last_app.clone()
        } else {
            // Get configured apps and use first one
            let apps = ws.list_apps_for_repo(&repo_to_open)?;
            if apps.is_empty() {
                return Ok(json!({
                    "status": "error",
                    "message": format!("No apps configured for repository '{}'", repo_to_open)
                }));
            }
            apps[0].0.clone()
        };

        // Open the repository
        ws.open_repo_with_app(&repo_to_open, &app_to_use).await?;

        // Update state with this access
        state.add_recent_repo(
            repo_to_open.clone(),
            repo_info.path.clone(),
            Some(app_to_use.clone()),
        );
        state.save()?;

        Ok(json!({
            "status": "success",
            "repository": repo_to_open,
            "app": app_to_use
        }))
    }
}

/// MCP tool for opening a repository
pub struct OpenRepoTool;

#[async_trait]
impl VibeToolHandler for OpenRepoTool {
    fn tool_name(&self) -> &str {
        "open_repo"
    }

    fn tool_description(&self) -> &str {
        "Open repository with configured app"
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
                    "description": "App to open with",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "no_itermocil": {
                    "type": "boolean",
                    "description": "Disable iTermocil for iTerm2 (use Dynamic Profiles instead)",
                    "default": false
                }
            },
            "required": ["repo"]
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

        let app = args.get("app").and_then(|v| v.as_str());
        let no_itermocil = args
            .get("no_itermocil")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ws = workspace.lock().await;

        if let Some(app_name) = app {
            // Open with specific app
            if no_itermocil {
                ws.open_repo_with_app_options(repo, app_name, no_itermocil)
                    .await?;
            } else {
                ws.open_repo_with_app(repo, app_name).await?;
            }

            Ok(json!({
                "status": "success",
                "repository": repo,
                "app": app_name
            }))
        } else {
            // Open with default or show available apps
            let apps = ws.list_apps_for_repo(repo)?;
            if apps.is_empty() {
                Ok(json!({
                    "status": "error",
                    "message": format!("No apps configured for repository '{}'. Configure with: vibe apps configure {} <app>", repo, repo)
                }))
            } else if apps.len() == 1 {
                // Only one app configured, use it
                let (app_name, _) = &apps[0];
                ws.open_repo_with_app(repo, app_name).await?;

                Ok(json!({
                    "status": "success",
                    "repository": repo,
                    "app": app_name
                }))
            } else {
                // Multiple apps configured, return options
                Ok(json!({
                    "status": "multiple_apps",
                    "repository": repo,
                    "available_apps": apps.into_iter().map(|(app, template)| {
                        json!({
                            "app": app,
                            "template": template
                        })
                    }).collect::<Vec<_>>(),
                    "message": "Multiple apps configured. Please specify one with the 'app' parameter."
                }))
            }
        }
    }
}

/// MCP tool for cloning and opening a repository
pub struct CloneAndOpenTool;

#[async_trait]
impl VibeToolHandler for CloneAndOpenTool {
    fn tool_name(&self) -> &str {
        "clone_and_open"
    }

    fn tool_description(&self) -> &str {
        "Clone, configure, and open a repository in one command"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Repository URL or GitHub shorthand (owner/repo)"
                },
                "app": {
                    "type": "string",
                    "description": "App to open with after cloning",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "no_configure": {
                    "type": "boolean",
                    "description": "Skip app configuration",
                    "default": false
                },
                "no_open": {
                    "type": "boolean",
                    "description": "Skip opening after clone",
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

        let app = args.get("app").and_then(|v| v.as_str()).map(String::from);
        let no_configure = args
            .get("no_configure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_open = args
            .get("no_open")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Use workflow system if not skipping steps
        if !no_configure || !no_open {
            let workflow = Box::new(CloneAndOpenWorkflow {
                url: url.to_string(),
                app,
            });

            let mut ws = workspace.lock().await;
            execute_workflow(workflow, &mut *ws).await?;

            Ok(json!({
                "status": "success",
                "message": "Repository cloned and opened successfully"
            }))
        } else {
            // Just clone without workflow
            let git_config = crate::git::GitConfig::default();
            let mut ws = workspace.lock().await;
            let cloned_path = crate::git::CloneCommand::execute(
                url.to_string(),
                None,
                false,
                false,
                &mut *ws,
                &git_config,
            )
            .await?;

            Ok(json!({
                "status": "success",
                "message": "Repository cloned successfully",
                "path": cloned_path.to_string_lossy()
            }))
        }
    }
}
