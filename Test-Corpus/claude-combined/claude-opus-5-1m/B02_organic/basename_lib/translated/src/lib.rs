use std::ffi::c_char;

/// Find last occurrence of byte `c` (as i32) in null-terminated C string starting at `s`.
/// Returns null if not found, otherwise pointer to the matching byte.
/// Mirrors the C strrchr semantics, including matching the terminating null byte
/// when `c == 0`.
unsafe fn strrchr(s: *const c_char, c: i32) -> *const c_char {
    let needle = c as u8 as c_char;
    let mut last: *const c_char = std::ptr::null();
    let mut p = s;
    loop {
        let ch = unsafe { *p };
        if ch == needle {
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
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let s1 = unsafe { strrchr(path, '/' as i32) } as *mut c_char;
    let s2 = unsafe { strrchr(path, '\\' as i32) } as *mut c_char;

    let mut result = path;

    if !s1.is_null() && !s2.is_null() {
        result = if s1 > s2 {
            unsafe { s1.add(1) }
        } else {
            unsafe { s2.add(1) }
        };
    } else if !s1.is_null() {
        result = unsafe { s1.add(1) };
    } else if !s2.is_null() {
        result = unsafe { s2.add(1) };
    }

    result
}
