use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{PackageManager, PackageManagerType};
use crate::utils::platform::is_binary_available;

/// Cargo package manager implementation for Rust applications
pub struct CargoManager;

impl CargoManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PackageManager for CargoManager {
    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Cargo
    }

    async fn is_available(&self) -> bool {
        is_binary_available("cargo")
    }

    async fn check_installed(&self, package: &str) -> Result<bool> {
        // Check if the binary exists in PATH
        // Most cargo-installed binaries have the same name as the package
        Ok(is_binary_available(package))
    }

    async fn install(&self, package: &str, _args: &[String]) -> Result<()> {
        println!("📦 Installing {} via Cargo...", package);

        let output = tokio::process::Command::new("cargo")
            .args(["install", package])
            .output()
            .await
            .context("Failed to install package via Cargo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install {}: {}", package, stderr);
        }

        println!("✅ Successfully installed {}", package);
        Ok(())
    }

    async fn get_version(&self, package: &str) -> Result<Option<String>> {
        // Try to get version by running the binary with --version
        let output = tokio::process::Command::new(package)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                let version_output = String::from_utf8_lossy(&output.stdout);
                // Most tools output "name version" or "name x.y.z"
                let parts: Vec<&str> = version_output.split_whitespace().collect();
                if parts.len() >= 2 {
                    // Try to find the version number (usually starts with a digit)
                    for part in parts {
                        if part
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_numeric() || c == 'v')
                        {
                            return Ok(Some(part.trim_start_matches('v').to_string()));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
