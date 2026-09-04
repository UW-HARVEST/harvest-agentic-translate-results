//! Translated from pcre2_ord2utf.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/*************************************************
*          Convert code point to UTF             *
*************************************************/

/*
Arguments:
  cvalue     the character value
  buffer     pointer to buffer for result

Returns:     number of code units placed in the buffer
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ord2utf_8(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> u32 {
    /* Convert to UTF-8 */

    let mut cvalue = cvalue;
    let mut buffer = buffer;
    let mut i: u32;

    i = 0;
    while i < crate::tables::_pcre2_utf8_table1_size {
        if (cvalue as i32) <= *crate::tables::_pcre2_utf8_table1.as_ptr().add(i as usize) {
            break;
        }
        i += 1;
    }
    buffer = buffer.add(i as usize);
    let mut j: u32 = i;
    while j != 0 {
        *buffer = (0x80u32 | (cvalue & 0x3f)) as PCRE2_UCHAR;
        buffer = buffer.wrapping_sub(1);
        cvalue >>= 6;
        j -= 1;
    }
    *buffer = (*crate::tables::_pcre2_utf8_table2.as_ptr().add(i as usize) | (cvalue as i32)) as PCRE2_UCHAR;
    return i + 1;
}

/* End of pcre2_ord2utf.c */
