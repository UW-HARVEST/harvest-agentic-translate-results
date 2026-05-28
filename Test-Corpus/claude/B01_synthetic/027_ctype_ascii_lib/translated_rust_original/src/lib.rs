// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of c_src/src/driver.c
//
// To produce byte-identical output to the original C program (which uses
// glibc's `printf`, `setlocale`, and ctype functions whose nonzero return
// values encode implementation-specific bitmasks), this translation calls
// directly into libc.

use std::ffi::c_char;
use std::ffi::c_int;

// LC_ALL is defined in glibc's <locale.h>. On Linux/glibc its value is 6.
const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isblank(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;

    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    // The C code passes `c` (a char) as an `int` argument to the ctype
    // functions. In C, the char-to-int promotion sign-extends or zero-extends
    // depending on whether `char` is signed; on the typical x86_64 Linux
    // target where this library is built, `char` is signed. We mirror that
    // by going through `c_char` -> `c_int` which is a sign-extending cast.
    let ci: c_int = c as c_int;

    unsafe {
        setlocale(LC_ALL, b"C\0".as_ptr() as *const c_char);

        printf(b"alphanumeric: %d\n\0".as_ptr() as *const c_char, isalnum(ci));
        printf(b"alphabetic: %d\n\0".as_ptr() as *const c_char, isalpha(ci));
        printf(b"lowercase: %d\n\0".as_ptr() as *const c_char, islower(ci));
        printf(b"uppercase: %d\n\0".as_ptr() as *const c_char, isupper(ci));
        printf(b"digit: %d\n\0".as_ptr() as *const c_char, isdigit(ci));
        printf(b"hexadecimal: %d\n\0".as_ptr() as *const c_char, isxdigit(ci));
        printf(b"control: %d\n\0".as_ptr() as *const c_char, iscntrl(ci));
        printf(b"graphical: %d\n\0".as_ptr() as *const c_char, isgraph(ci));
        printf(b"space: %d\n\0".as_ptr() as *const c_char, isspace(ci));
        printf(b"blank: %d\n\0".as_ptr() as *const c_char, isblank(ci));
        printf(b"printing: %d\n\0".as_ptr() as *const c_char, isprint(ci));
        printf(b"punctuation: %d\n\0".as_ptr() as *const c_char, ispunct(ci));
        printf(b"to lower: %c\n\0".as_ptr() as *const c_char, tolower(ci));
        printf(b"to upper: %c\n\0".as_ptr() as *const c_char, toupper(ci));
    }
}
