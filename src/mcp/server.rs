//! MCP server implementation for vibe-workspace

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use ultrafast_mcp::{
    ListToolsRequest, ListToolsResponse, MCPResult, ServerCapabilities, ServerInfo, Tool, ToolCall,
    ToolContent, ToolHandler, ToolResult, ToolsCapability, UltraFastServer,
};

use crate::workspace::WorkspaceManager;

use super::handlers;
use super::registry::{ToolRegistry, ToolRegistryBuilder};

/// MCP server for vibe-workspace
pub struct VibeMCPServer {
    registry: ToolRegistry,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl VibeMCPServer {
    /// Creates a new MCP server with the given workspace manager
    pub fn new(workspace_manager: Arc<Mutex<WorkspaceManager>>) -> Self {
        let registry = Self::build_registry();

        Self {
            registry,
            workspace_manager,
        }
    }

    /// Builds the tool registry with all available tools
    fn build_registry() -> ToolRegistry {
        ToolRegistryBuilder::new()
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
            .with_tool(Arc::new(handlers::CloneAndOpenTool))
            // Git operation tools
            .with_tool(Arc::new(handlers::GitStatusTool))
            .with_tool(Arc::new(handlers::ScanReposTool))
            .with_tool(Arc::new(handlers::SyncReposTool))
            .with_tool(Arc::new(handlers::CloneRepoTool))
            .with_tool(Arc::new(handlers::ExecGitCommandTool))
            .with_tool(Arc::new(handlers::ResetGitConfigTool))
            // Validation tool
            .with_tool(Arc::new(handlers::ValidateMcpInterfaceTool))
            .build()
    }

    /// Creates and configures the UltraFast MCP server
    pub fn create_server(self) -> UltraFastServer {
        let server_info = ServerInfo {
            name: "vibe-workspace".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some(
                "MCP server for vibe-workspace Git repository management".to_string(),
            ),
            homepage: Some("https://github.com/toolprint/vibe-workspace".to_string()),
            repository: Some("https://github.com/toolprint/vibe-workspace".to_string()),
            authors: Some(vec!["Brian Cripe <brian@onegrep.dev>".to_string()]),
            license: Some("MIT".to_string()),
        };

        let capabilities = ServerCapabilities {
            tools: Some(ToolsCapability {
                list_changed: Some(false),
            }),
            ..Default::default()
        };

        UltraFastServer::new(server_info, capabilities).with_tool_handler(Arc::new(self))
    }

    /// Runs the MCP server
    pub async fn run(self) -> Result<()> {
        info!("Starting vibe-workspace MCP server");

        let server = self.create_server();
        server
            .run_stdio()
            .await
            .map_err(|e| anyhow!("MCP server error: {}", e))
    }
}

/// Bridge between vibe ToolHandler trait and UltraFast ToolHandler trait
#[async_trait]
impl ToolHandler for VibeMCPServer {
    async fn handle_tool_call(&self, call: ToolCall) -> MCPResult<ToolResult> {
        // Delegate to our registry
        match self
            .registry
            .handle_call(
                &call.name,
                call.arguments
                    .unwrap_or(Value::Object(serde_json::Map::new())),
                self.workspace_manager.clone(),
            )
            .await
        {
            Ok(result) => {
                // Convert our result to MCP ToolResult
                let content = if let Some(text) = result.as_str() {
                    vec![ToolContent::text(text.to_string())]
                } else {
                    vec![ToolContent::text(
                        serde_json::to_string_pretty(&result).unwrap(),
                    )]
                };

                Ok(ToolResult {
                    content,
                    is_error: Some(false),
                })
            }
            Err(e) => {
                // Return error as tool result
                Ok(ToolResult {
                    content: vec![ToolContent::text(format!("Error: {}", e))],
                    is_error: Some(true),
                })
            }
        }
    }

    async fn list_tools(&self, _request: ListToolsRequest) -> MCPResult<ListToolsResponse> {
        let tools = self
            .registry
            .list_tools()
            .into_iter()
            .map(|(name, description, schema)| Tool {
                name,
                description,
                input_schema: schema,
                output_schema: None,
                annotations: None,
            })
            .collect();

        Ok(ListToolsResponse {
            tools,
            next_cursor: None,
        })
    }
}

/// Builder for creating an MCP server with custom configuration
pub struct MCPServerBuilder {
    #[allow(dead_code)]
    workspace_manager: Option<Arc<Mutex<WorkspaceManager>>>,
}

impl MCPServerBuilder {
    /// Creates a new MCP server builder
    pub fn new() -> Self {
        Self {
            workspace_manager: None,
        }
    }

    /// Sets the workspace manager
    #[allow(dead_code)]
    pub fn with_workspace_manager(mut self, manager: Arc<Mutex<WorkspaceManager>>) -> Self {
        self.workspace_manager = Some(manager);
        self
    }

    /// Builds the MCP server
    #[allow(dead_code)]
    pub fn build(self) -> Result<VibeMCPServer> {
        let workspace_manager = self
            .workspace_manager
            .ok_or_else(|| anyhow!("Workspace manager is required"))?;

        Ok(VibeMCPServer::new(workspace_manager))
    }
}

impl Default for MCPServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
