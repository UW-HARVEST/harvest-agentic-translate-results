/*
 * Rust translation of c_src/src/logger.c
 */

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

/// Initialize the logger.
///
/// Returns 0 on success, -1 on failure (mirroring the C return convention).
pub fn initialize_logger() -> i32 {
    let log_file_path = std::env::var("LOG_FILE").unwrap_or_else(|_| "default.log".to_string());

    let file = match OpenOptions::new().create(true).append(true).open(&log_file_path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Failed to open log file: {}", log_file_path);
            return -1;
        }
    };

    {
        let mut guard = LOG_FILE.lock().unwrap();
        *guard = Some(file);
    }

    log_info("Logger initialized.");
    0
}

fn write_log(level: &str, message: &str) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        let _ = writeln!(file, "[{}] {}", level, message);
    }
}

pub fn log_info(message: &str) {
    write_log("INFO", message);
}

pub fn log_warning(message: &str) {
    write_log("WARNING", message);
}

pub fn log_error(message: &str) {
    write_log("ERROR", message);
}

pub fn finalize_logger() {
    // Match the C behavior: only log "finalized" and close if the file was open.
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(ref mut file) = *guard {
        let _ = writeln!(file, "[INFO] Logger finalized.");
    }
    // Drop the file (closes it) by replacing with None.
    *guard = None;
}
