//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C `libdriver.so`):
//!   * `w_utf8_drop`
//!   * `w_utf8_filter`
//!
//! The translation reproduces the original semantics byte for byte, including
//! the exact validation order of the `valid_1` .. `valid_4` macros, the lazy
//! (short-circuit) reads of continuation bytes, the malloc/realloc/strdup
//! allocation behaviour (so callers may `free()` the result), and the original
//! (slightly odd but non-overflowing) `repl` growth bookkeeping.

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_uchar, c_uint, c_void};

/// `#define REPLACEMENT_INC 4096`
const REPLACEMENT_INC: usize = 4096;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// Reproduces `assert(string != NULL)` from `<assert.h>` (asserts are enabled:
/// the CMake build does not define `NDEBUG`).
#[cold]
#[inline(never)]
fn assert_fail_string_not_null(line: c_uint, function: &'static core::ffi::CStr) -> ! {
    unsafe {
        __assert_fail(
            c"string != NULL".as_ptr(),
            c"c_src/src/lib.c".as_ptr(),
            line,
            function.as_ptr(),
        )
    }
}

/// Read the byte at `p[i]`.
#[inline(always)]
unsafe fn at(p: *const c_char, i: usize) -> u8 {
    unsafe { *p.add(i) as u8 }
}

/* Single byte: 0xxxxxxx */
/* `#define valid_1(x) (((x)[0] & 0x80) == 0)` */
#[inline(always)]
unsafe fn valid_1(x: *const c_char) -> bool {
    unsafe { at(x, 0) & 0x80 == 0 }
}

/* Two bytes: 110xxxxx 10xxxxxx */
/* Starting bytes 0xC0 and 0xC1 are forbidden (overlong) */
/* Note: `(x)[0] >= (char)0xC2` is a *signed* char comparison in the C. */
#[inline(always)]
unsafe fn valid_2(x: *const c_char) -> bool {
    unsafe {
        let b0 = at(x, 0);
        (b0 & 0xE0) == 0xC0 && (b0 as i8) >= (0xC2u8 as i8) && (at(x, 1) & 0xC0) == 0x80
    }
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx */
/* 0xE0 could start overlong encodings */
/* 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves */
#[inline(always)]
unsafe fn valid_3(x: *const c_char) -> bool {
    unsafe {
        let b0 = at(x, 0);
        if (b0 & 0xF0) != 0xE0 {
            return false;
        }
        let b1 = at(x, 1);
        (b1 & 0xC0) == 0x80
            && (at(x, 2) & 0xC0) == 0x80
            && (b0 != 0xE0 || b1 >= 0xA0)
            && (b0 != 0xED || b1 < 0xA0)
            && (b0 != 0xEF || b1 <= 0xBF)
    }
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
/* 0xF0 could start overlong encodings */
/* Start bytes 0xF5 and above are invalid for UTF-8 */
#[inline(always)]
unsafe fn valid_4(x: *const c_char) -> bool {
    unsafe {
        let b0 = at(x, 0);
        if (b0 & 0xF8) != 0xF0 || b0 > 0xF4 {
            return false;
        }
        let b1 = at(x, 1);
        (b1 & 0xC0) == 0x80
            && (at(x, 2) & 0xC0) == 0x80
            && (at(x, 3) & 0xC0) == 0x80
            && (b0 != 0xF0 || b1 >= 0x90)
            && (b0 != 0xF4 || b1 <= 0x8F)
    }
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    if string.is_null() {
        assert_fail_string_not_null(40, c"w_utf8_drop");
    }

    let mut string = string;

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
    }

    string
}

/// `char * w_utf8_filter(const char * string, bool replacement);`
///
/// `replacement` is declared `bool` (`_Bool`) in C; it is passed in the low byte
/// of a register and the C code tests it with `cmpb $0, ...`, i.e. any non-zero
/// byte is true. Taking it as `c_uchar` here keeps the ABI identical while
/// avoiding UB for non-canonical values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: c_uchar) -> *mut c_char {
    if string.is_null() {
        assert_fail_string_not_null(60, c"w_utf8_filter");
    }

    let replacement = replacement != 0;

    unsafe {
        let mut valid: *const c_char = w_utf8_drop(string);

        if *valid == 0 {
            let copy: *mut c_char = strdup(string);
            return copy;
        }

        let mut size: usize = strlen(string) + 1;
        let mut copy: *mut c_char;
        let mut i: usize = valid.offset_from(string) as usize;
        let mut repl: usize = 0;

        copy = malloc(size).cast::<c_char>();
        if copy.is_null() {
            return core::ptr::null_mut();
        }
        memcpy(copy.cast::<c_void>(), string.cast::<c_void>(), i);

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
                        copy = realloc(copy.cast::<c_void>(), size).cast::<c_char>();
                        if copy.is_null() {
                            return core::ptr::null_mut();
                        }
                        repl += REPLACEMENT_INC;
                    }

                    *copy.add(i) = 0xEFu8 as c_char;
                    i += 1;
                    *copy.add(i) = 0xBFu8 as c_char;
                    i += 1;
                    *copy.add(i) = 0xBDu8 as c_char;
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
