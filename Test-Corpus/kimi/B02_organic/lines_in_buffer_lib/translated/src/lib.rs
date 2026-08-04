use std::ffi::{c_char, c_void};
use std::os::raw::c_size_t;
use std::alloc::{alloc, dealloc, Layout};

#[unsafe(no_mangle)]
pub extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    numLines: c_size_t,
    bufferSize: c_size_t,
) -> *mut *const c_char {
    if buffer.is_null() || numLines == 0 {
        return std::ptr::null_mut();
    }

    let layout = match Layout::array::<*const c_char>(numLines) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    let bufferPtrs = unsafe { alloc(layout) } as *mut *const c_char;
    if bufferPtrs.is_null() {
        return std::ptr::null_mut();
    }

    let mut lineIndex: usize = 0;
    let mut pos: usize = 0;

    unsafe {
        while lineIndex < numLines && pos < bufferSize {
            *bufferPtrs.add(lineIndex) = buffer.add(pos);
            lineIndex += 1;

            let mut len: usize = 0;
            while pos + len < bufferSize && *buffer.add(pos + len) != 0 {
                len += 1;
            }

            pos += len;
            if pos < bufferSize {
                pos += 1;
            }
        }
    }

    if lineIndex != numLines {
        unsafe {
            dealloc(bufferPtrs as *mut u8, layout);
        }
        return std::ptr::null_mut();
    }

    bufferPtrs
}
