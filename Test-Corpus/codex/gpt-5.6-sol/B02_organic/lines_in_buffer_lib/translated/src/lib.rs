use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

/// Creates an allocated array of pointers to the null-separated lines in `buffer`.
///
/// The returned array is allocated by C's `malloc` and must be released with `free`.
///
/// # Safety
///
/// `buffer` must point to at least `buffer_size` readable bytes and must remain valid
/// while the returned line pointers are used.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char {
    let allocation_size = num_lines.wrapping_mul(size_of::<*const *const c_char>());
    let buffer_ptrs = unsafe { malloc(allocation_size) };
    if buffer_ptrs.is_null() {
        return ptr::null_mut();
    }

    let line_pointers = buffer_ptrs.cast::<*const c_char>();
    let mut line_index = 0usize;
    let mut pos = 0usize;

    while line_index < num_lines && pos < buffer_size {
        unsafe {
            line_pointers
                .add(line_index)
                .write(buffer.add(pos).cast_const());
        }
        line_index = line_index.wrapping_add(1);

        let mut len = 0usize;
        while pos.wrapping_add(len) < buffer_size
            && unsafe { *buffer.add(pos.wrapping_add(len)) != 0 }
        {
            len = len.wrapping_add(1);
        }

        pos = pos.wrapping_add(len);
        if pos < buffer_size {
            pos = pos.wrapping_add(1);
        }
    }

    if line_index != num_lines {
        unsafe { free(buffer_ptrs) };
        return ptr::null_mut();
    }

    line_pointers
}
