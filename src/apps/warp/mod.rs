use anyhow::{Context, Result};
use console::style;
use tokio::fs;

use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

mod automation;
mod launcher;

pub use launcher::{LaunchMethod, WarpLauncher};

/// Clean up Warp configuration files for a repository
pub async fn cleanup_warp_config(config: &WorkspaceConfig, repo: &Repository) -> Result<()> {
    let warp_integration = config
        .apps
        .warp
        .as_ref()
        .context("Warp integration is not configured")?;

    if !warp_integration.enabled {
        // If Warp is disabled, no cleanup needed
        return Ok(());
    }

    // Generate the config file name that would have been created
    let config_name = format!("vibe-{}-{}.yaml", config.workspace.name, repo.name);
    let config_path = warp_integration.config_dir.join(&config_name);

    if config_path.exists() {
        fs::remove_file(&config_path)
            .await
            .with_context(|| format!("Failed to remove Warp config: {}", config_path.display()))?;

        println!(
            "{} Removed Warp configuration: {}",
            style("🗑️").red(),
            style(config_path.display()).cyan()
        );
    }

    Ok(())
}

/// Open a repository with Warp using cross-platform launcher
pub async fn open_with_warp(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    let warp_integration = config
        .apps
        .warp
        .as_ref()
        .context("Warp integration is not configured")?;

    if !warp_integration.enabled {
        anyhow::bail!("Warp integration is disabled in configuration");
    }

    // Create the cross-platform launcher
    let launcher = WarpLauncher::new(Some(warp_integration.config_dir.clone()))
        .context("Failed to create Warp launcher")?;

    println!(
        "{} Launching Warp with cross-platform support...",
        style("🚀").blue()
    );

    // Launch using the new cross-platform launcher
    let launch_result = launcher
        .launch_with_repo(config, repo, template_manager)
        .await?;

    // Report the launch result
    match launch_result.method {
        LaunchMethod::UriScheme => {
            println!("{} {}", style("✓").green().bold(), launch_result.message);
        }
        LaunchMethod::PlatformAutomation => {
            println!("{} {}", style("✓").green().bold(), launch_result.message);
        }
        LaunchMethod::ManualInstructions => {
            // Manual instructions were already printed by the launcher
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{Repository, WorkspaceInfo};
    use std::path::PathBuf;

    fn create_test_config() -> WorkspaceConfig {
        let mut config = WorkspaceConfig::default();
        config.workspace = WorkspaceInfo {
            name: "test-workspace".to_string(),
            root: PathBuf::from("/tmp/test"),
            auto_discover: false,
        };

        config.repositories = vec![
            Repository::new("frontend", "./frontend"),
            Repository::new("backend", "./backend"),
        ];

        config
    }
}
