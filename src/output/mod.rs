//! Unified output interface for CLI and MCP modes
//!
//! This module provides a consistent interface for output that works correctly
//! in both CLI mode (where user-facing output goes to stdout) and MCP mode
//! (where everything except protocol messages goes to stderr).

mod config;
mod display;
mod logging;
pub(crate) mod writer;

pub use config::{OutputConfig, OutputMode};

use once_cell::sync::OnceCell;
use std::sync::RwLock;

static OUTPUT_CONFIG: OnceCell<RwLock<OutputConfig>> = OnceCell::new();

/// Initialize the output system with the specified mode
pub fn init(mode: OutputMode) {
    init_with_verbosity(mode, false);
}

/// Initialize the output system with the specified mode and verbosity
pub fn init_with_verbosity(mode: OutputMode, verbose: bool) {
    let mut config = OutputConfig::new(mode);
    if verbose {
        config.set_verbose();
    }

    // Initialize tracing based on mode
    config.init_tracing();

    // Store config globally
    OUTPUT_CONFIG
        .set(RwLock::new(config))
        .expect("Output system already initialized");
}

/// Get the current output configuration
pub(crate) fn config() -> &'static RwLock<OutputConfig> {
    OUTPUT_CONFIG.get().expect("Output system not initialized")
}

/// Check if output system is initialized
pub fn is_initialized() -> bool {
    OUTPUT_CONFIG.get().is_some()
}

/// Get current output mode
pub fn current_mode() -> OutputMode {
    if let Some(config) = OUTPUT_CONFIG.get() {
        config.read().unwrap().mode()
    } else {
        // Default to CLI mode if not initialized
        OutputMode::Cli
    }
}
