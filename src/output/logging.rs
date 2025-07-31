//! Logging macros for developer/debug output

/// Log a message at the trace level
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {{
        ::tracing::trace!($($arg)*);
    }};
}

/// Log a message at the debug level
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {{
        ::tracing::debug!($($arg)*);
    }};
}

/// Log a message at the info level
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        ::tracing::info!($($arg)*);
    }};
}

/// Log a message at the warn level
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {{
        ::tracing::warn!($($arg)*);
    }};
}

/// Log a message at the error level
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        ::tracing::error!($($arg)*);
    }};
}
