/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Translation of `c_src/src/logger.c`.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::cbind::{fclose, fopen, fprintf, getenv, FILE};

/// `static FILE *log_file = NULL;`
static mut LOG_FILE: *mut FILE = ptr::null_mut();

/// `int initialize_logger();`
///
/// Opens `$LOG_FILE` (or `default.log`) for appending. Note that, exactly like
/// the C original, a second call simply overwrites the stored handle without
/// closing the previous one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_logger() -> c_int {
    let log_file_env: *const c_char = getenv(c"LOG_FILE".as_ptr());
    let log_file_path: *const c_char = if !log_file_env.is_null() {
        log_file_env
    } else {
        c"default.log".as_ptr()
    };

    LOG_FILE = fopen(log_file_path, c"a".as_ptr());
    if LOG_FILE.is_null() {
        fprintf(
            crate::cbind::stderr,
            c"Failed to open log file: %s\n".as_ptr(),
            log_file_path,
        );
        return -1;
    }

    log_info(c"Logger initialized.".as_ptr());
    0
}

/// `void log_info(const char *message);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_info(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(LOG_FILE, c"[INFO] %s\n".as_ptr(), message);
    }
}

/// `void log_warning(const char *message);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_warning(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(LOG_FILE, c"[WARNING] %s\n".as_ptr(), message);
    }
}

/// `void log_error(const char *message);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_error(message: *const c_char) {
    if !LOG_FILE.is_null() {
        fprintf(LOG_FILE, c"[ERROR] %s\n".as_ptr(), message);
    }
}

/// `void finalize_logger();`
///
/// Mirrors the C original: the handle is closed but *not* reset to `NULL`, so
/// subsequent logging calls still see a non-NULL (now dangling) handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn finalize_logger() {
    if !LOG_FILE.is_null() {
        log_info(c"Logger finalized.".as_ptr());
        fclose(LOG_FILE);
    }
}
