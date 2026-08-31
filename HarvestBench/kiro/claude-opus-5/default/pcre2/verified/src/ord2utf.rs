//! Translation of `c_src/src/pcre2_ord2utf.c`.
//!
//! Converts a Unicode character code point into a UTF string. Under the build
//! configuration (`PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE`) this is the
//! UTF-8 encoder.

#![allow(non_snake_case)]

use core::ffi::c_int;

use crate::internal::*;

/* Convert code point to UTF.

Arguments:
  cvalue     the character value
  buffer     pointer to buffer for result

Returns:     number of code units placed in the buffer */

pub unsafe fn ord2utf(mut cvalue: u32, buffer: *mut PCRE2_UCHAR) -> u32 {
    unsafe {
        /* Convert to UTF-8 */
        let mut i: u32 = 0;
        while i < UTF8_TABLE1_SIZE {
            if (cvalue as c_int) <= UTF8_TABLE1[i as usize] {
                break;
            }
            i += 1;
        }
        let mut buffer = buffer.add(i as usize);
        let mut j = i;
        while j != 0 {
            *buffer = 0x80 | (cvalue & 0x3f) as u8;
            buffer = buffer.sub(1);
            cvalue >>= 6;
            j -= 1;
        }
        *buffer = (UTF8_TABLE2[i as usize] | cvalue as c_int) as PCRE2_UCHAR;
        i + 1
    }
}

/// Exported as `_pcre2_ord2utf_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ord2utf_8(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> u32 {
    unsafe { ord2utf(cvalue, buffer) }
}
