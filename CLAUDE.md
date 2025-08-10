# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Core Commands

### Development Workflow
```bash
# Build and development
just build                    # Debug build
just build-release           # Release build with optimization
just install                 # Install from zigbuild release artifacts
just run -- [args]          # Run with arguments (e.g., just run -- --help)

# Testing and validation
just test                    # Run all tests
just test-mcp               # Run MCP-specific tests
just pre-commit             # Full validation (check, format, clippy, tests)

# Code quality
just check                  # Cargo check (fast compilation check)
just fmt                    # Format code
just clippy                 # Run clippy linter
just clean                  # Clean build artifacts
```

### Cross-Platform Release
```bash
# Build all platform binaries
just zigbuild-release       # Build for Linux x86_64, macOS x86_64/ARM64/Universal2

# Complete release workflow
just release-all            # Full release: validation → GitHub → cargo publish
just validate-artifacts     # Validate built binaries
just create-github-release  # Create GitHub release + upload artifacts
just cargo-publish          # Publish to crates.io
```

### MCP Development
```bash
# Interactive MCP testing
just mcp-inspector          # Launch Inspector UI for visual testing
just mcp-inspector-cli      # CLI mode testing
just mcp-validate           # Validate MCP-CLI consistency
```

## Architecture Overview

### Core Entry Point
- `src/main.rs` - CLI argument parsing using Clap, command routing, and application orchestration
- Binary name: `vibe` (defined in `Cargo.toml`)
- Uses async runtime (Tokio) with full feature set

### Module Structure

#### Workspace Management (`src/workspace/`)
- **`manager.rs`** - Core `WorkspaceManager` struct that coordinates all operations
- **`config.rs`** - Configuration structures for repositories, apps, and workspace settings
- **`constants.rs`** - Paths and configuration directories (`~/.toolprint/vibe-workspace/`)
- **`discovery.rs`** - Repository discovery and scanning logic
- **`operations.rs`** - Git operations (status, sync, execute commands)
- **`templates.rs`** - Template management for app integrations
- **`install.rs`** - Installation and setup workflows

#### Application Integrations (`src/apps/`)
- **Per-app modules**: `warp/`, `iterm2.rs`, `vscode.rs`, `cursor.rs`, `wezterm.rs`, `windsurf.rs`
- **`app_manager.rs`** - Unified app management and detection
- **`registry.rs`** - App registry and capability detection
- **`installer_ui.rs`** - Interactive installer for developer tools
- **`package_manager/`** - Package manager integrations (Homebrew, Cargo, etc.)

#### Git Operations (`src/git/`)
- **`clone.rs`** - Enhanced clone operations with automatic detection
- **`bulk_clone.rs`** - Bulk repository cloning for users/organizations
- **`search.rs`** - Repository search functionality
- **`provider/github_cli.rs`** - GitHub CLI integration for repo operations

#### User Interface (`src/ui/`)
- **`prompts.rs`** - Interactive command-line prompts and menu system
- **`quick_launcher.rs`** - Recent repository launcher with fuzzy search
- **`workflows.rs`** - Multi-step workflow system (setup, repository creation)
- **`smart_menu.rs`** - Context-aware menu system
- **`state.rs`** - User state management (`VibeState`)

#### MCP Integration (`src/mcp/`)
- **`server.rs`** - Model Context Protocol server implementation
- **`handlers/`** - Tool handlers for apps, config, git, repos, validation
- **`types.rs`** - MCP-specific type definitions
- **`registry.rs`** - Tool registration and management

#### Supporting Systems
- **`cache/`** - Performance caching (repository metadata, git status)
- **`output/`** - Logging, display formatting, and output management
- **`utils/`** - Shared utilities (filesystem, git, platform detection)
- **`uri/`** - URI scheme handling for deep linking

### Configuration Structure
- **Main config**: `~/.toolprint/vibe-workspace/config.yaml` - Repository definitions, app settings
- **User state**: `~/.toolprint/vibe-workspace/state.json` - Recent repos, preferences, setup status  
- **Templates**: `~/.toolprint/vibe-workspace/templates/` - App-specific configuration templates
- **Cache**: `~/.toolprint/vibe-workspace/cache/` - SQLite databases for performance

### Key Dependencies
- **CLI Framework**: `clap` with derive features for command parsing
- **Async Runtime**: `tokio` with full feature set
- **Git Operations**: `git2` with vendored OpenSSL/libgit2 
- **Interactive UI**: `inquire`, `dialoguer` for prompts
- **Database**: `rusqlite` + `tokio-rusqlite` for caching
- **MCP Server**: `ultrafast-mcp` for Model Context Protocol support
- **Cross-compilation**: Uses `cargo-zigbuild` Docker image for multi-platform builds

### Testing Strategy
- Unit tests throughout modules
- MCP-specific integration tests (run with `just test-mcp`)
- Pre-commit validation ensures code quality before commits
- MCP Inspector provides interactive testing of protocol tools

### Build System
- **Development**: Standard `cargo build/test` workflow
- **Release**: `cargo-zigbuild` for cross-platform binaries (Linux x86_64, macOS x86_64/ARM64/Universal2)  
- **Publishing**: Dual distribution via GitHub releases (cargo-binstall) and crates.io
- **Optimization**: Release builds use size optimization (`opt-level = "z"`, LTO enabled)

## Important Implementation Notes

### Path Handling in Bulk Clone Operations
The bulk clone system (`src/git/bulk_clone.rs`) uses proper path calculation that mirrors the regular installer logic:
- Repositories can be stored as `workspace_root/org/repo` (when `git_config.standardize_paths = true`)
- Or as `workspace_root/repo` (when `git_config.standardize_paths = false`)
- Always use `calculate_install_path()` and `parse_git_url()` methods for consistent path handling

### Error Handling Philosophy
- Use `anyhow::Result` for most operations
- Convert detailed errors to user-friendly messages in UI layers
- Preserve error context through the call stack for debugging

### Git Configuration
- Git operations respect user's `.gitconfig` settings through the `GitConfig` struct
- Path standardization is controlled by `git_config.standardize_paths` setting
- Always pass `GitConfig` to git-related operations for consistent behavior

### MCP Tool Development
- All MCP tools must be registered in `src/mcp/server.rs`
- Tool handlers go in `src/mcp/handlers/`
- Use the `validate_mcp_interface` tool to ensure CLI-MCP consistency
- Test new tools with `just mcp-inspector` before releasing