// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/driver.c to Rust.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn memset(s: *mut core::ffi::c_void, c: c_int, n: usize) -> *mut core::ffi::c_void;
}

/// Equivalent to the C `printLine` function:
/// ```c
/// void printLine (const char * line)
/// {
///     if(line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // Use libc's printf to produce byte-identical output to the C version.
        // The format string "%s\n" is null-terminated.
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Equivalent to the C `driver` function:
/// ```c
/// void driver(int data)
/// {
///     char source[100];
///     char dest[100] = "";
///     memset(source, 'A', 100-1);
///     source[100-1] = '\0';
///     if (data < 100)
///     {
///         strncpy(dest, source, data);
///         dest[data] = '\0';
///     }
///     printLine(dest);
/// }
/// ```
///
/// NOTE: The original C code has well-known buffer-handling issues
/// (e.g. when `data` is negative or close to 100, `dest[data] = '\0'` may
/// access out-of-bounds memory; when `data` is exactly 99, the destination
/// is not necessarily null terminated by `strncpy`, but `dest[data] = '\0'`
/// fixes that).  Per the translation requirements, we reproduce the exact
/// behavior of the C code rather than fixing these bugs.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    // char source[100];
    let mut source: [c_char; 100] = [0; 100];
    // char dest[100] = "";  -- in C, this zero-initializes the entire array.
    let mut dest: [c_char; 100] = [0; 100];

    // memset(source, 'A', 100-1);
    unsafe {
        memset(
            source.as_mut_ptr() as *mut core::ffi::c_void,
            b'A' as c_int,
            100 - 1,
        );
    }
    // source[100-1] = '\0';
    source[100 - 1] = 0;

    if data < 100 {
        // strncpy(dest, source, data);
        // In C, the third argument is size_t (unsigned). If `data` is negative,
        // it will be interpreted as a very large size_t. We mirror that exactly
        // by casting through usize via the C-equivalent sign-extending cast.
        // c_int -> isize -> usize preserves the bit pattern after sign extension,
        // matching what the C compiler emits for `(size_t)data`.
        let n: usize = data as isize as usize;
        unsafe {
            strncpy(dest.as_mut_ptr(), source.as_ptr(), n);
            // dest[data] = '\0';
            // In C, indexing with a negative int is UB; we mirror by using
            // pointer arithmetic with the (sign-extended) integer offset.
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    // printLine(dest);
    printLine(dest.as_ptr());
}
