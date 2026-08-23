// Translated from c_src/src/pcre2_string_utils.c
use crate::internal::*;

/* This module contains internal functions for comparing and finding the length
of strings. These are used instead of strcmp() etc because the standard
functions work only on 8-bit data. */

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
pub unsafe extern "C" fn _pcre2_strcmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    while *str1 != 0 || *str2 != 0 {
        let c1: PCRE2_UCHAR = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        let c2: PCRE2_UCHAR = {
            let t = *str2;
            str2 = str2.add(1);
            t
        };
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
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
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    while *str1 != 0 || *str2 != 0 {
        let c1: PCRE2_UCHAR = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        let c2: PCRE2_UCHAR = {
            let t = *str2 as PCRE2_UCHAR;
            str2 = str2.add(1);
            t
        };
        if c1 != c2 {
            return (((c1 > c2) as c_int) << 1) - 1;
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
pub unsafe extern "C" fn _pcre2_strncmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR, len: usize) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    let mut len = len;
    while len > 0 {
        let c1: PCRE2_UCHAR = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        let c2: PCRE2_UCHAR = {
            let t = *str2;
            str2 = str2.add(1);
            t
        };
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
    str1: PCRE2_SPTR,
    str2: *const c_char,
    len: usize,
) -> c_int {
    let mut str1 = str1;
    let mut str2 = str2;
    let mut len = len;
    while len > 0 {
        let c1: PCRE2_UCHAR = {
            let t = *str1;
            str1 = str1.add(1);
            t
        };
        let c2: PCRE2_UCHAR = {
            let t = *str2 as PCRE2_UCHAR;
            str2 = str2.add(1);
            t
        };
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

/*
Argument:    the string
Returns:     the length
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(str: PCRE2_SPTR) -> PCRE2_SIZE {
    let mut str = str;
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
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(buffer: *mut PCRE2_UCHAR, vptr: *const c_char) -> PCRE2_SIZE {
    let str1: *mut PCRE2_UCHAR = buffer;
    let mut str2: *const c_char = vptr;
    let mut t: *mut PCRE2_UCHAR = str1;
    while *str2 != 0 {
        *t = *str2 as PCRE2_UCHAR;
        t = t.add(1);
        str2 = str2.add(1);
    }
    *t = 0;
    t.offset_from(str1) as PCRE2_SIZE
}

/* End of pcre2_string_utils.c */
