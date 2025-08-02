//! Validation tool for MCP-CLI consistency

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mcp::types::VibeToolHandler;
use crate::workspace::WorkspaceManager;

/// MCP tool for validating MCP interface consistency with CLI
pub struct ValidateMcpInterfaceTool;

#[async_trait]
impl VibeToolHandler for ValidateMcpInterfaceTool {
    fn tool_name(&self) -> &str {
        "validate_mcp_interface"
    }

    fn tool_description(&self) -> &str {
        "Tests all MCP tools for consistency with CLI interface"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "verbose": {
                    "type": "boolean",
                    "description": "Show detailed validation output",
                    "default": false
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
        let verbose = args
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Define expected MCP tools and their CLI equivalents
        let tool_mappings = vec![
            // Configuration tools
            ("init_workspace", "vibe init"),
            ("show_config", "vibe config show"),
            ("init_config", "vibe config init"),
            ("validate_config", "vibe config validate"),
            ("reset_config", "vibe config reset"),
            ("backup_config", "vibe config backup"),
            ("restore_config", "vibe config restore"),
            // App management tools
            ("configure_app", "vibe apps configure"),
            ("show_apps", "vibe apps show"),
            ("list_app_templates", "vibe apps template list"),
            ("create_app_template", "vibe apps template create"),
            ("delete_app_template", "vibe apps template delete"),
            (
                "update_default_templates",
                "vibe apps template update-defaults",
            ),
            // Repository tools
            ("launch_repo", "vibe launch"),
            ("open_repo", "vibe open"),
            ("clone", "vibe clone"),
            // Git operation tools
            ("vibe_git_status", "vibe git status"),
            ("scan_repos", "vibe git scan"),
            ("sync_repos", "vibe git sync"),
            ("clone_repo", "vibe git clone"),
            ("exec_git_command", "vibe git exec"),
            ("reset_git_config", "vibe git reset"),
        ];

        let mut validation_results = Vec::new();
        let mut all_valid = true;

        // Check if each tool exists in the registry
        use crate::mcp::handlers;
        use crate::mcp::registry::ToolRegistryBuilder;

        let registry = ToolRegistryBuilder::new()
            // Configuration management tools
            .with_tool(Arc::new(handlers::InitWorkspaceTool))
            .with_tool(Arc::new(handlers::ShowConfigTool))
            .with_tool(Arc::new(handlers::InitConfigTool))
            .with_tool(Arc::new(handlers::ValidateConfigTool))
            .with_tool(Arc::new(handlers::ResetConfigTool))
            .with_tool(Arc::new(handlers::BackupConfigTool))
            .with_tool(Arc::new(handlers::RestoreConfigTool))
            // App management tools
            .with_tool(Arc::new(handlers::ConfigureAppTool))
            .with_tool(Arc::new(handlers::ShowAppsTool))
            .with_tool(Arc::new(handlers::ListAppTemplatesTool))
            .with_tool(Arc::new(handlers::CreateAppTemplateTool))
            .with_tool(Arc::new(handlers::DeleteAppTemplateTool))
            .with_tool(Arc::new(handlers::UpdateDefaultTemplatesTool))
            // Repository operation tools
            .with_tool(Arc::new(handlers::LaunchRepoTool))
            .with_tool(Arc::new(handlers::OpenRepoTool))
            .with_tool(Arc::new(handlers::CloneTool))
            // Git operation tools
            .with_tool(Arc::new(handlers::GitStatusTool))
            .with_tool(Arc::new(handlers::ScanReposTool))
            .with_tool(Arc::new(handlers::SyncReposTool))
            .with_tool(Arc::new(handlers::CloneRepoTool))
            .with_tool(Arc::new(handlers::ExecGitCommandTool))
            .with_tool(Arc::new(handlers::ResetGitConfigTool))
            .build();

        let available_tools = registry.list_tools();
        let tool_names: Vec<String> = available_tools
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect();

        for (mcp_tool, cli_command) in &tool_mappings {
            let exists = tool_names.contains(&mcp_tool.to_string());

            if !exists {
                all_valid = false;
            }

            let result = json!({
                "mcp_tool": mcp_tool,
                "cli_command": cli_command,
                "exists": exists,
                "status": if exists { "✓" } else { "✗" }
            });

            validation_results.push(result);
        }

        // Check for tools that exist but aren't mapped
        let mapped_tools: Vec<&str> = tool_mappings.iter().map(|(tool, _)| *tool).collect();
        let unmapped_tools: Vec<String> = tool_names
            .into_iter()
            .filter(|name| !mapped_tools.contains(&name.as_str()))
            .collect();

        // Summary statistics
        let total_expected = tool_mappings.len();
        let total_found = validation_results
            .iter()
            .filter(|r| r["exists"].as_bool().unwrap_or(false))
            .count();
        let total_missing = total_expected - total_found;

        let response = json!({
            "valid": all_valid,
            "summary": {
                "total_expected": total_expected,
                "total_found": total_found,
                "total_missing": total_missing,
                "unmapped_tools": unmapped_tools,
            },
            "results": if verbose { Value::Array(validation_results) } else { Value::Null }
        });

        Ok(response)
    }
}
