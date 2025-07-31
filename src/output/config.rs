//! Output configuration and mode management

use console::Term;
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Output mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Normal CLI operation - display to stdout, logs to stderr
    Cli,
    /// MCP server mode - everything to stderr except protocol messages
    Mcp,
}

/// Configuration for the output system
#[derive(Debug)]
pub struct OutputConfig {
    mode: OutputMode,
    color_enabled: bool,
    log_level: Level,
}

impl OutputConfig {
    /// Create a new output configuration
    pub fn new(mode: OutputMode) -> Self {
        // Detect color support
        let color_enabled = match mode {
            OutputMode::Cli => Term::stdout().features().colors_supported(),
            OutputMode::Mcp => false, // Disable colors in MCP mode
        };

        // Determine log level from environment or defaults
        let log_level = match std::env::var("RUST_LOG") {
            Ok(level) => match level.to_lowercase().as_str() {
                "trace" => Level::TRACE,
                "debug" => Level::DEBUG,
                "info" => Level::INFO,
                "warn" => Level::WARN,
                "error" => Level::ERROR,
                _ => match mode {
                    OutputMode::Cli => Level::INFO,
                    OutputMode::Mcp => Level::WARN, // Less verbose in MCP mode
                },
            },
            Err(_) => match mode {
                OutputMode::Cli => Level::INFO,
                OutputMode::Mcp => Level::WARN,
            },
        };

        Self {
            mode,
            color_enabled,
            log_level,
        }
    }

    /// Get the current output mode
    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Check if colors are enabled
    pub fn colors_enabled(&self) -> bool {
        self.color_enabled
    }

    /// Get the current log level
    pub fn log_level(&self) -> Level {
        self.log_level
    }

    /// Set verbose mode (DEBUG level)
    pub fn set_verbose(&mut self) {
        self.log_level = Level::DEBUG;
    }

    /// Initialize the tracing subscriber based on configuration
    pub fn init_tracing(&self) {
        let builder = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive(self.log_level.into()))
            .with_target(false)
            .with_level(true);

        match self.mode {
            OutputMode::Cli => {
                // In CLI mode, logs go to stderr with colors if supported
                builder
                    .with_ansi(self.color_enabled)
                    .with_writer(std::io::stderr)
                    .init();
            }
            OutputMode::Mcp => {
                // In MCP mode, everything goes to stderr without colors
                builder
                    .with_ansi(false)
                    .with_writer(std::io::stderr)
                    .without_time() // Simpler format for MCP
                    .compact() // More compact format
                    .init();
            }
        }
    }
}
