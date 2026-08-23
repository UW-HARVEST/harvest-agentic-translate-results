// Translated from pcre2_ord2utf.c
use crate::internal::*;
use crate::tables::*;
use core::ffi::c_uint;

/*************************************************
*          Convert code point to UTF             *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ord2utf_8(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> c_uint {
    let mut cvalue = cvalue;
    let mut buffer = buffer;
    let mut i: c_uint = 0;
    while i < _pcre2_utf8_table1_size {
        if (cvalue as i32) <= _pcre2_utf8_table1[i as usize] {
            break;
        }
        i += 1;
    }
    buffer = buffer.add(i as usize);
    let mut j: c_uint = i;
    while j != 0 {
        *buffer = (0x80 | (cvalue & 0x3f)) as PCRE2_UCHAR;
        buffer = buffer.sub(1);
        cvalue >>= 6;
        j -= 1;
    }
    *buffer = (_pcre2_utf8_table2[i as usize] | (cvalue as i32)) as PCRE2_UCHAR;
    i + 1
}
