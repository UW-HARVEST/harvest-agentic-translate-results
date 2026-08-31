//! Translated from pcre2_newline.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/*************************************************
*      Check for newline at given position       *
*************************************************/

/* This function is called only via the IS_NEWLINE macro, which does so only
when the newline type is NLTYPE_ANY or NLTYPE_ANYCRLF. The case of a fixed
newline (NLTYPE_FIXED) is handled inline. It is guaranteed that the code unit
pointed to by ptr is less than the end of the string.

Arguments:
  ptr          pointer to possible newline
  type         the newline type
  endptr       pointer to the end of the string
  lenptr       where to return the length
  utf          TRUE if in utf mode

Returns:       TRUE or FALSE
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_is_newline_8(ptr: PCRE2_SPTR, type_: u32, endptr: PCRE2_SPTR, lenptr: *mut u32, utf: BOOL) -> BOOL {
    let mut c: u32;

    /* SUPPORT_UNICODE */
    if utf != 0 {
        GETCHAR!(c, ptr);
    } else {
        c = *ptr as u32;
    }

    if type_ == NLTYPE_ANYCRLF {
        match c {
            0x0a /* CHAR_LF */ => {
                *lenptr = 1;
                return TRUE;
            }

            0x0d /* CHAR_CR */ => {
                *lenptr = if ptr < endptr.wrapping_sub(1) && *ptr.add(1) == 0x0a /* CHAR_LF */ { 2 } else { 1 };
                return TRUE;
            }

            _ => {
                return FALSE;
            }
        }
    }

    /* NLTYPE_ANY */
    else {
        match c {
            0x0a /* CHAR_LF */ | 0x0b /* CHAR_VT */ | 0x0c /* CHAR_FF */ => {
                *lenptr = 1;
                return TRUE;
            }

            0x0d /* CHAR_CR */ => {
                *lenptr = if ptr < endptr.wrapping_sub(1) && *ptr.add(1) == 0x0a /* CHAR_LF */ { 2 } else { 1 };
                return TRUE;
            }

            /* PCRE2_CODE_UNIT_WIDTH == 8 */
            0x85 /* CHAR_NEL */ => {
                *lenptr = if utf != 0 { 2 } else { 1 };
                return TRUE;
            }

            0x2028 /* LS */ | 0x2029 /* PS */ => {
                *lenptr = 3;
                return TRUE;
            }

            _ => {
                return FALSE;
            }
        }
    }
}

/*************************************************
*     Check for newline at previous position     *
*************************************************/

/* This function is called only via the WAS_NEWLINE macro, which does so only
when the newline type is NLTYPE_ANY or NLTYPE_ANYCRLF. The case of a fixed
newline (NLTYPE_FIXED) is handled inline. It is guaranteed that the initial
value of ptr is greater than the start of the string that is being processed.

Arguments:
  ptr          pointer to possible newline
  type         the newline type
  startptr     pointer to the start of the string
  lenptr       where to return the length
  utf          TRUE if in utf mode

Returns:       TRUE or FALSE
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_was_newline_8(ptr: PCRE2_SPTR, type_: u32, startptr: PCRE2_SPTR, lenptr: *mut u32, utf: BOOL) -> BOOL {
    let mut c: u32;
    let mut ptr = ptr;
    ptr = ptr.wrapping_sub(1);

    /* SUPPORT_UNICODE */
    if utf != 0 {
        BACKCHAR!(ptr);
        GETCHAR!(c, ptr);
    } else {
        c = *ptr as u32;
    }

    if type_ == NLTYPE_ANYCRLF {
        match c {
            0x0a /* CHAR_LF */ => {
                *lenptr = if ptr > startptr && *ptr.offset(-1) == 0x0d /* CHAR_CR */ { 2 } else { 1 };
                return TRUE;
            }

            0x0d /* CHAR_CR */ => {
                *lenptr = 1;
                return TRUE;
            }

            _ => {
                return FALSE;
            }
        }
    }

    /* NLTYPE_ANY */
    else {
        match c {
            0x0a /* CHAR_LF */ => {
                *lenptr = if ptr > startptr && *ptr.offset(-1) == 0x0d /* CHAR_CR */ { 2 } else { 1 };
                return TRUE;
            }

            0x0b /* CHAR_VT */ | 0x0c /* CHAR_FF */ | 0x0d /* CHAR_CR */ => {
                *lenptr = 1;
                return TRUE;
            }

            /* PCRE2_CODE_UNIT_WIDTH == 8 */
            0x85 /* CHAR_NEL */ => {
                *lenptr = if utf != 0 { 2 } else { 1 };
                return TRUE;
            }

            0x2028 /* LS */ | 0x2029 /* PS */ => {
                *lenptr = 3;
                return TRUE;
            }

            _ => {
                return FALSE;
            }
        }
    }
}

/* End of pcre2_newline.c */
