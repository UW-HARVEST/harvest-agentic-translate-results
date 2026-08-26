//! Translation of `c_src/src/logger.c` (public API: `c_src/include/logger.h`).
//!
//! ```c
//! int  initialize_logger();
//! void log_info(const char *message);
//! void log_warning(const char *message);
//! void log_error(const char *message);
//! void finalize_logger();
//! ```

use std::ffi::{c_char, c_int};
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::cffi::{FILE, fclose, fopen, fprintf, getenv, stderr};

/// `static FILE *log_file = NULL;`
///
/// Modelled with an `AtomicPtr` purely so the module needs no `static mut`;
/// the C original is a plain (non-atomic) file-scope static and all accesses
/// here use `Relaxed` ordering, so observable single-threaded behaviour is
/// identical.
static LOG_FILE: AtomicPtr<FILE> = AtomicPtr::new(ptr::null_mut());

#[inline]
fn log_file_get() -> *mut FILE {
    LOG_FILE.load(Ordering::Relaxed)
}

#[inline]
fn log_file_set(f: *mut FILE) {
    LOG_FILE.store(f, Ordering::Relaxed);
}

/// ```c
/// int initialize_logger() {
///     const char *log_file_env = getenv("LOG_FILE");
///     const char *log_file_path = log_file_env ? log_file_env : "default.log";
///
///     log_file = fopen(log_file_path, "a");
///     if (!log_file) {
///         fprintf(stderr, "Failed to open log file: %s\n", log_file_path);
///         return -1;
///     }
///
///     log_info("Logger initialized.");
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env: *const c_char = getenv(c"LOG_FILE".as_ptr());
        let log_file_path: *const c_char = if !log_file_env.is_null() {
            log_file_env
        } else {
            c"default.log".as_ptr()
        };

        // NB: the assignment happens before the NULL test in the C original,
        // so a failed re-initialisation clobbers any previously open handle.
        let f = fopen(log_file_path, c"a".as_ptr());
        log_file_set(f);
        if f.is_null() {
            fprintf(
                stderr,
                c"Failed to open log file: %s\n".as_ptr(),
                log_file_path,
            );
            return -1;
        }

        log_info(c"Logger initialized.".as_ptr());
        0
    }
}

/// ```c
/// void log_info(const char *message) {
///     if (log_file) fprintf(log_file, "[INFO] %s\n", message);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    let f = log_file_get();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[INFO] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void log_warning(const char *message) {
///     if (log_file) fprintf(log_file, "[WARNING] %s\n", message);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    let f = log_file_get();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[WARNING] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void log_error(const char *message) {
///     if (log_file) fprintf(log_file, "[ERROR] %s\n", message);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    let f = log_file_get();
    if !f.is_null() {
        unsafe {
            fprintf(f, c"[ERROR] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void finalize_logger() {
///     if (log_file) {
///         log_info("Logger finalized.");
///         fclose(log_file);
///     }
/// }
/// ```
///
/// Note: the C code does **not** reset `log_file` to `NULL` after `fclose`,
/// leaving a dangling handle behind. That (buggy) behaviour is preserved here
/// verbatim — we intentionally do not null the pointer out.
#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    let f = log_file_get();
    if !f.is_null() {
        log_info(c"Logger finalized.".as_ptr());
        unsafe {
            fclose(f);
        }
    }
}
