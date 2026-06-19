use libc::{free, malloc};
use std::ffi::c_char;
use std::mem::size_of;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char {
    let mut line_index = 0usize;
    let mut pos = 0usize;
    let buffer_ptrs = malloc(num_lines.wrapping_mul(size_of::<*const *const c_char>()));
    let line_pointers = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return std::ptr::null_mut();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len = 0usize;
        line_pointers.add(line_index).write(buffer.add(pos) as *const c_char);
        line_index += 1;

        while pos + len < buffer_size && *buffer.add(pos + len) != 0 {
            len += 1;
        }

        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }

    if line_index != num_lines {
        free(buffer_ptrs);
        return std::ptr::null_mut();
    }

    line_pointers
}
