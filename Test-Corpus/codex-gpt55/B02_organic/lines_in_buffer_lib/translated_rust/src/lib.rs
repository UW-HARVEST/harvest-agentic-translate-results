use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;
    let buffer_ptrs = unsafe {
        libc::malloc(num_lines.wrapping_mul(std::mem::size_of::<*const *const c_char>()))
    };
    let line_pointers = buffer_ptrs as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return std::ptr::null();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        unsafe {
            *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index = line_index.wrapping_add(1);

        while pos.wrapping_add(len) < buffer_size
            && unsafe { *buffer.add(pos.wrapping_add(len)) } != 0
        {
            len = len.wrapping_add(1);
        }

        pos = pos.wrapping_add(len);
        if pos < buffer_size {
            pos = pos.wrapping_add(1);
        }
    }

    if line_index != num_lines {
        unsafe {
            libc::free(buffer_ptrs);
        }
        return std::ptr::null();
    }

    line_pointers as *const *const c_char
}
