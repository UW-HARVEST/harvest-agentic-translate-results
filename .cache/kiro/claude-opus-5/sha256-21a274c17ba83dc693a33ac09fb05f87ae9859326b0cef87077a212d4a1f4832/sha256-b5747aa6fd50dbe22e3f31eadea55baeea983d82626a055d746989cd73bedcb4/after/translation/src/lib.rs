//! Rust translation of `c_src/src/lib.c` (UTF-8 filtering helpers).
//!
//! The behaviour of the original C — including its quirks — is reproduced
//! exactly:
//!   * The returned buffers come from `malloc`, so callers may `free` them.
//!   * The odd `repl` bookkeeping used to decide when to `realloc` is kept
//!     byte-for-byte identical (it over-allocates, but never under-allocates).
//!   * `valid_3`'s `0xEF` clause is a tautology in the C (a continuation byte
//!     is always `<= 0xBF`); it is preserved anyway.
//!   * `assert(string != NULL)` maps to an `abort()`, matching a C build
//!     without `NDEBUG`.

use std::ffi::{c_char, c_void};

const REPLACEMENT_INC: usize = 4096;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn abort() -> !;
}

/* Single byte: 0xxxxxxx */
#[inline]
fn valid_1(s: &[u8], i: usize) -> bool {
    s[i] & 0x80 == 0
}

/* Two bytes: 110xxxxx 10xxxxxx */
/* Starting bytes 0xC0 and 0xC1 are forbidden (overlong) */
#[inline]
fn valid_2(s: &[u8], i: usize) -> bool {
    s[i] & 0xE0 == 0xC0 && s[i] >= 0xC2 && s[i + 1] & 0xC0 == 0x80
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx */
/* 0xE0 could start overlong encodings */
/* 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves */
#[inline]
fn valid_3(s: &[u8], i: usize) -> bool {
    s[i] & 0xF0 == 0xE0
        && s[i + 1] & 0xC0 == 0x80
        && s[i + 2] & 0xC0 == 0x80
        && (s[i] != 0xE0 || s[i + 1] >= 0xA0)
        && (s[i] != 0xED || s[i + 1] < 0xA0)
        && (s[i] != 0xEF || s[i + 1] <= 0xBF)
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
/* 0xF0 could start overlong encodings */
/* Start bytes 0xF5 and above are invalid for UTF-8 */
#[inline]
fn valid_4(s: &[u8], i: usize) -> bool {
    s[i] & 0xF8 == 0xF0
        && s[i] <= 0xF4
        && s[i + 1] & 0xC0 == 0x80
        && s[i + 2] & 0xC0 == 0x80
        && s[i + 3] & 0xC0 == 0x80
        && (s[i] != 0xF0 || s[i + 1] >= 0x90)
        && (s[i] != 0xF4 || s[i + 1] <= 0x8F)
}

/// Length of the NUL-terminated string at `string`, excluding the terminator.
unsafe fn c_strlen(string: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *string.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// Borrow the string as bytes *including* the trailing NUL.
///
/// Including the terminator mirrors the C macros, which may legally inspect
/// the NUL byte (short-circuit evaluation guarantees they never look past it).
unsafe fn c_bytes_with_nul<'a>(string: *const c_char) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(string as *const u8, c_strlen(string) + 1) }
}

/// Offset of the first byte that does not start a valid UTF-8 sequence, or the
/// offset of the terminating NUL.
fn utf8_drop_offset(s: &[u8]) -> usize {
    let mut i = 0usize;

    while s[i] != 0 {
        if valid_1(s, i) {
            i += 1;
        } else if valid_2(s, i) {
            i += 2;
        } else if valid_3(s, i) {
            i += 3;
        } else if valid_4(s, i) {
            i += 4;
        } else {
            return i;
        }
    }

    i
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    if string.is_null() {
        unsafe { abort() };
    }

    let s = unsafe { c_bytes_with_nul(string) };
    unsafe { string.add(utf8_drop_offset(s)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    if string.is_null() {
        unsafe { abort() };
    }

    let s = unsafe { c_bytes_with_nul(string) };
    let len = s.len() - 1;

    // const char * valid = w_utf8_drop(string);
    let mut valid = utf8_drop_offset(s);

    // if (*valid == '\0') -> plain strdup
    if s[valid] == 0 {
        let copy = unsafe { malloc(len + 1) } as *mut u8;
        if copy.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), copy, len + 1) };
        return copy as *mut c_char;
    }

    let mut size = len + 1;
    let mut i = valid;
    let mut repl: usize = 0;

    let mut copy = unsafe { malloc(size) } as *mut u8;
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), copy, i) };

    // Append a single byte to the output buffer at the current cursor.
    macro_rules! put {
        ($byte:expr) => {{
            unsafe { *copy.add(i) = $byte };
            i += 1;
        }};
    }

    while s[valid] != 0 {
        if valid_1(s, valid) {
            put!(s[valid]);
            valid += 1;
        } else if valid_2(s, valid) {
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
        } else if valid_3(s, valid) {
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
        } else if valid_4(s, valid) {
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
            put!(s[valid]);
            valid += 1;
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = unsafe { realloc(copy as *mut c_void, size) } as *mut u8;
                    if copy.is_null() {
                        return std::ptr::null_mut();
                    }
                    repl += REPLACEMENT_INC;
                }

                put!(0xEF);
                put!(0xBF);
                put!(0xBD);
                repl = repl.wrapping_sub(3);
            }

            valid += 1;
        }
    }

    unsafe { *copy.add(i) = 0 };
    copy as *mut c_char
}
