use std::ffi::c_char;

/// Equivalent to C's `strrchr(s, c)`: return pointer to last occurrence of byte
/// `c` in the NUL-terminated string `s`, or NULL if not found.
unsafe fn strrchr(s: *const c_char, c: u8) -> *mut c_char {
    let mut p = s;
    let mut last: *const c_char = std::ptr::null();
    loop {
        let ch = *p as u8;
        if ch == c {
            last = p;
        }
        if ch == 0 {
            break;
        }
        p = p.add(1);
    }
    last as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut path = path;

    let s1 = strrchr(path, b'/');
    let s2 = strrchr(path, b'\\');

    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 { s1.add(1) } else { s2.add(1) };
    } else if !s1.is_null() {
        path = s1.add(1);
    } else if !s2.is_null() {
        path = s2.add(1);
    }

    path
}
