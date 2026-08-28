//! Rust translation of `c_src/src/lib.c`.
//!
//! The single exported function builds an array of pointers to the
//! NUL-terminated lines packed inside a caller-owned buffer.
//!
//! The returned array is allocated with libc's `malloc` (and released with
//! libc's `free` on the failure path) because the C contract requires the
//! caller to release it with `free`.

use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Create an array of pointers to the lines in a buffer.
///
/// Faithful translation of the original C, including its quirks:
/// * `numLines * sizeof(const char**)` is computed with wrapping arithmetic,
///   exactly as C `size_t` multiplication does.
/// * The NULL check on the allocation happens after the (harmless) cast.
/// * A zero-size request is forwarded to `malloc` as-is, so `numLines == 0`
///   returns whatever unique pointer the allocator hands back.
///
/// # Safety
///
/// `buffer` must point to at least `bufferSize` readable bytes (the pointer is
/// only offset and compared, never dereferenced here), and the returned array
/// must be released with `free`.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    numLines: usize,
    bufferSize: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    // C: malloc(numLines * sizeof(const char**)) -- wrapping, like size_t math.
    let buffer_ptrs: *mut c_void =
        unsafe { malloc(numLines.wrapping_mul(size_of::<*const *const c_char>())) };
    let line_pointers = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return std::ptr::null();
    }

    while line_index < numLines && pos < bufferSize {
        let mut len: usize = 0;
        unsafe {
            *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer.
        while (pos + len < bufferSize) && unsafe { *buffer.add(pos + len) } != 0 {
            len += 1;
        }

        // Move past this string and its null terminator.
        pos += len;
        if pos < bufferSize {
            pos += 1; // Skip the null terminator if we're not at buffer end.
        }
    }

    // Verify we processed the expected number of lines.
    if line_index != numLines {
        // Something went wrong - we didn't find as many lines as expected.
        unsafe { free(buffer_ptrs) };
        return std::ptr::null();
    }

    line_pointers as *const *const c_char
}
