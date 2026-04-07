use std::ffi::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    // C code uses: malloc(numLines * sizeof(const char**))
    // Use libc malloc directly to match C behavior (e.g. malloc(0) returns non-NULL on glibc)
    extern "C" {
        fn malloc(size: usize) -> *mut std::ffi::c_void;
        fn free(ptr: *mut std::ffi::c_void);
    }
    let buf_ptrs =
        unsafe { malloc(num_lines * std::mem::size_of::<*const c_char>()) } as *mut *const c_char;
    if buf_ptrs.is_null() {
        return ptr::null();
    }

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        unsafe {
            *buf_ptrs.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index += 1;

        while pos + len < buffer_size && unsafe { *buffer.add(pos + len) } != 0 {
            len += 1;
        }

        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }

    if line_index != num_lines {
        unsafe { free(buf_ptrs as *mut std::ffi::c_void) };
        return ptr::null();
    }

    buf_ptrs as *const *const c_char
}
