use std::ffi::c_char;
use std::os::raw::c_ulong;
use std::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: c_ulong,
    buffer_size: c_ulong,
) -> *const *const c_char {
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    let layout = match std::alloc::Layout::from_size_align(
        (num_lines as usize) * std::mem::size_of::<*const *const c_char>(),
        std::mem::align_of::<*const c_char>(),
    ) {
        Ok(l) => l,
        Err(_) => return ptr::null(),
    };

    // Match C: malloc(numLines * sizeof(const char**))
    let buffer_ptrs = if layout.size() == 0 {
        return ptr::null();
    } else {
        unsafe { std::alloc::alloc(layout) }
    };
    if buffer_ptrs.is_null() {
        return ptr::null();
    }
    let line_pointers = buffer_ptrs as *mut *const c_char;

    let num_lines = num_lines as usize;
    let buffer_size = buffer_size as usize;

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        unsafe {
            *line_pointers.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index += 1;

        while (pos + len < buffer_size) && unsafe { *buffer.add(pos + len) } != 0 {
            len += 1;
        }

        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }

    if line_index != num_lines {
        unsafe { std::alloc::dealloc(buffer_ptrs, layout) };
        return ptr::null();
    }

    line_pointers as *const *const c_char
}
