use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;

use crate::workspace::templates::DEFAULT_ITERMOCIL_TEMPLATE;
use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

/// Check if iTermocil is installed on the system
fn is_itermocil_available() -> bool {
    Command::new("which")
        .arg("itermocil")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the iTermocil configuration directory
fn get_itermocil_config_dir() -> PathBuf {
    // iTermocil looks for configs in ~/.itermocil or ~/.teamocil
    if let Some(home) = dirs::home_dir() {
        let itermocil_dir = home.join(".itermocil");
        if itermocil_dir.exists() {
            return itermocil_dir;
        }
        // Fall back to .teamocil directory
        home.join(".teamocil")
    } else {
        PathBuf::from(".itermocil")
    }
}

#[allow(dead_code)]
pub async fn open_with_iterm2(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    open_with_iterm2_options(config, repo, template_manager, false).await
}

pub async fn open_with_iterm2_options(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
    no_itermocil: bool,
) -> Result<()> {
    let iterm2_integration = config
        .apps
        .iterm2
        .as_ref()
        .context("iTerm2 integration is not configured")?;

    if !iterm2_integration.enabled {
        anyhow::bail!("iTerm2 integration is disabled in configuration");
    }

    // Check if iTermocil should be used
    if !no_itermocil && is_itermocil_available() {
        println!(
            "{} Using iTermocil for advanced pane layout",
            style("🎯").blue()
        );
        match create_and_launch_itermocil_layout(config, repo, template_manager).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                println!("{} iTermocil launch failed: {}", style("⚠️").yellow(), e);
                println!(
                    "{} Falling back to Dynamic Profile method",
                    style("ℹ️").blue()
                );
            }
        }
    } else {
        if no_itermocil {
            println!(
                "{} iTermocil disabled by --no-itermocil flag",
                style("ℹ️").blue()
            );
        } else {
            println!(
                "{} iTermocil not found. Install it for automatic pane layouts:",
                style("💡").yellow()
            );
            println!("   brew install TomAnthony/brews/itermocil");
        }
        println!("{} Using Dynamic Profile instead", style("ℹ️").blue());
    }

    // Get the template to use
    let template_name = repo
        .get_app_template("iterm2")
        .unwrap_or(&iterm2_integration.default_template);

    // Load the template
    let template_content = template_manager
        .load_template("iterm2", template_name)
        .await
        .with_context(|| format!("Failed to load template '{template_name}'"))?;

    // Create variables for substitution
    let variables = TemplateManager::create_variables(config, repo);

    // Apply variable substitution
    let profile_content = template_manager.substitute_variables(&template_content, &variables);

    // Generate a unique profile file name
    let profile_name = format!("vibe-{}-{}.json", config.workspace.name, repo.name);
    let profile_path = iterm2_integration.config_dir.join(&profile_name);

    // Create config directory if it doesn't exist
    let config_dir = &iterm2_integration.config_dir;
    if let Some(parent) = profile_path.parent() {
        fs::create_dir_all(parent).await.with_context(|| {
            format!(
                "Failed to create iTerm2 config directory: {}",
                parent.display()
            )
        })?;
    } else {
        fs::create_dir_all(config_dir).await.with_context(|| {
            format!(
                "Failed to create iTerm2 config directory: {}",
                config_dir.display()
            )
        })?;
    }

    // Write the profile
    fs::write(&profile_path, profile_content)
        .await
        .with_context(|| format!("Failed to write iTerm2 profile: {}", profile_path.display()))?;

    println!(
        "{} Created iTerm2 dynamic profile: {}",
        style("✅").green(),
        style(profile_path.display()).cyan()
    );

    // Open with iTerm2 using AppleScript
    let profile_guid = format!("vibe-{}-{}", config.workspace.name, repo.name);
    let profile_name = format!("{} - {}", config.workspace.name, repo.name);

    // Give iTerm2 a moment to register the dynamic profile
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let applescript = format!(
        r#"tell application "System Events"
    if not (exists process "iTerm2") then
        tell application "iTerm2" to activate
        delay 1
    end if
end tell

tell application "iTerm2"
    activate
    
    -- Check if any windows exist
    if (count of windows) = 0 then
        -- Create a new window if none exist
        create window with default profile
        delay 0.5
    end if
    
    try
        -- Try to find profile by name first
        set newWindow to create window with profile "{}"
    on error
        try
            -- Try to find profile by GUID
            set newWindow to create window with profile "{}"
        on error
            -- If profile still not found, create a new window and navigate manually
            set newWindow to create window with default profile
            tell current session of newWindow
                write text "cd '{}'"
                write text "clear"
                write text "echo '🚀 Welcome to {} repository'"
                write text "echo '📍 Branch: {}'"
                write text "echo ''"
                write text "git status"
            end tell
        end try
    end try
end tell"#,
        profile_name,
        profile_guid,
        config.workspace.root.join(&repo.path).display(),
        repo.name,
        repo.branch.as_deref().unwrap_or("main")
    );

    let result = Command::new("osascript")
        .arg("-e")
        .arg(&applescript)
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                println!(
                    "{} Opened iTerm2 with repository",
                    style("✓").green().bold()
                );
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                println!(
                    "{} iTerm2 opened but profile may not have loaded: {}",
                    style("ℹ️").blue(),
                    error
                );
                println!("Repository should still be accessible in the new terminal window");
            }
        }
        Err(e) => {
            println!(
                "{} Failed to execute AppleScript: {}",
                style("⚠️").yellow(),
                e
            );
            println!("\n{} Manual instructions:", style("📋").blue());
            println!("1. Open iTerm2");
            println!("2. Look for profile '{profile_name}' or '{profile_guid}'");
            println!(
                "3. Or manually navigate to: {}",
                config.workspace.root.join(&repo.path).display()
            );
        }
    }

    Ok(())
}

pub async fn cleanup_iterm2_config(config: &WorkspaceConfig, repo: &Repository) -> Result<()> {
    let iterm2_integration = config
        .apps
        .iterm2
        .as_ref()
        .context("iTerm2 integration is not configured")?;

    if !iterm2_integration.enabled {
        // If iTerm2 is disabled, no cleanup needed
        return Ok(());
    }

    // Generate the profile file name that would have been created
    let profile_name = format!("vibe-{}-{}.json", config.workspace.name, repo.name);
    let profile_path = iterm2_integration.config_dir.join(&profile_name);

    if profile_path.exists() {
        fs::remove_file(&profile_path).await.with_context(|| {
            format!(
                "Failed to remove iTerm2 profile: {}",
                profile_path.display()
            )
        })?;

        println!(
            "{} Removed iTerm2 dynamic profile: {}",
            style("🗑️").red(),
            style(profile_path.display()).cyan()
        );
    }

    Ok(())
}

/// Create an iTermocil layout file and launch it
async fn create_and_launch_itermocil_layout(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    let iterm2_integration = config
        .apps
        .iterm2
        .as_ref()
        .context("iTerm2 integration is not configured")?;

    // Get the template to use - check if we have an itermocil-specific template
    let template_name = repo
        .get_app_template("iterm2")
        .unwrap_or(&iterm2_integration.default_template);

    // Try to load an itermocil template first (YAML), fall back to generating from variables
    let yaml_content = match template_manager
        .load_template("iterm2", &format!("{template_name}-itermocil"))
        .await
    {
        Ok(content) => {
            // We have a specific iTermocil template, use it
            let variables = TemplateManager::create_variables(config, repo);
            template_manager.substitute_variables(&content, &variables)
        }
        Err(_) => {
            // Generate iTermocil YAML from our variables
            generate_itermocil_yaml(config, repo)
        }
    };

    // Generate a unique layout file name
    let layout_name = format!("vibe-{}-{}", config.workspace.name, repo.name);
    let itermocil_dir = get_itermocil_config_dir();
    let layout_path = itermocil_dir.join(format!("{layout_name}.yml"));

    // Ensure the parent directory of the layout file exists
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent).await.with_context(|| {
            format!("Failed to create iTermocil directory: {}", parent.display())
        })?;
    } else {
        fs::create_dir_all(&itermocil_dir).await.with_context(|| {
            format!(
                "Failed to create iTermocil directory: {}",
                itermocil_dir.display()
            )
        })?;
    }

    // Write the layout file
    fs::write(&layout_path, yaml_content)
        .await
        .with_context(|| {
            format!(
                "Failed to write iTermocil layout: {}",
                layout_path.display()
            )
        })?;

    println!(
        "{} Created iTermocil layout: {}",
        style("✅").green(),
        style(layout_path.display()).cyan()
    );

    // Launch iTermocil with our layout
    let result = Command::new("itermocil").arg(&layout_name).spawn();

    match result {
        Ok(_) => {
            println!(
                "{} Launched iTerm2 with 3-pane layout via iTermocil",
                style("✓").green().bold()
            );
            println!("{} Panes:", style("📋").blue());
            println!("   • Left: Agent launcher");
            println!("   • Top-right: Git manager");
            println!("   • Bottom-right: Project commands");
        }
        Err(e) => {
            anyhow::bail!("Failed to launch iTermocil: {}", e);
        }
    }

    Ok(())
}

/// Generate iTermocil YAML configuration from template variables
fn generate_itermocil_yaml(config: &WorkspaceConfig, repo: &Repository) -> String {
    // Use the DEFAULT_ITERMOCIL_TEMPLATE constant
    let template = DEFAULT_ITERMOCIL_TEMPLATE;
    let variables = TemplateManager::create_variables(config, repo);

    // Create a TemplateManager instance just for substitution
    let temp_dir = std::env::temp_dir();
    let template_manager = TemplateManager::new(temp_dir);

    // Substitute variables in the template
    template_manager.substitute_variables(template, &variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AppIntegrations, ITerm2Integration, Repository, WorkspaceInfo};
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
                vscode: None,
                iterm2: Some(ITerm2Integration {
                    enabled: true,
                    config_dir: temp_dir.path().to_path_buf(),
                    template_dir: temp_dir.path().join("templates").join("iterm2"),
                    default_template: "default".to_string(),
                }),
                wezterm: None,
                cursor: None,
                windsurf: None,
            },
            preferences: None,
        }
    }
}
