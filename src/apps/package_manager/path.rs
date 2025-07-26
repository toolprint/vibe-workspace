use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::{PackageManager, PackageManagerType};

/// Path-based package detection (checks if binary exists on PATH)
pub struct PathManager;

impl PathManager {
    pub fn new() -> Self {
        Self
    }

    /// Check if a binary exists on PATH and return its location
    pub async fn which(&self, binary: &str) -> Result<Option<String>> {
        let output = Command::new("which")
            .arg(binary)
            .output()
            .await
            .context("Failed to run which command")?;

        if output.status.success() && !output.stdout.is_empty() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    /// Get version by running the binary with version command
    pub async fn get_version_output(
        &self,
        binary: &str,
        args: &[String],
    ) -> Result<Option<String>> {
        // First check if binary exists
        if self.which(binary).await?.is_none() {
            return Ok(None);
        }

        let mut cmd = Command::new(binary);
        for arg in args {
            cmd.arg(arg);
        }

        // Capture both stdout and stderr as some apps output version to stderr
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Try stdout first, then stderr
                let version_output = if !stdout.trim().is_empty() {
                    stdout.to_string()
                } else {
                    stderr.to_string()
                };

                Ok(Some(version_output.trim().to_string()))
            }
            _ => Ok(None),
        }
    }

    /// Extract version from output using a pattern or heuristics
    pub fn extract_version(&self, output: &str, pattern: Option<&str>) -> Option<String> {
        if let Some(_pattern) = pattern {
            // TODO: Implement regex pattern matching if needed
            // For now, use simple heuristics
        }

        // Simple heuristic: look for version patterns like x.y.z
        let lines = output.lines();
        for line in lines {
            // Split by whitespace and look for version-like patterns
            for word in line.split_whitespace() {
                // Remove common prefixes
                let cleaned = word
                    .trim_start_matches('v')
                    .trim_start_matches("version")
                    .trim_start_matches("Version");

                // Check if it looks like a version (starts with digit and contains dots)
                if cleaned.chars().next().is_some_and(|c| c.is_numeric()) && cleaned.contains('.') {
                    return Some(cleaned.to_string());
                }
            }
        }

        None
    }
}

#[async_trait]
impl PackageManager for PathManager {
    fn manager_type(&self) -> PackageManagerType {
        // PathManager doesn't have a type - it's just for detection
        unreachable!("PathManager should not be used as a regular PackageManager")
    }

    async fn is_available(&self) -> bool {
        // which command should always be available on Unix systems
        true
    }

    async fn check_installed(&self, package: &str) -> Result<bool> {
        Ok(self.which(package).await?.is_some())
    }

    async fn install(&self, _package: &str, _args: &[String]) -> Result<()> {
        anyhow::bail!("PathManager cannot install packages")
    }

    async fn get_version(&self, package: &str) -> Result<Option<String>> {
        // Try common version flags
        let version_flags = vec![
            vec!["--version"],
            vec!["-version"],
            vec!["-v"],
            vec!["version"],
        ];

        for flags in version_flags {
            if let Some(output) = self
                .get_version_output(
                    package,
                    &flags.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                )
                .await?
            {
                if let Some(version) = self.extract_version(&output, None) {
                    return Ok(Some(version));
                }
            }
        }

        Ok(None)
    }
}
