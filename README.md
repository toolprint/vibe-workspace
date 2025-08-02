# Vibe Workspace

[![MCP](https://badge.mcpx.dev?type=server&features=resources,tools)](https://modelcontextprotocol.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/rust-1.88.0%2B-orange.svg)](https://forge.rust-lang.org/releases.html)
[![Cargo](https://img.shields.io/crates/v/vibe-workspace.svg)](https://crates.io/crates/vibe-workspace)
[![Downloads](https://img.shields.io/crates/d/vibe-workspace.svg)](https://crates.io/crates/vibe-workspace)

A lightweight CLI tool designed to help developers quickly adopt vibe-coding patterns across multiple repositories.

Vibe Workspace streamlines your development workflow by providing consistent environments, tooling configurations, and coding patterns for modern AI Agentic coding.

## Features

- 🚀 **Instant Setup** - Get productive in seconds with pre-configured vibe patterns
- 🎯 **Pattern Enforcement** - Consistent coding patterns across all your repositories
- 📁 **Multi-Repository Management** - Orchestrate vibe patterns across entire organizations
- 🔧 **Developer Tool Integration** - Pre-configured Warp, iTerm2, WezTerm, VS Code, Cursor, and Windsurf templates
- 🎨 **Vibe Templates** - Ready-to-use configurations for different tech stacks
- 🔄 **Pattern Propagation** - Apply vibe patterns to new and existing repositories
- 📱 **Menu Mode** - Guided setup for vibe-coding workflows
- 🤖 **MCP Support** - Model Context Protocol server for AI integration
- 🛠️ **Extensible** - Create custom vibe patterns for your team

## Platform Support

Currently, **macOS is the only officially tested and supported platform**, with universal binaries available for both arm64 and amd64 architectures. 

We cross-compile to Linux distributions, but full testing and support is not yet complete. Windows support is technically possible but not currently on the short-term roadmap unless highly requested by the community.

## Installation

### Recommended (macOS)

For the fastest installation on macOS, first ensure you have `cargo-binstall`:

```bash
brew install cargo-binstall
```

Then install vibe-workspace:

```bash
cargo binstall vibe-workspace
```

### From Cargo (Build from Source)

```bash
cargo install vibe-workspace
```

### From Source

```bash
git clone https://github.com/toolprint/vibe-workspace
cd vibe-workspace
cargo install --path .
```

## Quick Start

Get started with the interactive setup wizard:

```bash
# Run the setup wizard (recommended for first-time users)
vibe setup

# Or launch the interactive menu
vibe
```

The setup wizard will:
1. **Discover repositories** in your workspace automatically
2. **Check installed apps** (VS Code, Warp, iTerm2, WezTerm, Cursor, Windsurf)
3. **Configure default app** for opening repositories

After setup, use these essential commands:
```bash
vibe launch 1              # Quick launch recent repository
vibe go <github-url>       # Clone, configure, and open in one command
vibe                       # Interactive menu with smart actions
```

For detailed getting started guide, see [Quick Start Guide](docs/QUICK_START.md).

## Supported Applications

- **[Warp Terminal](https://www.warp.dev/)** - Modern terminal with AI features and collaborative tools
- **[iTerm2](https://iterm2.com/)** - Feature-rich terminal emulator for macOS
- **[WezTerm](https://wezfurlong.org/wezterm/)** - GPU-accelerated cross-platform terminal
- **[Visual Studio Code](https://code.visualstudio.com/)** - Popular code editor with extensive plugin ecosystem
- **[Cursor](https://cursor.sh/)** - AI-first code editor with built-in AI assistance and chat
- **[Windsurf](https://codeium.com/windsurf)** - Agentic IDE powered by AI Flow paradigm

For detailed app configuration, templates, and additional developer tools, see [App Integration Guide](docs/APPS.md).

## Configuration

Vibe stores its configuration and data in `~/.toolprint/vibe-workspace/`:

```
~/.toolprint/vibe-workspace/
├── config.yaml          # Main workspace configuration
├── state.json           # User preferences and recent repositories
├── templates/           # App-specific templates
│   ├── warp/
│   ├── iterm2/
│   ├── wezterm/
│   ├── vscode/
│   ├── cursor/
│   └── windsurf/
├── cache/               # Performance caches
│   ├── repositories.db  # Repository metadata cache
│   └── git_status.db   # Git status cache
└── backups/            # Configuration backups
```

**Key Configuration Files:**
- `config.yaml` - Repository definitions, app settings, and workspace configuration
- `state.json` - Recent repositories, user preferences, and setup completion status
- `templates/` - Customizable templates for how apps open repositories

Use these commands to manage configuration:
```bash
vibe config show           # View current configuration
vibe config edit           # Edit configuration file
vibe config backup         # Create backup archive
vibe config reset          # Factory reset (with confirmation)
```

## MCP Integration

vibe-workspace includes built-in MCP (Model Context Protocol) server capabilities for AI integration.

### Claude Code Integration

After installing the vibe-workspace binary on your system, you can add it to Claude Code with:

```bash
claude mcp add -s user -t stdio vibe vibe mcp
```

For detailed MCP configuration and capabilities, see [MCP Documentation](docs/MCP.md).

## Development

### Building

```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release

# Run tests
cargo test
```

### Architecture

The project is organized into modular components:

- `apps/` - Application integrations (Warp, iTerm2, VS Code, WezTerm, Cursor, Windsurf) and installer
- `cache/` - Performance caching system (repository metadata, git status)
- `git/` - Git operations (clone, search, status) and provider integrations
- `mcp/` - Model Context Protocol server for AI integration
- `ui/` - Terminal UI components (prompts, menus, workflows, smart actions)
- `uri/` - URI scheme handling for deep linking
- `utils/` - Shared utilities (filesystem, git, platform detection)
- `workspace/` - Core workspace management (config, discovery, operations)

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md) before submitting PRs.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Special thanks to **Anthropic** for their groundbreaking work with Claude Code, making agentic coding so powerful and accessible. Many of the best practices from [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices) have influenced the design and philosophy of vibe-workspace.

Built with:

- [Clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [Inquire](https://github.com/mikaelmello/inquire) - Interactive prompts
- [git2](https://github.com/rust-lang/git2-rs) - Git operations
- [Tokio](https://tokio.rs/) - Async runtime

---

**Start vibe-coding today and transform how your team builds software!**

Made with ❤️ by [Toolprint](https://www.toolprint.ai/)