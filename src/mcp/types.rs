//! Core types and traits for MCP integration

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::workspace::WorkspaceManager;

/// Trait for implementing MCP tool handlers in vibe-workspace
#[async_trait]
pub trait VibeToolHandler: Send + Sync {
    /// Returns the name of the tool as it will be exposed via MCP
    fn tool_name(&self) -> &str;

    /// Returns a human-readable description of what the tool does
    fn tool_description(&self) -> &str;

    /// Returns the JSON schema defining the tool's input parameters
    fn input_schema(&self) -> Value;

    /// Handles the actual tool invocation
    ///
    /// # Arguments
    /// * `args` - The input arguments as a JSON value
    /// * `workspace` - Shared reference to the workspace manager
    ///
    /// # Returns
    /// The result of the tool execution as a JSON value
    async fn handle_call(
        &self,
        args: Value,
        workspace: Arc<Mutex<WorkspaceManager>>,
    ) -> Result<Value>;
}

/// Common result type for Git operations
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GitOperationResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Repository information for MCP responses
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
}

/// Git status information for MCP responses
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GitStatusInfo {
    pub repository: String,
    pub path: String,
    pub is_dirty: bool,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub has_untracked_files: bool,
    pub ahead: usize,
    pub behind: usize,
}

/// App configuration result
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AppConfigResult {
    pub repository: String,
    pub app: String,
    pub template: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
