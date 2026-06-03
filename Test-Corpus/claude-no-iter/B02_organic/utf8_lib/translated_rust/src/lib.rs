// Translation of c_src/src/lib.c — must produce byte-identical output for
// the same inputs. The returned buffer is allocated via the C allocator so
// callers can free it with `free()`, matching the C library's contract.

use std::ffi::{c_char, c_void};
use std::ptr;

const REPLACEMENT_INC: usize = 4096;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/* Single byte: 0xxxxxxx */
#[inline]
unsafe fn valid_1(x: *const c_char) -> bool {
    let b0 = unsafe { *x } as u8;
    (b0 & 0x80) == 0
}

/* Two bytes: 110xxxxx 10xxxxxx */
/* Starting bytes 0xC0 and 0xC1 are forbidden (overlong) */
#[inline]
unsafe fn valid_2(x: *const c_char) -> bool {
    let b0 = unsafe { *x } as u8;
    if (b0 & 0xE0) != 0xC0 {
        return false;
    }
    if b0 < 0xC2 {
        return false;
    }
    let b1 = unsafe { *x.add(1) } as u8;
    (b1 & 0xC0) == 0x80
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx */
/* 0xE0 could start overlong encodings */
/* 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves */
#[inline]
unsafe fn valid_3(x: *const c_char) -> bool {
    let b0 = unsafe { *x } as u8;
    if (b0 & 0xF0) != 0xE0 {
        return false;
    }
    let b1 = unsafe { *x.add(1) } as u8;
    if (b1 & 0xC0) != 0x80 {
        return false;
    }
    let b2 = unsafe { *x.add(2) } as u8;
    if (b2 & 0xC0) != 0x80 {
        return false;
    }
    if b0 == 0xE0 && b1 < 0xA0 {
        return false;
    }
    if b0 == 0xED && b1 >= 0xA0 {
        return false;
    }
    // NOTE: original C check `(x[0] != 0xEF || x[1] <= 0xBF)` is always
    // satisfied because the earlier `(b1 & 0xC0) == 0x80` already
    // restricts b1 to 0x80..=0xBF. Preserved for byte-identical behavior.
    if b0 == 0xEF && b1 > 0xBF {
        return false;
    }
    true
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
/* 0xF0 could start overlong encodings */
/* Start bytes 0xF5 and above are invalid for UTF-8 */
#[inline]
unsafe fn valid_4(x: *const c_char) -> bool {
    let b0 = unsafe { *x } as u8;
    if (b0 & 0xF8) != 0xF0 {
        return false;
    }
    if b0 > 0xF4 {
        return false;
    }
    let b1 = unsafe { *x.add(1) } as u8;
    if (b1 & 0xC0) != 0x80 {
        return false;
    }
    let b2 = unsafe { *x.add(2) } as u8;
    if (b2 & 0xC0) != 0x80 {
        return false;
    }
    let b3 = unsafe { *x.add(3) } as u8;
    if (b3 & 0xC0) != 0x80 {
        return false;
    }
    if b0 == 0xF0 && b1 < 0x90 {
        return false;
    }
    if b0 == 0xF4 && b1 > 0x8F {
        return false;
    }
    true
}

/* Return pointer to the first character that does not match UTF-8, or the last byte (0) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    // assert(string != NULL);
    let mut s = string;
    unsafe {
        while *s != 0 {
            if valid_1(s) {
                s = s.add(1);
            } else if valid_2(s) {
                s = s.add(2);
            } else if valid_3(s) {
                s = s.add(3);
            } else if valid_4(s) {
                s = s.add(4);
            } else {
                return s;
            }
        }
    }
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    // assert(string != NULL);
    unsafe {
        let valid = w_utf8_drop(string);

        if *valid == 0 {
            return strdup(string);
        }

        let mut size = strlen(string) + 1;
        let mut i = (valid as usize) - (string as usize);
        let mut repl: usize = 0;

        let mut copy = malloc(size) as *mut c_char;
        if copy.is_null() {
            return ptr::null_mut();
        }
        memcpy(copy as *mut c_void, string as *const c_void, i);

        let mut v = valid;
        while *v != 0 {
            if valid_1(v) {
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
            } else if valid_2(v) {
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
            } else if valid_3(v) {
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
            } else if valid_4(v) {
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
                *copy.add(i) = *v;
                i += 1;
                v = v.add(1);
            } else {
                if replacement {
                    if repl < 3 {
                        size += REPLACEMENT_INC;
                        copy = realloc(copy as *mut c_void, size) as *mut c_char;
                        if copy.is_null() {
                            // NOTE: matches the C original — on realloc
                            // failure the previous buffer is leaked.
                            return ptr::null_mut();
                        }
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

                v = v.add(1);
            }
        }

        *copy.add(i) = 0;
        copy
    }
}
