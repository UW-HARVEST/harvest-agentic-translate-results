//! Translation of `src/utf.c`.

use core::ffi::{c_char, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_encode(
    codepoint: i32,
    buffer: *mut c_char,
    size: *mut usize,
) -> c_int {
    unsafe {
        if codepoint < 0 {
            return -1;
        } else if codepoint < 0x80 {
            *buffer.add(0) = codepoint as c_char;
            *size = 1;
        } else if codepoint < 0x800 {
            *buffer.add(0) = (0xC0 + ((codepoint & 0x7C0) >> 6)) as c_char;
            *buffer.add(1) = (0x80 + (codepoint & 0x03F)) as c_char;
            *size = 2;
        } else if codepoint < 0x10000 {
            *buffer.add(0) = (0xE0 + ((codepoint & 0xF000) >> 12)) as c_char;
            *buffer.add(1) = (0x80 + ((codepoint & 0x0FC0) >> 6)) as c_char;
            *buffer.add(2) = (0x80 + (codepoint & 0x003F)) as c_char;
            *size = 3;
        } else if codepoint <= 0x10FFFF {
            *buffer.add(0) = (0xF0 + ((codepoint & 0x1C0000) >> 18)) as c_char;
            *buffer.add(1) = (0x80 + ((codepoint & 0x03F000) >> 12)) as c_char;
            *buffer.add(2) = (0x80 + ((codepoint & 0x000FC0) >> 6)) as c_char;
            *buffer.add(3) = (0x80 + (codepoint & 0x00003F)) as c_char;
            *size = 4;
        } else {
            return -1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn utf8_check_first(byte: c_char) -> usize {
    let u = byte as u8;

    if u < 0x80 {
        return 1;
    }

    if (0x80..=0xBF).contains(&u) {
        /* second, third or fourth byte of a multi-byte
        sequence, i.e. a "continuation byte" */
        0
    } else if u == 0xC0 || u == 0xC1 {
        /* overlong encoding of an ASCII byte */
        0
    } else if (0xC2..=0xDF).contains(&u) {
        /* 2-byte sequence */
        2
    } else if (0xE0..=0xEF).contains(&u) {
        /* 3-byte sequence */
        3
    } else if (0xF0..=0xF4).contains(&u) {
        /* 4-byte sequence */
        4
    } else {
        /* u >= 0xF5: restricted (start of 4-, 5- or 6-byte sequence) or
        invalid UTF-8 */
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_check_full(
    buffer: *const c_char,
    size: usize,
    codepoint: *mut i32,
) -> usize {
    unsafe {
        let mut value: i32;
        let mut u = *(buffer as *const u8);

        if size == 2 {
            value = (u & 0x1F) as i32;
        } else if size == 3 {
            value = (u & 0xF) as i32;
        } else if size == 4 {
            value = (u & 0x7) as i32;
        } else {
            return 0;
        }

        let mut i = 1usize;
        while i < size {
            u = *(buffer as *const u8).add(i);

            if u < 0x80 || u > 0xBF {
                /* not a continuation byte */
                return 0;
            }

            value = (value << 6) + (u & 0x3F) as i32;
            i += 1;
        }

        if value > 0x10FFFF {
            /* not in Unicode range */
            return 0;
        } else if (0xD800..=0xDFFF).contains(&value) {
            /* invalid code point (UTF-16 surrogate halves) */
            return 0;
        } else if (size == 2 && value < 0x80)
            || (size == 3 && value < 0x800)
            || (size == 4 && value < 0x10000)
        {
            /* overlong encoding */
            return 0;
        }

        if !codepoint.is_null() {
            *codepoint = value;
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_iterate(
    buffer: *const c_char,
    bufsize: usize,
    codepoint: *mut i32,
) -> *const c_char {
    unsafe {
        if bufsize == 0 {
            return buffer;
        }

        let count = utf8_check_first(*buffer);
        if count == 0 {
            return core::ptr::null();
        }

        let value: i32;
        if count == 1 {
            value = *(buffer as *const u8) as i32;
        } else {
            let mut v: i32 = 0;
            if count > bufsize || utf8_check_full(buffer, count, &mut v) == 0 {
                return core::ptr::null();
            }
            value = v;
        }

        if !codepoint.is_null() {
            *codepoint = value;
        }

        buffer.add(count)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_check_string(string: *const c_char, length: usize) -> c_int {
    unsafe {
        let mut i = 0usize;
        while i < length {
            let count = utf8_check_first(*string.add(i));
            if count == 0 {
                return 0;
            } else if count > 1 {
                if count > length - i {
                    return 0;
                }

                if utf8_check_full(string.add(i), count, core::ptr::null_mut()) == 0 {
                    return 0;
                }

                i += count - 1;
            }
            i += 1;
        }

        1
    }
}
