use anyhow::{Context, Result};
use console::style;
use std::collections::HashMap;

use super::package_manager::{
    CargoManager, HomebrewManager, PackageManager, PackageManagerType, PathManager,
};
use super::registry::{get_app_registry, AppPackage};

/// Status of an app installation
#[derive(Debug, Clone)]
pub struct AppInstallStatus {
    pub app_name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub available_managers: Vec<PackageManagerType>,
    pub installed_path: Option<String>,
    pub installed_by_manager: Option<PackageManagerType>,
    pub is_managed: bool,
}

/// Manages application installations
pub struct AppManager {
    registry: Vec<AppPackage>,
    package_managers: HashMap<PackageManagerType, Box<dyn PackageManager>>,
    path_manager: PathManager,
}

impl AppManager {
    /// Create a new app manager
    pub async fn new() -> Result<Self> {
        let mut package_managers: HashMap<PackageManagerType, Box<dyn PackageManager>> =
            HashMap::new();

        // Initialize available package managers
        let homebrew = HomebrewManager::new();
        if homebrew.is_available().await {
            package_managers.insert(PackageManagerType::Homebrew, Box::new(homebrew));
        }

        let cargo = CargoManager::new();
        if cargo.is_available().await {
            package_managers.insert(PackageManagerType::Cargo, Box::new(cargo));
        }

        Ok(Self {
            registry: get_app_registry(),
            package_managers,
            path_manager: PathManager::new(),
        })
    }

    /// Get the list of available apps
    pub fn list_available(&self) -> &[AppPackage] {
        &self.registry
    }

    /// Check if an app is installed
    pub async fn check_installed(&self, app_name: &str) -> Result<AppInstallStatus> {
        let app = self
            .registry
            .iter()
            .find(|a| a.name == app_name)
            .with_context(|| format!("Unknown app: {app_name}"))?;

        let mut installed = false;
        let mut version = None;
        let mut available_managers = Vec::new();
        let mut installed_path = None;
        let mut installed_by_manager = None;
        let mut is_managed = false;

        // Check if app has a binary name (CLI tool) or is GUI-only
        if let Some(binary_name) = &app.binary_name {
            // For apps with CLI binaries, check PATH first
            if let Some(path) = self.path_manager.which(binary_name).await? {
                installed = true;
                installed_path = Some(path);

                // Try to get version using app-specific command or generic approach
                if let Some(version_cmd) = &app.version_command {
                    if let Some(output) = self
                        .path_manager
                        .get_version_output(binary_name, version_cmd)
                        .await?
                    {
                        version = self
                            .path_manager
                            .extract_version(&output, app.version_pattern.as_deref());
                    }
                } else {
                    // Use generic version detection
                    version = self.path_manager.get_version(binary_name).await?;
                }
            }
        }

        // Check each package manager
        for package_info in &app.packages {
            if let Some(manager) = self.package_managers.get(&package_info.manager) {
                available_managers.push(package_info.manager.clone());

                // Check if this manager claims to have installed it
                if manager.check_installed(&package_info.package_name).await? {
                    installed = true;
                    is_managed = true;
                    installed_by_manager = Some(package_info.manager.clone());

                    // Get version from package manager
                    if version.is_none() {
                        version = manager.get_version(&package_info.package_name).await?;
                    }
                    break;
                }
            }
        }

        Ok(AppInstallStatus {
            app_name: app_name.to_string(),
            installed,
            version,
            available_managers,
            installed_path,
            installed_by_manager,
            is_managed,
        })
    }

    /// Get installation status for all apps
    pub async fn get_all_status(&self) -> Result<Vec<AppInstallStatus>> {
        let mut statuses = Vec::new();

        for app in &self.registry {
            match self.check_installed(&app.name).await {
                Ok(status) => statuses.push(status),
                Err(e) => {
                    // Log error but continue
                    eprintln!("Warning: Failed to check {}: {}", app.name, e);
                }
            }
        }

        Ok(statuses)
    }

    /// Install an app
    pub async fn install(&self, app_name: &str) -> Result<()> {
        let app = self
            .registry
            .iter()
            .find(|a| a.name == app_name)
            .with_context(|| format!("Unknown app: {app_name}"))?;

        // Check if already installed
        let status = self.check_installed(app_name).await?;
        if status.installed {
            println!(
                "{} {} is already installed",
                style("✅").green(),
                style(&app.display_name).cyan()
            );
            return Ok(());
        }

        // Find the first available package manager for this app
        let package_info = app
            .packages
            .iter()
            .find(|p| self.package_managers.contains_key(&p.manager))
            .with_context(|| format!("No available package manager for {app_name}"))?;

        let manager = self
            .package_managers
            .get(&package_info.manager)
            .expect("Package manager should exist");

        // Install the package
        manager
            .install(&package_info.package_name, &package_info.install_args)
            .await?;

        Ok(())
    }

    /// Install multiple apps
    pub async fn install_multiple(&self, app_names: &[String]) -> Result<()> {
        let total = app_names.len();
        let mut installed = 0;
        let mut failed = 0;

        for (index, app_name) in app_names.iter().enumerate() {
            println!("\n[{}/{}] Installing {}...", index + 1, total, app_name);

            match self.install(app_name).await {
                Ok(_) => installed += 1,
                Err(e) => {
                    eprintln!(
                        "{} Failed to install {}: {}",
                        style("❌").red(),
                        style(app_name).red(),
                        e
                    );
                    failed += 1;
                }
            }
        }

        println!(
            "\n{} Installation complete: {} succeeded, {} failed",
            style("🎯").blue(),
            style(installed).green(),
            style(failed).red()
        );

        if failed > 0 {
            anyhow::bail!("{} apps failed to install", failed);
        }

        Ok(())
    }

    /// Get available package managers
    pub fn get_available_managers(&self) -> Vec<PackageManagerType> {
        self.package_managers.keys().cloned().collect()
    }
}
