use anyhow::{Context, Result};
use std::process::Command;

/// Platform-specific automation for launching Warp configurations
pub enum PlatformAutomation {
    MacOS(MacOSAutomation),
    Windows(WindowsAutomation),
    Linux(LinuxAutomation),
}

impl PlatformAutomation {
    /// Launch a Warp configuration by name using platform-specific automation
    pub async fn launch_warp_config(&self, config_name: &str) -> Result<()> {
        match self {
            PlatformAutomation::MacOS(auto) => auto.launch_warp_config(config_name).await,
            PlatformAutomation::Windows(auto) => auto.launch_warp_config(config_name).await,
            PlatformAutomation::Linux(auto) => auto.launch_warp_config(config_name).await,
        }
    }

    /// Get a description of this automation method
    pub fn description(&self) -> &'static str {
        match self {
            PlatformAutomation::MacOS(auto) => auto.description(),
            PlatformAutomation::Windows(auto) => auto.description(),
            PlatformAutomation::Linux(auto) => auto.description(),
        }
    }
}

/// macOS automation using AppleScript
pub struct MacOSAutomation;

impl MacOSAutomation {
    async fn launch_warp_config(&self, config_name: &str) -> Result<()> {
        // AppleScript to automate Warp launch
        // Based on the Raycast extension approach
        let apple_script = format!(
            r#"
            tell application "System Events"
                -- Activate Warp
                tell application "Warp" to activate
                delay 0.5
                
                -- Open launch configuration palette (Cmd+Ctrl+L)
                keystroke "l" using {{command down, control down}}
                delay 0.3
                
                -- Type the configuration name
                keystroke "{config_name}"
                delay 0.2
                
                -- Press Enter to launch
                key code 36
            end tell
            "#
        );

        // Execute the AppleScript
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&apple_script)
            .output()
            .context("Failed to execute AppleScript")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("AppleScript execution failed: {}", error);
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        // Check if we're on macOS and osascript is available
        cfg!(target_os = "macos")
            && Command::new("which")
                .arg("osascript")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn description(&self) -> &'static str {
        "macOS AppleScript automation"
    }
}

/// Windows automation using PowerShell
pub struct WindowsAutomation;

impl WindowsAutomation {
    async fn launch_warp_config(&self, config_name: &str) -> Result<()> {
        // PowerShell script to automate Warp launch on Windows
        let powershell_script = format!(
            r#"
            # Activate Warp window
            Add-Type @"
            using System;
            using System.Runtime.InteropServices;
            public class Win32 {{
                [DllImport("user32.dll")]
                public static extern bool SetForegroundWindow(IntPtr hWnd);
                [DllImport("user32.dll")]
                public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
            }}
"@
            
            $warp = Get-Process -Name "Warp" -ErrorAction SilentlyContinue
            if ($warp) {{
                [Win32]::ShowWindow($warp.MainWindowHandle, 9) # SW_RESTORE
                [Win32]::SetForegroundWindow($warp.MainWindowHandle)
                Start-Sleep -Milliseconds 500
                
                # Send Ctrl+L to open launch configuration
                Add-Type -AssemblyName System.Windows.Forms
                [System.Windows.Forms.SendKeys]::SendWait("^l")
                Start-Sleep -Milliseconds 300
                
                # Type configuration name
                [System.Windows.Forms.SendKeys]::SendWait("{config_name}")
                Start-Sleep -Milliseconds 200
                
                # Press Enter
                [System.Windows.Forms.SendKeys]::SendWait("{{ENTER}}")
            }} else {{
                Write-Error "Warp is not running"
                exit 1
            }}
            "#
        );

        // Execute the PowerShell script
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", &powershell_script])
            .output()
            .context("Failed to execute PowerShell script")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("PowerShell execution failed: {}", error);
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        // Check if we're on Windows and PowerShell is available
        cfg!(target_os = "windows")
            && Command::new("powershell")
                .args(["-Command", "echo test"])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn description(&self) -> &'static str {
        "Windows PowerShell automation"
    }
}

/// Linux automation using xdotool
pub struct LinuxAutomation;

impl LinuxAutomation {
    async fn launch_warp_config(&self, config_name: &str) -> Result<()> {
        // First, check if Warp window exists and activate it
        let search_output = Command::new("xdotool")
            .args(["search", "--name", "Warp"])
            .output()
            .context("Failed to search for Warp window")?;

        if !search_output.status.success() || search_output.stdout.is_empty() {
            anyhow::bail!("Warp window not found. Please ensure Warp is running.");
        }

        // Get the window ID (first line of output)
        let window_id_string = String::from_utf8_lossy(&search_output.stdout);
        let window_id = window_id_string
            .lines()
            .next()
            .context("Failed to parse window ID")?
            .trim();

        // Activate the Warp window
        Command::new("xdotool")
            .args(["windowactivate", window_id])
            .output()
            .context("Failed to activate Warp window")?;

        // Small delay to ensure window is active
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Send Ctrl+L to open launch configuration palette
        Command::new("xdotool")
            .args(["key", "ctrl+l"])
            .output()
            .context("Failed to send Ctrl+L")?;

        // Small delay for palette to open
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Type the configuration name
        Command::new("xdotool")
            .args(["type", config_name])
            .output()
            .context("Failed to type configuration name")?;

        // Small delay before pressing Enter
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Press Enter to launch
        Command::new("xdotool")
            .args(["key", "Return"])
            .output()
            .context("Failed to send Enter key")?;

        Ok(())
    }

    fn is_available(&self) -> bool {
        // Check if we're on Linux and xdotool is available
        cfg!(target_os = "linux")
            && Command::new("which")
                .arg("xdotool")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn description(&self) -> &'static str {
        "Linux xdotool automation"
    }
}

/// Get the appropriate automation implementation for the current platform
pub fn get_platform_automation() -> Option<PlatformAutomation> {
    if cfg!(target_os = "macos") {
        let automation = MacOSAutomation;
        if automation.is_available() {
            return Some(PlatformAutomation::MacOS(automation));
        }
    } else if cfg!(target_os = "windows") {
        let automation = WindowsAutomation;
        if automation.is_available() {
            return Some(PlatformAutomation::Windows(automation));
        }
    } else if cfg!(target_os = "linux") {
        let automation = LinuxAutomation;
        if automation.is_available() {
            return Some(PlatformAutomation::Linux(automation));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        // This test will pass on any platform
        let automation = get_platform_automation();

        if cfg!(target_os = "macos") || cfg!(target_os = "windows") || cfg!(target_os = "linux") {
            // On supported platforms, automation might be available if tools are installed
            // We can't guarantee it's Some, but we can check it doesn't panic
            match automation.as_ref() {
                Some(auto) => {
                    println!("Platform automation available: {}", auto.description());
                }
                None => {
                    println!("Platform automation not available on this system");
                }
            }
        } else {
            // On unsupported platforms, it should be None
            assert!(automation.is_none());
        }
    }

    #[test]
    fn test_macos_automation_availability() {
        let automation = MacOSAutomation;
        if cfg!(target_os = "macos") {
            // On macOS, availability depends on osascript being present
            let _ = automation.is_available(); // Just check it doesn't panic
        } else {
            // On other platforms, it should not be available
            assert!(!automation.is_available());
        }
    }

    #[test]
    fn test_windows_automation_availability() {
        let automation = WindowsAutomation;
        if cfg!(target_os = "windows") {
            // On Windows, availability depends on PowerShell being present
            let _ = automation.is_available(); // Just check it doesn't panic
        } else {
            // On other platforms, it should not be available
            assert!(!automation.is_available());
        }
    }

    #[test]
    fn test_linux_automation_availability() {
        let automation = LinuxAutomation;
        if cfg!(target_os = "linux") {
            // On Linux, availability depends on xdotool being present
            let _ = automation.is_available(); // Just check it doesn't panic
        } else {
            // On other platforms, it should not be available
            assert!(!automation.is_available());
        }
    }
}
