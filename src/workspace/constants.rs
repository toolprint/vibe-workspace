//! Constants for vibe-workspace configuration paths and settings

use std::path::PathBuf;

/// Configuration directory path relative to home directory
pub const CONFIG_DIR_PATH: &str = ".toolprint/vibe-workspace";

/// Display name for user messages
pub const CONFIG_DIR_DISPLAY: &str = "~/.toolprint/vibe-workspace";

/// Get the configuration directory path
pub fn get_config_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(CONFIG_DIR_PATH)
}

/// Get the default config file path
pub fn get_default_config_path() -> PathBuf {
    get_config_dir().join("config.yaml")
}

/// Get the state file path
pub fn get_state_file_path() -> PathBuf {
    get_config_dir().join("state.json")
}

/// Get the templates directory path
pub fn get_templates_dir() -> PathBuf {
    get_config_dir().join("templates")
}

/// Get the backups directory path
pub fn get_backups_dir() -> PathBuf {
    get_config_dir().join("backups")
}

/// Get app-specific template directory path
pub fn get_app_template_dir(app_name: &str) -> PathBuf {
    get_templates_dir().join(app_name)
}
