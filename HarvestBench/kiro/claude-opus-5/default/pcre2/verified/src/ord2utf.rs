//! Translation of `pcre2_ord2utf.c` (8-bit / UTF-8).

use crate::internal::*;
use crate::tables;
use core::ffi::c_uint;

/// `PRIV(ord2utf)` — convert a code point to UTF-8.
///
/// Returns the number of code units placed in the buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ord2utf_8(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> c_uint {
    unsafe {
        let mut cvalue = cvalue;
        let mut i: usize = 0;
        while i < tables::_pcre2_utf8_table1_size as usize {
            if (cvalue as i32) <= tables::_pcre2_utf8_table1[i] {
                break;
            }
            i += 1;
        }

        let mut p = buffer.add(i);
        let mut j = i;
        while j != 0 {
            *p = (0x80 | (cvalue & 0x3f)) as PCRE2_UCHAR;
            p = p.sub(1);
            cvalue >>= 6;
            j -= 1;
        }
        *p = (tables::_pcre2_utf8_table2[i] | cvalue as i32) as PCRE2_UCHAR;
        (i + 1) as c_uint
    }
}
