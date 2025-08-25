# Task 03: CLI Integration

## Goal

Integrate worktree management functionality with the existing CLI system by adding worktree subcommands to the main command structure. This task exposes the core operations through a user-friendly command-line interface following existing patterns in the codebase.

## Scope

- Add `WorktreeCommands` enum to the main CLI structure
- Implement command parsing and validation
- Add worktree subcommand to the main git commands
- Implement command handlers that bridge CLI to worktree operations
- Add proper error handling and user feedback
- Support all planned CLI operations (create, list, remove, status)

## Implementation Details

### 1. Extend Main CLI Structure in `src/main.rs`

Add to the `GitCommands` enum (around line 419 where Clone command ends):

```rust
    /// Manage git worktrees for parallel development
    Worktree {
        #[command(subcommand)]
        action: WorktreeCommands,
    },
```

Add the new `WorktreeCommands` enum after the existing command enums:

```rust
#[derive(Subcommand)]
enum WorktreeCommands {
    /// Create a new worktree for parallel development
    Create {
        /// Task identifier to create worktree for
        task_id: String,
        
        /// Base branch to create worktree from
        #[arg(short, long)]
        base_branch: Option<String>,
        
        /// Force creation even if branch exists
        #[arg(short, long)]
        force: bool,
        
        /// Custom path for the worktree (overrides default)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Open worktree in editor after creation
        #[arg(short, long)]
        open: bool,
        
        /// Editor command to use (overrides default)
        #[arg(long)]
        editor: Option<String>,
    },
    
    /// List all git worktrees with status
    List {
        /// Show only worktrees with the specified prefix
        #[arg(short, long)]
        prefix: Option<String>,
        
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
        
        /// Output format: table, json, compact
        #[arg(short, long, default_value = "table")]
        format: String,
        
        /// Show only worktrees with uncommitted changes
        #[arg(short, long)]
        dirty_only: bool,
    },
    
    /// Remove a worktree
    Remove {
        /// Branch name or worktree path to remove
        target: String,
        
        /// Force removal even with uncommitted changes
        #[arg(short, long)]
        force: bool,
        
        /// Also delete the branch after removing worktree
        #[arg(short, long)]
        delete_branch: bool,
        
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },
    
    /// Show detailed status for worktree(s)
    Status {
        /// Branch name to show status for (all if not specified)
        branch: Option<String>,
        
        /// Show all worktrees
        #[arg(short, long)]
        all: bool,
        
        /// Output format: table, json, compact
        #[arg(short, long, default_value = "table")]
        format: String,
        
        /// Show only files that have changed
        #[arg(long)]
        files_only: bool,
    },
    
    /// Clean up merged worktrees
    Clean {
        /// Show what would be done without executing
        #[arg(short, long)]
        dry_run: bool,
        
        /// Force cleanup even with uncommitted changes
        #[arg(short, long)]
        force: bool,
        
        /// Minimum age in hours before cleanup
        #[arg(long)]
        age: Option<u64>,
        
        /// Skip confirmation prompts
        #[arg(long)]
        yes: bool,
    },
    
    /// Open a worktree in configured editor
    Open {
        /// Branch name or worktree path to open
        target: String,
        
        /// Editor command to use (overrides default)
        #[arg(short, long)]
        editor: Option<String>,
    },
}
```

### 2. Add Command Handler Implementation

In the main function's match statement (around line 800), add the worktree handling:

```rust
        GitCommands::Worktree { action } => {
            handle_worktree_command(action, &workspace_manager).await?;
        }
```

Add the handler function before the main function:

```rust
/// Handle worktree subcommands
async fn handle_worktree_command(
    command: WorktreeCommands,
    workspace_manager: &Arc<Mutex<WorkspaceManager>>,
) -> Result<()> {
    use crate::worktree::{WorktreeManager, CreateOptions, RemoveOptions};
    
    // Get the current repository root
    let workspace = workspace_manager.lock().await;
    let current_dir = std::env::current_dir()?;
    let git_root = find_git_repository_root(&current_dir).await?;
    drop(workspace);
    
    // Create worktree manager
    let worktree_manager = WorktreeManager::new(git_root, None).await?;
    
    match command {
        WorktreeCommands::Create { 
            task_id, 
            base_branch, 
            force, 
            path, 
            open, 
            editor 
        } => {
            let options = CreateOptions {
                task_id: task_id.clone(),
                base_branch,
                force,
                custom_path: path,
            };
            
            println!("Creating worktree for task: {}", task_id.cyan());
            let worktree_info = worktree_manager.create_worktree_with_options(options).await?;
            
            println!("✅ Created worktree:");
            println!("  Branch: {}", worktree_info.branch.yellow());
            println!("  Path: {}", worktree_info.path.display().to_string().blue());
            
            if open {
                let editor_cmd = editor.unwrap_or_else(|| "code".to_string());
                open_worktree_in_editor(&worktree_info.path, &editor_cmd).await?;
            }
        }
        
        WorktreeCommands::List { 
            prefix, 
            verbose, 
            format, 
            dirty_only 
        } => {
            let worktrees = worktree_manager.list_worktrees().await?;
            let filtered_worktrees = filter_worktrees(worktrees, prefix.as_deref(), dirty_only);
            
            match format.as_str() {
                "json" => print_worktrees_json(&filtered_worktrees)?,
                "compact" => print_worktrees_compact(&filtered_worktrees),
                _ => print_worktrees_table(&filtered_worktrees, verbose),
            }
        }
        
        WorktreeCommands::Remove { 
            target, 
            force, 
            delete_branch, 
            yes 
        } => {
            if !yes && !force {
                let confirmation = prompt_for_confirmation(&format!(
                    "Remove worktree '{}'{}?", 
                    target,
                    if delete_branch { " and delete branch" } else { "" }
                ))?;
                
                if !confirmation {
                    println!("Cancelled.");
                    return Ok(());
                }
            }
            
            let options = RemoveOptions {
                target: target.clone(),
                force,
                delete_branch,
            };
            
            println!("Removing worktree: {}", target.yellow());
            worktree_manager.remove_worktree_with_options(options).await?;
            println!("✅ Worktree removed successfully");
        }
        
        WorktreeCommands::Status { 
            branch, 
            all, 
            format, 
            files_only 
        } => {
            let worktrees = worktree_manager.list_worktrees().await?;
            let target_worktrees = if let Some(branch_name) = branch {
                worktrees.into_iter()
                    .filter(|w| w.branch == branch_name)
                    .collect()
            } else if all {
                worktrees
            } else {
                // Show current worktree by default
                worktrees.into_iter()
                    .filter(|w| w.path == std::env::current_dir().unwrap_or_default())
                    .collect()
            };
            
            if target_worktrees.is_empty() {
                println!("No matching worktrees found");
                return Ok(());
            }
            
            match format.as_str() {
                "json" => print_status_json(&target_worktrees, files_only)?,
                "compact" => print_status_compact(&target_worktrees, files_only),
                _ => print_status_table(&target_worktrees, files_only),
            }
        }
        
        WorktreeCommands::Clean { 
            dry_run, 
            force, 
            age, 
            yes 
        } => {
            println!("🧹 Cleaning up merged worktrees...");
            
            if dry_run {
                println!("DRY RUN - no changes will be made");
            }
            
            // TODO: Implement cleanup logic in Task 06
            println!("⚠️  Cleanup functionality will be implemented in a future task");
        }
        
        WorktreeCommands::Open { target, editor } => {
            let worktrees = worktree_manager.list_worktrees().await?;
            let worktree = worktrees.iter()
                .find(|w| w.branch == target || w.path.to_string_lossy() == target)
                .ok_or_else(|| anyhow::anyhow!("Worktree not found: {}", target))?;
            
            let editor_cmd = editor.unwrap_or_else(|| "code".to_string());
            open_worktree_in_editor(&worktree.path, &editor_cmd).await?;
        }
    }
    
    Ok(())
}

/// Find git repository root from current directory
async fn find_git_repository_root(start_dir: &Path) -> Result<PathBuf> {
    let mut current = start_dir.to_path_buf();
    
    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return Err(anyhow::anyhow!("Not in a git repository")),
        }
    }
}

/// Open a worktree in the specified editor
async fn open_worktree_in_editor(path: &Path, editor: &str) -> Result<()> {
    use tokio::process::Command;
    
    println!("Opening worktree in {}: {}", editor, path.display());
    
    let status = Command::new(editor)
        .arg(path)
        .status()
        .await
        .with_context(|| format!("Failed to execute editor: {}", editor))?;
    
    if !status.success() {
        return Err(anyhow::anyhow!("Editor command failed with status: {}", status));
    }
    
    println!("✅ Successfully opened worktree in {}", editor);
    Ok(())
}

/// Filter worktrees based on criteria
fn filter_worktrees(
    worktrees: Vec<crate::worktree::status::WorktreeInfo>,
    prefix: Option<&str>,
    dirty_only: bool,
) -> Vec<crate::worktree::status::WorktreeInfo> {
    worktrees.into_iter()
        .filter(|w| {
            if let Some(prefix) = prefix {
                if !w.branch.starts_with(prefix) {
                    return false;
                }
            }
            
            if dirty_only && w.status.is_clean {
                return false;
            }
            
            true
        })
        .collect()
}

/// Print worktrees in table format
fn print_worktrees_table(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    verbose: bool,
) {
    use colored::*;
    
    if worktrees.is_empty() {
        println!("No worktrees found");
        return;
    }
    
    // Header
    if verbose {
        println!("{:<40} {:<20} {:<12} {:<8} {}", 
                 "PATH".bold(), 
                 "BRANCH".bold(), 
                 "STATUS".bold(),
                 "AGE".bold(),
                 "HEAD".bold());
        println!("{}", "─".repeat(90));
    } else {
        println!("{:<40} {:<20} {:<12}", 
                 "PATH".bold(), 
                 "BRANCH".bold(), 
                 "STATUS".bold());
        println!("{}", "─".repeat(75));
    }
    
    for worktree in worktrees {
        let path = worktree.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&worktree.path.to_string_lossy());
            
        let branch = if worktree.branch.len() > 18 {
            format!("{}…", &worktree.branch[..17])
        } else {
            worktree.branch.clone()
        };
        
        let status = format!("{} {}", 
                            worktree.status.status_icon(),
                            worktree.status.status_description());
        
        if verbose {
            let age = format_age(worktree.age);
            let head = if worktree.head.len() > 7 { 
                &worktree.head[..7] 
            } else { 
                &worktree.head 
            };
            
            println!("{:<40} {:<20} {:<12} {:<8} {}", 
                     path.blue(),
                     branch.yellow(),
                     status,
                     age.dimmed(),
                     head.dimmed());
        } else {
            println!("{:<40} {:<20} {:<12}", 
                     path.blue(),
                     branch.yellow(),
                     status);
        }
    }
}

/// Print worktrees in compact format
fn print_worktrees_compact(worktrees: &[crate::worktree::status::WorktreeInfo]) {
    use colored::*;
    
    for worktree in worktrees {
        println!("{} {} {}", 
                 worktree.status.status_icon(),
                 worktree.branch.yellow(),
                 worktree.path.display().to_string().blue());
    }
}

/// Print worktrees in JSON format
fn print_worktrees_json(
    worktrees: &[crate::worktree::status::WorktreeInfo]
) -> Result<()> {
    let json = serde_json::to_string_pretty(worktrees)?;
    println!("{}", json);
    Ok(())
}

/// Print status in table format
fn print_status_table(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    files_only: bool,
) {
    // TODO: Implement detailed status display
    // This will be enhanced in Task 04: Status Tracking System
    for worktree in worktrees {
        println!("Worktree: {}", worktree.branch.yellow());
        println!("Path: {}", worktree.path.display().to_string().blue());
        println!("Status: {}", worktree.status.status_description());
        println!();
    }
}

/// Print status in compact format
fn print_status_compact(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    files_only: bool,
) {
    for worktree in worktrees {
        println!("{} {}: {}", 
                 worktree.status.status_icon(),
                 worktree.branch,
                 worktree.status.status_description());
    }
}

/// Print status in JSON format
fn print_status_json(
    worktrees: &[crate::worktree::status::WorktreeInfo],
    files_only: bool,
) -> Result<()> {
    let json = serde_json::to_string_pretty(worktrees)?;
    println!("{}", json);
    Ok(())
}

/// Format age duration for display
fn format_age(age: std::time::Duration) -> String {
    let hours = age.as_secs() / 3600;
    let days = hours / 24;
    
    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h", hours)
    } else {
        format!("{}m", age.as_secs() / 60)
    }
}

/// Prompt user for confirmation
fn prompt_for_confirmation(message: &str) -> Result<bool> {
    use std::io::{self, Write};
    
    print!("{} (y/N): ", message);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}
```

### 3. Add Required Imports

Add to the top of `src/main.rs`:

```rust
use crate::worktree::status::WorktreeInfo;
```

### 4. Update Help Documentation

The CLI help will be automatically generated by clap, but ensure the descriptions are clear and actionable.

### 5. Add Error Handling and User Feedback

Ensure all error messages are user-friendly and actionable:

```rust
/// Enhanced error handling for worktree operations
fn handle_worktree_error(error: anyhow::Error) -> anyhow::Error {
    use colored::*;
    
    let error_msg = error.to_string();
    
    // Provide helpful error messages for common issues
    if error_msg.contains("not in a git repository") {
        anyhow::anyhow!("{} This directory is not in a git repository", "Error:".red())
    } else if error_msg.contains("Branch") && error_msg.contains("already exists") {
        anyhow::anyhow!("{} Branch already exists. Use --force to recreate or choose a different task ID", "Error:".red())
    } else if error_msg.contains("Permission denied") {
        anyhow::anyhow!("{} Permission denied. Check your git repository permissions", "Error:".red())
    } else {
        error
    }
}
```

## Integration Points

### With Existing CLI Structure
- **Command Parsing**: Follows the same clap derive patterns as existing commands
- **Error Handling**: Uses consistent `anyhow::Result` error handling
- **Output Formatting**: Supports table, JSON, and compact formats like other commands
- **Color Output**: Uses the `colored` crate consistently with existing commands

### With WorkspaceManager
- **Repository Discovery**: Integrates with existing repository finding logic
- **Configuration**: Ready to use workspace-level configuration when available
- **Caching**: Can leverage existing caching mechanisms for performance

### User Experience
- **Confirmation Prompts**: Follows existing patterns for destructive operations
- **Progress Indicators**: Uses consistent visual indicators (✅, ⚠️, etc.)
- **Help System**: Integrates with clap's automatic help generation
- **Tab Completion**: Ready for future shell completion integration

## Success Criteria

### Functional Requirements
- [ ] `vibe git worktree create <task-id>` creates new worktrees
- [ ] `vibe git worktree list` shows all worktrees with status
- [ ] `vibe git worktree remove <target>` removes worktrees safely
- [ ] `vibe git worktree status [branch]` shows detailed status
- [ ] `vibe git worktree open <target>` opens worktrees in editor
- [ ] All commands support appropriate flags and options
- [ ] JSON output works for programmatic use
- [ ] Error messages are clear and actionable

### User Experience Requirements
- [ ] Commands follow existing CLI patterns and conventions
- [ ] Help text is clear and comprehensive
- [ ] Confirmation prompts prevent accidental data loss
- [ ] Output is readable and properly formatted
- [ ] Colors and icons enhance usability without being distracting

### Integration Requirements
- [ ] Compiles without warnings with existing codebase
- [ ] Follows existing error handling patterns
- [ ] Uses consistent formatting and output styles
- [ ] Integrates smoothly with workspace management

## Testing Requirements

### CLI Integration Tests
Create integration tests to verify command parsing and execution:

```rust
#[cfg(test)]
mod worktree_cli_tests {
    use super::*;
    use clap::Parser;
    
    #[test]
    fn test_worktree_create_command_parsing() {
        let args = vec![
            "vibe", "git", "worktree", "create", "test-feature",
            "--base-branch", "main", "--force", "--open"
        ];
        
        // This would test the clap parsing
        // Implementation depends on how the main CLI struct is organized
    }
    
    #[test]
    fn test_worktree_list_command_parsing() {
        let args = vec![
            "vibe", "git", "worktree", "list", 
            "--verbose", "--format", "json", "--prefix", "vibe-ws/"
        ];
        
        // Test command parsing
    }
    
    #[test] 
    fn test_format_age() {
        use std::time::Duration;
        
        assert_eq!(format_age(Duration::from_secs(30 * 60)), "30m");
        assert_eq!(format_age(Duration::from_secs(2 * 3600)), "2h");
        assert_eq!(format_age(Duration::from_secs(3 * 24 * 3600)), "3d");
    }
}
```

### Manual Testing Checklist
- [ ] Help text displays correctly for all subcommands
- [ ] Tab completion works (if implemented)
- [ ] Error messages are helpful and colored appropriately
- [ ] Long branch names and paths are truncated nicely
- [ ] JSON output is valid and complete
- [ ] Confirmation prompts can be bypassed with --yes

## Dependencies

Ensure these are in `Cargo.toml`:
```toml
[dependencies]
clap = { version = "4.0", features = ["derive", "color"] }
colored = "2.0"
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1.0", features = ["process"] }
```

## Notes

- The CLI design prioritizes discoverability and safety
- All destructive operations require confirmation unless bypassed
- Output formats support both human and machine consumption  
- Editor integration provides seamless workflow integration
- The command structure is extensible for future enhancements

## Future Enhancements

- Shell completion for branch names and worktree paths
- Interactive worktree selection with fuzzy matching
- Integration with git aliases and hooks
- Custom output templates
- Bulk operations on multiple worktrees

## Next Task

After completing this task, proceed to **Task 04: Status Tracking System** to implement comprehensive status checking and reporting for the worktrees displayed by these CLI commands.