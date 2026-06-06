use std::ffi::c_char;

/// Equivalent of C's strrchr: find last occurrence of `c` in null-terminated string `s`.
/// Returns null pointer if not found.
unsafe fn strrchr(s: *const c_char, c: c_char) -> *mut c_char {
    let mut last: *mut c_char = std::ptr::null_mut();
    let mut p = s as *mut c_char;
    loop {
        let ch = unsafe { *p };
        if ch == c {
            last = p;
        }
        if ch == 0 {
            break;
        }
        p = unsafe { p.add(1) };
    }
    last
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(mut path: *mut c_char) -> *mut c_char {
    let s1 = unsafe { strrchr(path, b'/' as c_char) };
    let s2 = unsafe { strrchr(path, b'\\' as c_char) };

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
