//! Translation of `c_src/src/pcre2_newline.c`.
//!
//! Internal functions for testing newlines when more than one kind of newline
//! is to be recognized. When a newline is found, its length is returned. PCRE2
//! supports NLTYPE_FIXED (handled inline elsewhere), NLTYPE_ANYCRLF, and
//! NLTYPE_ANY. The full list of Unicode newline characters is taken from
//! http://unicode.org/unicode/reports/tr18/.

#![allow(non_snake_case)]

use crate::chars::*;
use crate::internal::*;

/* Check for newline at given position.

This function is called only via the IS_NEWLINE macro, which does so only when
the newline type is NLTYPE_ANY or NLTYPE_ANYCRLF. The case of a fixed newline
(NLTYPE_FIXED) is handled inline. It is guaranteed that the code unit pointed to
by ptr is less than the end of the string.

Arguments:
  ptr          pointer to possible newline
  type         the newline type
  endptr       pointer to the end of the string
  lenptr       where to return the length
  utf          TRUE if in utf mode

Returns:       TRUE or FALSE */

pub unsafe fn is_newline(
    ptr: PCRE2_SPTR,
    type_: u32,
    endptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe {
        let c: u32 = if utf != 0 { getchar_(ptr) } else { *ptr as u32 };

        if type_ == NLTYPE_ANYCRLF {
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
        }
        /* NLTYPE_ANY */
        else {
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
                0x2028 /* LS */ | 0x2029 /* PS */ => {
                    *lenptr = 3;
                    TRUE
                }
                _ => FALSE,
            }
        }
    }
}

/// Exported as `_pcre2_is_newline_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_is_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    endptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe { is_newline(ptr, type_, endptr, lenptr, utf) }
}

/* Check for newline at previous position.

This function is called only via the WAS_NEWLINE macro, which does so only when
the newline type is NLTYPE_ANY or NLTYPE_ANYCRLF. The case of a fixed newline
(NLTYPE_FIXED) is handled inline. It is guaranteed that the initial value of ptr
is greater than the start of the string that is being processed.

Arguments:
  ptr          pointer to possible newline
  type         the newline type
  startptr     pointer to the start of the string
  lenptr       where to return the length
  utf          TRUE if in utf mode

Returns:       TRUE or FALSE */

pub unsafe fn was_newline(
    mut ptr: PCRE2_SPTR,
    type_: u32,
    startptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe {
        ptr = ptr.sub(1);

        let c: u32 = if utf != 0 {
            backchar(&mut ptr);
            getchar_(ptr)
        } else {
            *ptr as u32
        };

        if type_ == NLTYPE_ANYCRLF {
            match c {
                CHAR_LF => {
                    *lenptr = if ptr > startptr && *ptr.sub(1) as u32 == CHAR_CR {
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
        }
        /* NLTYPE_ANY */
        else {
            match c {
                CHAR_LF => {
                    *lenptr = if ptr > startptr && *ptr.sub(1) as u32 == CHAR_CR {
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
                0x2028 /* LS */ | 0x2029 /* PS */ => {
                    *lenptr = 3;
                    TRUE
                }
                _ => FALSE,
            }
        }
    }
}

/// Exported as `_pcre2_was_newline_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_was_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    startptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    unsafe { was_newline(ptr, type_, startptr, lenptr, utf) }
}
