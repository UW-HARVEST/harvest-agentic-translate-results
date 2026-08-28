//! Translation of `c_src/src/logger.c` / `c_src/include/logger.h`.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::cstd::{self, FILE};

/// `static FILE *log_file = NULL;`
///
/// The C original keeps a single file-scope stream; it is deliberately not
/// reset by `finalize_logger` (which only `fclose`s it), so that behaviour is
/// mirrored here bug-for-bug.
static mut LOG_FILE: *mut FILE = ptr::null_mut();

#[inline]
fn log_file() -> *mut FILE {
    unsafe { LOG_FILE }
}

/// ```c
/// int initialize_logger();
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn initialize_logger() -> c_int {
    unsafe {
        let log_file_env: *mut c_char = cstd::getenv(c"LOG_FILE".as_ptr());
        let log_file_path: *const c_char = if !log_file_env.is_null() {
            log_file_env as *const c_char
        } else {
            c"default.log".as_ptr()
        };

        LOG_FILE = cstd::fopen(log_file_path, c"a".as_ptr());
        if LOG_FILE.is_null() {
            cstd::c_fprintf(
                cstd::stderr,
                c"Failed to open log file: %s\n".as_ptr(),
                log_file_path,
            );
            return -1;
        }
    }

    log_info(c"Logger initialized.".as_ptr());
    0
}

/// ```c
/// void log_info(const char *message);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_info(message: *const c_char) {
    let stream = log_file();
    if !stream.is_null() {
        unsafe {
            cstd::c_fprintf(stream, c"[INFO] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void log_warning(const char *message);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_warning(message: *const c_char) {
    let stream = log_file();
    if !stream.is_null() {
        unsafe {
            cstd::c_fprintf(stream, c"[WARNING] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void log_error(const char *message);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn log_error(message: *const c_char) {
    let stream = log_file();
    if !stream.is_null() {
        unsafe {
            cstd::c_fprintf(stream, c"[ERROR] %s\n".as_ptr(), message);
        }
    }
}

/// ```c
/// void finalize_logger();
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn finalize_logger() {
    let stream = log_file();
    if !stream.is_null() {
        log_info(c"Logger finalized.".as_ptr());
        unsafe {
            cstd::fclose(stream);
        }
    }
}
