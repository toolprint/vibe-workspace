# Vibe Workspace

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Cargo](https://img.shields.io/crates/v/vibe-workspace.svg)](https://crates.io/crates/vibe-workspace)

A lightweight CLI tool designed to help developers quickly adopt vibe-coding patterns across multiple repositories. Vibe Workspace streamlines your development workflow by providing consistent environments, tooling configurations, and coding patterns for modern AI Agentic coding.

## Features

- 🚀 **Instant Setup** - Get productive in seconds with pre-configured vibe patterns
- 🎯 **Pattern Enforcement** - Consistent coding patterns across all your repositories
- 📁 **Multi-Repository Management** - Orchestrate vibe patterns across entire organizations
- 🔧 **Developer Tool Integration** - Pre-configured Warp, iTerm2, WezTerm, and VS Code templates
- 🎨 **Vibe Templates** - Ready-to-use configurations for different tech stacks
- 🔄 **Pattern Propagation** - Apply vibe patterns to new and existing repositories
- 📱 **Interactive Mode** - Guided setup for vibe-coding workflows
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

## Core Commands

### Workspace Management

```bash
# Initialize a new workspace
vibe init

# Interactive mode - manage everything through a TUI
vibe interactive

# Manage workspace configuration
vibe config show
vibe config set <key> <value>
```

### Repository Operations

```bash
# Git operations across repositories
vibe git status              # Show status of all repos
vibe git pull                 # Pull updates for all repos
vibe git clone <url>          # Clone and add to workspace

# Open repository with configured app
vibe open <repo-name> [--app <app>]
```

### App Integration

```bash
# Configure app for a repository
vibe apps configure <repo> <app> [--template <name>]

# Show app configurations
vibe apps show [--repo <name>] [--app <name>]

# Install developer tools
vibe apps install
```

### Template Management

```bash
# List templates for an app
vibe apps template list <app>

# Create custom template
vibe apps template create <app> <name> --from-file <path>

# Delete template
vibe apps template delete <app> <name>
```

## Supported Applications

- **[Warp Terminal](https://www.warp.dev/)** - Modern terminal with AI features and collaborative tools
- **[iTerm2](https://iterm2.com/)** - Feature-rich terminal emulator for macOS
- **[WezTerm](https://wezfurlong.org/wezterm/)** - GPU-accelerated cross-platform terminal
- **[Visual Studio Code](https://code.visualstudio.com/)** - Popular code editor with extensive plugin ecosystem

## Template Variables

All templates support these variables for customization:

- `{{workspace_name}}` - Name of your vibe workspace
- `{{repo_name}}` - Repository name
- `{{repo_path}}` - Full path to repository
- `{{repo_branch}}` - Default branch name
- `{{repo_url}}` - Repository URL

## Configuration

Vibe stores its configuration and templates in:

```
~/.vibe-workspace/
├── config.yaml          # Main workspace configuration
├── workspaces/         # Individual workspace configs
└── templates/          # App-specific templates
    ├── warp/
    ├── iterm2/
    ├── wezterm/
    └── vscode/
```

## Example: Onboarding a New Developer

Here's how vibe-workspace accelerates team onboarding:

```bash
# 1. New developer clones the team workspace
git clone https://github.com/company/team-vibe-patterns
cd team-vibe-patterns
vibe init --from-template ./company-vibe.yaml

# 2. Clone all team repositories with vibe patterns
vibe git clone https://github.com/company/frontend-app
vibe git clone https://github.com/company/backend-api
vibe git clone https://github.com/company/mobile-app
vibe git clone https://github.com/company/data-pipeline

# 3. Apply team-wide vibe patterns
vibe apps configure frontend-app vscode --template company-react-vibe
vibe apps configure backend-api warp --template company-node-vibe
vibe apps configure mobile-app vscode --template company-flutter-vibe
vibe apps configure data-pipeline warp --template company-python-vibe

# 4. Developer is immediately productive
vibe open frontend-app    # Same setup as entire team
vibe open backend-api     # Identical shortcuts and tools
# They can start coding with the team's patterns right away!

# 5. Stay synchronized with team patterns
vibe sync              # Pull latest vibe patterns
vibe git pull          # Update all repositories
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

- `apps/` - Application integrations (Warp, iTerm2, VS Code, WezTerm)
- `workspace/` - Core workspace management functionality
- `ui/` - Terminal UI components and prompts
- `utils/` - Shared utilities (git, filesystem, platform detection)

## Why Vibe Workspace?

### For Individual Developers
- **Zero Setup Time**: Start coding immediately with pre-configured environments
- **Consistent Experience**: Same shortcuts and tools across all your projects
- **Best Practices Built-in**: Industry standards are the default, not an afterthought
- **Learning Accelerator**: Learn from embedded patterns as you code

### For Teams
- **Instant Onboarding**: New team members are productive on day one
- **Enforced Standards**: Code quality patterns are built into the workflow
- **Reduced Bike-shedding**: Decisions about tooling and setup are already made
- **Knowledge Sharing**: Best practices are embedded in the vibe patterns

### For Organizations
- **Scalable Standards**: Enforce coding patterns across hundreds of repositories
- **Reduced Complexity**: One tool manages all development environments
- **Audit Trail**: Track which patterns are used where
- **Evolution Path**: Update patterns centrally, propagate everywhere

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