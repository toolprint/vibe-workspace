use anyhow::{Context, Result};
use console::style;
use std::process::Command;
use tokio::fs;

use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

pub async fn open_with_vscode(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    let vscode_integration = config
        .apps
        .vscode
        .as_ref()
        .context("VS Code integration is not configured")?;

    if !vscode_integration.enabled {
        anyhow::bail!("VS Code integration is disabled in configuration");
    }

    // Get the template to use
    let template_name = repo
        .get_app_template("vscode")
        .unwrap_or(&vscode_integration.default_template);

    // Load the template
    let template_content = template_manager
        .load_template("vscode", template_name)
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
    let workspace_path = vscode_integration.workspace_dir.join(&workspace_name);

    // Create workspace directory if it doesn't exist
    fs::create_dir_all(&vscode_integration.workspace_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create VS Code workspace directory: {}",
                vscode_integration.workspace_dir.display()
            )
        })?;

    // Write the workspace file
    fs::write(&workspace_path, workspace_content)
        .await
        .with_context(|| {
            format!(
                "Failed to write VS Code workspace: {}",
                workspace_path.display()
            )
        })?;

    println!(
        "{} Created VS Code workspace: {}",
        style("✅").green(),
        style(workspace_path.display()).cyan()
    );

    // Try to open with VS Code
    let result = Command::new("code").arg(&workspace_path).spawn();

    match result {
        Ok(_) => {
            println!(
                "{} Opened VS Code with workspace",
                style("✓").green().bold()
            );
        }
        Err(e) => {
            println!("{} Failed to open VS Code: {}", style("⚠️").yellow(), e);
            println!("\n{} Manual instructions:", style("📋").blue());
            println!("1. Open VS Code");
            println!("2. File → Open Workspace from File...");
            println!("3. Navigate to: {}", workspace_path.display());
        }
    }

    Ok(())
}

pub async fn cleanup_vscode_config(config: &WorkspaceConfig, repo: &Repository) -> Result<()> {
    let vscode_integration = config
        .apps
        .vscode
        .as_ref()
        .context("VS Code integration is not configured")?;

    if !vscode_integration.enabled {
        // If VS Code is disabled, no cleanup needed
        return Ok(());
    }

    // Generate the workspace file name that would have been created
    let workspace_name = format!(
        "vibe-{}-{}.code-workspace",
        config.workspace.name, repo.name
    );
    let workspace_path = vscode_integration.workspace_dir.join(&workspace_name);

    if workspace_path.exists() {
        fs::remove_file(&workspace_path).await.with_context(|| {
            format!(
                "Failed to remove VS Code workspace: {}",
                workspace_path.display()
            )
        })?;

        println!(
            "{} Removed VS Code workspace file: {}",
            style("🗑️").red(),
            style(workspace_path.display()).cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AppIntegrations, Repository, VSCodeIntegration, WorkspaceInfo};
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
                vscode: Some(VSCodeIntegration {
                    enabled: true,
                    workspace_dir: temp_dir.path().to_path_buf(),
                    template_dir: temp_dir.path().join("templates").join("vscode"),
                    default_template: "default".to_string(),
                }),
                wezterm: None,
            },
        }
    }
}
