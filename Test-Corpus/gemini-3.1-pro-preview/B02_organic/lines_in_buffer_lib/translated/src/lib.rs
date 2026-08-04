use std::os::raw::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char {
    let bytes = match num_lines.checked_mul(std::mem::size_of::<*const c_char>()) {
        Some(b) => b,
        None => return ptr::null_mut(),
    };

    if buffer.is_null() && buffer_size > 0 {
        return ptr::null_mut();
    }

    let buffer_slice = if buffer_size > 0 {
        unsafe { std::slice::from_raw_parts(buffer as *const u8, buffer_size) }
    } else {
        &[]
    };

    let mut line_pointers = Vec::new();
    if line_pointers.try_reserve(num_lines).is_err() {
        return ptr::null_mut();
    }

    let mut pos = 0;

    while line_pointers.len() < num_lines && pos < buffer_size {
        line_pointers.push(unsafe { buffer.add(pos) } as *const c_char);

        let mut len = 0;
        while pos + len < buffer_size && buffer_slice[pos + len] != 0 {
            len += 1;
        }

        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }

    if line_pointers.len() != num_lines {
        return ptr::null_mut();
    }

    unsafe {
        let ptr = libc::malloc(bytes) as *mut *const c_char;
        if !ptr.is_null() {
            ptr::copy_nonoverlapping(line_pointers.as_ptr(), ptr, num_lines);
        }
        ptr
    }
}
