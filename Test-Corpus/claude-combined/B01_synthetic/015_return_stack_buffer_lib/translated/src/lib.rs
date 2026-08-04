// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/driver.c to Rust.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Equivalent of:
///     void printLine(const char *line)
///     {
///         if (line != NULL)
///         {
///             printf("%s\n", line);
///         }
///     }
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Re-use libc printf so the output goes to the same FILE* as the C
        // implementation (so byte-identical output is guaranteed when mixing
        // both libraries in the same process).
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

/// Equivalent of:
///     static char *helperBad()
///     {
///         char charString[] = "helperBad string";
///         return charString;
///     }
///
/// This is undefined behavior in C (returning the address of a local stack
/// buffer). GCC observably compiles this function to simply return NULL (0),
/// emitting a `-Wreturn-local-addr` warning. To produce byte-identical output
/// to the C library, we faithfully reproduce that observable result by
/// returning a null pointer here.
#[inline(never)]
fn helper_bad() -> *mut c_char {
    // Mirror the original local buffer initialization for completeness, even
    // though GCC discards the dangling pointer and returns NULL.
    let _char_string: [c_char; 17] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'B' as c_char,
        b'a' as c_char,
        b'd' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    std::ptr::null_mut()
}

/// Equivalent of:
///     void bad()
///     {
///         printLine(helperBad());
///     }
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let p = helper_bad();
    printLine(p);
}

/// Equivalent of:
///     static char *helperGood1()
///     {
///         static char charString[] = "helperGood1 string";
///         return charString;
///     }
fn helper_good1() -> *mut c_char {
    // Mutable static so we have a real, addressable, statically-allocated
    // buffer (matching `static char charString[]` in C).
    static mut CHAR_STRING: [c_char; 19] = [
        b'h' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'p' as c_char,
        b'e' as c_char,
        b'r' as c_char,
        b'G' as c_char,
        b'o' as c_char,
        b'o' as c_char,
        b'd' as c_char,
        b'1' as c_char,
        b' ' as c_char,
        b's' as c_char,
        b't' as c_char,
        b'r' as c_char,
        b'i' as c_char,
        b'n' as c_char,
        b'g' as c_char,
        0,
    ];
    (&raw mut CHAR_STRING) as *mut c_char
}

/// Equivalent of:
///     void good()
///     {
///         printLine(helperGood1());
///     }
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let p = helper_good1();
    printLine(p);
}

/// Equivalent of:
///     void driver(int useGood)
///     {
///         if (useGood) { good(); } else { bad(); }
///     }
#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

// Suppress unused warning for the CStr import in case it's not directly needed.
#[allow(dead_code)]
fn _unused_import_anchor() -> Option<&'static CStr> {
    None
}
