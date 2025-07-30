pub mod app_manager;
pub mod cursor;
pub mod installer_ui;
pub mod iterm2;
pub mod package_manager;
pub mod registry;
pub mod vscode;
pub mod warp;
pub mod wezterm;
pub mod windsurf;

pub use cursor::{cleanup_cursor_config, open_with_cursor};
pub use installer_ui::run_interactive_installer;
pub use iterm2::{cleanup_iterm2_config, open_with_iterm2_options};
pub use vscode::{cleanup_vscode_config, open_with_vscode};
pub use warp::{cleanup_warp_config, open_with_warp};
pub use wezterm::{cleanup_wezterm_config, open_with_wezterm_options};
pub use windsurf::{cleanup_windsurf_config, open_with_windsurf};
