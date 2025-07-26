use tracing::{debug, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod file_manager;
pub mod macros;

pub use file_manager::{FileManager, LogFileError, crash_with_error};
pub use macros::{log_debug, log_error, log_info, log_warn};

pub struct LoggingConfig {
    pub level: LogLevel,
    pub console_output: bool,
    pub file_path: Option<String>,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LogFormat {
    Text,
    Json,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            console_output: false,
            file_path: None,
            format: LogFormat::Text,
        }
    }
}

impl LogLevel {
    pub fn to_env_filter(&self) -> String {
        match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
        .to_string()
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(LogFormat::Text),
            "json" => Ok(LogFormat::Json),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

pub fn init_logging(config: LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    use std::io;
    
    // Create file manager and ensure log directory exists
    let file_manager = FileManager::new();
    if let Err(e) = file_manager.ensure_log_directory() {
        crash_with_error(e);
    }
    
    // Create env filter for log level
    let env_filter = EnvFilter::new(&config.level.to_env_filter());
    
    // Get file path for logging
    let file_path = config.file_path
        .unwrap_or_else(|| file_manager.get_log_file_path().to_string_lossy().to_string());
    
    // Handle potential rotation
    if file_manager.should_rotate().unwrap_or(false) {
        if let Err(e) = file_manager.rotate_files() {
            crash_with_error(e);
        }
    }
    
    // Create file appender
    let file_appender = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
    {
        Ok(file) => file,
        Err(e) => {
            crash_with_error(LogFileError::IoError {
                path: std::path::PathBuf::from(file_path),
                source: e,
            });
        }
    };
    
    // Build subscriber based on configuration
    match (config.console_output, config.format) {
        (true, LogFormat::Text) => {
            // Both console and file output, text format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_thread_ids(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(io::stdout)
                )
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_thread_ids(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(file_appender)
                )
                .try_init()?;
        }
        (true, LogFormat::Json) => {
            // Both console and file output, JSON format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_target(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(io::stdout)
                )
                .with(
                    fmt::layer()
                        .json()
                        .with_target(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(file_appender)
                )
                .try_init()?;
        }
        (false, LogFormat::Text) => {
            // File output only, text format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_target(false)
                        .with_thread_ids(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(file_appender)
                )
                .try_init()?;
        }
        (false, LogFormat::Json) => {
            // File output only, JSON format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .json()
                        .with_target(false)
                        .with_file(true)
                        .with_line_number(true)
                        .with_writer(file_appender)
                )
                .try_init()?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_format_parsing() {
        assert_eq!("text".parse::<LogFormat>().unwrap(), LogFormat::Text);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.console_output, false);
        assert_eq!(config.file_path, None);
        assert_eq!(config.format, LogFormat::Text);
    }
}