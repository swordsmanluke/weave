use crate::weave::vm::vm::VM;
use crate::weave::shell::repl::repl;
use crate::weave::logging::{LoggingConfig, LogLevel, LogFormat};

mod weave;
use clap::Parser;
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(name = "weaver")]
#[command(about = "Weaver programming language interpreter")]
#[command(version)]
struct Cli {
    /// Script file to execute (if not provided, starts REPL)
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Set the logging level
    #[arg(long, value_enum, default_value = "info")]
    log_level: LogLevel,

    /// Also output logs to console (in addition to file)
    #[arg(long)]
    log_console: bool,

    /// Custom log file path
    #[arg(long, value_name = "PATH")]
    log_file: Option<PathBuf>,

    /// Log output format
    #[arg(long, value_enum, default_value = "text")]
    log_format: LogFormat,
}

fn main() {
    let cli = Cli::parse();

    // Create logging configuration from CLI arguments
    let logging_config = LoggingConfig {
        level: cli.log_level,
        console_output: cli.log_console,
        file_path: cli.log_file.map(|p| p.to_string_lossy().to_string()),
        format: cli.log_format,
    };

    // Initialize logging system with config
    if let Err(e) = crate::weave::logging::init_logging(logging_config) {
        eprintln!("FATAL: Failed to initialize logging system: {}", e);
        std::process::exit(1);
    }
    
    // Test log to verify logging is working
    crate::log_info!("Weaver interpreter starting", version = env!("CARGO_PKG_VERSION"));

    // Execute file or start REPL based on arguments
    if let Some(file_path) = cli.file {
        run_file(&file_path.to_string_lossy());
    } else {
        repl();
    }
}

fn run_file(path: &str) {
    let file_contents = std::fs::read_to_string(path).unwrap();
    let mut vm = VM::new(false);
    let res = vm.interpret(&file_contents);
    match res {
        Ok(_) => {},
        Err(e) => { 
            log_error!("File execution failed", error = format!("{:?}", e).as_str(), file = path);
            eprintln!("Error executing {}: {:?}", path, e); 
            exit(e.exit_code()) 
        },
    }
}

