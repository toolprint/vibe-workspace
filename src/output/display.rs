//! Display macros for user-facing terminal output

/// Print user-facing output without newline
/// Routes to stdout in CLI mode, stderr in MCP mode
#[macro_export]
macro_rules! display_print {
    ($($arg:tt)*) => {{
        use $crate::output::{current_mode, writer::write_output};
        let _ = write_output(current_mode(), true, format_args!($($arg)*));
    }};
}

/// Print user-facing output with newline
/// Routes to stdout in CLI mode, stderr in MCP mode
#[macro_export]
macro_rules! display_println {
    () => {
        $crate::display_print!("\n")
    };
    ($($arg:tt)*) => {{
        use $crate::output::{current_mode, writer::writeln_output};
        let _ = writeln_output(current_mode(), true, format_args!($($arg)*));
    }};
}

/// Print user-facing error output without newline
/// Always routes to stderr
#[macro_export]
macro_rules! display_eprint {
    ($($arg:tt)*) => {{
        eprint!($($arg)*);
        let _ = std::io::Write::flush(&mut std::io::stderr());
    }};
}

/// Print user-facing error output with newline
/// Always routes to stderr
#[macro_export]
macro_rules! display_eprintln {
    () => {
        eprintln!()
    };
    ($($arg:tt)*) => {{
        eprintln!($($arg)*);
    }};
}
