//! Repository operation MCP tool handlers

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::ui::state::VibeState;
use crate::ui::workflows::{execute_workflow, CloneWorkflow};
use crate::workspace::WorkspaceManager;

/// MCP tool for launching a repository
pub struct LaunchRepoTool;

#[async_trait]
impl VibeToolHandler for LaunchRepoTool {
    fn tool_name(&self) -> &str {
        "launch_repo"
    }

    fn tool_description(&self) -> &str {
        "Interactive recent repository selector (1-9)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn handle_call(
        &self,
        _args: Value,
        _workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        // Load state to get recent repos
        let state = VibeState::load().unwrap_or_default();
        let recent_repos = state.get_recent_repos(9);

        if recent_repos.is_empty() {
            return Ok(json!({
                "status": "empty",
                "message": "No recent repositories found. Use the interactive menu with 'vibe' to browse repositories."
            }));
        }

        // Always return the recent options for user selection
        let options: Vec<_> = recent_repos
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                json!({
                    "number": index + 1,
                    "repo": entry.repo_id,
                    "path": entry.path.to_string_lossy(),
                    "last_app": entry.last_app
                })
            })
            .collect();

        Ok(json!({
            "status": "success",
            "message": "Recent repositories:",
            "options": options,
            "instruction": "Use 'open_repo' tool with a repository name to open one, or use the interactive menu."
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
            // Open with preferred or show available apps
            let configured_apps = ws.list_apps_for_repo(repo)?;
            if configured_apps.len() == 1 {
                // Only one app configured, use it
                let (app_name, _) = &configured_apps[0];
                ws.open_repo_with_app_options(repo, app_name, false).await?;

                Ok(json!({
                    "status": "success",
                    "repository": repo,
                    "app": app_name,
                    "mode": "configured"
                }))
            } else if configured_apps.len() > 1 {
                // Multiple apps configured, return options
                let options: Vec<String> = configured_apps
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect();
                Ok(json!({
                    "status": "multiple_options",
                    "message": format!("Multiple apps configured for '{}'. Specify one with app parameter.", repo),
                    "configured_apps": options
                }))
            } else {
                // No apps configured, try with most common available app (VS Code)
                for app in &["vscode", "cursor", "warp", "iterm2"] {
                    if ws.is_app_available(app).await {
                        ws.open_repo_with_app_options(repo, app, false).await?;
                        return Ok(json!({
                            "status": "success",
                            "repository": repo,
                            "app": app,
                            "mode": "basic",
                            "message": format!("Opened with {} in basic mode. Configure templates with: vibe apps configure {} {}", app, repo, app)
                        }));
                    }
                }

                Ok(json!({
                    "status": "error",
                    "message": format!("No supported apps found on system for repository '{}'", repo)
                }))
            }
        }
    }
}

/// MCP tool for cloning and opening a repository
pub struct CloneTool;

#[async_trait]
impl VibeToolHandler for CloneTool {
    fn tool_name(&self) -> &str {
        "clone"
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
            let workflow = Box::new(CloneWorkflow {
                url: url.to_string(),
                app,
            });

            let mut ws = workspace.lock().await;
            execute_workflow(workflow, &mut ws).await?;

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
                &mut ws,
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

/// MCP tool for creating a new repository
pub struct CreateRepositoryTool;

#[async_trait]
impl VibeToolHandler for CreateRepositoryTool {
    fn tool_name(&self) -> &str {
        "create_repository"
    }

    fn tool_description(&self) -> &str {
        "Create a new local repository in the workspace"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Repository name (optional - will prompt if not provided)"
                },
                "owner": {
                    "type": "string",
                    "description": "Repository owner (GitHub username or org - will detect if not provided)"
                },
                "app": {
                    "type": "string",
                    "description": "App to configure and open with after creation",
                    "enum": ["warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"]
                },
                "skip_github_check": {
                    "type": "boolean",
                    "description": "Skip GitHub availability check",
                    "default": false
                },
                "no_configure": {
                    "type": "boolean",
                    "description": "Skip app configuration",
                    "default": false
                },
                "no_open": {
                    "type": "boolean",
                    "description": "Skip opening after create",
                    "default": false
                }
            }
        })
    }

    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        use crate::repository::RepositoryCreator;

        let suggested_name = args.get("name").and_then(|v| v.as_str()).map(String::from);
        let suggested_owner = args.get("owner").and_then(|v| v.as_str()).map(String::from);
        let suggested_app = args.get("app").and_then(|v| v.as_str()).map(String::from);
        let skip_github_check = args
            .get("skip_github_check")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_configure = args
            .get("no_configure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_open = args
            .get("no_open")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut ws = workspace.lock().await;
        let workspace_root = ws.get_workspace_root().clone();
        let creator = RepositoryCreator::new(workspace_root);

        // Get GitHub user info if not skipping
        let (owner, repo_name) = if skip_github_check {
            let owner = suggested_owner
                .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "user".to_string()));
            let name = suggested_name.ok_or_else(|| {
                anyhow::anyhow!("Repository name is required when skipping GitHub check")
            })?;
            (owner, name)
        } else {
            let user_info = creator.get_github_user_info().await.map_err(|e| {
                anyhow::anyhow!("Failed to get GitHub info. Use skip_github_check=true to continue without GitHub: {}", e)
            })?;

            let owner = suggested_owner.unwrap_or(user_info.username);
            let name =
                suggested_name.ok_or_else(|| anyhow::anyhow!("Repository name is required"))?;

            // Validate repository name
            creator.validate_repository_name(&name)?;

            // Check availability unless skipped
            let available = creator.check_repository_availability(&owner, &name).await?;
            if !available {
                return Ok(json!({
                    "status": "warning",
                    "message": format!("Repository {}/{} already exists on GitHub. Created locally anyway.", owner, name),
                    "github_exists": true
                }));
            }

            (owner, name)
        };

        // Create the repository
        let repo_path = creator
            .create_local_repository(&owner, &repo_name, &mut ws)
            .await?;

        // Configure and open based on parameters
        if let Some(app) = suggested_app {
            if !no_configure {
                ws.configure_app_for_repo(&repo_name, &app, "default")
                    .await?;
            }

            if !no_open {
                // Open the repository
                ws.open_repo_with_app(&repo_name, &app).await?;
            }

            let message = match (no_configure, no_open) {
                (true, true) => "Repository created successfully",
                (true, false) => "Repository created and opened successfully",
                (false, true) => "Repository created and configured successfully",
                (false, false) => "Repository created, configured, and opened successfully",
            };

            Ok(json!({
                "status": "success",
                "message": message,
                "repository": repo_name,
                "owner": owner,
                "path": repo_path.to_string_lossy(),
                "app": app
            }))
        } else {
            Ok(json!({
                "status": "success",
                "message": "Repository created successfully",
                "repository": repo_name,
                "owner": owner,
                "path": repo_path.to_string_lossy(),
                "note": "Use apps configure or open commands to set up development environment"
            }))
        }
    }
}
