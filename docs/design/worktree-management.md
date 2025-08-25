# Git Worktree Management System Design

## Executive Summary

This document outlines the design for a comprehensive Git worktree management system integrated into vibe-workspace. The system combines the best features from existing implementations (claude-task and workbloom) while meeting specific requirements for CLI-first operation, safety, and integration with vibe-workspace's architecture.

The system provides developers with parallel development environments through Git worktrees, intelligent status tracking, automated cleanup, and AI-assisted conflict resolution - all while maintaining strict safety guarantees to prevent data loss.

## Requirements & Constraints

### Functional Requirements

1. **CLI-First Design**: All operations must be possible via CLI (interactive modes optional)
2. **Rust Implementation**: Must be embeddable within vibe-workspace
3. **Configurable Base Directory**: Default `.worktrees/` with custom path support
4. **Automatic .gitignore Management**: Prevent worktrees within repo from being committed
5. **Branch Prefix System**: Default `vibe-ws/` prefix with subfolder support
6. **Comprehensive Status Tracking**: Track uncommitted changes, remote sync, and merge status
7. **Safety-First Cleanup**: Multiple validation layers before removing worktrees
8. **Conflict Detection**: Generate compact diffs for AI-assisted resolution
9. **Task Identifier Support**: Placeholder system for future task management integration

### Non-Functional Requirements

1. **Safety**: Never delete uncommitted work without explicit force flags
2. **Performance**: Leverage existing vibe-workspace caching and git operations
3. **User Experience**: Clear status indicators and actionable information
4. **Extensibility**: Support future enhancements like task integration
5. **Reliability**: Robust error handling and recovery mechanisms

## Architecture Overview

### Module Structure

```
src/worktree/
├── mod.rs                  # Module exports and public interface
├── manager.rs              # WorktreeManager - main coordinator
├── operations.rs           # Core git worktree operations
├── status.rs               # Status checking and reporting
├── merge_detection.rs      # Advanced merge detection algorithms
├── cleanup.rs              # Cleanup strategies and safety mechanisms
└── config.rs               # Worktree-specific configuration
```

### Key Types and Structures

```rust
pub struct WorktreeManager {
    pub workspace_manager: Arc<WorkspaceManager>,
    pub config: WorktreeConfig,
    pub git_config: GitConfig,
}

pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub head: String,
    pub status: WorktreeStatus,
    pub age: Duration,
}

pub struct WorktreeStatus {
    pub is_clean: bool,
    pub severity: StatusSeverity,
    pub uncommitted_changes: Vec<String>,
    pub untracked_files: Vec<String>,
    pub unpushed_commits: Vec<CommitInfo>,
    pub remote_status: RemoteStatus,
    pub merge_info: Option<MergeInfo>,
}

pub enum StatusSeverity {
    Clean,          // ✅ No issues
    LightWarning,   // ⚠️ Worktree issues (uncommitted/unsynced)
    Warning,        // ⚡ Feature branch issues
}
```

## Core Components

### 1. WorktreeManager

**Responsibilities:**
- Coordinate all worktree operations
- Integrate with existing WorkspaceManager
- Manage configuration and validation
- Provide unified API for CLI and MCP tools

**Key Methods:**
- `create_worktree(task_id: &str) -> Result<WorktreeInfo>`
- `list_worktrees() -> Result<Vec<WorktreeInfo>>`
- `get_worktree_status(branch: &str) -> Result<WorktreeStatus>`
- `cleanup_merged_worktrees(options: CleanupOptions) -> Result<CleanupReport>`
- `merge_to_feature_branch(branch: &str) -> Result<MergeResult>`
- `backup_to_origin(branch: &str) -> Result<BackupResult>`

### 2. Operations Module

**Core Git Operations:**
- **Create Worktree**: Handles branch creation, path resolution, and setup
- **Remove Worktree**: Safely removes worktrees with validation
- **List Worktrees**: Parses `git worktree list --porcelain` output
- **Branch Management**: Create, delete, and validate branches

**Path Management:**
- Resolve worktree base directory (absolute/relative/home paths)
- Handle subfolder creation for branches with `/` (e.g., `feat/new-ui`)
- Ensure .gitignore compliance for worktrees within repo

### 3. Status Tracking System

**Three-Tier Severity Model:**

**Clean Status (✅):**
- No uncommitted changes
- All commits pushed to remote OR branch is merged
- No untracked files

**Light Warning (⚠️) - Worktree Issues:**
- Uncommitted changes in worktree
- Unpushed commits (but remote tracking exists)
- No remote tracking branch set

**Warning (⚡) - Feature Branch Issues:**
- Stale branch (no activity for extended period)
- Conflicting remote changes
- Branch exists locally but not on remote

**Status Information Collected:**
- Changed files with modification types
- Untracked files list
- Unpushed commits with messages and IDs
- Remote tracking branch status
- Age of worktree directory
- Merge detection results

### 4. Advanced Merge Detection

**Multiple Detection Methods (from claude-task):**

1. **Standard Git Merge Detection**
   - Use `git branch --merged main` to find regularly merged branches

2. **Squash Merge Detection**
   - Compare diff between merge-base and branch tip
   - Check if all changes are present in main branch
   - Analyze commit messages for squash merge patterns

3. **GitHub PR Integration**
   - Use GitHub CLI to check PR merge status
   - Parse PR metadata for merge information

4. **File Content Analysis**
   - Compare file contents between branch and main
   - Detect when changes are present but commits differ (squash scenario)

**Safety Validations (from workbloom):**
- Verify remote branch existence before considering merge
- Check branch age (skip branches < 24 hours old)
- Compare HEAD commits to detect empty branches
- Validate against known active development branches

### 5. Cleanup System

**Cleanup Strategies:**

1. **Discard Worktree**
   - Remove worktree directory
   - Optionally delete branch (with confirmation)
   - Multiple safety checks before deletion

2. **Merge to Feature Branch**
   - Merge worktree changes into main feature branch
   - Handle merge conflicts with AI assistance
   - Provide conflict summaries for resolution

3. **Backup to Origin**
   - Push worktree branch to remote
   - Ensure branch is replicated before removal
   - Update remote tracking information

**Safety Layers:**
1. **Age Check**: Skip worktrees < 24 hours old
2. **Remote Verification**: Ensure branch exists on remote before cleanup
3. **Merge Status**: Verify actual merge status (not just branch comparison)
4. **Force Requirement**: Require explicit `--force` for uncommitted changes
5. **User Confirmation**: Interactive confirmation for bulk operations

## CLI Interface Specification

### Core Commands

```bash
# Create new worktree from task identifier
vibe worktree create <task-id> [--base-branch <branch>]

# List all worktrees with status indicators
vibe worktree list [--verbose] [--prefix <prefix>]

# Show detailed status for worktree(s)
vibe worktree status [<branch>] [--all]

# Clean up merged worktrees
vibe worktree clean [--dry-run] [--force] [--age <hours>]

# Remove specific worktree
vibe worktree remove <branch> [--force] [--keep-branch]

# Merge worktree to feature branch
vibe worktree merge <branch> [--strategy <strategy>]

# Backup worktree to origin before cleanup
vibe worktree backup <branch> [--cleanup-after]

# Show conflict summary for merge resolution
vibe worktree conflicts <branch> [--compact]

# Open worktree in configured editor
vibe worktree open <branch> [--editor <command>]
```

### Command Options

**Global Options:**
- `--worktree-dir <path>`: Override default worktree base directory
- `--prefix <prefix>`: Override default branch prefix
- `--config <file>`: Use specific configuration file

**Safety Options:**
- `--dry-run`: Show what would be done without executing
- `--force`: Override safety checks for risky operations
- `--confirm`: Skip interactive confirmations
- `--age <hours>`: Minimum age threshold for operations

**Output Options:**
- `--verbose`: Show detailed information
- `--quiet`: Minimal output mode
- `--json`: Machine-readable JSON output
- `--compact`: Condensed output format

## Configuration System

### Worktree Configuration Schema

```yaml
# ~/.toolprint/vibe-workspace/config.yaml
worktree:
  # Base directory for all worktrees (relative to repo root)
  base_dir: ".worktrees"
  
  # Default branch prefix for managed worktrees
  prefix: "vibe-ws/"
  
  # Automatically manage .gitignore for worktree directories
  auto_gitignore: true
  
  # Default editor command for opening worktrees
  default_editor: "code"
  
  # Cleanup configuration
  cleanup:
    # Minimum age (hours) before worktree can be cleaned
    age_threshold_hours: 24
    
    # Verify remote branch exists before cleanup
    verify_remote: true
    
    # Automatically delete branch after worktree removal
    auto_delete_branch: false
    
    # Require confirmation for bulk operations
    require_confirmation: true
  
  # Merge detection configuration
  merge_detection:
    # Enable GitHub CLI integration for PR status
    use_github_cli: true
    
    # Methods to use for merge detection (in order of preference)
    methods: ["standard", "squash", "github_pr", "file_content"]
    
    # Main branches to check merges against
    main_branches: ["main", "master"]
  
  # Status display configuration
  status:
    # Show file lists in status output
    show_files: true
    
    # Maximum number of files to display
    max_files_shown: 10
    
    # Show commit messages for unpushed commits
    show_commit_messages: true
    
    # Maximum number of commits to display
    max_commits_shown: 5
  
  # Task identifier configuration (future)
  task_integration:
    # Provider for task IDs (future extension point)
    provider: null
    
    # Pattern for generating branch names from task IDs
    branch_pattern: "{prefix}{task_id}"
    
    # Whether to include task metadata in branch
    include_metadata: false
```

### Environment Variables

- `VIBE_WORKTREE_BASE`: Override base directory
- `VIBE_WORKTREE_PREFIX`: Override branch prefix
- `VIBE_WORKTREE_EDITOR`: Override default editor
- `VIBE_WORKTREE_FORCE`: Enable force mode (use with caution)

## Safety & Validation Rules

### Branch Name Validation

1. **Security Validation**: Prevent command injection via branch names
2. **Git Compatibility**: Ensure branch names conform to Git standards
3. **Character Restrictions**: Block dangerous characters (`$`, `` ` ``, `|`, `&`, etc.)
4. **Path Safety**: Prevent directory traversal attacks
5. **Length Limits**: Enforce reasonable branch name lengths

### Worktree Path Validation

1. **Base Directory Checks**: Ensure base directory exists and is writable
2. **Gitignore Compliance**: Verify worktrees within repo are gitignored
3. **Path Conflicts**: Prevent path conflicts with existing files/directories
4. **Permissions**: Validate write permissions for worktree creation

### Cleanup Safety Checks

1. **Age Verification**: Minimum 24-hour age before cleanup (configurable)
2. **Remote Existence**: Verify branch exists on remote before considering merged
3. **Uncommitted Changes**: Prevent deletion of uncommitted work without force
4. **Merge Status**: Use multiple methods to verify actual merge status
5. **User Confirmation**: Interactive confirmation for destructive operations

### Data Protection

1. **Backup Before Cleanup**: Option to push branches before removal
2. **Stash Integration**: Automatic stashing of uncommitted changes
3. **Recovery Information**: Log cleanup operations for potential recovery
4. **Dry-Run Mode**: Preview operations without execution

## Integration Points

### MCP Tools Integration

**Exposed MCP Tools:**
- `worktree_list`: List all worktrees with status
- `worktree_status`: Get detailed status for specific worktree
- `worktree_conflicts`: Generate conflict summaries for AI analysis
- `worktree_create`: Create new worktree
- `worktree_cleanup`: Clean up merged worktrees

**AI-Assisted Features:**
- Conflict summarization for merge resolution
- Automated merge strategy suggestions
- Branch health analysis and recommendations
- Cleanup decision assistance based on branch activity

### Workspace Manager Integration

**Shared Components:**
- Git configuration and credentials
- Repository path management
- Caching system for performance
- Error handling and logging

**Configuration Integration:**
- Extend existing workspace configuration schema
- Share repository settings and preferences
- Integrate with existing CLI command structure

### Cache System Integration

**Cached Information:**
- Worktree status and metadata
- Branch merge status
- Remote branch existence
- File modification times

**Cache Invalidation:**
- Git operations (commits, pushes, merges)
- Branch creation/deletion
- Worktree creation/removal
- Configuration changes

## Implementation Roadmap

### Phase 1: Core Operations (Weeks 1-2)
- Basic worktree creation and removal
- Simple status checking and listing
- CLI command structure and parsing
- Configuration system foundation
- Safety validation framework

**Deliverables:**
- `vibe worktree create <task-id>`
- `vibe worktree list`
- `vibe worktree remove <branch>`
- Basic configuration support
- Branch name validation

### Phase 2: Advanced Status & Safety (Weeks 3-4)
- Comprehensive merge detection algorithms
- Multi-layer safety checks for cleanup
- Detailed status reporting with file lists
- Remote branch synchronization
- Age-based protection mechanisms

**Deliverables:**
- `vibe worktree status [branch]`
- `vibe worktree clean --dry-run`
- Advanced merge detection (squash, PR integration)
- Enhanced safety checks
- Status severity classification

### Phase 3: Cleanup & Automation (Weeks 5-6)
- Smart cleanup with multiple safety filters
- Merge to feature branch functionality
- Backup to origin before removal
- Interactive and batch cleanup modes
- Conflict detection and reporting

**Deliverables:**
- `vibe worktree clean [--force]`
- `vibe worktree merge <branch>`
- `vibe worktree backup <branch>`
- `vibe worktree conflicts <branch>`
- Comprehensive cleanup strategies

### Phase 4: Integration & Enhancement (Weeks 7-8)
- MCP tool exposure for AI integration
- Advanced conflict summarization
- Task identifier system foundation
- Performance optimization with caching
- Documentation and testing completion

**Deliverables:**
- MCP tools for worktree management
- AI-assisted conflict resolution
- Task ID integration framework
- Performance benchmarks
- Complete documentation

## Future Enhancements

### Advanced Task Integration
- External task management system integration
- Automatic branch naming from task metadata
- Task status synchronization
- Time tracking integration

### Enhanced AI Assistance
- Automated conflict resolution suggestions
- Code change analysis and summarization
- Branch merge recommendations
- Development workflow optimization

### Performance & Scalability
- Parallel worktree operations
- Incremental status updates
- Background cleanup processes
- Large repository optimizations

### Developer Experience
- IDE integrations (VS Code, etc.)
- Shell completions and aliases
- Visual status indicators
- Workflow automation scripts

## Risk Assessment & Mitigation

### Data Loss Risks
**Risk**: Accidental deletion of uncommitted work
**Mitigation**: Multiple safety layers, force flags, automatic stashing

### Performance Risks
**Risk**: Slow operations on large repositories
**Mitigation**: Caching system, parallel operations, incremental updates

### Security Risks
**Risk**: Command injection via branch names
**Mitigation**: Strict input validation, allowlist approach

### Integration Risks
**Risk**: Conflicts with existing vibe-workspace functionality
**Mitigation**: Careful API design, extensive testing, gradual rollout

## Success Criteria

### Functional Success
- ✅ All CLI operations work reliably
- ✅ Zero data loss incidents during testing
- ✅ Successful integration with existing vibe-workspace features
- ✅ MCP tools provide useful AI assistance

### Performance Success
- ✅ Worktree operations complete within acceptable time limits
- ✅ Status checking scales to repositories with many worktrees
- ✅ Cache system provides measurable performance improvements

### User Experience Success
- ✅ Clear, actionable status information
- ✅ Intuitive CLI interface
- ✅ Helpful error messages and recovery suggestions
- ✅ Seamless integration with developer workflows

---

This design document serves as the foundation for implementing a robust, safe, and user-friendly Git worktree management system within vibe-workspace. The system combines proven patterns from existing tools while introducing novel safety mechanisms and AI integration capabilities.