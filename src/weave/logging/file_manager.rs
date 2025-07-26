use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, ErrorKind};

const LOG_DIR: &str = ".weaver/logs";
const LOG_FILE_NAME: &str = "weaver.log";
const MAX_LOG_FILES: usize = 10;
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB

#[derive(Debug)]
pub enum LogFileError {
    DirectoryCreationFailed { path: PathBuf, source: io::Error },
    PermissionDenied { path: PathBuf },
    DiskFull { path: PathBuf },
    IoError { path: PathBuf, source: io::Error },
}

impl std::fmt::Display for LogFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFileError::DirectoryCreationFailed { path, source } => {
                write!(f, "Failed to create log directory '{}': {}", path.display(), source)
            }
            LogFileError::PermissionDenied { path } => {
                write!(f, "Permission denied when accessing log directory '{}'", path.display())
            }
            LogFileError::DiskFull { path } => {
                write!(f, "Insufficient disk space for log directory '{}'", path.display())
            }
            LogFileError::IoError { path, source } => {
                write!(f, "I/O error with log file '{}': {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for LogFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LogFileError::DirectoryCreationFailed { source, .. } |
            LogFileError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub struct FileManager {
    log_dir: PathBuf,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            log_dir: PathBuf::from(LOG_DIR),
        }
    }

    pub fn with_custom_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            log_dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Create the log directory if it doesn't exist
    pub fn ensure_log_directory(&self) -> Result<(), LogFileError> {
        if self.log_dir.exists() {
            if !self.log_dir.is_dir() {
                return Err(LogFileError::IoError {
                    path: self.log_dir.clone(),
                    source: io::Error::new(
                        ErrorKind::InvalidInput,
                        "Log path exists but is not a directory"
                    ),
                });
            }
            return Ok(());
        }

        match fs::create_dir_all(&self.log_dir) {
            Ok(()) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::PermissionDenied => Err(LogFileError::PermissionDenied {
                    path: self.log_dir.clone(),
                }),
                ErrorKind::StorageFull => Err(LogFileError::DiskFull {
                    path: self.log_dir.clone(),
                }),
                _ => Err(LogFileError::DirectoryCreationFailed {
                    path: self.log_dir.clone(),
                    source: e,
                }),
            }
        }
    }

    /// Get the path to the main log file
    pub fn get_log_file_path(&self) -> PathBuf {
        self.log_dir.join(LOG_FILE_NAME)
    }

    /// Get the path to a rotated log file (e.g., weaver.log.1)
    pub fn get_rotated_log_path(&self, index: usize) -> PathBuf {
        if index == 0 {
            self.get_log_file_path()
        } else {
            self.log_dir.join(format!("{}.{}", LOG_FILE_NAME, index))
        }
    }

    /// Check if the current log file exceeds the size limit
    pub fn should_rotate(&self) -> Result<bool, LogFileError> {
        let log_path = self.get_log_file_path();
        
        if !log_path.exists() {
            return Ok(false);
        }

        match fs::metadata(&log_path) {
            Ok(metadata) => Ok(metadata.len() >= MAX_FILE_SIZE),
            Err(e) => Err(LogFileError::IoError {
                path: log_path,
                source: e,
            }),
        }
    }

    /// Rotate log files by shifting them (weaver.log -> weaver.log.1, etc.)
    pub fn rotate_files(&self) -> Result<(), LogFileError> {
        // Remove the oldest file if it exists (weaver.log.10 when MAX_LOG_FILES=10)
        let oldest_file = self.get_rotated_log_path(MAX_LOG_FILES);  // This will be weaver.log.10
        if oldest_file.exists() {
            fs::remove_file(&oldest_file).map_err(|e| LogFileError::IoError {
                path: oldest_file.clone(),
                source: e,
            })?;
        }

        // Shift all existing files by one (weaver.log.8 -> weaver.log.9, etc.)
        // We go from MAX_LOG_FILES-1 down to 1
        for i in (1..MAX_LOG_FILES).rev() {
            let current_file = self.get_rotated_log_path(i);
            let next_file = self.get_rotated_log_path(i + 1);
            
            if current_file.exists() {
                fs::rename(&current_file, &next_file).map_err(|e| LogFileError::IoError {
                    path: current_file.clone(),
                    source: e,
                })?;
            }
        }

        // Move current log file to weaver.log.1
        let current_log = self.get_log_file_path();
        let first_rotated = self.get_rotated_log_path(1);
        
        if current_log.exists() {
            fs::rename(&current_log, &first_rotated).map_err(|e| LogFileError::IoError {
                path: current_log.clone(),
                source: e,
            })?;
        }

        Ok(())
    }

    /// Get available disk space at the log directory
    pub fn get_available_space(&self) -> Result<u64, LogFileError> {
        // For cross-platform compatibility, we'll use a simple approach
        // In a real implementation, you might want to use platform-specific APIs
        match fs::metadata(&self.log_dir) {
            Ok(_) => {
                // This is a simplified check - in practice you'd want to use
                // platform-specific APIs to get actual disk space
                Ok(u64::MAX) // Assume sufficient space for now
            }
            Err(e) => Err(LogFileError::IoError {
                path: self.log_dir.clone(),
                source: e,
            }),
        }
    }

    /// Get the total size of all log files
    pub fn get_total_log_size(&self) -> Result<u64, LogFileError> {
        let mut total_size = 0;
        
        for i in 0..MAX_LOG_FILES {
            let log_path = self.get_rotated_log_path(i);
            if log_path.exists() {
                match fs::metadata(&log_path) {
                    Ok(metadata) => total_size += metadata.len(),
                    Err(e) => return Err(LogFileError::IoError {
                        path: log_path,
                        source: e,
                    }),
                }
            }
        }
        
        Ok(total_size)
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Crash the application with a detailed error message
pub fn crash_with_error(error: LogFileError) -> ! {
    eprintln!("FATAL: Logging system failure - {}", error);
    eprintln!("The application cannot continue without proper logging functionality.");
    eprintln!("Please check file permissions and available disk space.");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_manager_creation() {
        let manager = FileManager::new();
        assert_eq!(manager.log_dir, PathBuf::from(LOG_DIR));
    }

    #[test]
    fn test_custom_directory() {
        let custom_dir = "/tmp/custom_logs";
        let manager = FileManager::with_custom_dir(custom_dir);
        assert_eq!(manager.log_dir, PathBuf::from(custom_dir));
    }

    #[test]
    fn test_ensure_log_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("test_logs");
        let manager = FileManager::with_custom_dir(&log_dir);

        assert!(!log_dir.exists());
        assert!(manager.ensure_log_directory().is_ok());
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());
    }

    #[test]
    fn test_log_file_paths() {
        let manager = FileManager::new();
        
        let main_log = manager.get_log_file_path();
        assert_eq!(main_log, PathBuf::from(LOG_DIR).join(LOG_FILE_NAME));

        let rotated_1 = manager.get_rotated_log_path(1);
        assert_eq!(rotated_1, PathBuf::from(LOG_DIR).join("weaver.log.1"));

        let rotated_0 = manager.get_rotated_log_path(0);
        assert_eq!(rotated_0, main_log);
    }

    #[test]
    fn test_should_rotate_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileManager::with_custom_dir(temp_dir.path());
        
        manager.ensure_log_directory().unwrap();
        assert_eq!(manager.should_rotate().unwrap(), false);
    }

    #[test]
    fn test_should_rotate_small_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileManager::with_custom_dir(temp_dir.path());
        
        manager.ensure_log_directory().unwrap();
        let log_file = manager.get_log_file_path();
        fs::write(&log_file, "small content").unwrap();
        
        assert_eq!(manager.should_rotate().unwrap(), false);
    }

    #[test]
    fn test_get_total_log_size() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileManager::with_custom_dir(temp_dir.path());
        
        manager.ensure_log_directory().unwrap();
        
        // Create a few log files
        let log_file = manager.get_log_file_path();
        fs::write(&log_file, "main log content").unwrap();
        
        let rotated_1 = manager.get_rotated_log_path(1);
        fs::write(&rotated_1, "rotated content").unwrap();
        
        let total_size = manager.get_total_log_size().unwrap();
        assert!(total_size > 0);
        assert_eq!(total_size, "main log content".len() as u64 + "rotated content".len() as u64);
    }

    #[test]
    fn test_file_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileManager::with_custom_dir(temp_dir.path());
        
        manager.ensure_log_directory().unwrap();
        
        // Create initial log file and some rotated files
        let log_file = manager.get_log_file_path();
        fs::write(&log_file, "current log").unwrap();
        
        let rotated_1 = manager.get_rotated_log_path(1);
        fs::write(&rotated_1, "old log 1").unwrap();
        
        let rotated_2 = manager.get_rotated_log_path(2);
        fs::write(&rotated_2, "old log 2").unwrap();
        
        // Perform rotation
        manager.rotate_files().unwrap();
        
        // Check that files were rotated correctly
        assert!(!log_file.exists()); // Original log should be moved
        assert!(rotated_1.exists()); // Should contain the old current log
        assert!(rotated_2.exists()); // Should contain the old rotated_1
        assert!(manager.get_rotated_log_path(3).exists()); // Should contain the old rotated_2
        
        // Check contents
        assert_eq!(fs::read_to_string(&rotated_1).unwrap(), "current log");
        assert_eq!(fs::read_to_string(&rotated_2).unwrap(), "old log 1");
        assert_eq!(fs::read_to_string(&manager.get_rotated_log_path(3)).unwrap(), "old log 2");
    }

    #[test]
    fn test_simple_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileManager::with_custom_dir(temp_dir.path());
        
        manager.ensure_log_directory().unwrap();
        
        // Create current log and a few rotated files
        let current_log = manager.get_log_file_path();
        fs::write(&current_log, "current log content").unwrap();
        
        let file1 = manager.get_rotated_log_path(1);
        fs::write(&file1, "old content 1").unwrap();
        
        let file2 = manager.get_rotated_log_path(2);
        fs::write(&file2, "old content 2").unwrap();
        
        // Perform rotation
        manager.rotate_files().unwrap();
        
        // Verify rotation worked
        assert!(!current_log.exists()); // Current log should be moved
        assert_eq!(fs::read_to_string(&manager.get_rotated_log_path(1)).unwrap(), "current log content");
        assert_eq!(fs::read_to_string(&manager.get_rotated_log_path(2)).unwrap(), "old content 1");
        assert_eq!(fs::read_to_string(&manager.get_rotated_log_path(3)).unwrap(), "old content 2");
    }
}