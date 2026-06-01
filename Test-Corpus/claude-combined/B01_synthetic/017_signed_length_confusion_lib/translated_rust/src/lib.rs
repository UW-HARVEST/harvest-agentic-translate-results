// Translated from c_src/src/driver.c
// Preserves exact behavior of original C, including use of stdio printf
// so output is byte-identical.

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memset(s: *mut core::ffi::c_void, c: c_int, n: usize) -> *mut core::ffi::c_void;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf(fmt, line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
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
        strncpy(
            dest.as_mut_ptr(),
            source.as_ptr(),
            data as usize,
        );
        // dest[data] = '\0';
        // Note: matches C behavior; if data is negative, this is UB in C
        // and will likely crash in Rust as well.
        let idx = data as isize;
        *dest.as_mut_ptr().offset(idx) = 0;
    }

    printLine(dest.as_ptr());
}
