//! Translation of `src/error.c`.

use crate::cffi;
use crate::jtypes::{JSON_ERROR_SOURCE_LENGTH, JSON_ERROR_TEXT_LENGTH, json_error_t};
use crate::valist::VaList;
use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_init(error: *mut json_error_t, source: *const c_char) {
    unsafe {
        if !error.is_null() {
            (*error).text[0] = 0;
            (*error).line = -1;
            (*error).column = -1;
            (*error).position = 0;
            if !source.is_null() {
                jsonp_error_set_source(error, source);
            } else {
                (*error).source[0] = 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set_source(error: *mut json_error_t, source: *const c_char) {
    unsafe {
        if error.is_null() || source.is_null() {
            return;
        }

        let length = cffi::c_strlen(source);
        if length < JSON_ERROR_SOURCE_LENGTH {
            /* strncpy(error->source, source, length + 1) */
            core::ptr::copy_nonoverlapping(
                source as *const u8,
                (*error).source.as_mut_ptr() as *mut u8,
                length + 1,
            );
        } else {
            let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
            let dst = (*error).source.as_mut_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(b"...".as_ptr(), dst, 3);
            core::ptr::copy_nonoverlapping(
                (source as *const u8).add(extra),
                dst.add(3),
                length - extra + 1,
            );
        }
    }
}

/* jsonp_error_set() is the variadic entry point defined by the assembly
   trampoline in `trampolines.rs`; it forwards straight to jsonp_error_vset(). */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_vset(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    msg: *const c_char,
    ap: VaList,
) {
    unsafe {
        if error.is_null() {
            return;
        }

        if (*error).text[0] != 0 {
            /* error already set */
            return;
        }

        (*error).line = line;
        (*error).column = column;
        (*error).position = position as c_int;

        cffi::vsnprintf(
            (*error).text.as_mut_ptr(),
            JSON_ERROR_TEXT_LENGTH - 1,
            msg,
            ap,
        );
        (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
        (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
    }
}

/// Helper used by the internal call sites that pass an already formatted
/// message: equivalent to `jsonp_error_set(error, ..., "%s", text)`.
pub unsafe fn jsonp_error_set_str(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    text: *const c_char,
) {
    unsafe {
        if error.is_null() {
            return;
        }

        if (*error).text[0] != 0 {
            return;
        }

        (*error).line = line;
        (*error).column = column;
        (*error).position = position as c_int;

        cffi::snprintf(
            (*error).text.as_mut_ptr(),
            JSON_ERROR_TEXT_LENGTH - 1,
            c"%s".as_ptr(),
            text,
        );
        (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
        (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
    }
}
