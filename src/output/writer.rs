//! Low-level writing logic for output routing

use super::config::OutputMode;
use std::io::{self, Write};

/// Write output based on the current mode and output type
pub fn write_output(
    mode: OutputMode,
    is_display: bool,
    args: std::fmt::Arguments,
) -> io::Result<()> {
    match (mode, is_display) {
        // In CLI mode, display goes to stdout, logs go to stderr
        (OutputMode::Cli, true) => {
            print!("{args}");
            io::stdout().flush()
        }
        (OutputMode::Cli, false) => {
            eprint!("{args}");
            io::stderr().flush()
        }
        // In MCP mode, everything goes to stderr
        (OutputMode::Mcp, _) => {
            eprint!("{args}");
            io::stderr().flush()
        }
    }
}

/// Write output with newline based on the current mode and output type
pub fn writeln_output(
    mode: OutputMode,
    is_display: bool,
    args: std::fmt::Arguments,
) -> io::Result<()> {
    match (mode, is_display) {
        // In CLI mode, display goes to stdout, logs go to stderr
        (OutputMode::Cli, true) => {
            println!("{args}");
            io::stdout().flush()
        }
        (OutputMode::Cli, false) => {
            eprintln!("{args}");
            io::stderr().flush()
        }
        // In MCP mode, everything goes to stderr
        (OutputMode::Mcp, _) => {
            eprintln!("{args}");
            io::stderr().flush()
        }
    }
}
