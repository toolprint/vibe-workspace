use anyhow::{Context, Result};
use console::style;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::automation::get_platform_automation;
use crate::utils::platform::{open_uri, PlatformInfo};
use crate::workspace::{Repository, TemplateManager, WorkspaceConfig};

/// Warp launch configuration launcher with cross-platform support
pub struct WarpLauncher {
    platform_info: PlatformInfo,
    config_dir: PathBuf,
}

/// Launch method attempted and its result
#[derive(Debug)]
pub enum LaunchMethod {
    UriScheme,
    PlatformAutomation,
    ManualInstructions,
}

/// Result of a launch attempt
#[derive(Debug)]
pub struct LaunchResult {
    pub method: LaunchMethod,
    pub success: bool,
    pub message: String,
}

impl WarpLauncher {
    /// Create a new Warp launcher
    pub fn new(config_dir: Option<PathBuf>) -> Result<Self> {
        let platform_info = PlatformInfo::current();

        // Use provided config dir or platform default
        let config_dir = config_dir
            .or_else(|| platform_info.warp_config_dir.clone())
            .context("Could not determine Warp configuration directory")?;

        Ok(WarpLauncher {
            platform_info,
            config_dir,
        })
    }

    /// Launch Warp with the specified repository configuration
    pub async fn launch_with_repo(
        &self,
        config: &WorkspaceConfig,
        repo: &Repository,
        template_manager: &TemplateManager,
    ) -> Result<LaunchResult> {
        // Create the launch configuration file
        let config_path = self
            .create_launch_config(config, repo, template_manager)
            .await?;

        // Attempt to launch using various methods
        self.launch_with_config_file(&config_path).await
    }

    /// Create a launch configuration file for the repository
    async fn create_launch_config(
        &self,
        config: &WorkspaceConfig,
        repo: &Repository,
        template_manager: &TemplateManager,
    ) -> Result<PathBuf> {
        let warp_integration = config
            .apps
            .warp
            .as_ref()
            .context("Warp integration is not configured")?;

        if !warp_integration.enabled {
            anyhow::bail!("Warp integration is disabled in configuration");
        }

        // Get the template to use
        let template_name = repo
            .get_app_template("warp")
            .unwrap_or(&warp_integration.default_template);

        // Load the template
        let template_content = template_manager
            .load_template("warp", template_name)
            .await
            .with_context(|| format!("Failed to load template '{template_name}'"))?;

        // Create variables for substitution
        let variables = TemplateManager::create_variables(config, repo);

        // Apply variable substitution
        let config_content = template_manager.substitute_variables(&template_content, &variables);

        // Generate a unique config file name
        let config_name = format!("vibe-{}-{}.yaml", config.workspace.name, repo.name);
        let config_path = self.config_dir.join(&config_name);

        // Create config directory if it doesn't exist
        fs::create_dir_all(&self.config_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create Warp config directory: {}",
                    self.config_dir.display()
                )
            })?;

        // Write the configuration
        fs::write(&config_path, config_content)
            .await
            .with_context(|| format!("Failed to write Warp config: {}", config_path.display()))?;

        println!(
            "{} Created Warp launch configuration: {}",
            style("✅").green(),
            style(config_path.display()).cyan()
        );

        Ok(config_path)
    }

    /// Launch Warp using the specified configuration file
    async fn launch_with_config_file(&self, config_path: &Path) -> Result<LaunchResult> {
        // Method 1: Try URI scheme launch first (better cross-platform support)
        if self.platform_info.supports_uri_scheme {
            match self.launch_via_uri_scheme(config_path).await {
                Ok(_) => {
                    return Ok(LaunchResult {
                        method: LaunchMethod::UriScheme,
                        success: true,
                        message: "Launched Warp via URI scheme".to_string(),
                    });
                }
                Err(e) => {
                    println!("{} URI scheme launch failed: {}", style("⚠️").yellow(), e);
                }
            }
        }

        // Method 2: Try platform-specific automation as fallback
        match self.try_platform_automation(config_path).await {
            Ok(_) => {
                return Ok(LaunchResult {
                    method: LaunchMethod::PlatformAutomation,
                    success: true,
                    message: "Launched Warp via platform automation".to_string(),
                });
            }
            Err(e) => {
                println!("{} Platform automation failed: {}", style("⚠️").yellow(), e);

                // Check and display automation availability info
                let availability = crate::utils::platform::check_automation_availability();
                if !availability.is_available {
                    println!(
                        "{} {} is not available on your system",
                        style("ℹ️").blue(),
                        style(availability.tool_name).cyan()
                    );
                    if let Some(hint) = availability.install_hint {
                        println!("   {}", style(hint).dim());
                    }
                }
            }
        }

        // Method 3: Provide manual instructions (always works)
        self.provide_manual_instructions(config_path);

        Ok(LaunchResult {
            method: LaunchMethod::ManualInstructions,
            success: true,
            message: "Provided manual launch instructions".to_string(),
        })
    }

    /// Launch Warp using URI scheme
    async fn launch_via_uri_scheme(&self, config_path: &Path) -> Result<()> {
        // The Raycast extension shows that Warp expects URL-encoded absolute paths
        // Format: warp://launch/<encoded-absolute-path>

        // Get the absolute path to the launch configuration
        let absolute_path = config_path.canonicalize().with_context(|| {
            format!("Failed to get absolute path for {}", config_path.display())
        })?;

        let path_str = absolute_path
            .to_str()
            .context("Path contains invalid UTF-8")?;

        // URL encode the absolute path
        let encoded_path = urlencoding::encode(path_str);
        let uri = format!("warp://launch/{encoded_path}");

        println!(
            "{} Attempting to launch Warp via URI scheme",
            style("🚀").blue()
        );
        println!(
            "   {} {}",
            style("Configuration:").dim(),
            style(config_path.display()).cyan()
        );
        println!(
            "   {} {}",
            style("Absolute path:").dim(),
            style(path_str).cyan()
        );
        println!(
            "   {} {}",
            style("Computed URI:").dim(),
            style(&uri).yellow()
        );

        open_uri(&uri).context("Failed to open Warp URI")?;

        // Give the system a moment to process the URI
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        Ok(())
    }

    /// Try platform-specific automation methods
    async fn try_platform_automation(&self, config_path: &Path) -> Result<()> {
        // Extract configuration name from file path
        let config_name = config_path
            .file_stem()
            .and_then(|name| name.to_str())
            .context("Failed to extract configuration name from path")?;

        // Get platform-specific automation
        let automation = get_platform_automation()
            .context("No platform automation available for this system")?;

        println!(
            "{} Using {} to launch configuration...",
            style("🤖").blue(),
            style(automation.description()).cyan()
        );

        // Attempt to launch using platform automation
        automation
            .launch_warp_config(config_name)
            .await
            .with_context(|| {
                format!(
                    "Failed to launch configuration '{}' using {}",
                    config_name,
                    automation.description()
                )
            })?;

        Ok(())
    }

    /// Provide comprehensive manual instructions for launching
    fn provide_manual_instructions(&self, config_path: &Path) {
        let shortcuts = self.platform_info.get_warp_shortcuts();
        let config_name = config_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("launch configuration");

        println!();
        println!(
            "{} Manual Launch Instructions for {}",
            style("📋").blue().bold(),
            style(self.platform_info.platform.display_name()).cyan()
        );
        println!("{}", style("─".repeat(50)).dim());

        println!();
        println!("{} Method 1: Command Palette", style("1️⃣").blue());
        println!("   • Open Warp");
        println!(
            "   • Press {} to open Command Palette",
            style(shortcuts.command_palette).yellow().bold()
        );
        println!("   • Type \"Launch Configuration\"");
        println!("   • Select: {}", style(config_name).cyan());

        println!();
        println!(
            "{} Method 2: Launch Configuration Palette",
            style("2️⃣").blue()
        );
        println!("   • Open Warp");
        println!(
            "   • Press {} to open Launch Configuration palette",
            style(shortcuts.launch_config_palette).yellow().bold()
        );
        println!("   • Select: {}", style(config_name).cyan());
        println!(
            "   • Press {} to open in active window",
            style(shortcuts.active_window_modifier).yellow()
        );

        if matches!(
            self.platform_info.platform,
            crate::utils::platform::Platform::MacOS
        ) {
            println!();
            println!("{} Method 3: Menu (macOS)", style("3️⃣").blue());
            println!("   • Open Warp");
            println!(
                "   • Navigate to {} → {}",
                style("File").yellow(),
                style("Launch Configurations").yellow()
            );
            println!("   • Select: {}", style(config_name).cyan());

            println!();
            println!("{} Method 4: New Tab Menu (macOS)", style("4️⃣").blue());
            println!("   • Open Warp");
            println!(
                "   • Right-click the {} button",
                style("\"New Tab +\"").yellow()
            );
            println!("   • Select: {}", style(config_name).cyan());
        }

        println!();
        println!("{} Configuration file location:", style("📍").blue());
        println!("   {}", style(config_path.display()).dim());

        println!();
    }

    /// Get the configuration directory being used
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get platform information
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.platform_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_launcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let launcher = WarpLauncher::new(Some(config_dir.clone())).unwrap();
        assert_eq!(launcher.config_dir(), config_dir);
    }

    #[test]
    fn test_platform_detection() {
        // Test that we can detect platform info
        let platform_info = PlatformInfo::current();
        assert!(
            platform_info.platform.supports_warp()
                || matches!(
                    platform_info.platform,
                    crate::utils::platform::Platform::Unknown
                )
        );
    }
}
