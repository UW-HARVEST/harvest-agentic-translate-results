//! Rust translation of `c_src/src/lib.c` (UTF-8 validation / filtering helpers).
//!
//! The C library exports exactly two public symbols:
//!   * `w_utf8_drop`
//!   * `w_utf8_filter`
//!
//! Both are reproduced here with identical signatures, identical validation
//! order and identical (bug-for-bug) allocation behaviour.  Buffers handed back
//! to the caller are allocated with the C allocator (`malloc`/`realloc`/
//! `strdup`) exactly like the original code, so the caller may `free()` them.

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings (declared locally so the crate has no external dependencies)
// ---------------------------------------------------------------------------

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_int,
        function: *const c_char,
    ) -> !;
}

/// `#define REPLACEMENT_INC 4096`
const REPLACEMENT_INC: usize = 4096;

/// `__FILE__` as the C compiler sees it.
///
/// CMake compiles `c_src/src/lib.c` with an absolute path, so glibc's
/// `__assert_fail` prints the absolute path of that file.  Deriving the same
/// string from `CARGO_MANIFEST_DIR` makes the abort message that reaches stderr
/// byte-identical to the C library's when both are built from one checkout.
const C_FILE: &[u8] = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/src/lib.c\0").as_bytes();

/// Reproduces `assert(string != NULL);` (the C library is compiled without
/// `NDEBUG`, so the assertion is live and aborts the process).
#[cold]
#[inline(never)]
fn assert_string_not_null(line: c_int, function: &[u8]) -> ! {
    unsafe {
        __assert_fail(
            b"string != NULL\0".as_ptr() as *const c_char,
            C_FILE.as_ptr() as *const c_char,
            line,
            function.as_ptr() as *const c_char,
        )
    }
}

// ---------------------------------------------------------------------------
// UTF-8 sequence validation macros
//
// NOTE: every one of these reads the trailing bytes of a candidate sequence
// lazily (C's `&&` short-circuits), which is what keeps the original code from
// reading past the NUL terminator.  Rust's `&&` short-circuits identically, so
// the read pattern is preserved byte for byte.
// ---------------------------------------------------------------------------

/// `valid_1(x)`: single byte -- `0xxxxxxx`
#[inline]
unsafe fn valid_1(x: *const u8) -> bool {
    (*x & 0x80) == 0
}

/// `valid_2(x)`: two bytes -- `110xxxxx 10xxxxxx`
///
/// Starting bytes 0xC0 and 0xC1 are forbidden (overlong).  The original test
/// `(x)[0] >= (char)0xC2` is a *signed* `char` comparison on the target ABI,
/// which is reproduced literally below.
#[inline]
unsafe fn valid_2(x: *const u8) -> bool {
    (*x & 0xE0) == 0xC0 && (*x as i8) >= (0xC2u8 as i8) && (*x.add(1) & 0xC0) == 0x80
}

/// `valid_3(x)`: three bytes -- `1110xxxx 10xxxxxx 10xxxxxx`
///
/// 0xE0 could start overlong encodings, and 0xED (range U+D800..U+DFFF) is
/// reserved for UTF-16 surrogate halves.  The final `0xEF` clause is a no-op in
/// practice (a continuation byte is always <= 0xBF) but is kept for fidelity.
#[inline]
unsafe fn valid_3(x: *const u8) -> bool {
    (*x & 0xF0) == 0xE0
        && (*x.add(1) & 0xC0) == 0x80
        && (*x.add(2) & 0xC0) == 0x80
        && (*x != 0xE0 || *x.add(1) >= 0xA0)
        && (*x != 0xED || *x.add(1) < 0xA0)
        && (*x != 0xEF || *x.add(1) <= 0xBF)
}

/// `valid_4(x)`: four bytes -- `11110xxx 10xxxxxx 10xxxxxx 10xxxxxx`
///
/// 0xF0 could start overlong encodings; start bytes 0xF5 and above are invalid.
#[inline]
unsafe fn valid_4(x: *const u8) -> bool {
    (*x & 0xF8) == 0xF0
        && *x <= 0xF4
        && (*x.add(1) & 0xC0) == 0x80
        && (*x.add(2) & 0xC0) == 0x80
        && (*x.add(3) & 0xC0) == 0x80
        && (*x != 0xF0 || *x.add(1) >= 0x90)
        && (*x != 0xF4 || *x.add(1) <= 0x8F)
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// Return pointer to the first character that does not match UTF-8, or the last
/// byte (0).
///
/// `const char * w_utf8_drop(const char * string);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    if string.is_null() {
        assert_string_not_null(40, b"w_utf8_drop\0");
    }

    let mut string = string as *const u8;

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
                return string as *const c_char;
            }
        }
    }

    string as *const c_char
}

/// `char * w_utf8_filter(const char * string, bool replacement);`
///
/// `replacement` is taken as a raw byte (C `_Bool` is a single byte with the
/// same ABI) and tested for non-zero, mirroring C's `if (replacement)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: u8) -> *mut c_char {
    if string.is_null() {
        assert_string_not_null(60, b"w_utf8_filter\0");
    }

    let replacement = replacement != 0;

    unsafe {
        let mut valid = w_utf8_drop(string) as *const u8;

        if *valid == b'\0' {
            let copy = strdup(string);
            return copy;
        }

        let mut size: usize = strlen(string) + 1;
        let mut i: usize = valid as usize - string as usize;
        let mut repl: usize = 0;

        let mut copy = malloc(size) as *mut u8;
        if copy.is_null() {
            return ptr::null_mut();
        }
        memcpy(copy as *mut c_void, string as *const c_void, i);

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
                        copy = realloc(copy as *mut c_void, size) as *mut u8;
                        if copy.is_null() {
                            return ptr::null_mut();
                        }
                        repl += REPLACEMENT_INC;
                    }

                    *copy.add(i) = 0xEF;
                    i += 1;
                    *copy.add(i) = 0xBF;
                    i += 1;
                    *copy.add(i) = 0xBD;
                    i += 1;
                    repl = repl.wrapping_sub(3);
                }

                valid = valid.add(1);
            }
        }

        *copy.add(i) = b'\0';
        copy as *mut c_char
    }
}
