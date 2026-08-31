//! Translation of `src/error.c`.

use core::ffi::{c_char, c_int};

use crate::ffi::{self, VaListTag};
use crate::jansson::{json_error_t, JSON_ERROR_SOURCE_LENGTH, JSON_ERROR_TEXT_LENGTH};

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
    if error.is_null() || source.is_null() {
        return;
    }

    let length = ffi::strlen(source);
    if length < JSON_ERROR_SOURCE_LENGTH {
        ffi::strncpy((*error).source.as_mut_ptr(), source, length + 1);
    } else {
        let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
        ffi::memcpy(
            (*error).source.as_mut_ptr() as *mut _,
            b"...\0".as_ptr() as *const _,
            3,
        );
        ffi::strncpy(
            (*error).source.as_mut_ptr().add(3),
            source.add(extra),
            length - extra + 1,
        );
    }
}

/// The variadic `jsonp_error_set` symbol is produced by the assembly shim in
/// `varargs.rs`, which forwards to `jsonp_error_vset`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_vset(
    error: *mut json_error_t,
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

    ffi::vsnprintf(
        (*error).text.as_mut_ptr(),
        JSON_ERROR_TEXT_LENGTH - 1,
        msg,
        ap,
    );
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}

/// Internal helper mirroring `jsonp_error_vset`, but where the caller formats
/// the message directly into `error->text` (using the same 159 byte limit that
/// `vsnprintf` is given in the C sources).
pub unsafe fn error_vset_with<F>(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    fmt: F,
) where
    F: FnOnce(*mut c_char, usize),
{
    if error.is_null() {
        return;
    }

    if (*error).text[0] != 0 {
        return;
    }

    (*error).line = line;
    (*error).column = column;
    (*error).position = position as c_int;

    fmt((*error).text.as_mut_ptr(), JSON_ERROR_TEXT_LENGTH - 1);
    (*error).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
    (*error).text[JSON_ERROR_TEXT_LENGTH - 1] = code as c_char;
}
