use crate::error_texts::{COMPILE_ERROR_TEXTS, MATCH_ERROR_TEXTS};
use crate::pcre2_internal::*;
use core::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_error_message_8(
    enumber: c_int,
    buffer: *mut PCRE2_UCHAR,
    size: PCRE2_SIZE,
) -> c_int {
    if size == 0 {
        return PCRE2_ERROR_NOMEMORY;
    }

    let (message_base, mut n): (*const u8, c_int) = if enumber >= COMPILE_ERROR_BASE {
        (COMPILE_ERROR_TEXTS.as_ptr(), enumber - COMPILE_ERROR_BASE)
    } else if enumber < 0 {
        (MATCH_ERROR_TEXTS.as_ptr(), -enumber)
    } else {
        (b"\0".as_ptr(), 1)
    };

    let mut message = message_base;
    while n > 0 {
        while *message != CHAR_NUL as u8 {
            message = message.add(1);
        }
        message = message.add(1);
        if *message == CHAR_NUL as u8 {
            return PCRE2_ERROR_BADDATA;
        }
        n -= 1;
    }

    let mut rc: c_int = 0;
    let mut i: PCRE2_SIZE = 0;
    while *message != 0 {
        if i >= size - 1 {
            rc = PCRE2_ERROR_NOMEMORY;
            break;
        }
        *buffer.add(i) = *message;
        message = message.add(1);
        i += 1;
    }

    *buffer.add(i) = 0;
    if rc != 0 {
        rc
    } else {
        i as c_int
    }
}
