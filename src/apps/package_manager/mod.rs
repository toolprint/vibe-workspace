use anyhow::Result;
use async_trait::async_trait;

pub mod cargo;
pub mod homebrew;
pub mod path;

pub use cargo::CargoManager;
pub use homebrew::HomebrewManager;
pub use path::PathManager;

/// Type of package manager
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageManagerType {
    Homebrew,
    Cargo,
}

/// Installation status for a package
#[derive(Debug, Clone)]
pub struct InstallStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub manager: PackageManagerType,
}

/// Trait for package managers
#[async_trait]
pub trait PackageManager: Send + Sync {
    /// Get the type of this package manager
    fn manager_type(&self) -> PackageManagerType;

    /// Check if the package manager is available on the system
    async fn is_available(&self) -> bool;

    /// Check if a package is installed
    async fn check_installed(&self, package: &str) -> Result<bool>;

    /// Install a package with optional arguments
    async fn install(&self, package: &str, args: &[String]) -> Result<()>;

    /// Get the version of an installed package
    async fn get_version(&self, package: &str) -> Result<Option<String>>;
}
