# Vibe Workspace App Integration Guide

This guide explains how to configure and use app integrations in Vibe Workspace to quickly open repositories in your favorite development tools and install additional developer tools.

## Quick Start

**Interactive Mode** (recommended): Just run `vibe` with no arguments to access the interactive menu:
```bash
vibe
```

**CLI Commands**: Use specific commands for automation:
```bash
# Initialize workspace
vibe init

# Configure apps for repositories  
vibe apps configure <repo-name> <app>

# Open repositories quickly
vibe open <repo-name> --app <app>

# Install additional developer tools
vibe apps install
```

## Applications Overview

Vibe Workspace supports two types of applications:

### Apps for Opening Repositories (4 apps)
These apps support full integration with repository opening, templates, and advanced configuration:
- **Warp Terminal** - Modern terminal with AI features
- **iTerm2** - Powerful macOS terminal emulator  
- **WezTerm** - GPU-accelerated cross-platform terminal
- **Visual Studio Code** - Microsoft's code editor

### Additional Developer Tools (10+ apps)
These tools can be installed via the app installer but don't support repository opening:
- **Cursor** - AI-first code editor
- **GitHub CLI** - GitHub's official command line tool
- **GitUI** - Terminal-based git interface
- **Just** - Command runner and task automation
- **Claude Squad** - Claude conversation management
- **Container Use** - Agent sandbox launcher
- **Weztermocil** - WezTerm session manager
- **iTermocil** - iTerm2 session manager

## Repository Opening Apps (Detailed)

### 1. Warp Terminal
- **Documentation**: https://docs.warp.dev/terminal/sessions/launch-configurations
- **Configuration Format**: YAML
- **Features**: Multi-pane layouts, custom commands, color themes, AI assistance
- **Usage**: `vibe open <repo> --app warp`

### 2. iTerm2
- **Documentation**: https://iterm2.com/documentation-dynamic-profiles.html
- **Configuration Format**: JSON (Dynamic Profiles)
- **Features**: Custom profiles, badges, initial commands, color schemes
- **Usage**: `vibe open <repo> --app iterm2`

### 3. WezTerm
- **Documentation**: https://wezfurlong.org/wezterm/
- **Configuration Format**: YAML (via Weztermocil integration)
- **Features**: GPU acceleration, cross-platform, multiplexing, programmable
- **Usage**: `vibe open <repo> --app wezterm`

### 4. Visual Studio Code
- **Documentation**: https://code.visualstudio.com/docs/editor/workspaces
- **Configuration Format**: JSON (Workspace files)
- **Features**: Multi-root workspaces, extensions, tasks, settings
- **Usage**: `vibe open <repo> --app vscode`

## Additional Developer Tools

The following tools can be installed via `vibe apps install` but don't support repository opening. They're useful development tools that can be used independently:

### Code Editors
- **Cursor** - AI-first code editor with built-in AI assistance
  - Installation: Available via Homebrew cask
  - Usage: Independent application, not integrated with repository opening

### CLI Development Tools
- **GitHub CLI (gh)** - GitHub's official command line tool
  - Features: Repository management, pull requests, issues, actions
  - Installation: Available via Homebrew formula
  - Usage: `gh repo clone`, `gh pr create`, etc.

- **GitUI** - Blazing fast terminal-based git client
  - Features: Fast git operations, terminal-based interface
  - Installation: Available via Homebrew formula
  - Usage: Run `gitui` in any git repository

- **Just** - Command runner and task automation tool
  - Features: Simple task runner, cross-platform, fast
  - Installation: Available via Homebrew formula
  - Usage: Create `justfile` with tasks, run `just <task>`

- **Claude Squad** - Tool for managing Claude conversations
  - Features: Conversation management, session tracking
  - Installation: Available via Homebrew formula

### Container & Development Tools
- **Container Use** - Tool for launching agent sandboxes
  - Features: Containerized development environments
  - Installation: Available via Homebrew cask

### Terminal Session Managers
- **Weztermocil** - WezTerm session manager
  - Features: Complex terminal layouts, session persistence
  - Installation: Available via custom Homebrew tap
  - Integration: Used by WezTerm repository opening feature

- **iTermocil** - iTerm2 session manager  
  - Features: iTerm2 window/pane management
  - Installation: Available via custom Homebrew tap
  - Integration: Can be used with iTerm2 repository opening (disabled with `--no-itermocil`)

### Installing Additional Tools

Use the interactive installer to browse and install any of these tools:

```bash
vibe apps install
```

The installer will:
1. Show available applications with descriptions
2. Check if tools are already installed
3. Install selected tools using appropriate package managers
4. Verify successful installation

## Configuration Overview

App integrations are configured at two levels:

1. **Global Configuration** (`~/.vibe-workspace/config.yaml`):
   - App-specific settings (config directories, template paths)
   - Default template names

2. **Per-Repository Configuration**:
   - Which apps are enabled for each repository
   - Which template to use for each app

## Template System

Templates define how repositories are opened in each app. They support variable substitution for dynamic configuration.

### Template Locations
```
~/.vibe-workspace/templates/
├── warp/
│   ├── default.yaml
│   └── custom-dev.yaml
├── iterm2/
│   ├── default.json
│   └── production.json
├── wezterm/
│   ├── default.yaml
│   └── development.yaml
└── vscode/
    ├── default.json
    └── react-project.json
```

### Template Variables

All templates support these variables:
- `{{workspace_name}}` - The vibe workspace name
- `{{repo_name}}` - Repository name
- `{{repo_path}}` - Full path to repository
- `{{repo_branch}}` - Default branch (or "main" if not set)
- `{{repo_url}}` - Repository URL (if available)

### Default Templates

#### Warp Default Template
```yaml
---
name: {{workspace_name}} - {{repo_name}}
windows:
  - tabs:
      - title: {{repo_name}}
        layout:
          cwd: {{repo_path}}
          split_direction: vertical
          panes:
            - cwd: {{repo_path}}
              commands:
                - exec: git status
            - cwd: {{repo_path}}
              commands:
                - exec: echo "Ready for development!"
        color: blue
```

#### iTerm2 Default Template
```json
{
  "Profiles": [{
    "Name": "{{workspace_name}} - {{repo_name}}",
    "Guid": "vibe-{{workspace_name}}-{{repo_name}}",
    "Working Directory": "{{repo_path}}",
    "Custom Directory": "Yes",
    "Badge Text": "📁 {{repo_name}}",
    "Tab Color": {
      "Red Component": 0.2,
      "Green Component": 0.4,
      "Blue Component": 0.8
    },
    "Initial Text": "clear\\necho '🚀 Welcome to {{repo_name}}'\\ngit status"
  }]
}
```

#### WezTerm Default Template
```yaml
---
name: {{workspace_name}}-{{repo_name}}
root: "{{repo_path}}"
windows:
  - name: {{repo_name}}
    root: "{{repo_path}}"
    layout: main-vertical
    panes:
      - root: "{{repo_path}}"
        commands:
          - git status
      - root: "{{repo_path}}"
        commands:
          - echo "🚀 Welcome to {{repo_name}}"
```

#### VS Code Default Template
```json
{
  "folders": [
    {
      "name": "{{repo_name}}",
      "path": "{{repo_path}}"
    }
  ],
  "settings": {
    "window.title": "{{repo_name}} - {{workspace_name}}"
  },
  "extensions": {
    "recommendations": [
      "eamodio.gitlens",
      "ms-vscode.vscode-json"
    ]
  }
}
```

## Configuration Examples

### Basic Configuration
```yaml
repositories:
  - name: "frontend"
    path: "./frontend"
    apps:
      warp: true        # Uses default template
      vscode: true      # Uses default template
```

### Advanced Configuration
```yaml
repositories:
  - name: "backend"
    path: "./backend"
    apps:
      warp:
        template: "microservices"
      vscode:
        template: "python-dev"
      iterm2:
        template: "production"
      wezterm:
        template: "development"
```

### Global App Settings
```yaml
apps:
  warp:
    enabled: true
    config_dir: ~/.warp/launch_configurations
    template_dir: ~/.vibe-workspace/templates/warp
    default_template: "default"
  iterm2:
    enabled: true
    config_dir: ~/Library/Application Support/iTerm2/DynamicProfiles
    template_dir: ~/.vibe-workspace/templates/iterm2
    default_template: "default"
  wezterm:
    enabled: true
    config_dir: ~/.config/wezterm
    template_dir: ~/.vibe-workspace/templates/wezterm
    default_template: "default"
  vscode:
    enabled: true
    workspace_dir: ~/.vscode/workspaces
    template_dir: ~/.vibe-workspace/templates/vscode
    default_template: "default"
```

## Creating Custom Templates

### Step 1: Create Template File
Create a new file in the appropriate template directory:
```bash
# For Warp
~/.vibe-workspace/templates/warp/my-template.yaml

# For iTerm2
~/.vibe-workspace/templates/iterm2/my-template.json

# For WezTerm
~/.vibe-workspace/templates/wezterm/my-template.yaml

# For VS Code
~/.vibe-workspace/templates/vscode/my-template.json
```

### Step 2: Customize Template
Use the default template as a starting point and modify as needed.

#### Example: Warp Template for React Development
```yaml
---
name: {{workspace_name}} - {{repo_name}}
windows:
  - tabs:
      - title: {{repo_name}}
        layout:
          cwd: {{repo_path}}
          split_direction: horizontal
          panes:
            - cwd: {{repo_path}}
              split_direction: vertical
              panes:
                - commands:
                    - exec: npm run dev
                - commands:
                    - exec: npm test -- --watch
            - cwd: {{repo_path}}
              commands:
                - exec: git status
        color: cyan
```

### Step 3: Configure Repository
Update your repository configuration to use the custom template:
```yaml
repositories:
  - name: "my-react-app"
    path: "./my-react-app"
    apps:
      warp:
        template: "my-template"
```

## App-Specific Features

### Warp Features
- **Multi-pane Layouts**: Split terminals horizontally/vertically
- **Pre-configured Commands**: Run commands on startup
- **Tab Organization**: Multiple tabs per window
- **Color Coding**: Visual distinction between projects

### iTerm2 Features
- **Dynamic Profiles**: Automatically appear in iTerm2
- **Badge System**: Visual indicators on tabs
- **Color Schemes**: Custom colors per project
- **Initial Commands**: Setup commands on launch

### WezTerm Features
- **GPU Acceleration**: Hardware-accelerated rendering
- **Cross-platform**: Works on macOS, Linux, and Windows
- **Multiplexing**: Built-in terminal multiplexing
- **Session Management**: Integration with Weztermocil for complex layouts

### VS Code Features
- **Multi-root Workspaces**: Open multiple folders
- **Extension Recommendations**: Project-specific extensions
- **Task Definitions**: Pre-configured build/test tasks
- **Settings Override**: Workspace-specific settings

## Troubleshooting

### Common Issues

1. **Template Not Found**
   - Ensure template file exists in correct directory
   - Check file extension (.yaml for Warp/WezTerm, .json for iTerm2/VS Code)
   - Verify template name in configuration

2. **App Won't Open**
   - Ensure app is installed and in PATH
   - Check app integration is enabled in global config
   - Try manual launch instructions provided

3. **Variables Not Substituted**
   - Use double braces: `{{variable}}`
   - Ensure variable names are spelled correctly
   - Check if repository has required fields (url, branch)

### Manual Launch Instructions

If automatic launching fails, each app provides manual instructions:

**Warp**:
1. Open Warp
2. Press Cmd+Shift+L
3. Select the generated configuration

**iTerm2**:
1. Open iTerm2
2. Profile should appear in Profiles menu
3. May need to restart iTerm2 for dynamic profiles

**WezTerm**:
1. Open WezTerm
2. Configuration uses Weztermocil for session management
3. Sessions should launch automatically with generated config

**VS Code**:
1. Open VS Code
2. File → Open Workspace from File
3. Navigate to generated workspace file

## Best Practices

1. **Template Organization**
   - Use descriptive template names
   - Create templates for different project types
   - Document custom templates

2. **Repository Configuration**
   - Configure apps based on project needs
   - Use consistent templates across similar projects
   - Consider team conventions

3. **Performance**
   - Limit startup commands to essentials
   - Avoid heavy operations in initial commands
   - Use appropriate pane counts

4. **Security**
   - Don't include sensitive data in templates
   - Be cautious with startup commands
   - Review templates from others before use