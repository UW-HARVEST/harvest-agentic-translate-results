use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char {
    let buffer_ptrs = unsafe { malloc(num_lines.wrapping_mul(size_of::<*const *const c_char>())) };
    let line_pointers = buffer_ptrs.cast::<*const c_char>();
    if buffer_ptrs.is_null() {
        return ptr::null_mut();
    }

    let mut line_index = 0usize;
    let mut pos = 0usize;
    while line_index < num_lines && pos < buffer_size {
        let mut len = 0usize;
        unsafe {
            line_pointers
                .add(line_index)
                .write(buffer.add(pos).cast_const());
        }
        line_index += 1;

        while pos.wrapping_add(len) < buffer_size
            && unsafe { *buffer.add(pos.wrapping_add(len)) != 0 }
        {
            len += 1;
        }

        pos = pos.wrapping_add(len);
        if pos < buffer_size {
            pos += 1;
        }
    }

    if line_index != num_lines {
        unsafe {
            free(buffer_ptrs);
        }
        return ptr::null_mut();
    }

    line_pointers
}
