use anyhow::{Context, Result};
use async_trait::async_trait;

use super::{PackageManager, PackageManagerType};
use crate::utils::platform::{is_binary_available, Platform};

/// Homebrew package manager implementation
pub struct HomebrewManager;

impl HomebrewManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse tap from package name (e.g., "alexcaza/weztermocil" -> ("alexcaza/weztermocil", "weztermocil"))
    fn parse_tap_and_package<'a>(&self, package: &'a str) -> (Option<&'a str>, &'a str) {
        if package.contains('/') {
            let parts: Vec<&str> = package.split('/').collect();
            if parts.len() == 3 {
                // Format: owner/repo/package
                let tap = format!("{}/{}", parts[0], parts[1]);
                (Some(package.split_at(tap.len()).0), parts[2])
            } else if parts.len() == 2 {
                // Format: owner/package (assume tap name is owner/homebrew-package)
                (None, package)
            } else {
                (None, package)
            }
        } else {
            (None, package)
        }
    }

    /// Check if a tap is already tapped
    async fn is_tap_installed(&self, tap: &str) -> Result<bool> {
        let output = tokio::process::Command::new("brew")
            .args(["tap"])
            .output()
            .await
            .context("Failed to list Homebrew taps")?;

        let taps = String::from_utf8_lossy(&output.stdout);
        Ok(taps.lines().any(|line| line.trim() == tap))
    }

    /// Install a tap
    async fn install_tap(&self, tap: &str) -> Result<()> {
        println!("📥 Adding Homebrew tap: {tap}");

        let output = tokio::process::Command::new("brew")
            .args(["tap", tap])
            .output()
            .await
            .context("Failed to add Homebrew tap")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to add tap {}: {}", tap, stderr);
        }

        Ok(())
    }
}

#[async_trait]
impl PackageManager for HomebrewManager {
    fn manager_type(&self) -> PackageManagerType {
        PackageManagerType::Homebrew
    }

    async fn is_available(&self) -> bool {
        // Homebrew is only available on macOS and Linux
        match Platform::current() {
            Platform::MacOS | Platform::Linux => is_binary_available("brew"),
            _ => false,
        }
    }

    async fn check_installed(&self, package: &str) -> Result<bool> {
        let (_tap, package_name) = self.parse_tap_and_package(package);

        // First check as a formula
        let formula_output = tokio::process::Command::new("brew")
            .args(["list", "--formula", package_name])
            .output()
            .await
            .context("Failed to check Homebrew formula")?;

        if formula_output.status.success() {
            return Ok(true);
        }

        // Then check as a cask
        let cask_output = tokio::process::Command::new("brew")
            .args(["list", "--cask", package_name])
            .output()
            .await
            .context("Failed to check Homebrew cask")?;

        Ok(cask_output.status.success())
    }

    async fn install(&self, package: &str, args: &[String]) -> Result<()> {
        let (tap, package_name) = self.parse_tap_and_package(package);

        // Install tap if needed
        if let Some(tap_name) = tap {
            if !self.is_tap_installed(tap_name).await? {
                self.install_tap(tap_name).await?;
            }
        }

        // Build install command
        let mut cmd_args = vec!["install"];

        // Add any additional arguments (like --cask)
        for arg in args {
            cmd_args.push(arg);
        }

        // Add the full package name (including tap if present)
        cmd_args.push(package);

        println!("📦 Installing {} via Homebrew...", package_name);

        // Run the install command
        let output = tokio::process::Command::new("brew")
            .args(&cmd_args)
            .output()
            .await
            .context("Failed to install package via Homebrew")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install {}: {}", package, stderr);
        }

        println!("✅ Successfully installed {}", package_name);
        Ok(())
    }

    async fn get_version(&self, package: &str) -> Result<Option<String>> {
        let (_tap, package_name) = self.parse_tap_and_package(package);

        // Try formula first
        let formula_output = tokio::process::Command::new("brew")
            .args(["list", "--versions", "--formula", package_name])
            .output()
            .await
            .context("Failed to get formula version")?;

        if formula_output.status.success() && !formula_output.stdout.is_empty() {
            let version_line = String::from_utf8_lossy(&formula_output.stdout);
            // Format is "package version1 version2...", we want the last (latest) version
            let parts: Vec<&str> = version_line.split_whitespace().collect();
            if parts.len() > 1 {
                return Ok(Some(parts.last().unwrap().to_string()));
            }
        }

        // Try cask
        let cask_output = tokio::process::Command::new("brew")
            .args(["list", "--versions", "--cask", package_name])
            .output()
            .await
            .context("Failed to get cask version")?;

        if cask_output.status.success() && !cask_output.stdout.is_empty() {
            let version_line = String::from_utf8_lossy(&cask_output.stdout);
            // Format is "package version", we want the version
            let parts: Vec<&str> = version_line.split_whitespace().collect();
            if parts.len() > 1 {
                return Ok(Some(parts.last().unwrap().to_string()));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tap_and_package() {
        let manager = HomebrewManager::new();

        // Test simple package
        let (tap, pkg) = manager.parse_tap_and_package("git");
        assert_eq!(tap, None);
        assert_eq!(pkg, "git");

        // Test package with full tap
        let (tap, pkg) = manager.parse_tap_and_package("alexcaza/weztermocil/weztermocil");
        assert_eq!(tap, Some("alexcaza/weztermocil"));
        assert_eq!(pkg, "weztermocil");

        // Test package with partial tap
        let (tap, pkg) = manager.parse_tap_and_package("homebrew/cask");
        assert_eq!(tap, None);
        assert_eq!(pkg, "homebrew/cask");
    }
}
