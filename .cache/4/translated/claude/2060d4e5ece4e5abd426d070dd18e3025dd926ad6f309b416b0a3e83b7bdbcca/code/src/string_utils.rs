// Translated from pcre2_string_utils.c
use crate::internal::*;
use core::ffi::{c_char, c_int};

/*************************************************
*    Compare two zero-terminated PCRE2 strings   *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while *str1 != 0 || *str2 != 0 {
        c1 = *str1;
        str1 = str1.add(1);
        c2 = *str2;
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
        }
    }
    0
}

/*************************************************
*  Compare zero-terminated PCRE2 & 8-bit strings *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2 as *const u8;
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while *str1 != 0 || *str2 != 0 {
        c1 = *str1;
        str1 = str1.add(1);
        c2 = *str2;
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
        }
    }
    0
}

/*************************************************
*    Compare two PCRE2 strings, given a length   *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_8(
    str1: PCRE2_SPTR,
    str2: PCRE2_SPTR,
    len: usize,
) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    let mut len = len;
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while len > 0 {
        c1 = *str1;
        str1 = str1.add(1);
        c2 = *str2;
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
        }
        len -= 1;
    }
    0
}

/*************************************************
* Compare PCRE2 string to 8-bit string by length *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_c8_8(
    str1: PCRE2_SPTR,
    str2: *const c_char,
    len: usize,
) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2 as *const u8;
    let mut len = len;
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while len > 0 {
        c1 = *str1;
        str1 = str1.add(1);
        c2 = *str2;
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
        }
        len -= 1;
    }
    0
}

/*************************************************
*        Find the length of a PCRE2 string       *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(str: PCRE2_SPTR) -> PCRE2_SIZE {
    let mut str = str;
    let mut c: PCRE2_SIZE = 0;
    loop {
        let v = *str;
        str = str.add(1);
        if v == 0 {
            break;
        }
        c += 1;
    }
    c
}

/*************************************************
* Copy 8-bit 0-terminated string to PCRE2 string *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(
    str1: *mut PCRE2_UCHAR,
    str2: *const c_char,
) -> PCRE2_SIZE {
    let mut str2 = str2 as *const u8;
    let mut t: *mut PCRE2_UCHAR = str1;
    while *str2 != 0 {
        *t = *str2;
        t = t.add(1);
        str2 = str2.add(1);
    }
    *t = 0;
    t.offset_from(str1) as PCRE2_SIZE
}
