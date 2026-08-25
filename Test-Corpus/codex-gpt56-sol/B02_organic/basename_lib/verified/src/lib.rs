use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strrchr(string: *const c_char, character: c_int) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let slash = unsafe { strrchr(path, b'/' as c_int) };
    let backslash = unsafe { strrchr(path, b'\\' as c_int) };

    unsafe {
        if !slash.is_null() && !backslash.is_null() {
            if slash > backslash {
                slash.add(1)
            } else {
                backslash.add(1)
            }
        } else if !slash.is_null() {
            slash.add(1)
        } else if !backslash.is_null() {
            backslash.add(1)
        } else {
            path
        }
    }
}
