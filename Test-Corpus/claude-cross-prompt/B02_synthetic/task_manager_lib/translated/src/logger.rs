use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub struct Logger {
    file: Option<std::fs::File>,
}

impl Logger {
    pub fn new() -> Self {
        Logger { file: None }
    }

    /// Returns 0 on success, -1 on failure (matching C's initialize_logger).
    pub fn initialize(&mut self) -> i32 {
        let log_file_env = env::var("LOG_FILE").ok();
        let log_file_path: String = log_file_env.unwrap_or_else(|| "default.log".to_string());

        match OpenOptions::new().create(true).append(true).open(&log_file_path) {
            Ok(f) => {
                self.file = Some(f);
            }
            Err(_) => {
                // Match: fprintf(stderr, "Failed to open log file: %s\n", log_file_path);
                eprintln!("Failed to open log file: {}", log_file_path);
                return -1;
            }
        }

        self.log_info("Logger initialized.");
        0
    }

    pub fn log_info(&mut self, message: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "[INFO] {}", message);
        }
    }

    pub fn log_warning(&mut self, message: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "[WARNING] {}", message);
        }
    }

    pub fn log_error(&mut self, message: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "[ERROR] {}", message);
        }
    }

    pub fn finalize(&mut self) {
        if self.file.is_some() {
            self.log_info("Logger finalized.");
            // Drop the file handle (closes it)
            self.file = None;
        }
    }
}
