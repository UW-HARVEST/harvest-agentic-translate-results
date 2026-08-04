use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let s1 = unsafe { strrchr(path.cast_const(), b'/' as c_int) };
    let s2 = unsafe { strrchr(path.cast_const(), b'\\' as c_int) };

    if !s1.is_null() && !s2.is_null() {
        if s1 > s2 {
            unsafe { s1.add(1) }
        } else {
            unsafe { s2.add(1) }
        }
    } else if !s1.is_null() {
        unsafe { s1.add(1) }
    } else if !s2.is_null() {
        unsafe { s2.add(1) }
    } else {
        path
    }
}
