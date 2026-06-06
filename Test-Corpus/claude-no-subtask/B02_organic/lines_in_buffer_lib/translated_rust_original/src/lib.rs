#![allow(non_snake_case)]

use core::ffi::c_char;
use core::ptr;

/// Create an array of pointers to the lines in a buffer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    numLines: usize,
    bufferSize: usize,
) -> *const *const c_char {
    // Allocate numLines * sizeof(const char**) bytes via libc::malloc to match C behavior.
    let alloc_size = numLines.wrapping_mul(core::mem::size_of::<*const *const c_char>());
    let buffer_ptrs = libc::malloc(alloc_size) as *mut *const c_char;
    let line_pointers = buffer_ptrs;
    if buffer_ptrs.is_null() {
        return ptr::null();
    }

    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    while line_index < numLines && pos < bufferSize {
        let mut len: usize = 0;
        *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer
        while (pos + len) < bufferSize && *buffer.add(pos + len) != 0 {
            len += 1;
        }

        // Move past this string and its null terminator
        pos += len;
        if pos < bufferSize {
            pos += 1;
        }
    }

    // Verify we processed the expected number of lines
    if line_index != numLines {
        // Something went wrong - we didn't find as many lines as expected
        libc::free(buffer_ptrs as *mut core::ffi::c_void);
        return ptr::null();
    }

    line_pointers as *const *const c_char
}
