use std::ffi::c_char;

/// Find the last occurrence of `needle` byte in the C string starting at `s`.
/// Returns null if not found, otherwise pointer to that byte.
/// Mirrors the semantics of C's `strrchr`.
unsafe fn strrchr(s: *mut c_char, needle: c_char) -> *mut c_char {
    let mut last: *mut c_char = std::ptr::null_mut();
    let mut p = s;
    loop {
        let c = unsafe { *p };
        if c == needle {
            last = p;
        }
        if c == 0 {
            return last;
        }
        p = unsafe { p.add(1) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(mut path: *mut c_char) -> *mut c_char {
    let s1: *mut c_char = unsafe { strrchr(path, b'/' as c_char) };
    let s2: *mut c_char = unsafe { strrchr(path, b'\\' as c_char) };

    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 {
            unsafe { s1.add(1) }
        } else {
            unsafe { s2.add(1) }
        };
    } else if !s1.is_null() {
        path = unsafe { s1.add(1) };
    } else if !s2.is_null() {
        path = unsafe { s2.add(1) };
    }

    path
}
