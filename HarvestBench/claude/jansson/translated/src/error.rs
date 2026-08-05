//! Translation of error.c
use crate::types::*;
use core::ffi::{c_char, c_int, c_void, VaList};

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: VaList) -> c_int;
}

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

    let length = strlen(source);
    if length < JSON_ERROR_SOURCE_LENGTH {
        strncpy((*error).source.as_mut_ptr(), source, length + 1);
    } else {
        let extra = length - JSON_ERROR_SOURCE_LENGTH + 4;
        memcpy(
            (*error).source.as_mut_ptr() as *mut c_void,
            b"...\0".as_ptr() as *const c_void,
            3,
        );
        strncpy(
            (*error).source.as_mut_ptr().add(3),
            source.add(extra),
            length - extra + 1,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    msg: *const c_char,
    ap: ...
) {
    jsonp_error_vset(error, line, column, position, code, msg, ap);
}

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
