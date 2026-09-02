//! Rust translation of `c_src/src/lib.c` (UTF-8 validation / filtering).
//!
//! The original C library exports exactly two public symbols:
//!   * `w_utf8_drop`   — declared only in the .c file, but non-static, so exported.
//!   * `w_utf8_filter` — declared in `include/lib.h`.
//!
//! Buffers handed back to the caller are allocated with the C allocator
//! (`malloc`/`realloc`/`strdup`) because callers of the C library free them with
//! `free()`. Behaviour — including the original's quirks — is reproduced exactly.

use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// `#define REPLACEMENT_INC 4096`
const REPLACEMENT_INC: usize = 4096;

/* Single byte: 0xxxxxxx */
#[inline]
unsafe fn valid_1(x: *const u8) -> bool {
    (unsafe { *x } & 0x80) == 0
}

/* Two bytes: 110xxxxx 10xxxxxx
 * Starting bytes 0xC0 and 0xC1 are forbidden (overlong).
 * NOTE: the C source compares `(x)[0] >= (char)0xC2` using a *signed* char,
 * which is reproduced here with an `i8` comparison. */
#[inline]
unsafe fn valid_2(x: *const u8) -> bool {
    unsafe {
        (*x & 0xE0) == 0xC0
            && (*x as i8) >= (0xC2_u8 as i8)
            && (*x.add(1) & 0xC0) == 0x80
    }
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
 * 0xE0 could start overlong encodings.
 * 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves. */
#[inline]
unsafe fn valid_3(x: *const u8) -> bool {
    unsafe {
        (*x & 0xF0) == 0xE0
            && (*x.add(1) & 0xC0) == 0x80
            && (*x.add(2) & 0xC0) == 0x80
            && (*x != 0xE0 || *x.add(1) >= 0xA0)
            && (*x != 0xED || *x.add(1) < 0xA0)
            && (*x != 0xEF || *x.add(1) <= 0xBF)
    }
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
 * 0xF0 could start overlong encodings.
 * Start bytes 0xF5 and above are invalid for UTF-8. */
#[inline]
unsafe fn valid_4(x: *const u8) -> bool {
    unsafe {
        (*x & 0xF8) == 0xF0
            && *x <= 0xF4
            && (*x.add(1) & 0xC0) == 0x80
            && (*x.add(2) & 0xC0) == 0x80
            && (*x.add(3) & 0xC0) == 0x80
            && (*x != 0xF0 || *x.add(1) >= 0x90)
            && (*x != 0xF4 || *x.add(1) <= 0x8F)
    }
}

/// Return pointer to the first character that does not match UTF-8, or the last
/// byte (0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    debug_assert!(!string.is_null());

    let mut p = string as *const u8;

    unsafe {
        while *p != 0 {
            if valid_1(p) {
                p = p.add(1);
            } else if valid_2(p) {
                p = p.add(2);
            } else if valid_3(p) {
                p = p.add(3);
            } else if valid_4(p) {
                p = p.add(4);
            } else {
                return p as *const c_char;
            }
        }
    }

    p as *const c_char
}

/// Return a newly allocated copy of `string` with every byte that is not part of
/// a valid UTF-8 sequence dropped, or (when `replacement` is true) substituted
/// with U+FFFD (EF BF BD).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    debug_assert!(!string.is_null());

    let valid_start = unsafe { w_utf8_drop(string) };

    if unsafe { *valid_start } == 0 {
        return unsafe { strdup(string) };
    }

    let mut size = unsafe { strlen(string) } + 1;
    let mut i: usize = valid_start as usize - string as usize;
    let mut repl: usize = 0;

    let mut copy = unsafe { malloc(size) } as *mut u8;
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { memcpy(copy as *mut c_void, string as *const c_void, i) };

    let mut valid = valid_start as *const u8;

    unsafe {
        while *valid != 0 {
            if valid_1(valid) {
                *copy.add(i) = *valid;
                i += 1;
                valid = valid.add(1);
            } else if valid_2(valid) {
                for _ in 0..2 {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            } else if valid_3(valid) {
                for _ in 0..3 {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            } else if valid_4(valid) {
                for _ in 0..4 {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            } else {
                if replacement {
                    if repl < 3 {
                        size += REPLACEMENT_INC;
                        copy = realloc(copy as *mut c_void, size) as *mut u8;
                        if copy.is_null() {
                            return std::ptr::null_mut();
                        }
                        repl += REPLACEMENT_INC;
                    }

                    *copy.add(i) = 0xEF;
                    i += 1;
                    *copy.add(i) = 0xBF;
                    i += 1;
                    *copy.add(i) = 0xBD;
                    i += 1;
                    repl -= 3;
                }

                valid = valid.add(1);
            }
        }

        *copy.add(i) = 0;
    }

    copy as *mut c_char
}
