//! Translation of c_src/src/error.c
use crate::jansson::{json_error_t, JSON_ERROR_SOURCE_LENGTH, JSON_ERROR_TEXT_LENGTH};
use crate::libc;
use crate::libc::va_list;
use std::ffi::{c_char, c_int, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_init(error: *mut json_error_t, source: *const c_char) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set_source(error: *mut json_error_t, source: *const c_char) {
    let length: usize;

    if error.is_null() || source.is_null() {
        return;
    }

    length = libc::strlen(source);
    if length < JSON_ERROR_SOURCE_LENGTH {
        libc::strncpy(
            (*error).source.as_mut_ptr(),
            source,
            length + 1,
        );
    } else {
        let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
        libc::memcpy(
            (*error).source.as_mut_ptr() as *mut c_void,
            b"...\0".as_ptr() as *const c_void,
            3,
        );
        libc::strncpy(
            (*error).source.as_mut_ptr().add(3),
            source.add(extra),
            length - extra + 1,
        );
    }
}

/* jsonp_error_set() is variadic; the trampoline in va.rs provides the C ABI
   entry point and forwards to jsonp_error_vset(). */

/// Equivalent of `jsonp_error_set(error, line, column, position, code, "%s", s)`.
pub unsafe fn jsonp_error_set_1s(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    s: *const c_char,
) {
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

    libc::snprintf(
        (*error).text.as_mut_ptr(),
        JSON_ERROR_TEXT_LENGTH - 1,
        b"%s\0".as_ptr() as *const c_char,
        s,
    );
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}

/// Equivalent of `jsonp_error_vset(error, line, column, position, code, fmt, ap)`
/// where the message has already been formatted into `text`.
pub unsafe fn jsonp_error_set_text(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    text: *const c_char,
) {
    jsonp_error_set_1s(error, line, column, position, code, text)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_vset(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    msg: *const c_char,
    ap: va_list,
) {
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

    libc::vsnprintf(
        (*error).text.as_mut_ptr(),
        JSON_ERROR_TEXT_LENGTH - 1,
        msg,
        ap,
    );
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}
