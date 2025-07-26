/// Structured logging macros for the Weaver interpreter
/// 
/// These macros provide a clean API for logging with structured metadata.
/// Usage examples:
/// - log_info!("Message")
/// - log_debug!("Processing token", token_type = "IDENTIFIER", line = 42)
/// - log_error!("Compilation failed", error = "syntax error", file = "test.wv")

/// Log a debug message with optional structured data
#[macro_export]
macro_rules! log_debug {
    ($msg:expr) => {
        tracing::debug!($msg)
    };
    ($msg:expr, $($key:ident = $value:expr),+ $(,)?) => {
        tracing::debug!($($key = $value),+, $msg)
    };
}

/// Log an info message with optional structured data
#[macro_export]
macro_rules! log_info {
    ($msg:expr) => {
        tracing::info!($msg)
    };
    ($msg:expr, $($key:ident = $value:expr),+ $(,)?) => {
        tracing::info!($($key = $value),+, $msg)
    };
}

/// Log a warning message with optional structured data
#[macro_export]
macro_rules! log_warn {
    ($msg:expr) => {
        tracing::warn!($msg)
    };
    ($msg:expr, $($key:ident = $value:expr),+ $(,)?) => {
        tracing::warn!($($key = $value),+, $msg)
    };
}

/// Log an error message with optional structured data
#[macro_export]
macro_rules! log_error {
    ($msg:expr) => {
        tracing::error!($msg)
    };
    ($msg:expr, $($key:ident = $value:expr),+ $(,)?) => {
        tracing::error!($($key = $value),+, $msg)
    };
}

// Re-export the macros for easier use within the crate
pub use crate::{log_debug, log_error, log_info, log_warn};

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_subscriber() {
        INIT.call_once(|| {
            tracing_subscriber::registry()
                .with(fmt::layer().with_test_writer())
                .init();
        });
    }

    #[test]
    fn test_log_debug_simple() {
        init_test_subscriber();
        log_debug!("This is a debug message");
        // Test passes if no panic occurs
    }

    #[test]
    fn test_log_info_with_context() {
        init_test_subscriber();
        log_info!("Processing token", token_type = "IDENTIFIER", line = 42);
        // Test passes if no panic occurs
    }

    #[test]
    fn test_log_warn_with_multiple_fields() {
        init_test_subscriber();
        log_warn!(
            "Warning message", 
            module = "parser", 
            function = "parse_expression",
            severity = "low"
        );
        // Test passes if no panic occurs
    }

    #[test]
    fn test_log_error_simple() {
        init_test_subscriber();
        log_error!("Critical error occurred");
        // Test passes if no panic occurs
    }

    #[test]
    fn test_all_log_levels() {
        init_test_subscriber();
        
        log_debug!("Debug message", component = "test");
        log_info!("Info message", component = "test");
        log_warn!("Warning message", component = "test");
        log_error!("Error message", component = "test");
        
        // Test passes if no panic occurs
    }

    #[test]
    fn test_complex_structured_data() {
        init_test_subscriber();
        
        let file_name = "test.wv";
        let line_number = 123;
        let token_count = 45;
        
        log_info!(
            "Compilation progress",
            file = file_name,
            line = line_number,
            tokens_processed = token_count,
            stage = "parsing"
        );
        
        // Test passes if no panic occurs
    }
}