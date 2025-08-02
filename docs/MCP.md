# MCP (Model Context Protocol) Integration

vibe-workspace includes built-in MCP server capabilities that allow AI models to interact with your workspace through standardized tools.

## Overview

The MCP integration provides:
- **Modular Tool System**: Easy-to-add tools that expose workspace functionality
- **Type-Safe Implementation**: Rust traits ensure consistent tool behavior
- **Workspace Integration**: Direct access to vibe's workspace management features
- **Testing Framework**: Comprehensive testing using MCP Inspector and direct JSON-RPC

## Architecture

### Core Components

1. **VibeToolHandler Trait** (`src/mcp/types.rs`)
   - Defines the interface for all MCP tools
   - Provides async execution with workspace access
   - Handles JSON schema generation for tool discovery

2. **Tool Registry** (`src/mcp/registry.rs`)
   - Manages available tools
   - Handles tool discovery and invocation
   - Provides thread-safe access to tools

3. **MCP Server** (`src/mcp/server.rs`)
   - Bridges ultrafast-mcp framework with vibe
   - Manages stdio transport for communication
   - Handles tool registration and routing

## Running the MCP Server

### Start the Server

```bash
# Run MCP server on stdio (recommended for AI tools)
vibe mcp --stdio

# Run MCP server on TCP port (for debugging) - Coming soon
vibe mcp --port 3000
```

### Using with AI Tools

Configure your AI tool to use vibe as an MCP server:

```json
{
  "mcpServers": {
    "vibe": {
      "command": "vibe",
      "args": ["mcp", "--stdio"]
    }
  }
}
```

For Claude Desktop specifically, add this to your `claude_desktop_config.json` with the full path to your vibe binary.

## Available Tools

### launch_repo
Interactive recent repository selector that shows available recent repositories for user selection.

**Parameters:**
- None - This tool provides an interactive selection interface

**Example:**
```json
{}
```

**Response Example:**
```json
{
  "status": "success",
  "message": "Recent repositories:",
  "options": [
    {
      "number": 1,
      "repo": "my-repo",
      "path": "/Users/dev/workspace/my-repo",
      "last_app": "cursor"
    }
  ],
  "instruction": "Use 'open_repo' tool with a repository name to open one, or use the interactive menu."
}
```

**Features:**
- Lists up to 9 most recent repositories
- Shows last used app for each repository
- Provides instructions for next steps
- Returns empty status if no recent repositories found

### git_status
Get git status for all repositories in the workspace.

**Parameters:**
- `dirty_only` (boolean, optional): Show only repositories with uncommitted changes
- `format` (string, optional): Output format - "json" (default), "table", or "compact"
- `group` (string, optional): Filter by repository group name

**Example:**
```json
{
  "dirty_only": true,
  "format": "json"
}
```

**Response Example:**
```json
{
  "repositories": [
    {
      "name": "my-repo",
      "path": "/path/to/my-repo",
      "branch": "main",
      "status": "Clean",
      "ahead": 0,
      "behind": 0
    }
  ],
  "total": 5,
  "dirty_count": 0
}
```

### create_repository
Create a new local repository in the workspace with development-ready templates.

**Parameters:**
- `name` (string, optional): Repository name - will prompt interactively if not provided
- `owner` (string, optional): Repository owner (GitHub username or org) - will auto-detect if not provided
- `app` (string, optional): App to configure and open with after creation - one of: "warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"
- `skip_github_check` (boolean, optional): Skip GitHub availability check - defaults to false
- `no_configure` (boolean, optional): Skip app configuration - defaults to false
- `no_open` (boolean, optional): Skip opening after create - defaults to false

**Example:**
```json
{
  "name": "my-prototype",
  "owner": "myorg",
  "app": "cursor"
}
```

**Example with skip flags:**
```json
{
  "name": "my-prototype",
  "app": "vscode",
  "no_configure": true,
  "no_open": true
}
```

**Response Example:**
```json
{
  "status": "success",
  "repository": "my-prototype",
  "owner": "myorg",
  "path": "/Users/dev/workspace/myorg/my-prototype",
  "app": "cursor",
  "message": "Repository created, configured, and opened successfully"
}
```

**Features:**
- GitHub organization detection via GitHub CLI
- Repository name validation for GitHub compatibility
- Default development templates (README, .gitignore, src/, docs/TODO.md)
- Automatic git initialization with initial commit
- Seamless app configuration and launch

### clone
Clone, configure, and open an existing repository in one command.

**Parameters:**
- `url` (string, required): Repository URL or GitHub shorthand (owner/repo)
- `app` (string, optional): App to open with after cloning - one of: "warp", "iterm2", "vscode", "wezterm", "cursor", "windsurf"
- `no_configure` (boolean, optional): Skip app configuration - defaults to false
- `no_open` (boolean, optional): Skip opening after clone - defaults to false

**Example:**
```json
{
  "url": "https://github.com/owner/repo",
  "app": "cursor"
}
```

**Response Example:**
```json
{
  "status": "success",
  "message": "Repository cloned and opened successfully"
}
```

## Testing

### Testing with MCP Inspector

The MCP Inspector provides both visual and programmatic ways to test and debug MCP servers:

#### Visual UI Mode

1. **Launch Inspector UI**:
   ```bash
   just mcp-inspector
   ```
   Opens a web UI at http://localhost:6274 for interactive testing.

**UI Features**:
- Visual request/response debugging
- Interactive tool testing with custom inputs
- Real-time protocol monitoring
- Export configurations for Claude/Cursor
- Session token authentication for security

#### CLI Mode (Non-Interactive)

The Inspector also supports a powerful CLI mode for programmatic testing using the `--cli` flag:

1. **Basic CLI Usage**:
   ```bash
   just mcp-inspector-cli
   ```
   Lists all available tools in CLI mode.

2. **List Tools**:
   ```bash
   just mcp-inspector-list-tools
   ```

3. **Call a Tool**:
   ```bash
   just mcp-inspector-call-tool git_status '{"dirty_only": true}'
   ```

4. **View Examples**:
   ```bash
   just mcp-inspector-cli-examples
   ```

**CLI Mode Benefits**:
- Scriptable and automatable
- Ideal for CI/CD integration
- No browser required
- Direct command-line output
- Works with ultrafast-mcp despite protocol version issues

**CLI Mode Syntax**:
```bash
npx @modelcontextprotocol/inspector --cli <server_command> --method <method> [options]
```

Available methods:
- `tools/list` - List all available tools
- `tools/call` - Call a specific tool with parameters

## Adding New Tools

To add a new MCP tool:

1. **Create tool handler** in `src/mcp/handlers/`:
   ```rust
   use crate::mcp::types::VibeToolHandler;
   
   pub struct MyNewTool;
   
   #[async_trait]
   impl VibeToolHandler for MyNewTool {
       fn tool_name(&self) -> &str {
           "vibe_my_tool"
       }
       
       fn tool_description(&self) -> &str {
           "Description of what the tool does"
       }
       
       fn input_schema(&self) -> Value {
           json!({
               "type": "object",
               "properties": {
                   "param1": {
                       "type": "string",
                       "description": "Parameter description"
                   }
               }
           })
       }
       
       async fn handle_call(
           &self,
           args: Value,
           workspace: Arc<Mutex<WorkspaceManager>>
       ) -> Result<Value> {
           // Tool implementation
           Ok(json!({"result": "success"}))
       }
   }
   ```

2. **Register the tool** in `src/mcp/server.rs`:
   ```rust
   registry.register(Arc::new(MyNewTool));
   ```

3. **Add tests** to the test scenarios in `tests/mcp/test_scenarios.json`

## Troubleshooting

### Known Issues

#### ultrafast-mcp Protocol Version Bug

**Issue**: The ultrafast-mcp library (version 202506018.1.0) has a bug in its protocol version negotiation. It expects a field called `version` in the initialize request parameters, but the MCP specification clearly states the field should be called `protocolVersion`.

**Error Message**:
```
Invalid initialize request: missing field `version`
```

**Impact**: This prevents standard MCP clients like mcptools from connecting to servers built with ultrafast-mcp.

**Status**:
- **Reported**: Not yet reported to ultrafast-mcp maintainers
- **Severity**: High - blocks interoperability with standard MCP clients
- **Affected Version**: ultrafast-mcp 202506018.1.0

**Workarounds**:

1. **Use MCP Inspector (Recommended)**: The MCP Inspector works correctly with our server and provides both visual and programmatic debugging. See the Testing section above for details.

2. **Direct JSON-RPC Testing**: Test the MCP server directly with JSON-RPC:
   ```bash
   # Test initialization
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | vibe mcp --stdio
   
   # Call a tool
   echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"git_status","arguments":{}}}' | vibe mcp --stdio
   ```

3. **Custom Test Scripts**: Use the provided test scripts in `tests/mcp/`:
   - `test_init.sh` - Tests initialization
   - `quick_test.sh` - Basic functionality test

4. **Alternative MCP Libraries**: Consider switching to a different Rust MCP implementation:
   - `rust-mcp-sdk` - Appears to follow the spec correctly
   - `mcp-core` - Another alternative

**Resolution Plan**:
1. Report the issue to ultrafast-mcp maintainers
2. Consider submitting a PR to fix the field name
3. In the meantime, document that mcptools is not compatible
4. Consider switching to a different MCP library if the issue isn't resolved

### Common Issues

1. **Server won't start**: Check if another process is using the port
2. **Tools not listed**: Ensure tools are properly registered in the server
3. **Invalid responses**: Validate JSON schema matches actual parameters
4. **mcptools compatibility**: Use MCP Inspector or direct JSON-RPC testing instead
5. **AI tool integration**: For AI tools that support MCP, you may need to patch their initialization code to send `version` instead of `protocolVersion`, or wait for a fix to ultrafast-mcp

### Debug Mode

Enable debug logging:
```bash
RUST_LOG=debug vibe mcp --stdio
```

### Test Logs

View test execution logs:
```bash
just mcp-test-with-logs
```

## Future Improvements

- [ ] Automatic tool registration from CLI commands
- [ ] JSON schema generation using schemars
