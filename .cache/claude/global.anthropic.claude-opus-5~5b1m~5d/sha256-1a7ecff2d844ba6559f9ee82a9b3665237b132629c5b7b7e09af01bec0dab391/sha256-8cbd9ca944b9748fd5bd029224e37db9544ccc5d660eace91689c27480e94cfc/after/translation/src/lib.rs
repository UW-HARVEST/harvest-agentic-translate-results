//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (must match the C `libdriver.so` exactly):
//!   const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
//!
//! The returned block is allocated with the C `malloc` (and released with the C
//! `free` on the failure path) so that callers can `free()` it exactly as they
//! do with the original C library.

use core::ffi::{c_char, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Create an array of pointers to the lines in a buffer.
///
/// Faithful translation of `UTIL_createLinePointers` from `c_src/src/lib.c`,
/// including its overflow behaviour (`numLines * sizeof(const char**)` is a
/// plain wrapping `size_t` multiplication in C) and its exact order of checks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    /* malloc(numLines * sizeof(const char**)) -- unsigned wraparound, as in C */
    let buffer_ptrs: *mut c_void =
        malloc(num_lines.wrapping_mul(core::mem::size_of::<*const c_char>()));
    let line_pointers: *mut *const c_char = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return core::ptr::null();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        line_pointers
            .add(line_index)
            .write(buffer.wrapping_add(pos) as *const c_char);
        line_index += 1;

        /* Find the next null terminator, being careful not to go past the buffer */
        while (pos + len < buffer_size) && buffer.wrapping_add(pos + len).read() != 0 {
            len += 1;
        }

        /* Move past this string and its null terminator */
        pos += len;
        if pos < buffer_size {
            pos += 1; /* Skip the null terminator if we're not at buffer end */
        }
    }

    /* Verify we processed the expected number of lines */
    if line_index != num_lines {
        /* Something went wrong - we didn't find as many lines as expected */
        free(buffer_ptrs);
        return core::ptr::null();
    }

    line_pointers as *const *const c_char
}
