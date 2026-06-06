// Translation of c_src/src/driver.c to Rust.
// Preserves the exact behavior of the original C code, including any
// bugs/UB present in the original.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memset(s: *mut core::ffi::c_void, c: c_int, n: usize) -> *mut core::ffi::c_void;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

/// Direct translation of `printLine` from driver.c.
///
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
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // The C source uses the format string "%s\n".
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf(fmt, line);
    }
}

/// Direct translation of `driver` from driver.c.
///
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    // `char source[100];` (uninitialized in C) and `char dest[100] = "";`
    // (zero-initialized in C).
    let mut source: [c_char; 100] = [0; 100];
    let mut dest: [c_char; 100] = [0; 100];

    // memset(source, 'A', 100-1);
    memset(
        source.as_mut_ptr() as *mut core::ffi::c_void,
        b'A' as c_int,
        100 - 1,
    );
    // source[100-1] = '\0';
    source[100 - 1] = 0;

    if data < 100 {
        // strncpy(dest, source, data);
        // Note: `data` is a signed int. If it is negative the C code passes it
        // as a size_t which produces a very large unsigned value (UB-prone).
        // We reproduce the same conversion here using `as usize`.
        strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
        // dest[data] = '\0';
        // Reproduce the same indexing semantics as the C code.
        *dest.as_mut_ptr().offset(data as isize) = 0;
    }

    printLine(dest.as_ptr());
}
