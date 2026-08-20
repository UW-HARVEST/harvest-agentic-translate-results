// Rust translation of the C library in c_src/ (MIT Lincoln Laboratory `driver`).
//
// The C library consists of a single translation unit (c_src/src/driver.c) with
// four external (non-`static`) functions, all of which are exported by the
// shared object:
//
//     void printLine(const char *line);
//     void bad(void);
//     void good(void);
//     void driver(int useGood);
//
// Only `driver` is declared in the public header (c_src/include/driver.h), but
// the C build exports all four symbols, so all four are reproduced here with
// their exact C linkage names and signatures.
//
// Output is produced by calling libc's `printf` directly (rather than Rust's
// `println!`) so that the FILE* stream, buffering behaviour and byte output are
// identical to the C library's.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    // int printf(const char *restrict format, ...);
    #[link_name = "printf"]
    unsafe fn libc_printf(format: *const c_char, ...) -> c_int;
}

/// Format string used by the C code: `printf("%s\n", line);`
const FMT_S_NL: &[u8] = b"%s\n\0";

/// The string literal assigned in the C `good()` function.
const GOOD_STRING: &[u8] = b"string\0";

/// C:
/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc_printf(FMT_S_NL.as_ptr() as *const c_char, line);
        }
    }
}

/// C:
/// ```c
/// void bad()
/// {
///     char *data;
///     printLine(data);
/// }
/// ```
///
/// `data` is never initialised in the C source: this is the intentional defect
/// (use of an uninitialised variable). The bug is *not* fixed here. Reading the
/// uninitialised object is undefined behaviour in C, and there is no way to
/// reproduce indeterminate stack residue deterministically. Every optimising
/// build of the original translation unit (gcc `-O1`, `-O2`, `-O3`, `-Os`)
/// resolves the uninitialised pointer to a null pointer and therefore calls
/// `printLine(NULL)`, which prints nothing; that observable behaviour is what
/// is reproduced. The defective call to `printLine` with an uninitialised
/// (null) pointer is preserved rather than removed or "corrected".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // char *data;  /* deliberately left uninitialised in the C original */
    let data: *const c_char = ptr::null();
    unsafe {
        printLine(data);
    }
}

/// C:
/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: *const c_char = GOOD_STRING.as_ptr() as *const c_char;
    unsafe {
        printLine(data);
    }
}

/// C:
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood)
///     {
///         good();
///     }
///     else
///     {
///         bad();
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    unsafe {
        if useGood != 0 {
            good();
        } else {
            bad();
        }
    }
}
