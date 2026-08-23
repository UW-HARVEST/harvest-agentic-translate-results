// Translated from c_src/src/pcre2_ord2utf.c
use crate::internal::*;

/* This file contains a function that converts a Unicode character code point
into a UTF string. The behaviour is different for each code unit width. */

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
pub unsafe extern "C" fn _pcre2_ord2utf_8(
    mut cvalue: u32,
    mut buffer: *mut PCRE2_UCHAR,
) -> c_uint {
    /* Convert to UTF-8 */

    let mut i: c_uint;

    i = 0;
    while i < _pcre2_utf8_table1_size {
        if (cvalue as c_int) <= *_pcre2_utf8_table1.as_ptr().add(i as usize) {
            break;
        }
        i = i.wrapping_add(1);
    }
    buffer = buffer.add(i as usize);
    let mut j: c_uint = i;
    while j != 0 {
        *buffer = (0x80 | (cvalue & 0x3f)) as PCRE2_UCHAR;
        buffer = buffer.sub(1);
        cvalue >>= 6;
        j = j.wrapping_sub(1);
    }
    *buffer = (*_pcre2_utf8_table2.as_ptr().add(i as usize) | (cvalue as c_int)) as PCRE2_UCHAR;
    i.wrapping_add(1)
}

/* End of pcre2_ord2utf.c */
