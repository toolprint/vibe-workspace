# Vibe Workspace Quick Start

## 🎉 First-Time Setup

### Getting Started with Setup Wizard

The easiest way to get started with Vibe Workspace is to run the setup wizard:

```bash
# Run the interactive setup wizard
vibe setup
```

Or just run `vibe` with no arguments - if it's your first time, you'll automatically be prompted to run the setup wizard.

### What the Setup Wizard Does

The setup wizard walks you through 3 simple steps:

**Step 1: Repository Discovery**
- Automatically scans your workspace for git repositories
- Shows you what it found organized by Git hosting organization:
  ```
  📁 toolprint (3 repos)
    ✅ vibe-workspace (/Users/dev/workspace/vibe-workspace)
    🆕 new-tool (/Users/dev/workspace/new-tool)
  
  📁 microsoft (2 repos)  
    ✅ vscode (/Users/dev/workspace/vscode)
    ❌ missing-repo (tracked but missing)
  ```
- Prompts to add discovered repositories to your workspace

**Step 2: App Installation Check**
- Checks which supported apps you have installed:
  - ✅ VS Code, Warp Terminal, iTerm2, WezTerm, Cursor, Windsurf
- Offers to install missing apps if needed

**Step 3: Default App Configuration**
- Lets you choose a default app for opening repositories
- Automatically configures all discovered repositories with your chosen app

### Skip Setup (Advanced)
```bash
# Skip the setup wizard entirely
vibe setup --skip
```

## 🚀 Essential Commands After Setup

### Quick Launch (The Fastest Way)

Once setup is complete, these are the quickest ways to work with your repositories:

```bash
# Launch the interactive menu
vibe

# Interactive recent repository selector
vibe launch      # Opens interactive selector with recent repositories (1-9)
```

**Pro tip**: In the interactive menu, just press number keys `1-9` to instantly launch recent repositories!

After setup, use these essential commands:
```bash
vibe launch                # Interactive recent repository selector
vibe create my-prototype   # Create new repository for prototyping
vibe clone <github-url>    # Clone, configure, and open in one command
vibe                       # Interactive menu with smart actions
```

### Create New Repository for Prototyping

The `vibe create` command is perfect for quickly starting new prototype projects:

```bash
# Create a new repository interactively
vibe create

# Create with a specific name
vibe create my-new-prototype

# Create and open with a specific app
vibe create my-prototype --app cursor

# Create without configuring app (skip app selection)
vibe create my-prototype --no-configure

# Create without opening (just create the repository)
vibe create my-prototype --no-open

# Create with both flags (just create, no configuration or opening)
vibe create my-prototype --no-configure --no-open
```

This command automatically:
1. Detects your GitHub organizations (if authenticated)
2. Validates the repository name for GitHub compatibility
3. Creates a local git repository with development-ready templates
4. Prompts you to configure an app (unless `--no-configure` is used)
5. Opens the repository with your chosen app (unless `--no-open` is used)

The repository includes:
- README.md with project structure
- .gitignore for common development files
- src/ directory for your code
- docs/TODO.md with development checklist

### Clone Existing Repository

The `vibe clone` command makes it easy to clone and start working with existing repositories:

```bash
# Clone, configure, and open a repository in one command
vibe clone https://github.com/owner/repo

# Clone with a specific app
vibe clone https://github.com/owner/repo --app cursor

# Just clone without opening
vibe clone https://github.com/owner/repo --no-open
```

This command automatically:
1. Clones the repository to your workspace
2. Prompts you to configure an app (VS Code, Warp, etc.)
3. Opens the repository with your chosen app
4. Adds it to your recent repositories for quick access

### Interactive Menu Navigation

The main menu adapts to your workspace state with smart actions:

```bash
vibe  # Launch interactive menu
```

**🏃 Quick Launch Section**: Press `1-9` to instantly open recent repositories

**🧠 Smart Actions** (context-aware suggestions):
- "🎉 Run setup wizard" - If you're a first-time user
- "🔍 Discover repositories" - If no repositories found
- "⚙️ Configure apps for X repos" - If you have unconfigured repositories
- "📦 Install apps" - If supported apps aren't installed
- "🔄 Sync repositories" - If repositories need updating
- "🔍 Clone & Open" - Search GitHub and clone repositories

**Navigation**: Use `ESC` key to go back or exit at any time.

## 🔧 Repository Management

### Repository Status Understanding
- **✅ Tracked** - Repository exists and is properly configured
- **🆕 New** - Repository found but not yet tracked in your config
- **❌ Missing** - Repository is tracked but missing from filesystem

### Discover and Import Repositories
```bash
# Scan workspace for repositories (non-destructive)
vibe git scan

# Import newly discovered repositories
vibe git scan --import

# Scan deeper directory levels
vibe git scan --depth 5

# Clean up missing repositories from config
vibe git scan --clean

# Restore missing repositories from remote
vibe git scan --restore
```

### Configure Apps for Repositories
```bash
# Configure an app for a specific repository
vibe apps configure my-repo cursor
vibe apps configure my-repo windsurf --template agentic

# Show current configurations
vibe apps show
vibe apps show --repo my-repo
vibe apps show --app warp
```

### Open Repositories
```bash
# Open with configured app
vibe open my-repo

# Open with specific app
vibe open my-repo --app windsurf

# Interactive recent repository selector
vibe launch
```

## 📱 Supported Apps

- **VS Code** (`vscode`) - Workspace files with extensions and tasks
- **Warp Terminal** (`warp`) - Multi-pane terminal layouts with AI features  
- **iTerm2** (`iterm2`) - Dynamic profiles with badges and color schemes
- **WezTerm** (`wezterm`) - GPU-accelerated cross-platform terminal
- **Cursor** (`cursor`) - AI-first code editor with built-in AI assistance
- **Windsurf** (`windsurf`) - Agentic IDE powered by AI Flow paradigm

## 🔄 Ongoing Maintenance

### Keep Repositories Synchronized
```bash
# Sync all repositories (fetch and pull)
vibe git sync

# Only fetch updates (don't pull)
vibe git sync --fetch-only

# Save dirty changes before sync
vibe git sync --save-dirty
```

### Fresh Start When Needed
```bash
# Clear repository configuration (keeps other settings)
vibe git reset

# Clear and re-discover repositories
vibe git reset --force
vibe git scan --import
```

## 🎯 Complete Workflow Examples

### New Developer Onboarding
```bash
# 1. First time setup
vibe setup                    # Run setup wizard

# 2. Clone and start working on a project
vibe clone https://github.com/company/main-app

# 3. Quick access later
vibe launch 1                 # Opens main-app instantly
```

### Daily Development Workflow
```bash
# Morning: Launch your main project
vibe launch

# Clone a new repository you need to work on
vibe clone https://github.com/team/new-feature

# End of day: Sync all repositories
vibe git sync --save-dirty
```

### Team Repository Management
```bash
# Discover all team repositories in your workspace
vibe git scan --import

# Configure Cursor for all frontend repositories
vibe apps configure frontend-app cursor
vibe apps configure web-dashboard cursor

# Configure Windsurf for all backend repositories  
vibe apps configure api-server windsurf
vibe apps configure worker-service windsurf

# Quick access to any project
vibe                          # Interactive menu with all projects
```

## 💡 Pro Tips

1. **Number Shortcuts**: In the main menu, press `1-9` to instantly launch recent repositories
2. **ESC Navigation**: Use ESC to go back in any menu or cancel any prompt
3. **Quick Commands**: Use `vibe launch` to access interactive recent repository selector
4. **Smart Menu**: The menu shows different options based on your workspace state
5. **Template System**: Customize how repositories open with templates in `~/.toolprint/vibe-workspace/templates/`
6. **Recent History**: Your last 15 repositories are always accessible via quick launch
7. **Consistent Formatting**: UI now uses unified color schemes - red (no remote), yellow (changes), green (clean)

## 🆘 Troubleshooting

**"No repositories found"**: Run `vibe git scan --import` to discover repositories

**"Repository not opening"**: Check if the app is configured with `vibe apps show --repo <name>`

**"Can't find recent repository"**: Use `vibe` menu to browse all repositories

**"Setup wizard not showing"**: Run `vibe setup` manually or reset with `vibe config reset`

For detailed app configuration and advanced features, see [APPS.md](./APPS.md) and [NAVIGATION.md](./NAVIGATION.md).