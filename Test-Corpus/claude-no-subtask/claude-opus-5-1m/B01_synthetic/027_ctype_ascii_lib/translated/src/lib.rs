// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical stdout output by
// delegating to libc's printf and character-classification routines, which
// match the original C implementation's behavior (including the
// implementation-specific non-zero return values from is*() functions).

use std::ffi::c_char;
use std::os::raw::c_int;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    fn printf(format: *const c_char, ...) -> c_int;

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
}

// LC_ALL value on glibc-based Linux systems is 6.
// This matches the C source which simply passes LC_ALL.
#[cfg(target_os = "linux")]
const LC_ALL: c_int = 6;
#[cfg(target_os = "macos")]
const LC_ALL: c_int = 0;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const LC_ALL: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        let c_locale = b"C\0".as_ptr() as *const c_char;
        setlocale(LC_ALL, c_locale);

        // The C code passes a `char` directly to is*()/tolower()/toupper(),
        // which is an int-promotion. On platforms where char is signed, this
        // can yield negative values for bytes >= 0x80, producing
        // implementation-defined behavior identical to the C original.
        let ci: c_int = c as c_int;

        let fmt_d = b"%s: %d\n\0".as_ptr() as *const c_char;
        let fmt_c = b"%s: %c\n\0".as_ptr() as *const c_char;

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

        // Suppress unused-variable warnings for the (intentionally unused) format placeholders.
        let _ = fmt_d;
        let _ = fmt_c;
    }
}
