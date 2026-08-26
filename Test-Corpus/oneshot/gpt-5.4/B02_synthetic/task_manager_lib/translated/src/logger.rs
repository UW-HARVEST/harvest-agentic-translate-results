use std::env;
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn initialize_logger_internal() -> c_int {
    let log_file_path = env::var("LOG_FILE").unwrap_or_else(|_| "default.log".to_string());

    match OpenOptions::new().create(true).append(true).open(&log_file_path) {
        Ok(file) => {
            let mut guard = LOG_FILE.lock().unwrap();
            *guard = Some(file);
            log_info_internal("Logger initialized.");
            0
        }
        Err(_) => {
            eprintln!("Failed to open log file: {}", log_file_path);
            -1
        }
    }
}

pub fn log_info_internal(message: &str) {
    write_log("INFO", message);
}

pub fn log_warning_internal(message: &str) {
    write_log("WARNING", message);
}

pub fn log_error_internal(message: &str) {
    write_log("ERROR", message);
}

pub fn finalize_logger_internal() {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(file) = guard.as_mut() {
        let _ = writeln!(file, "[INFO] Logger finalized.");
        let _ = file.flush();
    }
    *guard = None;
}

fn write_log(level: &str, message: &str) {
    let mut guard = LOG_FILE.lock().unwrap();
    if let Some(file) = guard.as_mut() {
        let _ = writeln!(file, "[{}] {}", level, message);
        let _ = file.flush();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    initialize_logger_internal()
}

#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    if message.is_null() {
        return;
    }
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    log_info_internal(&message);
}

#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    if message.is_null() {
        return;
    }
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    log_warning_internal(&message);
}

#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    if message.is_null() {
        return;
    }
    let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
    log_error_internal(&message);
}

#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    finalize_logger_internal();
}
