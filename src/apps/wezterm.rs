use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;

use crate::workspace::templates::DEFAULT_WEZTERMOCIL_TEMPLATE;
use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

/// Check if weztermocil is installed on the system
fn is_weztermocil_available() -> bool {
    Command::new("which")
        .arg("weztermocil")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the weztermocil configuration directory
fn get_weztermocil_config_dir() -> PathBuf {
    // weztermocil looks for configs in ~/.config/weztermocil
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("weztermocil")
}

pub async fn open_with_wezterm(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    open_with_wezterm_options(config, repo, template_manager, false).await
}

pub async fn open_with_wezterm_options(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
    no_weztermocil: bool,
) -> Result<()> {
    let wezterm_integration = config
        .apps
        .wezterm
        .as_ref()
        .context("WezTerm integration is not configured")?;

    if !wezterm_integration.enabled {
        anyhow::bail!("WezTerm integration is disabled in configuration");
    }

    // Check if weztermocil should be used
    if !no_weztermocil && is_weztermocil_available() {
        println!(
            "{} Using weztermocil for advanced pane layout",
            style("🎯").blue()
        );
        match create_and_launch_weztermocil_layout(config, repo, template_manager).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                println!("{} weztermocil launch failed: {}", style("⚠️").yellow(), e);
                println!(
                    "{} Falling back to Lua configuration method",
                    style("ℹ️").blue()
                );
            }
        }
    } else {
        if no_weztermocil {
            println!(
                "{} weztermocil disabled by --no-weztermocil flag",
                style("ℹ️").blue()
            );
        } else {
            println!(
                "{} weztermocil not found. Install it for automatic pane layouts:",
                style("💡").yellow()
            );
            println!("   brew update && brew install alexcaza/tap/weztermocil");
        }
        println!("{} Using Lua configuration instead", style("ℹ️").blue());
    }

    // Get the template to use
    let template_name = repo
        .get_app_template("wezterm")
        .unwrap_or(&wezterm_integration.default_template);

    // Load the template
    let template_content = template_manager
        .load_template("wezterm", template_name)
        .await
        .with_context(|| format!("Failed to load template '{template_name}'"))?;

    // Create variables for substitution
    let variables = TemplateManager::create_variables(config, repo);

    // Apply variable substitution
    let config_content = template_manager.substitute_variables(&template_content, &variables);

    // Generate a unique config file name
    let config_name = format!("vibe-{}-{}.lua", config.workspace.name, repo.name);
    let config_path = wezterm_integration.config_dir.join(&config_name);

    // Create config directory if it doesn't exist
    fs::create_dir_all(&wezterm_integration.config_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create WezTerm config directory: {}",
                wezterm_integration.config_dir.display()
            )
        })?;

    // Write the config
    fs::write(&config_path, config_content)
        .await
        .with_context(|| format!("Failed to write WezTerm config: {}", config_path.display()))?;

    println!(
        "{} Created WezTerm configuration: {}",
        style("✅").green(),
        style(config_path.display()).cyan()
    );

    // Launch WezTerm with the custom config
    // Try different methods to launch WezTerm with our config
    let methods = [
        // Method 1: Use --config-file flag
        (
            "--config-file",
            vec!["--config-file", config_path.to_str().unwrap()],
        ),
        // Method 2: Use WEZTERM_CONFIG_FILE environment variable
        ("WEZTERM_CONFIG_FILE", vec![]),
    ];

    let mut launched = false;
    for (method_name, args) in &methods {
        let mut cmd = Command::new("wezterm");

        if method_name == &"WEZTERM_CONFIG_FILE" {
            cmd.env("WEZTERM_CONFIG_FILE", &config_path);
        } else {
            for arg in args {
                cmd.arg(arg);
            }
        }

        match cmd.spawn() {
            Ok(_) => {
                println!(
                    "{} Launched WezTerm with custom configuration (method: {})",
                    style("✓").green().bold(),
                    method_name
                );
                launched = true;
                break;
            }
            Err(e) => {
                println!(
                    "{} Failed to launch WezTerm with {}: {}",
                    style("⚠️").yellow(),
                    method_name,
                    e
                );
            }
        }
    }

    if !launched {
        println!("\n{} Manual instructions:", style("📋").blue());
        println!("1. Open WezTerm");
        println!("2. Set WEZTERM_CONFIG_FILE={}", config_path.display());
        println!("3. Or copy config to: ~/.wezterm.lua");
        println!("4. Or run: wezterm --config-file {}", config_path.display());
    }

    Ok(())
}

pub async fn cleanup_wezterm_config(config: &WorkspaceConfig, repo: &Repository) -> Result<()> {
    let wezterm_integration = config
        .apps
        .wezterm
        .as_ref()
        .context("WezTerm integration is not configured")?;

    if !wezterm_integration.enabled {
        // If WezTerm is disabled, no cleanup needed
        return Ok(());
    }

    // Clean up Lua config
    let config_name = format!("vibe-{}-{}.lua", config.workspace.name, repo.name);
    let config_path = wezterm_integration.config_dir.join(&config_name);

    if config_path.exists() {
        fs::remove_file(&config_path).await.with_context(|| {
            format!("Failed to remove WezTerm config: {}", config_path.display())
        })?;

        println!(
            "{} Removed WezTerm configuration: {}",
            style("🗑️").red(),
            style(config_path.display()).cyan()
        );
    }

    // Clean up weztermocil layout
    let layout_name = format!("vibe-{}-{}", config.workspace.name, repo.name);
    let layout_path = get_weztermocil_config_dir().join(format!("{layout_name}.yml"));

    if layout_path.exists() {
        fs::remove_file(&layout_path).await.with_context(|| {
            format!(
                "Failed to remove weztermocil layout: {}",
                layout_path.display()
            )
        })?;

        println!(
            "{} Removed weztermocil layout: {}",
            style("🗑️").red(),
            style(layout_path.display()).cyan()
        );
    }

    Ok(())
}

/// Create a weztermocil layout file and launch it
async fn create_and_launch_weztermocil_layout(
    config: &WorkspaceConfig,
    repo: &Repository,
    template_manager: &TemplateManager,
) -> Result<()> {
    let wezterm_integration = config
        .apps
        .wezterm
        .as_ref()
        .context("WezTerm integration is not configured")?;

    // Get the template to use - check if we have a weztermocil-specific template
    let template_name = repo
        .get_app_template("wezterm")
        .unwrap_or(&wezterm_integration.default_template);

    // Try to load a weztermocil template first (YAML), fall back to generating from variables
    let yaml_content = match template_manager
        .load_template("wezterm", &format!("{template_name}-weztermocil"))
        .await
    {
        Ok(content) => {
            // We have a specific weztermocil template, use it
            let variables = TemplateManager::create_variables(config, repo);
            template_manager.substitute_variables(&content, &variables)
        }
        Err(_) => {
            // Generate weztermocil YAML from our variables
            generate_weztermocil_yaml(config, repo)
        }
    };

    // Create the weztermocil configuration directory if it doesn't exist
    let weztermocil_dir = get_weztermocil_config_dir();
    fs::create_dir_all(&weztermocil_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create weztermocil directory: {}",
                weztermocil_dir.display()
            )
        })?;

    // Generate a unique layout file name
    let layout_name = format!("vibe-{}-{}", config.workspace.name, repo.name);
    let layout_path = weztermocil_dir.join(format!("{layout_name}.yml"));

    // Write the layout file
    fs::write(&layout_path, yaml_content)
        .await
        .with_context(|| {
            format!(
                "Failed to write weztermocil layout: {}",
                layout_path.display()
            )
        })?;

    println!(
        "{} Created weztermocil layout: {}",
        style("✅").green(),
        style(layout_path.display()).cyan()
    );

    // Launch weztermocil with our layout and wait for it to complete
    let result = Command::new("weztermocil").arg(&layout_name).output();

    match result {
        Ok(output) => {
            if output.status.success() {
                println!(
                    "{} Launched WezTerm with 3-pane layout via weztermocil",
                    style("✓").green().bold()
                );
                println!("{} Panes:", style("📋").blue());
                println!("   • Left: Agent launcher");
                println!("   • Top-right: Git manager");
                println!("   • Bottom-right: Project commands");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                anyhow::bail!(
                    "weztermocil failed with exit code {:?}\nStdout: {}\nStderr: {}",
                    output.status.code(),
                    stdout,
                    stderr
                );
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to execute weztermocil: {}", e);
        }
    }

    Ok(())
}

/// Generate weztermocil YAML configuration from template variables
fn generate_weztermocil_yaml(config: &WorkspaceConfig, repo: &Repository) -> String {
    // Use the DEFAULT_WEZTERMOCIL_TEMPLATE constant
    let template = DEFAULT_WEZTERMOCIL_TEMPLATE;
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
    use crate::workspace::{AppIntegrations, Repository, WezTermIntegration, WorkspaceInfo};
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
                iterm2: None,
                wezterm: Some(WezTermIntegration {
                    enabled: true,
                    config_dir: temp_dir.path().to_path_buf(),
                    template_dir: temp_dir.path().join("templates").join("wezterm"),
                    default_template: "default".to_string(),
                }),
            },
            preferences: None,
        }
    }
}
