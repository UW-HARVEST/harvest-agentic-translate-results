//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI mirrored from `c_src/include/lib.h`:
//!   const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
//!
//! Implementation notes / fidelity constraints:
//!  * The C code allocates with `malloc` and releases with `free` on the failure
//!    path. We call the very same libc entry points rather than Rust's global
//!    allocator so that observable behaviour matches exactly -- in particular
//!    `malloc(0)` on glibc returns a unique NON-NULL pointer, which the C code
//!    happily returns when `numLines == 0`. Rust's `alloc` would be UB for a
//!    zero-sized layout, and its returned pointers could not be `free`d by a
//!    caller of the C library.
//!  * `numLines * sizeof(const char**)` is unsigned `size_t` arithmetic in C and
//!    therefore wraps modulo 2^64 on overflow. `wrapping_mul` reproduces that,
//!    including the resulting undersized allocation and out-of-bounds stores
//!    (a latent bug in the original that we deliberately preserve).
//!  * The order of operations is preserved verbatim: the allocation result is
//!    cast to `const char**` *before* the NULL check, and the NULL check happens
//!    after that cast, exactly as in the C source.

use std::ffi::c_void;
use std::os::raw::c_char;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Create an array of pointers to the lines in a buffer.
///
/// Direct translation of `UTIL_createLinePointers` from `c_src/src/lib.c`.
///
/// # Safety
///
/// Same contract (and same lack of one) as the C original: `buffer` must be
/// readable for `bufferSize` bytes whenever the scanning loop runs, and the
/// returned array is owned by the caller and must be released with `free`.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    numLines: usize,
    bufferSize: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    // void* const bufferPtrs = malloc(numLines * sizeof(const char**));
    let buffer_ptrs: *mut c_void =
        malloc(numLines.wrapping_mul(std::mem::size_of::<*const *const c_char>()));
    // const char** const linePointers = (const char**)bufferPtrs;
    let line_pointers: *mut *const c_char = buffer_ptrs as *mut *const c_char;
    // if (bufferPtrs == NULL) return NULL;
    if buffer_ptrs.is_null() {
        return std::ptr::null();
    }

    while line_index < numLines && pos < bufferSize {
        let mut len: usize = 0;

        // linePointers[lineIndex++] = buffer+pos;
        *line_pointers.add(line_index) = buffer.wrapping_add(pos) as *const c_char;
        line_index += 1;

        /* Find the next null terminator, being careful not to go past the buffer */
        while (pos + len < bufferSize) && *buffer.wrapping_add(pos + len) != 0 {
            len += 1;
        }

        /* Move past this string and its null terminator */
        pos += len;
        if pos < bufferSize {
            pos += 1; /* Skip the null terminator if we're not at buffer end */
        }
    }

    /* Verify we processed the expected number of lines */
    if line_index != numLines {
        /* Something went wrong - we didn't find as many lines as expected */
        free(buffer_ptrs);
        return std::ptr::null();
    }

    line_pointers as *const *const c_char
}
