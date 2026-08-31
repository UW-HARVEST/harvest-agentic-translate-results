//! Translation of `src/error.c`.

use crate::types::*;
use crate::varargs::VaListTag;
use core::ffi::{c_char, c_int, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_init(error: *mut JsonErrorT, source: *const c_char) {
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
pub unsafe extern "C" fn jsonp_error_set_source(error: *mut JsonErrorT, source: *const c_char) {
    let length: usize;

    if error.is_null() || source.is_null() {
        return;
    }

    length = strlen(source);
    if length < JSON_ERROR_SOURCE_LENGTH {
        /* strncpy(error->source, source, length + 1) */
        memcpy(
            (*error).source.as_mut_ptr() as *mut c_void,
            source as *const c_void,
            length + 1,
        );
    } else {
        let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
        memcpy(
            (*error).source.as_mut_ptr() as *mut c_void,
            b"...".as_ptr() as *const c_void,
            3,
        );
        /* strncpy(error->source + 3, source + extra, length - extra + 1) */
        memcpy(
            (*error).source.as_mut_ptr().add(3) as *mut c_void,
            source.add(extra) as *const c_void,
            length - extra + 1,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_vset(
    error: *mut JsonErrorT,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    msg: *const c_char,
    ap: *mut VaListTag,
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

    vsnprintf(
        (*error).text.as_mut_ptr(),
        JSON_ERROR_TEXT_LENGTH - 1,
        msg,
        ap,
    );
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}

/// Equivalent of `jsonp_error_vset()` for callers inside the library, which
/// format their message with Rust code instead of `vsnprintf()`. `text` holds
/// the already formatted bytes.
pub unsafe fn jsonp_error_set_bytes(
    error: *mut JsonErrorT,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    text: &[u8],
) {
    if error.is_null() {
        return;
    }

    if (*error).text[0] != 0 {
        return;
    }

    (*error).line = line;
    (*error).column = column;
    (*error).position = position as c_int;

    /* vsnprintf(error->text, JSON_ERROR_TEXT_LENGTH - 1, ...) stores at most
       JSON_ERROR_TEXT_LENGTH - 2 bytes plus a terminating NUL. */
    let n = if text.len() > JSON_ERROR_TEXT_LENGTH - 2 {
        JSON_ERROR_TEXT_LENGTH - 2
    } else {
        text.len()
    };
    for i in 0..n {
        (*error).text[i] = text[i] as c_char;
    }
    (*error).text[n] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}
