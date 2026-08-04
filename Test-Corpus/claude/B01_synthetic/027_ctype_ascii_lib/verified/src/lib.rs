// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of c_src/src/driver.c
//
// To produce byte-identical output to the original C program, this translation
// has to match what glibc's <ctype.h> macros do, NOT what the externally
// linked ctype function symbols do. The macros bypass the function calls and
// instead index into the per-locale ctype tables exposed by glibc through
// `__ctype_b_loc()`, `__ctype_tolower_loc()`, and `__ctype_toupper_loc()`.
//
// As a result, e.g. `isdigit('0')` from the macro returns the `_ISdigit` bit
// (2048) rather than 1 — and that is what the C driver prints. We replicate
// that behavior here.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_ushort;

// LC_ALL is defined in glibc's <locale.h>. On Linux/glibc its value is 6.
const LC_ALL: c_int = 6;

// glibc <ctype.h> bitmask constants. With little-endian (x86_64), _ISbit(b)
// is `(1 << b) << 8` for b<8 and `(1 << b) >> 8` for b>=8 — but the table
// returned by __ctype_b_loc is an array of `unsigned short`, so the values
// are simply the bit positions in that 16-bit word:
const _IS_UPPER: c_ushort = 1 << 8; // 0x0100
const _IS_LOWER: c_ushort = 1 << 9; // 0x0200
const _IS_ALPHA: c_ushort = 1 << 10; // 0x0400
const _IS_DIGIT: c_ushort = 1 << 11; // 0x0800
const _IS_XDIGIT: c_ushort = 1 << 12; // 0x1000
const _IS_SPACE: c_ushort = 1 << 13; // 0x2000
const _IS_PRINT: c_ushort = 1 << 14; // 0x4000
const _IS_GRAPH: c_ushort = 1 << 15; // 0x8000
const _IS_BLANK: c_ushort = 1 << 0; // 0x0001
const _IS_CNTRL: c_ushort = 1 << 1; // 0x0002
const _IS_PUNCT: c_ushort = 1 << 2; // 0x0004
const _IS_ALNUM: c_ushort = 1 << 3; // 0x0008

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    // glibc-internal accessors used by the <ctype.h> macros.
    fn __ctype_b_loc() -> *mut *const c_ushort;
    fn __ctype_tolower_loc() -> *mut *const c_int;
    fn __ctype_toupper_loc() -> *mut *const c_int;

    fn printf(fmt: *const c_char, ...) -> c_int;
}

// Replicate `(*__ctype_b_loc())[c] & mask`. The table is indexed by the
// character value treated as an int. The pointer is offset by 128 inside
// glibc so that EOF (-1) and other negative chars in [-128, -1] are valid.
unsafe fn ctype_b_lookup(c: c_int) -> c_ushort {
    unsafe {
        let table = *__ctype_b_loc();
        // The table[-128 .. 256] is valid; indexing with `c` works because
        // glibc's __ctype_b_loc returns a pointer already offset by 128.
        *table.offset(c as isize)
    }
}

unsafe fn ctype_tolower_lookup(c: c_int) -> c_int {
    unsafe {
        let table = *__ctype_tolower_loc();
        *table.offset(c as isize)
    }
}

unsafe fn ctype_toupper_lookup(c: c_int) -> c_int {
    unsafe {
        let table = *__ctype_toupper_loc();
        *table.offset(c as isize)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    // C's char-to-int promotion is sign-extending on platforms (like x86_64
    // Linux) where char is signed. `c_char as c_int` matches that.
    let ci: c_int = c as c_int;

    unsafe {
        setlocale(LC_ALL, b"C\0".as_ptr() as *const c_char);

        // Each macro from glibc's <ctype.h> evaluates to
        // `((*__ctype_b_loc())[(int)(c)] & (unsigned short int) <mask>)`,
        // promoted to int when used as a printf argument. The result is the
        // masked-off bit (or zero), not 0/1.
        let bits = ctype_b_lookup(ci);

        printf(
            b"alphanumeric: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_ALNUM) as c_int,
        );
        printf(
            b"alphabetic: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_ALPHA) as c_int,
        );
        printf(
            b"lowercase: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_LOWER) as c_int,
        );
        printf(
            b"uppercase: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_UPPER) as c_int,
        );
        printf(
            b"digit: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_DIGIT) as c_int,
        );
        printf(
            b"hexadecimal: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_XDIGIT) as c_int,
        );
        printf(
            b"control: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_CNTRL) as c_int,
        );
        printf(
            b"graphical: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_GRAPH) as c_int,
        );
        printf(
            b"space: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_SPACE) as c_int,
        );
        printf(
            b"blank: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_BLANK) as c_int,
        );
        printf(
            b"printing: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_PRINT) as c_int,
        );
        printf(
            b"punctuation: %d\n\0".as_ptr() as *const c_char,
            (bits & _IS_PUNCT) as c_int,
        );
        printf(
            b"to lower: %c\n\0".as_ptr() as *const c_char,
            ctype_tolower_lookup(ci),
        );
        printf(
            b"to upper: %c\n\0".as_ptr() as *const c_char,
            ctype_toupper_lookup(ci),
        );
    }
}
