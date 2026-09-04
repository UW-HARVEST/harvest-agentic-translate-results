//! Translation of `pcre2_string_utils.c`.

use crate::internal::*;
use core::ffi::{c_char, c_int};

/// `PRIV(strcmp)` — compare two zero-terminated PCRE2 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> c_int {
    unsafe {
        let mut s1 = str1;
        let mut s2 = str2;
        while *s1 != 0 || *s2 != 0 {
            let c1 = *s1;
            s1 = s1.add(1);
            let c2 = *s2;
            s2 = s2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
        }
        0
    }
}

/// `PRIV(strcmp_c8)` — compare a PCRE2 string with an 8-bit C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char) -> c_int {
    unsafe {
        let mut s1 = str1;
        let mut s2 = str2;
        while *s1 != 0 || *s2 != 0 {
            let c1 = *s1;
            s1 = s1.add(1);
            // The C code assigns `*str2++` (a char) to a PCRE2_UCHAR (uint8_t).
            let c2 = *s2 as u8;
            s2 = s2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
        }
        0
    }
}

/// `PRIV(strncmp)` — compare two PCRE2 strings for a given length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_8(
    str1: PCRE2_SPTR,
    str2: PCRE2_SPTR,
    len: usize,
) -> c_int {
    unsafe {
        let mut s1 = str1;
        let mut s2 = str2;
        let mut len = len;
        while len > 0 {
            let c1 = *s1;
            s1 = s1.add(1);
            let c2 = *s2;
            s2 = s2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
            len -= 1;
        }
        0
    }
}

/// `PRIV(strncmp_c8)` — compare a PCRE2 string with an 8-bit C string by length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_c8_8(
    str1: PCRE2_SPTR,
    str2: *const c_char,
    len: usize,
) -> c_int {
    unsafe {
        let mut s1 = str1;
        let mut s2 = str2;
        let mut len = len;
        while len > 0 {
            let c1 = *s1;
            s1 = s1.add(1);
            let c2 = *s2 as u8;
            s2 = s2.add(1);
            if c1 != c2 {
                return (((c1 > c2) as c_int) << 1) - 1;
            }
            len -= 1;
        }
        0
    }
}

/// `PRIV(strlen)` — length of a zero-terminated PCRE2 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(str: PCRE2_SPTR) -> PCRE2_SIZE {
    unsafe {
        let mut p = str;
        let mut c: PCRE2_SIZE = 0;
        while *p != 0 {
            p = p.add(1);
            c += 1;
        }
        c
    }
}

/// `PRIV(strcpy_c8)` — copy an 8-bit zero-terminated string into a PCRE2 string.
///
/// Returns the number of code units used, excluding the trailing zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(
    str1: *mut PCRE2_UCHAR,
    str2: *const c_char,
) -> PCRE2_SIZE {
    unsafe {
        let mut t = str1;
        let mut s = str2;
        while *s != 0 {
            *t = *s as u8;
            t = t.add(1);
            s = s.add(1);
        }
        *t = 0;
        t.offset_from(str1) as PCRE2_SIZE
    }
}
