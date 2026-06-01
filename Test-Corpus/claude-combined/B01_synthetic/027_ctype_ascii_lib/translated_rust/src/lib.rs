// Copyright 2025 MIT Lincoln Laboratory
// Translation of driver.c to Rust.
//
// IMPORTANT: glibc's <ctype.h> defines isalnum/isalpha/etc. as macros that
// expand inline to a lookup-table indexed bitmask AND, returning the raw
// bitmask value (e.g. 2048) rather than the 1/0 returned by the library
// function symbols. tolower/toupper are also expanded inline as table
// lookups. Since the C source uses these via the headers, we must reproduce
// the macro expansion exactly via __ctype_b_loc / __ctype_tolower_loc /
// __ctype_toupper_loc to match the C output byte-for-byte.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;

    fn __ctype_b_loc() -> *mut *const u16;
    fn __ctype_tolower_loc() -> *mut *const i32;
    fn __ctype_toupper_loc() -> *mut *const i32;
}

// LC_ALL is 6 on glibc.
const LC_ALL: c_int = 6;

// Bitmask values matching glibc's _ISbit on little-endian:
//   bit < 8  ->  (1 << bit) << 8
//   bit >= 8 ->  (1 << bit) >> 8
const IS_UPPER: u16 = 1 << 8; // _ISbit(0)
const IS_LOWER: u16 = 1 << 9; // _ISbit(1)
const IS_ALPHA: u16 = 1 << 10; // _ISbit(2)
const IS_DIGIT: u16 = 1 << 11; // _ISbit(3)
const IS_XDIGIT: u16 = 1 << 12; // _ISbit(4)
const IS_SPACE: u16 = 1 << 13; // _ISbit(5)
const IS_PRINT: u16 = 1 << 14; // _ISbit(6)
const IS_GRAPH: u16 = 1 << 15; // _ISbit(7)
const IS_BLANK: u16 = 1 << 0; // _ISbit(8)
const IS_CNTRL: u16 = 1 << 1; // _ISbit(9)
const IS_PUNCT: u16 = 1 << 2; // _ISbit(10)
const IS_ALNUM: u16 = 1 << 3; // _ISbit(11)

#[inline]
unsafe fn isctype(c: c_int, mask: u16) -> c_int {
    // Mirrors `(*__ctype_b_loc ())[(int) c] & (unsigned short)mask`.
    // The table is indexed from -128..=255; the pointer returned by
    // __ctype_b_loc is positioned so that negative indices work.
    let table = *__ctype_b_loc();
    let val = *table.offset(c as isize);
    (val & mask) as c_int
}

#[inline]
unsafe fn rust_tolower(c: c_int) -> c_int {
    if c >= -128 && c < 256 {
        let table = *__ctype_tolower_loc();
        *table.offset(c as isize)
    } else {
        c
    }
}

#[inline]
unsafe fn rust_toupper(c: c_int) -> c_int {
    if c >= -128 && c < 256 {
        let table = *__ctype_toupper_loc();
        *table.offset(c as isize)
    } else {
        c
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        let locale = CString::new("C").unwrap();
        setlocale(LC_ALL, locale.as_ptr());

        // The C source passes a signed `char c` to int-taking functions, so
        // the value is sign-extended (matters for bytes >= 128).
        let cv: c_int = c as c_int;

        let fmt = CString::new("alphanumeric: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_ALNUM));

        let fmt = CString::new("alphabetic: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_ALPHA));

        let fmt = CString::new("lowercase: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_LOWER));

        let fmt = CString::new("uppercase: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_UPPER));

        let fmt = CString::new("digit: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_DIGIT));

        let fmt = CString::new("hexadecimal: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_XDIGIT));

        let fmt = CString::new("control: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_CNTRL));

        let fmt = CString::new("graphical: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_GRAPH));

        let fmt = CString::new("space: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_SPACE));

        let fmt = CString::new("blank: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_BLANK));

        let fmt = CString::new("printing: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_PRINT));

        let fmt = CString::new("punctuation: %d\n").unwrap();
        printf(fmt.as_ptr(), isctype(cv, IS_PUNCT));

        let fmt = CString::new("to lower: %c\n").unwrap();
        printf(fmt.as_ptr(), rust_tolower(cv));

        let fmt = CString::new("to upper: %c\n").unwrap();
        printf(fmt.as_ptr(), rust_toupper(cv));
    }
}
