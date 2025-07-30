# Vibe Workspace Quick Start

## Git Repository Management

### Repository Discovery and Analysis
```bash
# Scan workspace for all git repositories
vibe git scan

# Import newly discovered repositories to your config
vibe git scan --import

# Scan with specific options
vibe git scan --depth 5                # Scan deeper directory levels
vibe git scan --clean                   # Remove missing repositories from config
vibe git scan --restore                 # Re-clone missing repositories from remote
```

### Repository Status Understanding
- **✅ Tracked** - Repository exists on filesystem and is tracked in config
- **🆕 New** - Repository exists on filesystem but not tracked in config
- **❌ Missing** - Repository is tracked in config but missing from filesystem

### Repository Lifecycle Management
```bash
# Clear all repository tracking (preserves other settings)
vibe git reset                          # With confirmation prompt
vibe git reset --force                  # Skip confirmation

# Synchronize repositories
vibe git sync                           # Fetch and pull all repositories  
vibe git sync --fetch-only              # Only fetch, don't pull
vibe git sync --save-dirty              # Save dirty changes before sync
```

### Repository Organization Display
Repositories are automatically organized by Git hosting organization:
```
📁 toolprint (3 repos)
  ✅ vibe-workspace (/Users/dev/workspace/vibe-workspace)
  🆕 new-tool (/Users/dev/workspace/new-tool)

📁 microsoft (2 repos)  
  ✅ vscode (/Users/dev/workspace/vscode)
  ❌ missing-repo (tracked but missing)
```

## App Integration Quick Usage

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

## Complete Workflow Example

### Initial Setup
1. Initialize workspace and discover repositories:
   ```bash
   vibe init --name "my-workspace"
   vibe git scan --import              # Auto-discover and import repositories
   ```

2. Review discovered repositories:
   ```bash
   vibe git scan                       # Shows organized repository structure
   ```

### Repository Management
3. Configure apps for your repositories:
   ```bash
   vibe apps configure frontend vscode
   vibe apps configure backend warp
   vibe apps configure mobile iterm2
   vibe apps configure api wezterm
   ```

4. Open repositories quickly:
   ```bash
   vibe open frontend    # Opens with VS Code
   vibe open backend     # Opens with Warp
   vibe open mobile      # Opens with iTerm2
   vibe open api         # Opens with WezTerm
   ```

### Ongoing Maintenance
5. Keep repositories synchronized:
   ```bash
   vibe git sync --save-dirty          # Save changes and sync all repos
   ```

6. Add new repositories as you discover them:
   ```bash
   vibe git scan --import              # Import newly found repositories
   vibe git clone <new-repo-url>       # Clone and automatically add new repositories
   ```

7. Fresh start when needed:
   ```bash
   vibe git reset --force              # Clear repository config
   vibe git scan --import              # Re-discover repositories
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

## Repository Status Troubleshooting

### Common Repository Issues and Solutions

**Issue: Repositories showing as "Missing" after system changes**
```bash
# Solution: Restore missing repositories from remote
vibe git scan --restore
```

**Issue: Duplicate repositories in different locations**
```bash
# Solution: Clean up duplicates and re-scan
vibe git reset --force                  # Clear config
vibe git scan --import                  # Re-discover repositories
```

**Issue: Repository not detected during scan**
```bash
# Solution: Check if it's a valid git repository
cd /path/to/suspected/repo
git status                              # Should show git info

# Or increase scan depth
vibe git scan --depth 5                # Scan deeper directory levels
```

**Issue: Too many "New" repositories cluttering the display**
```bash
# Solution: Import only the ones you want
vibe git scan                           # Review what's found
vibe apps configure wanted-repo vscode  # Configure specific repositories
# Don't run --import until you've decided which repos to track
```

**Issue: Need to start fresh with repository tracking**
```bash
# Solution: Complete reset and selective re-import
vibe git reset --force                  # Clear all tracked repositories
vibe git scan                           # See what's available
vibe git scan --import                  # Import everything
# Or manually configure specific repositories
```

### Repository Status Summary

The repository status system helps you understand your workspace:

- **Tracked (✅)**: Everything is working correctly
- **New (🆕)**: Found repositories you might want to add
- **Missing (❌)**: Repositories that may have been moved or deleted

Use `vibe git scan` regularly to keep your workspace organized and up-to-date.

For detailed documentation, see [docs/APPS.md](./APPS.md).