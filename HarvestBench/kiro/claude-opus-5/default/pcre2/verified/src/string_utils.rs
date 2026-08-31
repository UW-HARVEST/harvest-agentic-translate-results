//! Translation of `c_src/src/pcre2_string_utils.c`.
//!
//! Internal functions for comparing and finding the length of strings. These
//! are used instead of `strcmp()` etc because the standard functions work only
//! on 8-bit data.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

use crate::internal::*;

/* Compare two zero-terminated PCRE2 strings.

Returns: 0, 1, or -1 */

pub unsafe fn strcmp(mut str1: PCRE2_SPTR, mut str2: PCRE2_SPTR) -> c_int {
    unsafe {
        while *str1 != b'\0' || *str2 != b'\0' {
            let c1 = *str1;
            str1 = str1.add(1);
            let c2 = *str2;
            str2 = str2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
        }
        0
    }
}

/// Exported as `_pcre2_strcmp_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> c_int {
    unsafe { strcmp(str1, str2) }
}

/* Compare a zero-terminated PCRE2 string with an 8-bit string. As the 8-bit
string is almost always a literal, its type is specified as const char *.

Returns: 0, 1, or -1 */

pub unsafe fn strcmp_c8(mut str1: PCRE2_SPTR, mut str2: *const c_char) -> c_int {
    unsafe {
        while *str1 != b'\0' || *str2 != 0 {
            let c1 = *str1;
            str1 = str1.add(1);
            let c2 = *str2 as u8;
            str2 = str2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
        }
        0
    }
}

/// Exported as `_pcre2_strcmp_c8_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char) -> c_int {
    unsafe { strcmp_c8(str1, str2) }
}

/* Compare two PCRE2 strings, given a length.

Returns: 0, 1, or -1 */

pub unsafe fn strncmp(mut str1: PCRE2_SPTR, mut str2: PCRE2_SPTR, mut len: usize) -> c_int {
    unsafe {
        while len > 0 {
            let c1 = *str1;
            str1 = str1.add(1);
            let c2 = *str2;
            str2 = str2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
            len -= 1;
        }
        0
    }
}

/// Exported as `_pcre2_strncmp_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_8(
    str1: PCRE2_SPTR,
    str2: PCRE2_SPTR,
    len: usize,
) -> c_int {
    unsafe { strncmp(str1, str2, len) }
}

/* Compare a PCRE2 string to an 8-bit string by length. As the 8-bit string is
almost always a literal, its type is specified as const char *.

Returns: 0, 1, or -1 */

pub unsafe fn strncmp_c8(mut str1: PCRE2_SPTR, mut str2: *const c_char, mut len: usize) -> c_int {
    unsafe {
        while len > 0 {
            let c1 = *str1;
            str1 = str1.add(1);
            let c2 = *str2 as u8;
            str2 = str2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
            len -= 1;
        }
        0
    }
}

/// Exported as `_pcre2_strncmp_c8_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_c8_8(
    str1: PCRE2_SPTR,
    str2: *const c_char,
    len: usize,
) -> c_int {
    unsafe { strncmp_c8(str1, str2, len) }
}

/* Find the length of a PCRE2 string.

Returns: the length */

pub unsafe fn strlen(mut str: PCRE2_SPTR) -> PCRE2_SIZE {
    unsafe {
        let mut c: PCRE2_SIZE = 0;
        while *str != 0 {
            str = str.add(1);
            c += 1;
        }
        c
    }
}

/// Exported as `_pcre2_strlen_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(str: PCRE2_SPTR) -> PCRE2_SIZE {
    unsafe { strlen(str) }
}

/* Copy an 8-bit zero-terminated string to a PCRE2 string.

Returns: the number of code units used (excluding trailing zero) */

pub unsafe fn strcpy_c8(str1: *mut PCRE2_UCHAR, mut str2: *const c_char) -> PCRE2_SIZE {
    unsafe {
        let mut t = str1;
        while *str2 != 0 {
            *t = *str2 as u8;
            t = t.add(1);
            str2 = str2.add(1);
        }
        *t = 0;
        t.offset_from(str1) as PCRE2_SIZE
    }
}

/// Exported as `_pcre2_strcpy_c8_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(
    str1: *mut PCRE2_UCHAR,
    str2: *const c_char,
) -> PCRE2_SIZE {
    unsafe { strcpy_c8(str1, str2) }
}
