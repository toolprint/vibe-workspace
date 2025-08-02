use super::package_manager::PackageManagerType;

/// Information about a package in a specific package manager
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub manager: PackageManagerType,
    pub package_name: String,
    pub install_args: Vec<String>,
    #[allow(dead_code)]
    pub tap: Option<String>,
}

/// An application that can be installed
#[derive(Debug, Clone)]
pub struct AppPackage {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub packages: Vec<PackageInfo>,
    pub binary_name: Option<String>,
    pub version_command: Option<Vec<String>>,
    pub version_pattern: Option<String>,
}

impl AppPackage {
    /// Create a new app package
    pub fn new(name: &str, display_name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: description.to_string(),
            packages: Vec::new(),
            binary_name: None,
            version_command: None,
            version_pattern: None,
        }
    }

    /// Add a Homebrew cask package
    pub fn with_brew_cask(mut self, package_name: &str) -> Self {
        self.packages.push(PackageInfo {
            manager: PackageManagerType::Homebrew,
            package_name: package_name.to_string(),
            install_args: vec!["--cask".to_string()],
            tap: None,
        });
        self
    }

    /// Add a Homebrew formula package
    pub fn with_brew_formula(mut self, package_name: &str) -> Self {
        self.packages.push(PackageInfo {
            manager: PackageManagerType::Homebrew,
            package_name: package_name.to_string(),
            install_args: vec![],
            tap: None,
        });
        self
    }

    /// Add a Homebrew formula with custom tap
    pub fn with_brew_tap(mut self, tap: &str, package_name: &str) -> Self {
        self.packages.push(PackageInfo {
            manager: PackageManagerType::Homebrew,
            package_name: format!("{tap}/{package_name}"),
            install_args: vec![],
            tap: Some(tap.to_string()),
        });
        self
    }

    /// Add a Cargo package
    #[allow(dead_code)]
    pub fn with_cargo(mut self, package_name: &str) -> Self {
        self.packages.push(PackageInfo {
            manager: PackageManagerType::Cargo,
            package_name: package_name.to_string(),
            install_args: vec![],
            tap: None,
        });
        self
    }

    /// Set the binary name for PATH checking
    pub fn with_binary_name(mut self, binary_name: &str) -> Self {
        self.binary_name = Some(binary_name.to_string());
        self
    }

    /// Set the version command
    pub fn with_version_command(mut self, command: Vec<&str>) -> Self {
        self.version_command = Some(command.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set the version pattern for parsing
    #[allow(dead_code)]
    pub fn with_version_pattern(mut self, pattern: &str) -> Self {
        self.version_pattern = Some(pattern.to_string());
        self
    }
}

/// Get the built-in app registry
pub fn get_app_registry() -> Vec<AppPackage> {
    vec![
        // Terminal emulators
        AppPackage::new("iterm2", "iTerm2", "A powerful terminal emulator for macOS")
            .with_brew_cask("iterm2"),
        AppPackage::new(
            "warp",
            "Warp",
            "The terminal reimagined with AI and modern features",
        )
        .with_brew_cask("warp"),
        AppPackage::new(
            "wezterm",
            "WezTerm",
            "A GPU-accelerated cross-platform terminal emulator",
        )
        .with_binary_name("wezterm")
        .with_version_command(vec!["--version"])
        .with_brew_cask("wezterm"),
        // Code editors
        AppPackage::new(
            "vscode",
            "Visual Studio Code",
            "Free source-code editor by Microsoft",
        )
        .with_binary_name("code")
        .with_version_command(vec!["--version"])
        .with_brew_cask("visual-studio-code"),
        AppPackage::new("cursor", "Cursor", "The AI-first code editor")
            .with_binary_name("cursor")
            .with_version_command(vec!["--version"])
            .with_brew_cask("cursor"),
        AppPackage::new(
            "windsurf",
            "Windsurf",
            "Agentic IDE powered by AI Flow paradigm",
        )
        .with_binary_name("windsurf")
        .with_version_command(vec!["--version"])
        .with_brew_cask("windsurf"),
        // CLI tools
        AppPackage::new("gh", "GitHub CLI", "GitHub's official command line tool")
            .with_binary_name("gh")
            .with_version_command(vec!["--version"])
            .with_brew_formula("gh"),
        AppPackage::new("gitui", "GitUI", "Blazing fast terminal-ui for git")
            .with_binary_name("gitui")
            .with_version_command(vec!["--version"])
            .with_brew_formula("gitui"),
        AppPackage::new("just", "Just", "A command runner and task automation tool")
            .with_binary_name("just")
            .with_version_command(vec!["--version"])
            .with_brew_formula("just"),
        AppPackage::new(
            "claude-squad",
            "Claude Squad",
            "A tool for managing Claude conversations",
        )
        .with_binary_name("claude-squad")
        .with_version_command(vec!["--version"])
        .with_brew_formula("claude-squad"),
        // Container tools
        AppPackage::new(
            "container-use",
            "Container Use",
            "Tool for launching agent sandboxes",
        )
        .with_binary_name("container-use")
        .with_version_command(vec!["--version"])
        .with_brew_cask("container-use"),
        // Terminal session managers
        AppPackage::new("weztermocil", "Weztermocil", "WezTerm session manager")
            .with_binary_name("weztermocil")
            .with_version_command(vec!["--version"])
            .with_brew_tap("alexcaza/weztermocil", "weztermocil"),
        AppPackage::new("itermocil", "iTermocil", "iTerm2 session manager")
            .with_binary_name("itermocil")
            .with_version_command(vec!["--version"])
            .with_brew_tap("TomAnthony/brews", "itermocil"),
    ]
}
