//! MCP (Model Context Protocol) server integration for vibe-workspace
//!
//! This module provides MCP server capabilities, exposing vibe-workspace
//! commands as tools that can be invoked by AI models.

pub mod handlers;
pub mod registry;
pub mod server;
pub mod types;

pub use server::VibeMCPServer;
