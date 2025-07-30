use anyhow::{Context, Result};
use console::style;
use std::process::Command;
use tokio::fs;

use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

pub async fn open_with_cursor(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    let cursor_integration = config
        .apps
        .cursor
        .as_ref()
        .context("Cursor integration is not configured")?;

    if !cursor_integration.enabled {
        anyhow::bail!("Cursor integration is disabled in configuration");
    }

    // Get the template to use
    let template_name = repo
        .get_app_template("cursor")
        .unwrap_or(&cursor_integration.default_template);

    // Load the template
    let template_content = template_manager
        .load_template("cursor", template_name)
        .await
        .with_context(|| format!("Failed to load template '{template_name}'"))?;

    // Create variables for substitution
    let variables = TemplateManager::create_variables(config, repo);

    // Apply variable substitution
    let workspace_content = template_manager.substitute_variables(&template_content, &variables);

    // Generate a unique workspace file name
    let workspace_name = format!(
        "vibe-{}-{}.code-workspace",
        config.workspace.name, repo.name
    );
    let workspace_path = cursor_integration.workspace_dir.join(&workspace_name);

    // Create workspace directory if it doesn't exist
    fs::create_dir_all(&cursor_integration.workspace_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create Cursor workspace directory: {}",
                cursor_integration.workspace_dir.display()
            )
        })?;

    // Write the workspace file
    fs::write(&workspace_path, workspace_content)
        .await
        .with_context(|| {
            format!(
                "Failed to write Cursor workspace: {}",
                workspace_path.display()
            )
        })?;

    println!(
        "{} Created Cursor workspace: {}",
        style("✅").green(),
        style(workspace_path.display()).cyan()
    );

    // Try to open with Cursor
    let result = Command::new("cursor").arg(&workspace_path).spawn();

    match result {
        Ok(_) => {
            println!("{} Opened Cursor with workspace", style("✓").green().bold());
        }
        Err(e) => {
            println!("{} Failed to open Cursor: {}", style("⚠️").yellow(), e);
            println!("\n{} Manual instructions:", style("📋").blue());
            println!("1. Open Cursor");
            println!("2. File → Open Workspace from File...");
            println!("3. Navigate to: {}", workspace_path.display());
        }
    }

    Ok(())
}

pub async fn cleanup_cursor_config(config: &WorkspaceConfig, repo: &Repository) -> Result<()> {
    let cursor_integration = config
        .apps
        .cursor
        .as_ref()
        .context("Cursor integration is not configured")?;

    if !cursor_integration.enabled {
        // If Cursor is disabled, no cleanup needed
        return Ok(());
    }

    // Generate the workspace file name that would have been created
    let workspace_name = format!(
        "vibe-{}-{}.code-workspace",
        config.workspace.name, repo.name
    );
    let workspace_path = cursor_integration.workspace_dir.join(&workspace_name);

    if workspace_path.exists() {
        fs::remove_file(&workspace_path).await.with_context(|| {
            format!(
                "Failed to remove Cursor workspace: {}",
                workspace_path.display()
            )
        })?;

        println!(
            "{} Removed Cursor workspace file: {}",
            style("🗑️").red(),
            style(workspace_path.display()).cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AppIntegrations, CursorIntegration, Repository, WorkspaceInfo};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_config() -> WorkspaceConfig {
        let temp_dir = TempDir::new().unwrap();

        WorkspaceConfig {
            workspace: WorkspaceInfo {
                name: "test-workspace".to_string(),
                root: PathBuf::from("/tmp/test"),
                auto_discover: false,
            },
            repositories: vec![
                Repository::new("frontend", "./frontend"),
                Repository::new("backend", "./backend"),
            ],
            groups: vec![],
            apps: AppIntegrations {
                github: None,
                warp: None,
                iterm2: None,
                vscode: None,
                wezterm: None,
                cursor: Some(CursorIntegration {
                    enabled: true,
                    workspace_dir: temp_dir.path().to_path_buf(),
                    template_dir: temp_dir.path().join("templates").join("cursor"),
                    default_template: "default".to_string(),
                }),
                windsurf: None,
            },
            preferences: None,
        }
    }
}
