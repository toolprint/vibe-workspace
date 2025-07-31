//! Tool registry system for managing and discovering MCP tools

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::workspace::WorkspaceManager;

use super::types::VibeToolHandler;

/// Registry for managing MCP tool handlers
pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn VibeToolHandler>>,
}

impl ToolRegistry {
    /// Creates a new empty tool registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a new tool handler
    ///
    /// # Arguments
    /// * `handler` - The tool handler to register
    pub fn register(&mut self, handler: Arc<dyn VibeToolHandler>) {
        let tool_name = handler.tool_name().to_string();
        info!("Registering MCP tool: {}", tool_name);
        self.handlers.insert(tool_name, handler);
    }

    /// Registers multiple tool handlers at once
    ///
    /// # Arguments
    /// * `handlers` - A vector of tool handlers to register
    pub fn register_all(&mut self, handlers: Vec<Arc<dyn VibeToolHandler>>) {
        for handler in handlers {
            self.register(handler);
        }
    }

    /// Gets a tool handler by name
    ///
    /// # Arguments
    /// * `name` - The name of the tool to retrieve
    ///
    /// # Returns
    /// The tool handler if found, None otherwise
    pub fn get(&self, name: &str) -> Option<&Arc<dyn VibeToolHandler>> {
        self.handlers.get(name)
    }

    /// Lists all registered tools
    ///
    /// # Returns
    /// A vector of tool information (name, description, schema)
    pub fn list_tools(&self) -> Vec<(String, String, Value)> {
        self.handlers
            .values()
            .map(|handler| {
                (
                    handler.tool_name().to_string(),
                    handler.tool_description().to_string(),
                    handler.input_schema(),
                )
            })
            .collect()
    }

    /// Handles a tool call by delegating to the appropriate handler
    ///
    /// # Arguments
    /// * `tool_name` - The name of the tool to invoke
    /// * `args` - The arguments to pass to the tool
    /// * `workspace` - The workspace manager instance
    ///
    /// # Returns
    /// The result of the tool invocation
    pub async fn handle_call(
        &self,
        tool_name: &str,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value> {
        self.handlers
            .get(tool_name)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?
            .handle_call(args, workspace)
            .await
    }

    /// Future: Automatically registers tools based on CLI commands
    /// This is a placeholder for future functionality
    #[allow(dead_code)]
    pub fn auto_register_from_commands(&mut self) {
        // TODO: Implement automatic tool generation from clap Commands
        // This would involve:
        // 1. Parsing the Commands enum
        // 2. Generating tool handlers for each command
        // 3. Mapping command arguments to tool parameters
        // 4. Creating appropriate JSON schemas
        info!("Auto-registration from commands not yet implemented");
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a pre-configured tool registry
pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
}

impl ToolRegistryBuilder {
    /// Creates a new tool registry builder
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    /// Adds a tool handler to the registry
    pub fn with_tool(mut self, handler: Arc<dyn VibeToolHandler>) -> Self {
        self.registry.register(handler);
        self
    }

    /// Builds the tool registry with all registered handlers
    pub fn build(self) -> ToolRegistry {
        self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct MockToolHandler;

    #[async_trait]
    impl VibeToolHandler for MockToolHandler {
        fn tool_name(&self) -> &str {
            "mock_tool"
        }

        fn tool_description(&self) -> &str {
            "A mock tool for testing"
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "test": {"type": "string"}
                }
            })
        }

        async fn handle_call(
            &self,
            _args: Value,
            _workspace: Arc<Mutex<WorkspaceManager>>,
        ) -> Result<Value> {
            Ok(json!({"result": "mock"}))
        }
    }

    #[test]
    fn test_tool_registration() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(MockToolHandler);

        registry.register(handler);

        assert!(registry.get("mock_tool").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_tools() {
        let mut registry = ToolRegistry::new();
        let handler = Arc::new(MockToolHandler);

        registry.register(handler);

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "mock_tool");
        assert_eq!(tools[0].1, "A mock tool for testing");
    }
}
