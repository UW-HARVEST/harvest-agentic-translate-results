use std::ffi::c_void;
use std::os::raw::c_char;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Create an array of pointers to the lines in a buffer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    // malloc(numLines * sizeof(const char**))
    let alloc_size = num_lines.wrapping_mul(std::mem::size_of::<*const *const c_char>());
    let buffer_ptrs = malloc(alloc_size);
    let line_pointers = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return std::ptr::null();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer
        while (pos + len) < buffer_size && *buffer.add(pos + len) != 0 {
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
        free(buffer_ptrs);
        return std::ptr::null();
    }

    line_pointers as *const *const c_char
}
