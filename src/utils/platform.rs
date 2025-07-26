use std::path::PathBuf;

/// Supported operating system platforms
#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
    Unknown,
}

/// Platform-specific information for app integrations
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub warp_config_dir: Option<PathBuf>,
    pub supports_uri_scheme: bool,
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Unknown
        }
    }

    /// Get the display name for the platform
    pub fn display_name(&self) -> &'static str {
        match self {
            Platform::MacOS => "macOS",
            Platform::Linux => "Linux",
            Platform::Windows => "Windows",
            Platform::Unknown => "Unknown",
        }
    }

    /// Check if Warp is likely to be available on this platform
    pub fn supports_warp(&self) -> bool {
        match self {
            Platform::MacOS => true,
            Platform::Linux => true,
            Platform::Windows => true, // Warp supports Windows
            Platform::Unknown => false,
        }
    }
}

impl PlatformInfo {
    /// Get platform information for the current system
    pub fn current() -> Self {
        let platform = Platform::current();
        let warp_config_dir = get_warp_config_dir(&platform);
        let supports_uri_scheme = platform.supports_warp();

        PlatformInfo {
            platform,
            warp_config_dir,
            supports_uri_scheme,
        }
    }

    /// Get platform-specific keyboard shortcuts for Warp
    pub fn get_warp_shortcuts(&self) -> WarpShortcuts {
        match self.platform {
            Platform::MacOS => WarpShortcuts {
                command_palette: "CMD-P",
                launch_config_palette: "CMD-L",
                active_window_modifier: "CMD-ENTER",
                ctrl_modifier: "CMD",
            },
            Platform::Linux | Platform::Windows => WarpShortcuts {
                command_palette: "CTRL-SHIFT-P",
                launch_config_palette: "CTRL-L",
                active_window_modifier: "CTRL-ENTER",
                ctrl_modifier: "CTRL",
            },
            Platform::Unknown => WarpShortcuts {
                command_palette: "CMD-P / CTRL-SHIFT-P",
                launch_config_palette: "CMD-L / CTRL-L",
                active_window_modifier: "CMD-ENTER / CTRL-ENTER",
                ctrl_modifier: "CMD / CTRL",
            },
        }
    }
}

/// Platform-specific keyboard shortcuts for Warp
#[derive(Debug, Clone)]
pub struct WarpShortcuts {
    pub command_palette: &'static str,
    pub launch_config_palette: &'static str,
    pub active_window_modifier: &'static str,
    pub ctrl_modifier: &'static str,
}

/// Get the default Warp launch configurations directory for the platform
fn get_warp_config_dir(platform: &Platform) -> Option<PathBuf> {
    match platform {
        Platform::MacOS => {
            // macOS: ~/.warp/launch_configurations
            dirs::home_dir().map(|home| home.join(".warp").join("launch_configurations"))
        }
        Platform::Linux => {
            // Linux: ~/.config/warp-terminal/launch_configurations or ~/.warp/launch_configurations
            dirs::config_dir()
                .map(|config| config.join("warp-terminal").join("launch_configurations"))
                .or_else(|| {
                    dirs::home_dir().map(|home| home.join(".warp").join("launch_configurations"))
                })
        }
        Platform::Windows => {
            // Windows: %APPDATA%\warp-terminal\launch_configurations
            dirs::config_dir()
                .map(|config| config.join("warp-terminal").join("launch_configurations"))
        }
        Platform::Unknown => None,
    }
}

/// Check if a binary is available in PATH
pub fn is_binary_available(binary_name: &str) -> bool {
    std::process::Command::new(binary_name)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if platform automation tools are available
pub fn check_automation_availability() -> AutomationAvailability {
    let platform = Platform::current();

    match platform {
        Platform::MacOS => {
            let osascript = std::process::Command::new("which")
                .arg("osascript")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            AutomationAvailability {
                platform,
                tool_name: "osascript",
                is_available: osascript,
                install_hint: if !osascript {
                    Some("osascript should be available by default on macOS")
                } else {
                    None
                },
            }
        }
        Platform::Linux => {
            let xdotool = std::process::Command::new("which")
                .arg("xdotool")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            AutomationAvailability {
                platform,
                tool_name: "xdotool",
                is_available: xdotool,
                install_hint: if !xdotool {
                    Some("Install xdotool: sudo apt-get install xdotool (Debian/Ubuntu) or equivalent")
                } else {
                    None
                },
            }
        }
        Platform::Windows => {
            let powershell = std::process::Command::new("powershell")
                .args(["-Command", "echo test"])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            AutomationAvailability {
                platform,
                tool_name: "PowerShell",
                is_available: powershell,
                install_hint: if !powershell {
                    Some("PowerShell should be available by default on Windows")
                } else {
                    None
                },
            }
        }
        Platform::Unknown => AutomationAvailability {
            platform,
            tool_name: "none",
            is_available: false,
            install_hint: Some("Platform not supported for automation"),
        },
    }
}

/// Information about automation tool availability
#[derive(Debug, Clone)]
pub struct AutomationAvailability {
    pub platform: Platform,
    pub tool_name: &'static str,
    pub is_available: bool,
    pub install_hint: Option<&'static str>,
}

/// Open a URI using the system's default handler
pub fn open_uri(uri: &str) -> Result<(), std::io::Error> {
    match Platform::current() {
        Platform::MacOS => {
            std::process::Command::new("open").arg(uri).spawn()?;
        }
        Platform::Linux => {
            std::process::Command::new("xdg-open").arg(uri).spawn()?;
        }
        Platform::Windows => {
            std::process::Command::new("cmd")
                .args(["/c", "start", uri])
                .spawn()?;
        }
        Platform::Unknown => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Unsupported platform for URI opening",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        assert_ne!(platform, Platform::Unknown);

        let info = PlatformInfo::current();
        assert_eq!(info.platform, platform);
    }

    #[test]
    fn test_platform_display_names() {
        assert_eq!(Platform::MacOS.display_name(), "macOS");
        assert_eq!(Platform::Linux.display_name(), "Linux");
        assert_eq!(Platform::Windows.display_name(), "Windows");
        assert_eq!(Platform::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn test_warp_support() {
        assert!(Platform::MacOS.supports_warp());
        assert!(Platform::Linux.supports_warp());
        assert!(Platform::Windows.supports_warp());
        assert!(!Platform::Unknown.supports_warp());
    }

    #[test]
    fn test_warp_shortcuts() {
        let macos_info = PlatformInfo {
            platform: Platform::MacOS,
            warp_config_dir: None,
            supports_uri_scheme: true,
        };
        let shortcuts = macos_info.get_warp_shortcuts();
        assert_eq!(shortcuts.command_palette, "CMD-P");
        assert_eq!(shortcuts.ctrl_modifier, "CMD");

        let linux_info = PlatformInfo {
            platform: Platform::Linux,
            warp_config_dir: None,
            supports_uri_scheme: true,
        };
        let shortcuts = linux_info.get_warp_shortcuts();
        assert_eq!(shortcuts.command_palette, "CTRL-SHIFT-P");
        assert_eq!(shortcuts.ctrl_modifier, "CTRL");
    }
}
