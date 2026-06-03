use libc::{c_char, malloc, realloc, size_t, strlen};
use std::ptr;

const REPLACEMENT_INC: usize = 4096;

/// Single byte: 0xxxxxxx
#[inline]
unsafe fn valid_1(x: *const c_char) -> bool {
    (*x as u8) & 0x80 == 0
}

/// Two bytes: 110xxxxx 10xxxxxx
/// Starting bytes 0xC0 and 0xC1 are forbidden (overlong)
#[inline]
unsafe fn valid_2(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    (b0 & 0xE0) == 0xC0 && b0 >= 0xC2 && (b1 & 0xC0) == 0x80
}

/// Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
/// 0xE0 could start overlong encodings
/// 0xED (range U+D800–U+DFFF) is reserved for UTF-16 surrogate halves
#[inline]
unsafe fn valid_3(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    let b2 = *x.add(2) as u8;
    (b0 & 0xF0) == 0xE0
        && (b1 & 0xC0) == 0x80
        && (b2 & 0xC0) == 0x80
        && (b0 != 0xE0 || b1 >= 0xA0)
        && (b0 != 0xED || b1 < 0xA0)
        && (b0 != 0xEF || b1 <= 0xBF)
}

/// Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
/// 0xF0 could start overlong encodings
/// Start bytes 0xF5 and above are invalid for UTF-8
#[inline]
unsafe fn valid_4(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    let b2 = *x.add(2) as u8;
    let b3 = *x.add(3) as u8;
    (b0 & 0xF8) == 0xF0
        && b0 <= 0xF4
        && (b1 & 0xC0) == 0x80
        && (b2 & 0xC0) == 0x80
        && (b3 & 0xC0) == 0x80
        && (b0 != 0xF0 || b1 >= 0x90)
        && (b0 != 0xF4 || b1 <= 0x8F)
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0)
unsafe fn w_utf8_drop(mut string: *const c_char) -> *const c_char {
    assert!(!string.is_null());

    while *string != 0 {
        if valid_1(string) {
            string = string.add(1);
        } else if valid_2(string) {
            string = string.add(2);
        } else if valid_3(string) {
            string = string.add(3);
        } else if valid_4(string) {
            string = string.add(4);
        } else {
            return string;
        }
    }

    string
}

/// Strdup-like helper using libc malloc so callers can free() the result.
unsafe fn c_strdup(string: *const c_char) -> *mut c_char {
    let len = strlen(string);
    let size = len + 1;
    let copy = malloc(size) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(string, copy, size);
    copy
}

#[no_mangle]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    assert!(!string.is_null());

    let mut valid = w_utf8_drop(string);

    if *valid == 0 {
        return c_strdup(string);
    }

    let mut size: size_t = strlen(string) + 1;
    let mut i: usize = valid.offset_from(string) as usize;
    let mut repl: usize = 0;

    let mut copy = malloc(size) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(string, copy, i);

    while *valid != 0 {
        if valid_1(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_2(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_3(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_4(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    let new_copy = realloc(copy as *mut libc::c_void, size) as *mut c_char;
                    if new_copy.is_null() {
                        return ptr::null_mut();
                    }
                    copy = new_copy;
                    repl += REPLACEMENT_INC;
                }

                *copy.add(i) = 0xEFu8 as c_char;
                i += 1;
                *copy.add(i) = 0xBFu8 as c_char;
                i += 1;
                *copy.add(i) = 0xBDu8 as c_char;
                i += 1;
                repl -= 3;
            }

            valid = valid.add(1);
        }
    }

    *copy.add(i) = 0;
    copy
}
