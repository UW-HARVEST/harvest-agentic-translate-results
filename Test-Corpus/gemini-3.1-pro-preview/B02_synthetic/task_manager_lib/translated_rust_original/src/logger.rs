use std::ffi::{CStr, c_char, c_int};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::env;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    let log_file_path = env::var("LOG_FILE").unwrap_or_else(|_| "default.log".to_string());
    let file = OpenOptions::new().create(true).append(true).open(&log_file_path);
    match file {
        Ok(f) => {
            if let Ok(mut guard) = LOG_FILE.lock() {
                *guard = Some(f);
            }
            log_info_internal("Logger initialized.");
            0
        }
        Err(_) => {
            eprintln!("Failed to open log file: {}", log_file_path);
            -1
        }
    }
}

pub(crate) fn log_info_internal(message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[INFO] {}", message);
        }
    }
}

pub(crate) fn log_warning_internal(message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[WARNING] {}", message);
        }
    }
}

pub(crate) fn log_error_internal(message: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[ERROR] {}", message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    if message.is_null() { return; }
    let c_str = unsafe { CStr::from_ptr(message) };
    log_info_internal(&c_str.to_string_lossy());
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    if message.is_null() { return; }
    let c_str = unsafe { CStr::from_ptr(message) };
    log_warning_internal(&c_str.to_string_lossy());
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    if message.is_null() { return; }
    let c_str = unsafe { CStr::from_ptr(message) };
    log_error_internal(&c_str.to_string_lossy());
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    log_info_internal("Logger finalized.");
    if let Ok(mut guard) = LOG_FILE.lock() {
        *guard = None;
    }
}
