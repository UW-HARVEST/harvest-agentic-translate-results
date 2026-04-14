use std::alloc::{alloc, Layout};
use std::ffi::c_char;
use std::os::raw::c_void;

#[unsafe(no_mangle)]
pub extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    numLines: usize,
    bufferSize: usize,
) -> *mut *const c_char {
    if numLines == 0 {
        return std::ptr::null_mut();
    }
    if buffer.is_null() {
        return std::ptr::null_mut();
    }

    let layout = match Layout::array::<*const c_char>(numLines) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };

    let buffer_ptrs = unsafe { alloc(layout) } as *mut *const c_char;
    if buffer_ptrs.is_null() {
        return std::ptr::null_mut();
    }

    let mut line_index = 0usize;
    let mut pos = 0usize;

    while line_index < numLines && pos < bufferSize {
        let mut len = 0usize;
        unsafe {
            *buffer_ptrs.add(line_index) = buffer.add(pos) as *const c_char;
        }
        line_index += 1;

        while pos + len < bufferSize {
            let byte = unsafe { *(buffer.add(pos + len) as *const u8) };
            if byte == 0 {
                break;
            }
            len += 1;
        }

        pos += len;
        if pos < bufferSize {
            pos += 1;
        }
    }

    if line_index != numLines {
        unsafe {
            std::alloc::dealloc(buffer_ptrs as *mut u8, layout);
        }
        return std::ptr::null_mut();
    }

    buffer_ptrs
}
