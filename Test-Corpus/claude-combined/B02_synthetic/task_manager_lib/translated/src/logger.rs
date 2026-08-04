// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT

use core::ffi::{c_char, c_int};
use core::ptr;

// Mirror of the C `static FILE *log_file = NULL;` global.
static mut LOG_FILE: *mut libc::FILE = ptr::null_mut();

unsafe fn log_file_ptr() -> *mut libc::FILE {
    unsafe { LOG_FILE }
}

unsafe fn set_log_file(p: *mut libc::FILE) {
    unsafe {
        LOG_FILE = p;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let getenv_key = b"LOG_FILE\0".as_ptr() as *const c_char;
    let log_file_env = unsafe { libc::getenv(getenv_key) };
    let default_path = b"default.log\0".as_ptr() as *const c_char;
    let log_file_path: *const c_char = if log_file_env.is_null() {
        default_path
    } else {
        log_file_env as *const c_char
    };

    let mode = b"a\0".as_ptr() as *const c_char;
    let f = unsafe { libc::fopen(log_file_path, mode) };
    if f.is_null() {
        let stderr = unsafe { stderr_handle() };
        let fmt = b"Failed to open log file: %s\n\0".as_ptr() as *const c_char;
        unsafe {
            libc::fprintf(stderr, fmt, log_file_path);
        }
        return -1;
    }
    unsafe {
        set_log_file(f);
    }

    let msg = b"Logger initialized.\0".as_ptr() as *const c_char;
    unsafe { log_info(msg) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    let f = unsafe { log_file_ptr() };
    if !f.is_null() {
        let fmt = b"[INFO] %s\n\0".as_ptr() as *const c_char;
        unsafe {
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    let f = unsafe { log_file_ptr() };
    if !f.is_null() {
        let fmt = b"[WARNING] %s\n\0".as_ptr() as *const c_char;
        unsafe {
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    let f = unsafe { log_file_ptr() };
    if !f.is_null() {
        let fmt = b"[ERROR] %s\n\0".as_ptr() as *const c_char;
        unsafe {
            libc::fprintf(f, fmt, message);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    let f = unsafe { log_file_ptr() };
    if !f.is_null() {
        let msg = b"Logger finalized.\0".as_ptr() as *const c_char;
        unsafe {
            log_info(msg);
            libc::fclose(f);
        }
    }
}

// Helper for stderr handle. libc provides `stderr` as an extern static via
// `libc::__stderrp` on macOS or as a function on glibc; use the platform-aware
// helper from libc.
unsafe fn stderr_handle() -> *mut libc::FILE {
    extern "C" {
        // On glibc, `stderr` is an exported symbol of type `FILE *`.
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}
