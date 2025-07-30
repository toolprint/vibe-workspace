# Vibe Workspace

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Cargo](https://img.shields.io/crates/v/vibe-workspace.svg)](https://crates.io/crates/vibe-workspace)

A lightweight CLI tool designed to help developers quickly adopt vibe-coding patterns across multiple repositories.

Vibe Workspace streamlines your development workflow by providing consistent environments, tooling configurations, and coding patterns for modern AI Agentic coding.

## Features

- 🚀 **Instant Setup** - Get productive in seconds with pre-configured vibe patterns
- 🎯 **Pattern Enforcement** - Consistent coding patterns across all your repositories
- 📁 **Multi-Repository Management** - Orchestrate vibe patterns across entire organizations
- 🔧 **Developer Tool Integration** - Pre-configured Warp, iTerm2, WezTerm, and VS Code templates
- 🎨 **Vibe Templates** - Ready-to-use configurations for different tech stacks
- 🔄 **Pattern Propagation** - Apply vibe patterns to new and existing repositories
- 📱 **Menu Mode** - Guided setup for vibe-coding workflows
- 🛠️ **Extensible** - Create custom vibe patterns for your team

## Installation

### From Cargo

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
2. **Check installed apps** (VS Code, Warp, iTerm2, WezTerm)
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

For detailed app configuration, templates, and additional developer tools, see [App Integration Guide](docs/APPS.md).

## Configuration

Vibe stores its configuration and data in `~/.vibe-workspace/`:

```
~/.vibe-workspace/
├── config.yaml          # Main workspace configuration
├── state.json           # User preferences and recent repositories
├── templates/           # App-specific templates
│   ├── warp/
│   ├── iterm2/
│   ├── wezterm/
│   └── vscode/
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

- `apps/` - Application integrations (Warp, iTerm2, VS Code, WezTerm) and installer
- `cache/` - Performance caching system (repository metadata, git status)
- `git/` - Git operations (clone, search, status) and provider integrations
- `ui/` - Terminal UI components (prompts, menus, workflows, smart actions)
- `uri/` - URI scheme handling for deep linking
- `utils/` - Shared utilities (filesystem, git, platform detection)
- `workspace/` - Core workspace management (config, discovery, operations)

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md) before submitting PRs.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with:
- [Clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [Inquire](https://github.com/mikaelmello/inquire) - Interactive prompts
- [git2](https://github.com/rust-lang/git2-rs) - Git operations
- [Tokio](https://tokio.rs/) - Async runtime

---

**Start vibe-coding today and transform how your team builds software!**

Made with ❤️ by [Toolprint](https://www.toolprint.ai/)