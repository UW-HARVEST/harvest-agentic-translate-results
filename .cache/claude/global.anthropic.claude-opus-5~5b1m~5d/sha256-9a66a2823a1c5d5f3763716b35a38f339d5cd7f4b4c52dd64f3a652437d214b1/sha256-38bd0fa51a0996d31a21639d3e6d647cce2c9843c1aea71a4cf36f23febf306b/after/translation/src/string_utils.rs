//! Translated from pcre2_string_utils.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/*************************************************
*    Compare two zero-terminated PCRE2 strings   *
*************************************************/

/*
Arguments:
  str1        first string
  str2        second string

Returns:      0, 1, or -1
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_8(mut str1: PCRE2_SPTR, mut str2: PCRE2_SPTR) -> i32 {
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while *str1 != 0 || *str2 != 0 {
        c1 = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        c2 = {
            let t = *str2;
            str2 = str2.add(1);
            t
        };
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
    }
    0
}

/*************************************************
*  Compare zero-terminated PCRE2 & 8-bit strings *
*************************************************/

/* As the 8-bit string is almost always a literal, its type is specified as
const char *.

Arguments:
  str1        first string
  str2        second string

Returns:      0, 1, or -1
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(mut str1: PCRE2_SPTR, mut str2: *const c_char) -> i32 {
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while *str1 != 0 || *str2 != 0 {
        c1 = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        c2 = {
            let t = *str2;
            str2 = str2.add(1);
            t
        } as PCRE2_UCHAR;
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
    }
    0
}

/*************************************************
*    Compare two PCRE2 strings, given a length   *
*************************************************/

/*
Arguments:
  str1        first string
  str2        second string
  len         the length

Returns:      0, 1, or -1
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_8(
    mut str1: PCRE2_SPTR,
    mut str2: PCRE2_SPTR,
    mut len: usize,
) -> i32 {
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while len > 0 {
        c1 = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        c2 = {
            let t = *str2;
            str2 = str2.add(1);
            t
        };
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
        len -= 1;
    }
    0
}

/*************************************************
* Compare PCRE2 string to 8-bit string by length *
*************************************************/

/* As the 8-bit string is almost always a literal, its type is specified as
const char *.

Arguments:
  str1        first string
  str2        second string
  len         the length

Returns:      0, 1, or -1
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_c8_8(
    mut str1: PCRE2_SPTR,
    mut str2: *const c_char,
    mut len: usize,
) -> i32 {
    let mut c1: PCRE2_UCHAR;
    let mut c2: PCRE2_UCHAR;
    while len > 0 {
        c1 = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        c2 = {
            let t = *str2;
            str2 = str2.add(1);
            t
        } as PCRE2_UCHAR;
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
        len -= 1;
    }
    0
}

/*************************************************
*        Find the length of a PCRE2 string       *
*************************************************/

/*
Argument:    the string
Returns:     the length
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(mut str: PCRE2_SPTR) -> PCRE2_SIZE {
    let mut c: PCRE2_SIZE = 0;
    while {
        let t = *str;
        str = str.add(1);
        t
    } != 0
    {
        c += 1;
    }
    c
}

/*************************************************
* Copy 8-bit 0-terminated string to PCRE2 string *
*************************************************/

/* Arguments:
  str1     buffer to receive the string
  str2     8-bit string to be copied

Returns:   the number of code units used (excluding trailing zero)
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(
    buffer: *mut PCRE2_UCHAR,
    mut str: *const c_char,
) -> PCRE2_SIZE {
    let str1: *mut PCRE2_UCHAR = buffer;
    let mut t: *mut PCRE2_UCHAR = str1;
    while *str != 0 {
        *t = {
            let v = *str;
            str = str.add(1);
            v
        } as PCRE2_UCHAR;
        t = t.add(1);
    }
    *t = 0;
    (t as usize - str1 as usize) as PCRE2_SIZE
}

