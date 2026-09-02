//! Translation of `pcre2_newline.c`.

use crate::internal::*;

const CHAR_LF: u32 = 0x0a;
const CHAR_VT: u32 = 0x0b;
const CHAR_FF: u32 = 0x0c;
const CHAR_CR: u32 = 0x0d;
const CHAR_NEL: u32 = 0x85;

/// `PRIV(is_newline)` — is there a newline at `ptr`?
///
/// Called only when the newline type is `NLTYPE_ANY` or `NLTYPE_ANYCRLF`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_is_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    endptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe {
        let c: u32 = if utf != 0 { GETCHAR(ptr) } else { *ptr as u32 };

        if type_ == NLTYPE_ANYCRLF as u32 {
            match c {
                CHAR_LF => {
                    *lenptr = 1;
                    TRUE
                }
                CHAR_CR => {
                    *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF {
                        2
                    } else {
                        1
                    };
                    TRUE
                }
                _ => FALSE,
            }
        } else {
            // NLTYPE_ANY
            match c {
                CHAR_LF | CHAR_VT | CHAR_FF => {
                    *lenptr = 1;
                    TRUE
                }
                CHAR_CR => {
                    *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF {
                        2
                    } else {
                        1
                    };
                    TRUE
                }
                CHAR_NEL => {
                    *lenptr = if utf != 0 { 2 } else { 1 };
                    TRUE
                }
                0x2028 | 0x2029 => {
                    *lenptr = 3;
                    TRUE
                }
                _ => FALSE,
            }
        }
    }
}

/// `PRIV(was_newline)` — was there a newline immediately before `ptr`?
///
/// Called only when the newline type is `NLTYPE_ANY` or `NLTYPE_ANYCRLF`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_was_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    startptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe {
        let mut ptr = ptr.sub(1);
        let c: u32 = if utf != 0 {
            BACKCHAR(&mut ptr);
            GETCHAR(ptr)
        } else {
            *ptr as u32
        };

        if type_ == NLTYPE_ANYCRLF as u32 {
            match c {
                CHAR_LF => {
                    *lenptr = if ptr > startptr && *ptr.offset(-1) as u32 == CHAR_CR {
                        2
                    } else {
                        1
                    };
                    TRUE
                }
                CHAR_CR => {
                    *lenptr = 1;
                    TRUE
                }
                _ => FALSE,
            }
        } else {
            // NLTYPE_ANY
            match c {
                CHAR_LF => {
                    *lenptr = if ptr > startptr && *ptr.offset(-1) as u32 == CHAR_CR {
                        2
                    } else {
                        1
                    };
                    TRUE
                }
                CHAR_VT | CHAR_FF | CHAR_CR => {
                    *lenptr = 1;
                    TRUE
                }
                CHAR_NEL => {
                    *lenptr = if utf != 0 { 2 } else { 1 };
                    TRUE
                }
                0x2028 | 0x2029 => {
                    *lenptr = 3;
                    TRUE
                }
                _ => FALSE,
            }
        }
    }
}
