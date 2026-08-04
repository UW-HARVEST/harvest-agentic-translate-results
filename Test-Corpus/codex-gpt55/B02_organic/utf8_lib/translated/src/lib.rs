use std::ffi::{c_char, c_void};
use std::ptr;

const REPLACEMENT_INC: usize = 4096;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
}

#[inline]
unsafe fn byte_at(string: *const c_char, offset: usize) -> u8 {
    unsafe { *string.add(offset).cast::<u8>() }
}

#[inline]
unsafe fn char_at(string: *const c_char, offset: usize) -> c_char {
    unsafe { *string.add(offset) }
}

#[inline]
fn c(value: u8) -> c_char {
    value as c_char
}

#[inline]
unsafe fn valid_1(string: *const c_char) -> bool {
    unsafe { (byte_at(string, 0) & 0x80) == 0 }
}

#[inline]
unsafe fn valid_2(string: *const c_char) -> bool {
    unsafe {
        (byte_at(string, 0) & 0xE0) == 0xC0
            && char_at(string, 0) >= c(0xC2)
            && (byte_at(string, 1) & 0xC0) == 0x80
    }
}

#[inline]
unsafe fn valid_3(string: *const c_char) -> bool {
    unsafe {
        (byte_at(string, 0) & 0xF0) == 0xE0
            && (byte_at(string, 1) & 0xC0) == 0x80
            && (byte_at(string, 2) & 0xC0) == 0x80
            && (char_at(string, 0) != c(0xE0) || byte_at(string, 1) >= 0xA0)
            && (char_at(string, 0) != c(0xED) || byte_at(string, 1) < 0xA0)
            && (char_at(string, 0) != c(0xEF) || byte_at(string, 1) <= 0xBF)
    }
}

#[inline]
unsafe fn valid_4(string: *const c_char) -> bool {
    unsafe {
        (byte_at(string, 0) & 0xF8) == 0xF0
            && byte_at(string, 0) <= 0xF4
            && (byte_at(string, 1) & 0xC0) == 0x80
            && (byte_at(string, 2) & 0xC0) == 0x80
            && (byte_at(string, 3) & 0xC0) == 0x80
            && (char_at(string, 0) != c(0xF0) || byte_at(string, 1) >= 0x90)
            && (char_at(string, 0) != c(0xF4) || byte_at(string, 1) <= 0x8F)
    }
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(mut string: *const c_char) -> *const c_char {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    unsafe {
        let mut valid = w_utf8_drop(string);

        if *valid == 0 {
            return strdup(string);
        }

        let mut size = strlen(string).wrapping_add(1);
        let mut i = valid.offset_from(string) as usize;
        let mut repl = 0usize;

        let mut copy = malloc(size).cast::<c_char>();
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
                        size = size.wrapping_add(REPLACEMENT_INC);
                        copy = realloc(copy.cast::<c_void>(), size).cast::<c_char>();
                        if copy.is_null() {
                            return ptr::null_mut();
                        }
                        repl = repl.wrapping_add(REPLACEMENT_INC);
                    }

                    *copy.add(i) = c(0xEF);
                    i += 1;
                    *copy.add(i) = c(0xBF);
                    i += 1;
                    *copy.add(i) = c(0xBD);
                    i += 1;
                    repl = repl.wrapping_sub(3);
                }

                valid = valid.add(1);
            }
        }

        *copy.add(i) = 0;
        copy
    }
}
