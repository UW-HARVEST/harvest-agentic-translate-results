use std::ffi::c_char;
use std::mem;
use std::ptr;

/// Create an array of pointers to the lines in a buffer.
///
/// This is a translation of the C function:
///     const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
///
/// The returned pointer is allocated via libc::malloc so it is compatible with
/// the caller using free() to release it (matching the original C behavior).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: libc::size_t,
    buffer_size: libc::size_t,
) -> *mut *const c_char {
    let mut line_index: libc::size_t = 0;
    let mut pos: libc::size_t = 0;

    let alloc_size = num_lines.wrapping_mul(mem::size_of::<*const *const c_char>());
    let buffer_ptrs = unsafe { libc::malloc(alloc_size) };
    let line_pointers = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return ptr::null_mut();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len: libc::size_t = 0;
        unsafe {
            *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer
        while (pos + len) < buffer_size
            && unsafe { *buffer.add(pos + len) } != 0
        {
            len += 1;
        }

        // Move past this string and its null terminator
        pos += len;
        if pos < buffer_size {
            pos += 1; // Skip the null terminator if we're not at buffer end
        }
    }

    // Verify we processed the expected number of lines
    if line_index != num_lines {
        // Something went wrong - we didn't find as many lines as expected
        unsafe { libc::free(buffer_ptrs) };
        return ptr::null_mut();
    }

    line_pointers
}
