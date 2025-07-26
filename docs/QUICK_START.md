# Vibe Workspace App Integration Quick Start

## Quick Usage

### Configure an app for a repository
```bash
# Use default template
vibe apps configure <repo-name> warp

# Use specific template
vibe apps configure <repo-name> vscode --template react-dev
```

### Open a repository with an app
```bash
# Open with specific app
vibe open <repo-name> --app warp

# Open with default/only configured app
vibe open <repo-name>
```

### Manage templates
```bash
# List available templates
vibe apps template list warp

# Create a new template
vibe apps template create warp my-custom --from-file ./my-warp-template.yaml

# Delete a template
vibe apps template delete warp my-custom
```

### Show configurations
```bash
# Show all app configurations
vibe apps show

# Show apps for a specific repository
vibe apps show --repo my-repo

# Show repositories with a specific app
vibe apps show --app warp
```

## Repository Opening Apps

- **Warp Terminal** (`warp`) - Multi-pane terminal layouts with AI features
- **iTerm2** (`iterm2`) - Dynamic profiles with badges and color schemes
- **WezTerm** (`wezterm`) - GPU-accelerated cross-platform terminal
- **Visual Studio Code** (`vscode`) - Workspace files with extensions and tasks

## Example Workflow

1. Initialize workspace (creates default templates):
   ```bash
   vibe init
   ```

2. Configure apps for your repositories:
   ```bash
   vibe apps configure frontend vscode
   vibe apps configure backend warp
   vibe apps configure mobile iterm2
   vibe apps configure api wezterm
   ```

3. Open repositories quickly:
   ```bash
   vibe open frontend    # Opens with VS Code
   vibe open backend     # Opens with Warp
   vibe open mobile      # Opens with iTerm2
   vibe open api         # Opens with WezTerm
   ```

## Template Variables

All templates support these variables:
- `{{workspace_name}}` - Your vibe workspace name
- `{{repo_name}}` - Repository name
- `{{repo_path}}` - Full path to repository
- `{{repo_branch}}` - Default branch
- `{{repo_url}}` - Repository URL

## Templates Location

Templates are stored in:
```
~/.vibe-workspace/templates/
├── warp/
│   └── default.yaml
├── iterm2/
│   └── default.json
├── wezterm/
│   └── default.yaml
└── vscode/
    └── default.json
```

For detailed documentation, see [docs/APPS.md](./APPS.md).