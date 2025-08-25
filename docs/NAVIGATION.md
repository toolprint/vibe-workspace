# Vibe Workspace Navigation System Overview

This document provides a comprehensive overview of the navigation systems in vibe-workspace, covering both the command-line interface and interactive menu system with full ESC key navigation support.

## Commands

### Command Tree Structure

The vibe CLI follows a hierarchical subcommand pattern:

```
vibe [global-options] <command> [command-options]

├── (no command)                    → Menu mode
├── menu                           → Enter interactive menu
├── launch                         → Interactive recent repository selector (1-9)
├── create [name] [--app] [--no-configure] [--no-open] → Create new local repository
├── clone <url> [--app] [--no-configure] [--no-open] → Clone, configure, and open in one command
├── setup [--skip]                → Run first-time setup wizard
├── mcp [--stdio] [--port]        → Run as MCP server (--port coming soon)
├── open <repo> [--app]           → Open repository with app
├── apps                          → App management
│   ├── configure <repo> <app>    → Configure app for repository
│   ├── show [--repo] [--app]     → Show app configurations
│   ├── install                   → Install developer tools
│   └── template                  → Template management
│       ├── list <app>            → List available templates
│       ├── create <app> <name>   → Create new template
│       ├── delete <app> <name>   → Delete template
│       └── update-defaults       → Update default templates
├── config                        → Configuration management
│   ├── init                      → Initialize workspace config
│   ├── edit [--direct]           → Edit configuration
│   ├── show [--format] [--section] → Show current config
│   ├── validate                  → Validate configuration
│   ├── reset [--force]           → Factory reset
│   ├── backup [--output] [--name] → Create backup archive
│   └── restore [--backup]        → Restore from backup archive
└── git                           → Git operations
    ├── scan [path] [--import]    → Scan for repositories
    ├── discover [path]           → Discover repositories (deprecated)
    ├── status [--dirty-only]     → Show repository status
    ├── exec <command>            → Execute git command across repos
    ├── sync [--fetch-only]       → Sync repositories
    ├── clone <url>               → Clone repository
    ├── search                    → Interactive repository search
    ├── reset [--force]           → Reset repository configuration
    └── worktree                  → Manage git worktrees
        ├── create <task-id>      → Create worktree for parallel development
        ├── list                  → List all worktrees with status
        ├── remove <target>       → Remove a worktree
        ├── status [branch]       → Show worktree health overview
        ├── clean                 → Clean up merged worktrees
        ├── open <target>         → Open worktree in editor
        ├── merge <target>        → Merge worktree changes
        ├── backup <target>       → Backup worktree to remote
        ├── conflicts <target>    → Show conflict analysis
        └── config                → Manage worktree configuration
            ├── show              → Show current configuration
            ├── set <key> <val>   → Set configuration value
            ├── reset             → Reset to defaults
            ├── validate          → Validate configuration
            └── info              → Show help and env variables
```

### Executable Commands

Commands that directly perform actions (leaf nodes and commands with default actions):

| Command | Action | Description |
|---------|--------|-------------|
| `vibe` | Menu mode | Launch smart interactive menu |
| `vibe menu` | Menu mode | Explicitly enter interactive menu |
| `vibe launch` | Interactive launcher | Interactive recent repository selector (1-9) |
| `vibe create [name]` | Create repository | Create new local repository for prototyping |
| `vibe clone <url>` | Clone workflow | Clone, configure, and open repository |
| `vibe setup` | Setup wizard | Run first-time workspace setup |
| `vibe mcp --stdio` | MCP server | Run as Model Context Protocol server (--port coming soon) |
| `vibe open <repo>` | Open repository | Open repo with configured app |
| `vibe apps configure` | Configure app | Set up app integration for repository |
| `vibe apps show` | Show configurations | Display current app configurations |
| `vibe apps install` | Install apps | Interactive app installer |
| `vibe apps template list` | List templates | Show available templates for app |
| `vibe apps template create` | Create template | Create new app template |
| `vibe apps template delete` | Delete template | Remove app template |
| `vibe apps template update-defaults` | Update templates | Refresh bundled templates |
| `vibe config init` | Initialize config | Create new workspace configuration |
| `vibe config edit` | Edit config | Open configuration in editor |
| `vibe config show` | Show config | Display workspace configuration |
| `vibe config validate` | Validate config | Check configuration integrity |
| `vibe config reset` | Factory reset | Clear all configuration |
| `vibe config backup` | Create backup | Archive configuration files |
| `vibe config restore` | Restore backup | Restore configuration from backup |
| `vibe git scan` | Scan repositories | Find git repositories in directory |
| `vibe git discover` | Discover repos | Legacy repository discovery |
| `vibe git status` | Repository status | Show git status across all repos |
| `vibe git exec` | Execute command | Run git command on multiple repos |
| `vibe git sync` | Sync repositories | Fetch/pull updates for all repos |
| `vibe git clone` | Clone repository | Clone single repository |
| `vibe git search` | Search repositories | Interactive GitHub repository search |
| `vibe git reset` | Reset git config | Clear repository configuration |
| `vibe git worktree create <task-id>` | Create worktree | Create new worktree for parallel task development |
| `vibe git worktree list` | List worktrees | Show all worktrees with status and health indicators |
| `vibe git worktree remove <target>` | Remove worktree | Remove worktree by task ID, branch, or path |
| `vibe git worktree status` | Show health | Display repository worktree health overview |
| `vibe git worktree status <branch>` | Branch status | Show detailed status for specific worktree |
| `vibe git worktree clean` | Cleanup merged | Automatically clean up merged worktrees |
| `vibe git worktree open <target>` | Open in editor | Open worktree in configured editor |
| `vibe git worktree merge <target>` | Merge changes | Merge worktree changes to feature branch |
| `vibe git worktree backup <target>` | Backup to remote | Push worktree changes to remote for safekeeping |
| `vibe git worktree conflicts <target>` | Analyze conflicts | Show merge conflict analysis for worktree |
| `vibe git worktree config show` | Show config | Display worktree configuration |
| `vibe git worktree config set` | Set config | Update worktree configuration values |
| `vibe git worktree config reset` | Reset config | Reset worktree config to defaults |
| `vibe git worktree config validate` | Validate config | Check worktree configuration validity |
| `vibe git worktree config info` | Config help | Show configuration help and environment variables |

## Menu

### Interactive Menu Flow (DAG)

```
Entry Points:
├── vibe (no args) → Main Menu
├── First-time user → Setup Wizard → Main Menu
└── Workflow completion → Suggested Actions

Main Menu
├── 🏃 Quick Launch (1-9) → Launch Repository → [Complete]
├── 🧠 Smart Actions (context-aware)
│   ├── Setup Workspace → Setup Wizard → Main Menu
│   ├── Configure Apps → App Configuration → Main Menu
│   ├── Discover Repos → Repository Discovery → Main Menu
│   ├── Install Apps → App Installer → Main Menu
│   ├── Cleanup Missing → Repository Cleanup → Main Menu
│   ├── Sync Repositories → Repository Sync → Main Menu
│   └── Clone & Open → GitHub Search → Clone Workflow → [Complete]
├── 🚀 Launch app → Repository Launcher → [Complete]
├── 📁 Manage repos → Repository Management Menu
└── ⚙️ Configure vibes → Configuration Management Menu

Repository Management Menu
├── Show repository status → Status Display → Repository Management Menu
├── Search & clone from GitHub → GitHub Search → Clone Workflow → [Complete]
├── Discover new repositories → Repository Discovery → Repository Management Menu
├── Sync repositories → Repository Sync → Repository Management Menu
├── Execute command on repositories → Command Execution → Repository Management Menu
├── Manage groups → Group Management → Repository Management Menu
└── Back to main menu → Main Menu

Configuration Management Menu
├── Configure apps for repositories → App Configuration → Configuration Management Menu
├── Manage app templates → Template Management → Configuration Management Menu
├── Factory Reset → Reset Confirmation → Main Menu
├── Create Backup → Backup Creation → Configuration Management Menu
├── Restore from Backup → Backup Selection → Restore Confirmation → Configuration Management Menu
└── Back to main menu → Main Menu

GitHub Search → Repository Selection → Clone Workflow
Clone Workflow → ConfigureAppWorkflow → OpenRepositoryWorkflow → [Complete with suggestions]

Setup Wizard → Repository Discovery → App Configuration → [Complete]
```

### Workflow Descriptions

#### Create Repository Workflow
**Entry Points**: 
- `vibe create [name]` - Create new repository
- `vibe create [name] --app <app>` - Create and open with specific app
- `vibe create [name] --no-configure` - Skip app configuration
- `vibe create [name] --no-open` - Skip opening after create
- Smart Action: "Create new repository"

**Flow**:
1. **CreateRepositoryWorkflow** - Gather repository details (name, owner)
2. GitHub integration - Detect organizations and validate name availability
3. Create local repository with default templates
4. **ConfigureAppWorkflow** - Prompt to configure apps (skipped with `--no-configure`)
5. **OpenRepositoryWorkflow** - Open repository with configured app (skipped with `--no-open`)
6. **Complete** - Repository ready for development

#### Clone Repository Workflow
**Entry Points**: 
- `vibe clone <url>`
- Smart Action: "Clone & Open"
- Menu: "Search & clone from GitHub"

**Flow**:
1. **CloneWorkflow** - Clone repository to workspace
2. **ConfigureAppWorkflow** - Prompt to configure apps (VS Code, iTerm2, Warp, etc.)
3. **OpenRepositoryWorkflow** - Open repository with configured app
4. **Complete** - Show suggestions for next actions

#### Setup Wizard Workflow
**Entry Points**:
- `vibe setup`
- First-time user detection
- Smart Action: "Setup Workspace"

**Flow**:
1. **SetupWorkspaceWorkflow** - Welcome and workspace initialization
2. Repository discovery (scan for existing git repos)
3. **ConfigureDefaultAppWorkflow** - Set up default app for all repositories
4. **Complete** - Mark setup as complete, return to main menu

#### 🏃 Quick Launch System
**Entry Points**:
- `vibe launch` - Interactive recent repository selector
- Main menu number keys (1-9)
- Quick Launch menu items

**Flow**:
- Load recent repositories from state
- Present interactive list with consistent formatting
- Recent repos show time since last access and last-used app
- Use last-used app or prompt for selection
- Update access history and launch

#### Worktree Development Workflow
**Entry Points**: 
- `vibe git worktree create <task-id>` - Create worktree for a task
- MCP Tool: `create_worktree` - AI-assisted worktree creation

**Flow**:
1. **Create Worktree** - Generate sanitized branch name from task ID
2. **Configure Location** - Determine storage mode (local/.worktrees or global)
3. **Setup GitIgnore** - Auto-add worktree directory to .gitignore
4. **Open in Editor** - Launch configured editor (optional)
5. **Track Status** - Monitor changes and merge status

#### Worktree Cleanup Workflow
**Entry Points**:
- `vibe git worktree clean` - Manual cleanup command
- MCP Tool: `recommend_cleanup` - AI recommendations
- MCP Tool: `execute_cleanup` - AI-driven cleanup

**Flow**:
1. **Analyze Worktrees** - Check merge status and age
2. **Detect Merged** - Use multiple detection methods (GitHub CLI, git log, etc.)
3. **Recommend Actions** - Suggest cleanup strategy
4. **Execute Cleanup** - Remove/backup/merge based on strategy
5. **Report Results** - Show cleanup summary

### 🧠 Smart Actions (Context-Aware Menu Items)

The interactive menu dynamically shows relevant actions based on workspace state:

| Condition | Smart Action | Description |
|-----------|--------------|-------------|
| First-time user | "Setup Workspace" | Run setup wizard |
| Unconfigured repos exist | "Configure Apps for X repos" | Batch configure repositories |
| No repositories found | "Discover Repositories" | Scan workspace for git repos |
| Missing repositories | "Cleanup Missing Repos" | Remove non-existent repos from config |
| Apps not installed | "Install Apps" | Interactive app installer |
| Long since sync | "Sync Repositories" | Update all repositories |
| Always available | "Create new repository" | Quick create for prototyping |
| Always available | "Clone & Open" | Search GitHub and clone |

### Menu State Management

**VibeState** (`~/.toolprint/vibe-workspace/state.json`):
- Recent repositories (with last-used apps)
- Setup wizard completion status
- User preferences and access patterns

**Smart Menu Analysis**:
- Workspace state (total repos, unconfigured repos, missing repos)
- Available apps detection
- Repository synchronization status
- User behavior patterns

### Key Navigation Features

1. **Number Shortcuts**: Press 1-9 in main menu to quickly launch recent repositories
2. **ESC Key Navigation**: ESC key provides consistent navigation in all menus:
   - Main menu: ESC exits the application
   - Submenus: ESC returns to parent menu
   - Text prompts: ESC cancels input and returns to previous screen
   - Repository selection: ESC cancels selection gracefully
3. **Visual Navigation Cues**: All menus include visual separators and [Back]/[Exit] options
4. **Workflow Continuity**: Actions flow seamlessly (clone → configure → open)
5. **Context Awareness**: Menu adapts based on workspace state and user history
6. **Quick Commands**: Single commands for common workflows (`vibe create`, `vibe clone`, `vibe launch`)
7. **Smart Defaults**: Remembers last-used apps and preferences
8. **Progressive Setup**: First-time users get guided setup experience
9. **Help Messages**: All prompts include contextual help with ESC instructions

### Entry Point Summary

**CLI Direct Actions**:
- `vibe launch` - Interactive recent repository selector
- `vibe create [name]` - Create new repository for prototyping
- `vibe clone <url>` - Complete clone-to-open workflow
- `vibe open <repo>` - Direct repository opening
- `vibe apps configure` - Direct app configuration

**Interactive Entry Points**:
- `vibe` (no args) - Smart main menu with quick launch
- First-time detection - Automatic setup wizard
- Workflow completion - Contextual suggestions

**Menu Navigation**:
- 🏃 Quick Launch (1-9) - Fastest access to recent repos
- 🧠 Smart Actions - Context-aware workflow shortcuts
- Standard Menu - Full feature access with submenu navigation
- Workflow continuation - Seamless multi-step processes
- ESC Navigation - Consistent back/exit behavior across all menus

## ESC Key Navigation System

### Behavior Patterns

All interactive menus in vibe-workspace support ESC key navigation with consistent behavior:

**ESC Key Handling**:
- **Main Menu**: ESC exits the application with "👋 Goodbye!" message
- **Submenus**: ESC returns to the parent menu (equivalent to selecting [Back])
- **Text Input**: ESC cancels input and returns to previous screen
- **Repository Selection**: ESC cancels selection and returns to parent context

**Visual Indicators**:
- All menus include help messages: "Use arrow keys to navigate • ESC to exit/go back"
- Navigation separators (────────────────────) visually separate options from navigation
- [Back] and [Exit] options are clearly marked with brackets
- Contextual help messages explain ESC behavior for each prompt type

**Implementation**:
- Uses `handle_prompt_result()` function to distinguish ESC from fatal errors
- Returns `Ok(None)` for ESC cancellation, enabling graceful navigation
- Consistent error handling across all UI components
- Native ESC support in Quick Launcher (no [Cancel] option needed)

### Menu Hierarchy with ESC Navigation

```
Main Menu
├── ESC → Exit Application
├── Quick Launch Items (1-9)
│   └── ESC → Return to Main Menu
├── Smart Actions
│   └── ESC → Return to Main Menu
├── 🚀 Launch app
│   ├── Repository Selection → ESC → Return to Main Menu
│   └── App Selection → ESC → Return to Repository Selection
├── 🔀 Manage repos
│   ├── ESC → Return to Main Menu
│   ├── Show status → ESC → Return to Manage repos
│   ├── Search & clone → ESC → Return to Manage repos
│   └── [Other options] → ESC → Return to Manage repos
└── ⚙️ Settings
    ├── ESC → Return to Main Menu
    ├── Configure apps → ESC → Return to Settings
    ├── Template management → ESC → Return to Settings
    └── [Other options] → ESC → Return to Settings
```

### Text Input ESC Handling

All text input prompts support ESC cancellation:

- Repository names, paths, and configuration values
- Search queries and filter inputs
- Template names and content
- Backup names and restore selections

**Example Help Messages**:
- "Enter repository name • ESC to cancel"
- "Choose directory path • ESC to go back"
- "Enter backup name • ESC to cancel"
- "Select app to open with • ESC to cancel"

## Worktree Management

### Command Options

#### `vibe git worktree create`
- `--base-branch, -b <branch>` - Base branch to create from (default: current branch)
- `--force, -f` - Force creation even if branch exists
- `--path, -p <path>` - Custom worktree path (overrides default)
- `--mode, -m <local|global>` - Storage mode (local within repo, global centralized)
- `--open, -o` - Open in editor after creation
- `--editor <cmd>` - Editor command to use

#### `vibe git worktree list`
- `--prefix, -p <prefix>` - Filter by branch prefix
- `--verbose, -v` - Show detailed information
- `--format, -f <table|json|compact>` - Output format (default: table)
- `--dirty-only, -d` - Show only worktrees with uncommitted changes

#### `vibe git worktree remove`
- `--force, -f` - Force removal even with uncommitted changes
- `--delete-branch, -d` - Also delete the branch after removing
- `--yes` - Skip confirmation prompts

#### `vibe git worktree clean`
- `--dry-run, -d` - Show what would be done without executing
- `--force, -f` - Force cleanup even with uncommitted changes
- `--age <hours>` - Minimum age in hours before cleanup
- `--yes` - Skip confirmation prompts

#### `vibe git worktree merge`
- `--strategy, -s <squash|rebase|standard>` - Merge strategy (default: standard)
- `--yes` - Skip confirmation prompts

#### `vibe git worktree backup`
- `--cleanup-after` - Remove worktree after successful backup
- `--yes` - Skip confirmation prompts

#### `vibe git worktree conflicts`
- `--compact` - Show compact format (summary only)
- `--format, -f <table|json|compact>` - Output format (default: table)

### MCP Worktree Management Tools

The following tools are available for AI systems via Model Context Protocol:

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| `create_worktree` | Create new worktree for task | task_id, base_branch, force |
| `list_worktrees` | List and analyze worktrees | include_status, verbose |
| `analyze_conflicts` | Analyze merge conflicts | target, verbose |
| `recommend_cleanup` | Get cleanup recommendations | min_age_hours, include_analysis |
| `execute_cleanup` | Perform worktree cleanup | strategy, targets, dry_run |
| `worktree_help` | Get contextual help | topic (optional) |

### Configuration

#### Environment Variables
- `VIBE_WORKTREE_PREFIX` - Branch prefix for worktrees (default: "vibe-ws/")
- `VIBE_WORKTREE_BASE` - Base directory for worktrees (default: ".worktrees")
- `VIBE_WORKTREE_MODE` - Storage mode: local or global (default: local)
- `VIBE_WORKTREE_AUTO_GITIGNORE` - Auto-add to .gitignore (default: true)
- `VIBE_WORKTREE_DEFAULT_EDITOR` - Default editor command (default: "code")

#### Configuration Keys
- `prefix` - Branch naming prefix
- `base_dir` - Worktree storage location
- `mode` - Storage mode (local/global)
- `auto_gitignore` - Automatic .gitignore management
- `default_editor` - Editor for opening worktrees
- `cleanup.age_threshold_hours` - Minimum age for cleanup
- `cleanup.verify_remote` - Check remote before cleanup
- `cleanup.auto_delete_branch` - Delete branch after cleanup
- `merge_detection.use_github_cli` - Use gh CLI for detection
- `merge_detection.methods` - Detection methods to use
- `merge_detection.main_branches` - Main branch names to check

### Status Indicators

#### Status Icons
- `✅` - Clean (no uncommitted changes)
- `📝` - Has uncommitted changes
- `↑` - Has unpushed commits
- `❓` - Has untracked files
- `🔀` - Merged to main/feature branch
- `⚠️` - Needs attention (conflicts or issues)

#### Health Scores
- 100% - All worktrees clean and synced
- 75-99% - Minor issues (unpushed commits)
- 50-74% - Moderate issues (uncommitted changes)
- 0-49% - Major issues (conflicts, missing remotes)

### Integration Note

> **Note**: Worktree management is currently available through CLI commands (`vibe git worktree`) and MCP tools for AI assistance. Interactive menu integration is planned for a future update.